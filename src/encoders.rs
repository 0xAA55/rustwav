#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

// use crate::adpcm::*;
use crate::errors::AudioWriteError;
use crate::wavcore::{WaveSampleType};
use crate::readwrite::{Writer};
use crate::sampleutils::{SampleType, i24, u24};

// S：输入的样本格式
// EncoderFrom 解决了通过泛型输入音频帧的问题。
pub trait EncoderFrom<S>: Debug
where S: SampleType {
    fn write_frame(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError>;
}

pub trait Encoder:
    EncoderFrom<i8 > +
    EncoderFrom<i16> +
    EncoderFrom<i32> +
    EncoderFrom<i64> +
    EncoderFrom<u8 > +
    EncoderFrom<u16> +
    EncoderFrom<u32> +
    EncoderFrom<u64> +
    EncoderFrom<f32> +
    EncoderFrom<f64> {}

impl<T> Encoder for T where T:
    EncoderFrom<i8 > +
    EncoderFrom<i16> +
    EncoderFrom<i32> +
    EncoderFrom<i64> +
    EncoderFrom<u8 > +
    EncoderFrom<u16> +
    EncoderFrom<u32> +
    EncoderFrom<u64> +
    EncoderFrom<f32> +
    EncoderFrom<f64> {}

impl<S> EncoderFrom<S> for PcmEncoder
where S: SampleType {
    fn write_frame(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError> {
        Self::write_frame::<S>(writer, frame)
    }
    fn write_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError> {
        Self::write_multiple_frames::<S>(writer, frames)
    }
}

#[derive(Debug, Clone)]
pub struct PcmEncoder {}

impl PcmEncoder {
    pub fn write_frame<S>(writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        for sample in frame.iter() {
            // T 从哪来
            T::from(*sample).write_le(writer)?;
        }
        Ok(())
    }

    pub fn write_multiple_frames<S>(writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        for frame in frames.iter() {
            for sample in frame.iter() {
                // T 从哪来
                T::from(*sample).write_le(writer)?;
            }
        }
        Ok(())
    }
}


