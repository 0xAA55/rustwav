#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fmt::Debug};

use crate::adpcm;
use crate::{AudioError, AudioReadError};
use crate::{Spec, WaveSampleType, FmtChunk};
use crate::{SampleType, i24, u24};
use crate::Reader;

// 解码器，解码出来的样本格式是 S
pub trait Decoder<S>: Debug
    where S: SampleType {
    fn decode(&mut self) -> Result<Option<Vec<S>>, AudioReadError>;
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn decode(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        self.decode()
    }
}

#[cfg(feature = "mp3dec")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn decode(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        self.decode::<S>()
    }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: Box<dyn Reader>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    cur_frames: u64,
    num_frames: u64,
    spec: Spec,
    decoder: fn(&mut dyn Reader, u16) -> Result<Vec<S>, AudioReadError>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, AudioError> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioError::Unimplemented(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other))),
        }
        let wave_sample_type = spec.get_sample_type();
        Ok(Self {
            reader,
            data_offset,
            data_length,
            cur_frames: 0,
            num_frames: data_length / fmt.block_align as u64,
            spec: spec.clone(),
            decoder: Self::choose_decoder(wave_sample_type)?,
        })
    }

    pub fn decode(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        if self.cur_frames >= self.num_frames {
            Ok(None)
        } else {
            self.cur_frames += 1;
            match (self.decoder)(&mut self.reader, self.spec.channels) {
                Ok(frame) => Ok(Some(frame)),
                Err(e) => Err(e),
            }
        }
    }

    // 这个函数用于给 choose_decoder 挑选
    fn decode_to<T>(r: &mut dyn Reader, channels: u16) -> Result<Vec<S>, AudioReadError>
    where T: SampleType {
        let mut ret = Vec::<S>::with_capacity(channels as usize);
        for _ in 0..channels {
            ret.push(S::from(T::read_le(r)?));
        }
        Ok(ret)
    }

    // 这个函数返回的 decoder 只负责读取和转换格式，不负责判断是否读到末尾
    fn choose_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader, u16) -> Result<Vec<S>, AudioReadError>, AudioError> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::decode_to::<i8 >),
            S16 => Ok(Self::decode_to::<i16>),
            S24 => Ok(Self::decode_to::<i24>),
            S32 => Ok(Self::decode_to::<i32>),
            S64 => Ok(Self::decode_to::<i64>),
            U8 =>  Ok(Self::decode_to::<u8 >),
            U16 => Ok(Self::decode_to::<u16>),
            U24 => Ok(Self::decode_to::<u24>),
            U32 => Ok(Self::decode_to::<u32>),
            U64 => Ok(Self::decode_to::<u64>),
            F32 => Ok(Self::decode_to::<f32>),
            F64 => Ok(Self::decode_to::<f64>),
            Unknown => return Err(AudioError::InvalidArguments(format!("unknown sample type \"{:?}\"", wave_sample_type))),
        }
    }
}

#[cfg(feature = "mp3dec")]
pub mod MP3 {
    use std::{io::Seek, fmt::Debug};
    use puremp3::{Frame, FrameHeader, Channels};
    use crate::errors::AudioReadError;
    use crate::readwrite::Reader;
    use crate::sampleutils::{SampleType};

    type TheDecoder = puremp3::Mp3Decoder<Box<dyn Reader>>;

    pub struct Mp3Decoder {
        data_offset: u64,
        data_length: u64,
        the_decoder: TheDecoder,
        cur_frame: Frame,
        sample_index: usize,
        num_frames: u64,
        print_debug: bool,
    }

