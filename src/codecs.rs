#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, io::BufReader};

use crate::wavcore::*;
use crate::readwrite::*;

pub trait Decoder<S>
    where S: SampleType {
    fn get_name(&self) -> &'static str;
    fn get_reader(&self) -> &BufReader<File>;
    fn decode(&mut self) -> Result<S, io::Error>;
}

impl<S> std::fmt::Debug for dyn Decoder<S>
where S: SampleType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct(&format!("Decoder<{}>", std::any::type_name::<S>()))
            .field("name", &self.get_name())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    frame_size: u16,
    spec: Spec,
    unpacker: fn(&mut dyn Reader) -> Result<S, io::Error>,
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn get_name(&self) -> &'static str {
        Self::get_name()
    }
    fn get_reader(&self) -> &BufReader<File> {
        &self.reader
    }
    fn decode(&mut self) -> Result<S, io::Error>
    where S: SampleType {
        self.unpack()
    }
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &fmt__Chunk) -> Result<Self, Box<dyn Error>> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioReadError::Unimplemented(format!("{} can't handle format_tag 0x{:x}", Self::get_name(), other)).into()),
        }
        let wave_sample_type = get_sample_type(spec.bits_per_sample, spec.sample_format)?;
        Ok(Self {
            reader,
            data_offset,
            data_length,
            frame_size: wave_sample_type.sizeof() * spec.channels,
            spec: spec.clone(),
            unpacker: Self::get_unpacker(wave_sample_type)?,
        })
    }

    pub fn get_name() -> &'static str {
        "PCM reader"
    }

    pub fn unpack(&mut self) -> Result<S, io::Error> {
        (self.unpacker)(&mut self.reader)
    }

    fn unpack_to<T>(r: &mut dyn Reader) -> Result<S, io::Error>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }

    fn get_unpacker(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader) -> Result<S, io::Error>, Box<dyn Error>> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::unpack_to::<i8 >),
            S16 => Ok(Self::unpack_to::<i16>),
            S24 => Ok(Self::unpack_to::<i24>),
            S32 => Ok(Self::unpack_to::<i32>),
            S64 => Ok(Self::unpack_to::<i64>),
            U8 =>  Ok(Self::unpack_to::<u8 >),
            U16 => Ok(Self::unpack_to::<u16>),
            U24 => Ok(Self::unpack_to::<u24>),
            U32 => Ok(Self::unpack_to::<u32>),
            U64 => Ok(Self::unpack_to::<u64>),
            F32 => Ok(Self::unpack_to::<f32>),
            F64 => Ok(Self::unpack_to::<f64>),
            Unknown => return Err(AudioError::UnknownSampleType.into()),
        }
    }
}
