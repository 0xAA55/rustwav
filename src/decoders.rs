#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, io::BufReader, fmt::Debug};

// use crate::adpcm::*;
use crate::errors::{AudioError, AudioReadError};
use crate::wavcore::{Spec, WaveSampleType, FmtChunk};
use crate::sampleutils::{SampleType, i24, u24};
use crate::readwrite::Reader;

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

#[cfg(feature = "mp3")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn decode(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        self.decode::<S>()
    }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    cur_frames: u64,
    num_frames: u64,
    spec: Spec,
    decoder: fn(&mut dyn Reader, u16) -> Result<Vec<S>, AudioReadError>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, AudioError> {
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
            decoder: Self::get_decoder(wave_sample_type)?,
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

    // 这个函数用于给 get_decoder 挑选
    fn decode_to<T>(r: &mut dyn Reader, channels: u16) -> Result<Vec<S>, AudioReadError>
    where T: SampleType {
        let mut ret = Vec::<S>::with_capacity(channels as usize);
        for _ in 0..channels {
            ret.push(S::from(T::read_le(r)?));
        }
        Ok(ret)
    }

    // 这个函数返回的 decoder 只负责读取和转换格式，不负责判断是否读到末尾
    fn get_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader, u16) -> Result<Vec<S>, AudioReadError>, AudioError> {
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

#[cfg(feature = "mp3")]
pub mod MP3 {
    use std::{fs::File, io::BufReader, fmt::Debug};
    use puremp3::{read_mp3, FrameHeader, Channels};
    use crate::errors::AudioReadError;
    use crate::wavcore::FmtChunk;
    use crate::sampleutils::{SampleType};

    pub struct Mp3Decoder {
        data_offset: u64,
        data_length: u64,
        frame_header: FrameHeader,
        iterator: Box<dyn Iterator<Item = (f32, f32)>>,
    }

    impl Mp3Decoder {
        pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, fmt: &FmtChunk) -> Result<Self, AudioReadError> {
            match fmt.format_tag {
                0x0055 => (),
                other => return Err(AudioReadError::Unimplemented(format!("`Mp3Decoder` can't handle format_tag 0x{:x}", other))),
            }
            let (frame_header, iterator) = read_mp3(reader)?;
            Ok(Self {
                data_offset,
                data_length,
                frame_header,
                iterator: Box::new(iterator),
            })
        }

        pub fn decode<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where S: SampleType {
            match self.iterator.next() {
                Some((l, r)) => {
                    let m = S::from((l + r) * 0.5);
                    let l = S::from(l);
                    let r = S::from(r);
                    match self.frame_header.channels {
                        Channels::Mono => Ok(Some(vec![m])),
                        Channels::DualMono => Ok(Some(vec![l, r])),
                        Channels::Stereo => Ok(Some(vec![l, r])),
                        Channels::JointStereo{ intensity_stereo: _, mid_side_stereo: _ }  => Ok(Some(vec![l, r])),
                    }
                },
                None => Ok(None),
            }
        }
    }

    impl Debug for Mp3Decoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Decoder")
                .field("data_offset", &self.data_offset)
                .field("data_length", &self.data_length)
                .field("frame_header", &self.frame_header)
                .field("iterator", &format_args!("Iterator<Item = (f32, f32)>"))
                .finish()
        }
    }
}

