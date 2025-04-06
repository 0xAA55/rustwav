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

    // 必须实现
    fn get_channels(&self) -> u16;
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError>;

    // 可选实现
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(match self.decode_frame()? {
                Some(samples) => Some((samples[0], samples[0])),
                None => None,
            }),
            2 => Ok(match self.decode_frame()? {
                Some(samples) => Some((samples[0], samples[1])),
                None => None,
            }),
            other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 2 channels."))),
        }
    }

    // 可选实现
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(match self.decode_frame()? {
                Some(samples) => Some(samples[0]),
                None => None,
            }),
            2 => Ok(match self.decode_frame()? {
                Some(samples) => Some(samples[0] / S::from(2) + samples[1] / S::from(2)),
                None => None,
            }),
            other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 1 channels."))),
        }
    }
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.spec.channels }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono() }
}

impl<S, D> Decoder<S> for AdpcmDecoderWrap<D>
    where S: SampleType,
          D: adpcm::AdpcmDecoder {
    fn get_channels(&self) -> u16 { if self.is_stereo {2} else {1} }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "mp3dec")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.get_channels() }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: Box<dyn Reader>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    spec: Spec,
    sample_decoder: fn(&mut dyn Reader) -> Result<S, AudioReadError>,
    samples_decoder: fn(&mut dyn Reader, usize) -> Result<Vec<S>, AudioReadError>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, AudioError> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioError::InvalidArguments(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other))),
        }
        let wave_sample_type = spec.get_sample_type();
        Ok(Self {
            reader,
            data_offset,
            data_length,
            spec: spec.clone(),
            sample_decoder: Self::choose_sample_decoder(wave_sample_type)?,
            samples_decoder: Self::choose_samples_decoder(wave_sample_type)?,
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    fn decode_sample<T>(&mut self) -> Result<Option<S>, AudioReadError>
    where T: SampleType {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            Ok(Some(S::from(T::read_le(&mut self.reader)?)))
        }
    }

    // 这个函数用于给 choose_decoder 挑选
    fn decode_sample_to<T>(r: &mut dyn Reader) -> Result<S, AudioReadError>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }
    fn decode_samples_to<T>(r: &mut dyn Reader, num_samples_to_read: usize) -> Result<Vec<S>, AudioReadError>
    where T: SampleType {
        let mut ret = Vec::<S>::with_capacity(num_samples_to_read);
        for _ in 0..num_samples_to_read {
            ret.push(Self::decode_sample_to::<T>(r)?);
        }
        Ok(ret)
    }

    // 这个函数返回的 decoder 只负责读取和转换格式，不负责判断是否读到末尾
    fn choose_sample_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader) -> Result<S, AudioReadError>, AudioError> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::decode_sample_to::<i8 >),
            S16 => Ok(Self::decode_sample_to::<i16>),
            S24 => Ok(Self::decode_sample_to::<i24>),
            S32 => Ok(Self::decode_sample_to::<i32>),
            S64 => Ok(Self::decode_sample_to::<i64>),
            U8 =>  Ok(Self::decode_sample_to::<u8 >),
            U16 => Ok(Self::decode_sample_to::<u16>),
            U24 => Ok(Self::decode_sample_to::<u24>),
            U32 => Ok(Self::decode_sample_to::<u32>),
            U64 => Ok(Self::decode_sample_to::<u64>),
            F32 => Ok(Self::decode_sample_to::<f32>),
            F64 => Ok(Self::decode_sample_to::<f64>),
            Unknown => Err(AudioError::InvalidArguments(format!("unknown sample type \"{:?}\"", wave_sample_type))),
        }
    }

    fn choose_samples_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader, usize) -> Result<Vec<S>, AudioReadError>, AudioError> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::decode_samples_to::<i8 >),
            S16 => Ok(Self::decode_samples_to::<i16>),
            S24 => Ok(Self::decode_samples_to::<i24>),
            S32 => Ok(Self::decode_samples_to::<i32>),
            S64 => Ok(Self::decode_samples_to::<i64>),
            U8 =>  Ok(Self::decode_samples_to::<u8 >),
            U16 => Ok(Self::decode_samples_to::<u16>),
            U24 => Ok(Self::decode_samples_to::<u24>),
            U32 => Ok(Self::decode_samples_to::<u32>),
            U64 => Ok(Self::decode_samples_to::<u64>),
            F32 => Ok(Self::decode_samples_to::<f32>),
            F64 => Ok(Self::decode_samples_to::<f64>),
            Unknown => Err(AudioError::InvalidArguments(format!("unknown sample type \"{:?}\"", wave_sample_type))),
        }
    }

    pub fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            Ok(Some((self.samples_decoder)(&mut self.reader, self.spec.channels as usize)?))
        }
    }

    pub fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.spec.channels {
                1 => {
                    let sample = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some((sample, sample)))
                },
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some((sample_l, sample_r)))
                },
                other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 2 channels."))),
            }
        }
    }

    pub fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.get_channels() {
                1 => {
                    Ok(Some((self.sample_decoder)(&mut self.reader)?))
                },
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some(sample_l / S::from(2) + sample_r / S::from(2)))
                },
                other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 1 channels."))),
            }
        }
    }
}

