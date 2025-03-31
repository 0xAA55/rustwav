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
mod MP3 {
    use std::{fs::File, io::{self, BufReader}};
    use puremp3::*;
    use crate::sampleutils::SampleType;

    #[derive(Debug)]
    pub struct Mp3Decoder<S>
    where S: SampleType {
        reader: BufReader<File>, // 数据读取器
        data_offset: u64,
        data_length: u64,
        frame_size: u16,
        
    }
}

