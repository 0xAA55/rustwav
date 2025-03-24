use std::{io::{Read, Write}};

pub use crate::errors::*;
pub use crate::audiocore::SampleFormat;
use crate::sampleutils::*;

pub enum WaveSampleType {
    U8,
    S16,
    S24,
    S32,
    F32,
    F64,
}

pub fn get_sample_type(bits_per_sample: u16, sample_format: SampleFormat) -> Result<WaveSampleType, AudioError> {
    use SampleFormat::{UInt, Int, Float};
    use WaveSampleType::{U8,S16,S24,S32,F32,F64};
    match (bits_per_sample, sample_format) {
        (8, UInt) => Ok(U8),
        (16, Int) => Ok(S16),
        (24, Int) => Ok(S24),
        (32, Int) => Ok(S32),
        (32, Float) => Ok(F32),
        (64, Float) => Ok(F64),
        _ => Err(AudioError::UnknownSampleType),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GUID (pub u32, pub u16, pub u16, pub [u8; 8]);

pub const GUID_PCM_FORMAT: GUID = GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
pub const GUID_IEEE_FLOAT_FORMAT: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);

impl GUID {
    pub fn read<T: Read>(r: &mut T) -> Result<Self, std::io::Error> {
        Ok( Self (
            u32::read_le(r)?,
            u16::read_le(r)?,
            u16::read_le(r)?,
            [
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
            ]
        ))
    }

    pub fn write<T: Write>(&self, w: &mut T) -> Result<(), std::io::Error> {
        self.0.write_le(w)?;
        self.1.write_le(w)?;
        self.2.write_le(w)?;
        w.write_all(&self.3)?;
        Ok(())
    }
}
