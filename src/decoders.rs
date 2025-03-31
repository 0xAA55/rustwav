#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, io::{BufReader}, fmt::Debug, error::Error};

// use crate::adpcm::*;
use crate::errors::AudioError;
use crate::wavcore::{Spec, WaveSampleType, FmtChunk};
use crate::sampleutils::{SampleType, i24, u24};
use crate::readwrite::{Reader};

// 解码器，解码出来的样本格式是 S
pub trait Decoder<S>: Debug
    where S: SampleType {
    fn decode(&mut self) -> Result<S, io::Error>;
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    frame_size: u16,
    spec: Spec,
    decoder: fn(&mut dyn Reader) -> Result<S, io::Error>,
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn decode(&mut self) -> Result<S, io::Error>
    where S: SampleType {
        self.decode()
    }
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &fmt__Chunk) -> Result<Self, Box<dyn Error>> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioReadError::Unimplemented(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other)).into()),
        }
        let wave_sample_type = get_sample_type(spec.bits_per_sample, spec.sample_format);
        Ok(Self {
            reader,
            data_offset,
            data_length,
            frame_size: wave_sample_type.sizeof() * spec.channels,
            spec: spec.clone(),
            decoder: Self::get_decoder(wave_sample_type)?,
        })
    }

    pub fn decode(&mut self) -> Result<S, io::Error> {
        (self.decoder)(&mut self.reader)
    }

    fn decode_to<T>(r: &mut dyn Reader) -> Result<S, io::Error>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }

    fn get_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader) -> Result<S, io::Error>, Box<dyn Error>> {
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


