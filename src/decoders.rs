#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, io::{self, BufReader}, fmt::Debug, error::Error};

// use crate::adpcm::*;
use crate::errors::AudioError;
use crate::wavcore::{Spec, WaveSampleType, FmtChunk};
use crate::sampleutils::{SampleType, i24, u24};
use crate::readwrite::Reader;

// 解码器，解码出来的样本格式是 S
pub trait Decoder<S>: Debug
    where S: SampleType {
    fn decode(&mut self) -> Result<Vec<S>, io::Error>;
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn decode(&mut self) -> Result<Vec<S>, io::Error> {
        self.decode()
    }
}

#[cfg(feature = "mp3")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn decode(&mut self) -> Result<Vec<S>, io::Error> {
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
    decoder: fn(&mut dyn Reader, u16) -> Result<Vec<S>, io::Error>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, Box<dyn Error>> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioError::Unimplemented(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other)).into()),
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

    pub fn decode(&mut self) -> Result<Vec<S>, io::Error> {
        if self.cur_frames >= self.num_frames {
            Err(io::Error::new(io::ErrorKind::Other, "Finished reading PCM file."))
        } else {
            self.cur_frames += 1;
            (self.decoder)(&mut self.reader, self.spec.channels)
        }
    }

    fn decode_to<T>(r: &mut dyn Reader, channels: u16) -> Result<Vec<S>, io::Error>
    where T: SampleType {
        let mut ret = Vec::<S>::with_capacity(channels as usize);
        for _ in 0..channels {
            ret.push(S::from(T::read_le(r)?));
        }
        Ok(ret)
    }

    fn get_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader, u16) -> Result<Vec<S>, io::Error>, Box<dyn Error>> {
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
            Unknown => return Err(AudioError::UnknownSampleType.into()),
        }
    }
}

#[cfg(feature = "mp3")]
pub mod MP3 {
    use std::{fs::File, io::{self, BufReader}, fmt::Debug, error::{Error}};
    use puremp3::{read_mp3, FrameHeader, Channels};
    use crate::errors::AudioError;
    use crate::wavcore::FmtChunk;
    use crate::sampleutils::{SampleType};

    pub struct Mp3Decoder {
        data_offset: u64,
        data_length: u64,
        frame_header: FrameHeader,
        iterator: Box<dyn Iterator<Item = (f32, f32)>>,
    }

    impl Mp3Decoder {
        pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, fmt: &FmtChunk) -> Result<Self, Box<dyn Error>> {
            match fmt.format_tag {
                0x0055 => (),
                other => return Err(AudioError::Unimplemented(format!("`Mp3Decoder` can't handle format_tag 0x{:x}", other)).into()),
            }
            let (frame_header, iterator) = read_mp3(reader)?;
            Ok(Self {
                data_offset,
                data_length,
                frame_header,
                iterator: Box::new(iterator),
            })
        }

        pub fn decode<S>(&mut self) -> Result<Vec<S>, io::Error>
        where S: SampleType {
            match self.iterator.next() {
                Some((l, r)) => {
                    let m = S::from((l + r) * 0.5);
                    let l = S::from(l);
                    let r = S::from(r);
                    match self.frame_header.channels {
                        Channels::Mono => Ok(vec![m]),
                        Channels::DualMono => Ok(vec![l, r]),
                        Channels::Stereo => Ok(vec![l, r]),
                        Channels::JointStereo{ intensity_stereo: _, mid_side_stereo: _ }  => Ok(vec![l, r]), 
                    }
                },
                None => Err(io::Error::new(io::ErrorKind::Other, "Finished reading MP3 file.")),
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