#[derive(Debug)]
pub struct AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    reader: Box<dyn Reader>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    is_stereo: bool,
    decoder_l: D,
    decoder_r: D,
    buffer_l: Vec<i16>,
    buffer_r: Vec<i16>,
}

impl<D> AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, fmt: &FmtChunk) -> Result<Self, AudioReadError> {
        Ok(Self {
            reader,
            data_offset,
            data_length,
            is_stereo: match fmt.channels {
                1 => false,
                2 => true,
                other => return Err(AudioReadError::InvalidArguments(format!("Num channels in the `fmt ` chunk is invalid: {other}"))),
            },
            decoder_l: D::new(),
            decoder_r: D::new(),
            buffer_l: Vec::<i16>::new(),
            buffer_r: Vec::<i16>::new(),
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    fn read_byte(&mut self) -> Option<u8> {
        match self.is_end_of_data() {
            Ok(val) => {
                match val {
                    true => None,
                    false => {
                        match u8::read_le(&mut self.reader) {
                            Ok(byte) => Some(byte),
                            Err(_) => None,
                        }
                    },
                }
            },
            Err(_) => None,
        }
    }

    pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
    where S: SampleType {
        match self.is_stereo {
            false => {
                while self.buffer_l.len() == 0 {
                    if self.is_end_of_data()? {
                        return Ok(None);
                    } 
                    // 利用迭代器，每个声道只给它一个数据
                    let mut l_iter = [u8::read_le(&mut self.reader)?].into_iter();
                    self.decoder_l.decode(
                        || -> Option<u8> {l_iter.next()},
                        |sample: i16| {self.buffer_l.push(sample);})?;
                }
                let sample = S::from(self.buffer_l[0]);
                self.buffer_l = self.buffer_l[1..].to_vec();
                Ok(Some(sample))
            },
            true => {
                match self.decode_stereo::<S>()? {
                    Some((l, r)) => Ok(Some(l / S::from(2) + r / S::from(2))),
                    None => Ok(None),
                }
            },
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where S: SampleType {
        match self.is_stereo {
            false => {
                match self.decode_mono::<S>()? {
                    Some(sample) => Ok(Some((sample, sample))),
                    None => Ok(None),
                }
            },
            true => {
                // 这个时候要左右一起读，因为声道数据是左右交替的
                while self.buffer_l.len() == 0 || self.buffer_r.len() == 0 {
                    if self.is_end_of_data()? {
                        return Ok(None);
                    } 
                    // 利用迭代器，每个声道只给它一个数据
                    let mut l_iter = [u8::read_le(&mut self.reader)?].into_iter();
                    let mut r_iter = [u8::read_le(&mut self.reader)?].into_iter();
                    self.decoder_l.decode(
                        || -> Option<u8> {l_iter.next()},
                        |sample: i16| {self.buffer_l.push(sample);})?;
                    self.decoder_r.decode(
                        || -> Option<u8> {r_iter.next()},
                        |sample: i16| {self.buffer_r.push(sample);})?;
                }
                let (l, r) = (self.buffer_l[0], self.buffer_r[0]);
                let (l, r) = (S::from(l), S::from(r));
                // 滚动数据
                self.buffer_l = self.buffer_l[1..].to_vec();
                self.buffer_r = self.buffer_r[1..].to_vec();
                Ok(Some((l, r)))
            },
        }
    }

    pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
    where S: SampleType {
        match self.is_stereo {
            false => {
                match self.decode_mono::<S>()? {
                    Some(sample) => Ok(Some(vec![sample])),
                    None => Ok(None),
                }
            },
            true => {
                match self.decode_stereo::<S>()? {
                    Some((l, r)) => Ok(Some(vec![l, r])),
                    None => Ok(None),
                }
            },
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
        target_sample_format: u32,
        data_offset: u64,
        data_length: u64,
        the_decoder: TheDecoder,
        cur_frame: Frame,
        sample_index: usize,
        num_frames: u64,
        print_debug: bool,
    }

    impl Mp3Decoder {
        pub fn new(reader: Box<dyn Reader>, target_sample_format: u32, data_offset: u64, data_length: u64, print_debug: bool) -> Result<Self, AudioReadError> {
            let mut the_decoder = puremp3::Mp3Decoder::new(reader);
            let cur_frame = the_decoder.next_frame()?;
            let num_frames = 1;
            Ok(Self {
                target_sample_format,
                data_offset,
                data_length,
                the_decoder,
                cur_frame,
                sample_index: 0,
                num_frames,
                print_debug,
            })
        }

        pub fn get_sample_rate(&self) -> u32 {
            self.target_sample_format
        }

        pub fn get_frame_sample_rate(&self) -> u32 {
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

        fn next_mp3_frame(&mut self) -> Result<bool, AudioReadError> {
            loop {
                let reader = self.the_decoder.get_mut();
                if reader.stream_position()? >= self.data_offset + self.data_length {
                    // 真正完成读取
                    return Ok(false)
                }
                match self.the_decoder.next_frame() {
                    Ok(frame) => {
                        // 下一个 Frame
                        // TODO:
                        // 检测 Frame 里面的参数变化，比如采样率和声道数的变化，如果采样率变化了，要做 resample。如果声道数变化了，要做声道数处理。
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
            self.sample_index = 0;
            Ok(true)
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where S: SampleType {
            let cur_frame = &self.cur_frame;
            if self.sample_index < cur_frame.num_samples {
                let (l, r) = (
                    cur_frame.samples[0][self.sample_index],
                    cur_frame.samples[1][self.sample_index]
                );
                self.sample_index += 1;
                let l = S::from(l);
                let r = S::from(r);
                Ok(Some((l, r)))
            } else {
                match self.next_mp3_frame()? {
                    false => return Ok(None),
                    true => (),
                }
                self.decode_stereo::<S>()
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where S: SampleType {
            let cur_frame = &self.cur_frame;
            if self.sample_index < cur_frame.num_samples {
                let (l, r) = (
                    cur_frame.samples[0][self.sample_index],
                    cur_frame.samples[1][self.sample_index]
                );
                self.sample_index += 1;
                let m = S::from((l + r) * 0.5);
                Ok(Some(m))
            } else {
                match self.next_mp3_frame()? {
                    false => return Ok(None),
                    true => (),
                }
                self.decode_mono::<S>()
            }
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
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
                match self.next_mp3_frame()? {
                    false => return Ok(None),
                    true => (),
                }
                self.decode_frame::<S>()
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