    impl Mp3Decoder {
        pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, print_debug: bool) -> Result<Self, AudioReadError> {
            let mut the_decoder = puremp3::Mp3Decoder::new(reader);
            let cur_frame = the_decoder.next_frame()?;
            let num_frames = 1;
            if print_debug {
                let reader = the_decoder.get_mut();
                println!("{}, {}, 0x{:x}, 0x{:x}", num_frames, cur_frame.num_samples, reader.stream_position()?, data_length);
            }
            Ok(Self {
                data_offset,
                data_length,
                the_decoder,
                cur_frame, // TODO: 取得 frame 后，判断它的采样率是否和 WAV 相同，如果不相同，要做重采样
                sample_index: 0,
                num_frames,
                print_debug,
            })
        }

        pub fn get_sample_rate(&self) -> u32 {
            self.cur_frame.header.sample_rate.hz()
        }

        pub fn get_channels(&self) -> u16 {
            match self.cur_frame.header.channels {
                Channels::Mono => 1,
                Channels::DualMono => 2,
                Channels::Stereo => 2,
                Channels::JointStereo{ intensity_stereo: _, mid_side_stereo: _ } => 2,
            }
        }

        pub fn decode<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where S: SampleType {
            let cur_frame = &self.cur_frame;
            if self.sample_index < cur_frame.num_samples {
                let (l, r) = (
                    cur_frame.samples[0][self.sample_index],
                    cur_frame.samples[1][self.sample_index]
                );
                self.sample_index += 1;
                let m = S::from((l + r) * 0.5);
                let l = S::from(l);
                let r = S::from(r);
                match cur_frame.header.channels {
                    Channels::Mono => Ok(Some(vec![m])),
                    Channels::DualMono => Ok(Some(vec![l, r])),
                    Channels::Stereo => Ok(Some(vec![l, r])),
                    Channels::JointStereo{ intensity_stereo: _, mid_side_stereo: _ } => Ok(Some(vec![l, r])),
                }
            } else {
                // 下一个 Frame
                // TODO:
                // 检测 Frame 里面的参数变化，比如采样率和声道数的变化，如果采样率变化了，要做 resample。如果声道数变化了，要做声道数处理。
                loop {
                    let reader = self.the_decoder.get_mut();
                    if reader.stream_position()? >= self.data_offset + self.data_length {
                        // 真正完成读取
                        return Ok(None)
                    }
                    match self.the_decoder.next_frame() {
                        Ok(frame) => {
                            self.cur_frame = frame;
                            break;
                        },
                        Err(err) => {
                            match err {
                                puremp3::Error::Mp3Error(_) => {
                                    if self.print_debug {
                                        eprintln!("Mp3Error: {:?}", err);
                                    }
                                    return Err(err.into())
                                },
                                puremp3::Error::IoError(_) => {
                                    // 返回去强制重新读取帧，直到读取位置达到 MP3 文件长度为止
                                    continue;
                                },
                            }
                        },
                    };
                }
                self.num_frames += 1;
                if self.print_debug {
                    let reader = self.the_decoder.get_mut();
                    println!("{}, {}, 0x{:x}, 0x{:x}", self.num_frames, self.cur_frame.num_samples, reader.stream_position()? - self.data_offset, self.data_length);
                }
                self.sample_index = 0;
                self.decode::<S>()
            }
        }
    }

    struct FakeFrame {
        header: FrameHeader,
        num_samples: usize,
    }

    impl FakeFrame {
        fn from(frame: &Frame) -> Self{
            Self {
                header: frame.header.clone(),
                num_samples: frame.num_samples,
            }
        }
    }

    impl Debug for Mp3Decoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Decoder")
                .field("data_offset", &self.data_offset)
                .field("data_length", &self.data_length)
                .field("iterator", &format_args!("Iterator<Item = Frame>"))
                .field("cur_frame", &FakeFrame::from(&self.cur_frame))
                .field("sample_index", &self.sample_index)
                .finish()
        }
    }

    impl Debug for FakeFrame {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Frame")
                .field("header", &self.header)
                .field("samples", &format_args!("[[f32; 1152]; 2]"))
                .field("num_samples", &self.num_samples)
                .finish()
        }
    }
}

