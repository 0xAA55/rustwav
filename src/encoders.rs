#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

use crate::Writer;
use crate::{SampleType, i24, u24};
use crate::AudioWriteError;
use crate::adpcm;
use crate::wavcore::{Spec, WaveSampleType};
use crate::wavcore::{FmtChunk, FmtExtension, ExtensibleData, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
use crate::utils::{self, sample_conv, stereo_conv, stereos_conv, sample_conv_batch};
use crate::xlaw::{XLaw, PcmXLawEncoder};

// An encoder that accepts samples of type `S` and encodes them into the file's target format.
// Due to trait bounds prohibiting generic parameters, each function must be explicitly 
// implemented for every supported type.
pub trait EncoderToImpl: Debug {
    fn get_bitrate(&self) -> u32;
    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError>;
    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError>;
    fn begin_encoding(&mut self) -> Result<(), AudioWriteError>;
    fn finish(&mut self) -> Result<(), AudioWriteError>;

    // Channel-agnostic low-level sample writers.
    fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError>;
    fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError>;
    fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError>;
    fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError>;
    fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError>;
    fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError>;
    fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError>;
    fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError>;
    fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError>;
    fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError>;
    fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError>;
    fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError>;

    // Convenience interfaces for writing audio frames. Each frame is an array of samples per channel. Default implementations are provided.
    fn write_frame__i8(&mut self, frame: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(frame)}
    fn write_frame_i16(&mut self, frame: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(frame)}
    fn write_frame_i24(&mut self, frame: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(frame)}
    fn write_frame_i32(&mut self, frame: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(frame)}
    fn write_frame_i64(&mut self, frame: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(frame)}
    fn write_frame__u8(&mut self, frame: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(frame)}
    fn write_frame_u16(&mut self, frame: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(frame)}
    fn write_frame_u24(&mut self, frame: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(frame)}
    fn write_frame_u32(&mut self, frame: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(frame)}
    fn write_frame_u64(&mut self, frame: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(frame)}
    fn write_frame_f32(&mut self, frame: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(frame)}
    fn write_frame_f64(&mut self, frame: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(frame)}

    // Convenience interfaces for writing multiple audio frames. Default implementations are provided.
    fn write_frames__i8(&mut self, frames: &[Vec<i8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__i8(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i16(&mut self, frames: &[Vec<i16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i16(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i24(&mut self, frames: &[Vec<i24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i24(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i32(&mut self, frames: &[Vec<i32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i32(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i64(&mut self, frames: &[Vec<i64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i64(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames__u8(&mut self, frames: &[Vec<u8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__u8(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u16(&mut self, frames: &[Vec<u16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u24(&mut self, frames: &[Vec<u24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u24(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u32(&mut self, frames: &[Vec<u32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u32(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u64(&mut self, frames: &[Vec<u64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u64(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_f32(&mut self, frames: &[Vec<f32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f32(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_f64(&mut self, frames: &[Vec<f64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f64(&utils::frames_to_interleaved_samples(frames, Some(channels))?)}

    // Interfaces for writing mono audio frames (single-channel). Default implementations are provided.
    fn write_sample__i8(&mut self, sample: i8 ) -> Result<(), AudioWriteError> {self.write_samples__i8(&[sample])}
    fn write_sample_i16(&mut self, sample: i16) -> Result<(), AudioWriteError> {self.write_samples_i16(&[sample])}
    fn write_sample_i24(&mut self, sample: i24) -> Result<(), AudioWriteError> {self.write_samples_i24(&[sample])}
    fn write_sample_i32(&mut self, sample: i32) -> Result<(), AudioWriteError> {self.write_samples_i32(&[sample])}
    fn write_sample_i64(&mut self, sample: i64) -> Result<(), AudioWriteError> {self.write_samples_i64(&[sample])}
    fn write_sample__u8(&mut self, sample: u8 ) -> Result<(), AudioWriteError> {self.write_samples__u8(&[sample])}
    fn write_sample_u16(&mut self, sample: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(&[sample])}
    fn write_sample_u24(&mut self, sample: u24) -> Result<(), AudioWriteError> {self.write_samples_u24(&[sample])}
    fn write_sample_u32(&mut self, sample: u32) -> Result<(), AudioWriteError> {self.write_samples_u32(&[sample])}
    fn write_sample_u64(&mut self, sample: u64) -> Result<(), AudioWriteError> {self.write_samples_u64(&[sample])}
    fn write_sample_f32(&mut self, sample: f32) -> Result<(), AudioWriteError> {self.write_samples_f32(&[sample])}
    fn write_sample_f64(&mut self, sample: f64) -> Result<(), AudioWriteError> {self.write_samples_f64(&[sample])}

    // Interfaces for writing batched mono audio frames. Default implementations are provided.
    fn write_mono_channel__i8(&mut self, monos: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(monos)}
    fn write_mono_channel_i16(&mut self, monos: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(monos)}
    fn write_mono_channel_i24(&mut self, monos: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(monos)}
    fn write_mono_channel_i32(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(monos)}
    fn write_mono_channel_i64(&mut self, monos: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(monos)}
    fn write_mono_channel__u8(&mut self, monos: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(monos)}
    fn write_mono_channel_u16(&mut self, monos: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(monos)}
    fn write_mono_channel_u24(&mut self, monos: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(monos)}
    fn write_mono_channel_u32(&mut self, monos: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(monos)}
    fn write_mono_channel_u64(&mut self, monos: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(monos)}
    fn write_mono_channel_f32(&mut self, monos: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(monos)}
    fn write_mono_channel_f64(&mut self, monos: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(monos)}

    // Interfaces for writing stereo audio (composed of two separate channel buffers). Default implementations are provided.
    fn write_dual_sample__i8(&mut self, mono1: i8 , mono2: i8 ) -> Result<(), AudioWriteError> {self.write_samples__i8(&[mono1, mono2])}
    fn write_dual_sample_i16(&mut self, mono1: i16, mono2: i16) -> Result<(), AudioWriteError> {self.write_samples_i16(&[mono1, mono2])}
    fn write_dual_sample_i24(&mut self, mono1: i24, mono2: i24) -> Result<(), AudioWriteError> {self.write_samples_i24(&[mono1, mono2])}
    fn write_dual_sample_i32(&mut self, mono1: i32, mono2: i32) -> Result<(), AudioWriteError> {self.write_samples_i32(&[mono1, mono2])}
    fn write_dual_sample_i64(&mut self, mono1: i64, mono2: i64) -> Result<(), AudioWriteError> {self.write_samples_i64(&[mono1, mono2])}
    fn write_dual_sample__u8(&mut self, mono1: u8 , mono2: u8 ) -> Result<(), AudioWriteError> {self.write_samples__u8(&[mono1, mono2])}
    fn write_dual_sample_u16(&mut self, mono1: u16, mono2: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(&[mono1, mono2])}
    fn write_dual_sample_u24(&mut self, mono1: u24, mono2: u24) -> Result<(), AudioWriteError> {self.write_samples_u24(&[mono1, mono2])}
    fn write_dual_sample_u32(&mut self, mono1: u32, mono2: u32) -> Result<(), AudioWriteError> {self.write_samples_u32(&[mono1, mono2])}
    fn write_dual_sample_u64(&mut self, mono1: u64, mono2: u64) -> Result<(), AudioWriteError> {self.write_samples_u64(&[mono1, mono2])}
    fn write_dual_sample_f32(&mut self, mono1: f32, mono2: f32) -> Result<(), AudioWriteError> {self.write_samples_f32(&[mono1, mono2])}
    fn write_dual_sample_f64(&mut self, mono1: f64, mono2: f64) -> Result<(), AudioWriteError> {self.write_samples_f64(&[mono1, mono2])}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_dual_monos__i8(&mut self, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i16(&mut self, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i24(&mut self, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i32(&mut self, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i64(&mut self, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos__u8(&mut self, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u16(&mut self, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u24(&mut self, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u32(&mut self, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u64(&mut self, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f32(&mut self, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f64(&mut self, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_monos__i8(&mut self, monos_array: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_samples__i8(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i16(&mut self, monos_array: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_samples_i16(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i24(&mut self, monos_array: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_samples_i24(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i32(&mut self, monos_array: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_samples_i32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i64(&mut self, monos_array: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_samples_i64(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos__u8(&mut self, monos_array: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_samples__u8(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u16(&mut self, monos_array: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_samples_u16(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u24(&mut self, monos_array: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_samples_u24(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u32(&mut self, monos_array: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_samples_u32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u64(&mut self, monos_array: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_samples_u64(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f32(&mut self, monos_array: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_samples_f32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f64(&mut self, monos_array: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_samples_f64(&utils::monos_to_interleaved_samples(monos_array)?)}

    // Interfaces for writing stereo audio frames using tuples (L, R). Default implementations are provided.
    fn write_stereo__i8(&mut self, stereo: (i8 , i8 )) -> Result<(), AudioWriteError> {self.write_samples__i8(&[stereo.0, stereo.1])}
    fn write_stereo_i16(&mut self, stereo: (i16, i16)) -> Result<(), AudioWriteError> {self.write_samples_i16(&[stereo.0, stereo.1])}
    fn write_stereo_i24(&mut self, stereo: (i24, i24)) -> Result<(), AudioWriteError> {self.write_samples_i24(&[stereo.0, stereo.1])}
    fn write_stereo_i32(&mut self, stereo: (i32, i32)) -> Result<(), AudioWriteError> {self.write_samples_i32(&[stereo.0, stereo.1])}
    fn write_stereo_i64(&mut self, stereo: (i64, i64)) -> Result<(), AudioWriteError> {self.write_samples_i64(&[stereo.0, stereo.1])}
    fn write_stereo__u8(&mut self, stereo: (u8 , u8 )) -> Result<(), AudioWriteError> {self.write_samples__u8(&[stereo.0, stereo.1])}
    fn write_stereo_u16(&mut self, stereo: (u16, u16)) -> Result<(), AudioWriteError> {self.write_samples_u16(&[stereo.0, stereo.1])}
    fn write_stereo_u24(&mut self, stereo: (u24, u24)) -> Result<(), AudioWriteError> {self.write_samples_u24(&[stereo.0, stereo.1])}
    fn write_stereo_u32(&mut self, stereo: (u32, u32)) -> Result<(), AudioWriteError> {self.write_samples_u32(&[stereo.0, stereo.1])}
    fn write_stereo_u64(&mut self, stereo: (u64, u64)) -> Result<(), AudioWriteError> {self.write_samples_u64(&[stereo.0, stereo.1])}
    fn write_stereo_f32(&mut self, stereo: (f32, f32)) -> Result<(), AudioWriteError> {self.write_samples_f32(&[stereo.0, stereo.1])}
    fn write_stereo_f64(&mut self, stereo: (f64, f64)) -> Result<(), AudioWriteError> {self.write_samples_f64(&[stereo.0, stereo.1])}

    // Interfaces for writing stereo audio frames using arrays of tuples. Default implementations are provided.
    fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_samples__i8(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_samples_i16(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_samples_i24(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_samples_i32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_samples_i64(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_samples__u8(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_samples_u16(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_samples_u24(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_samples_u32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_samples_u64(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_samples_f32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_samples_f64(&utils::stereos_to_interleaved_samples(stereos))}
}

#[derive(Debug, Clone, Copy)]
pub struct DummyEncoder;

// Default implementations: all input formats are normalized to `f32` for encoding.
impl EncoderToImpl for DummyEncoder {
    fn get_bitrate(&self) -> u32 {
        panic!("Must implement `get_bitrate()` for your encoder.");
    }

    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        panic!("Must implement `begin_encoding()` for your encoder.");
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        panic!("Must implement `new_fmt_chunk()` for your encoder.");
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        panic!("Must implement `update_fmt_chunk()` for your encoder.");
    }

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        panic!("Must implement `finish()` for your encoder to flush the data.");
    }

    fn write_samples_f32(&mut self, _samples: &[f32]) -> Result<(), AudioWriteError> {
        panic!("Must at lease implement `write_samples_f32()` for your encoder to get samples.");
    }

    fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
    fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f32(&sample_conv(samples))}
}

#[derive(Debug)]
pub struct Encoder<'a> {
    encoder: Box<dyn EncoderToImpl + 'a>,
}

impl<'a> Default for Encoder<'_> {
    fn default() -> Self {
        Self::new(DummyEncoder{})
    }
}

impl<'a> Encoder<'a> {
    pub fn new<T>(encoder: T) -> Self
    where T: EncoderToImpl + 'a {
        Self {
            encoder: Box::new(encoder),
        }
    }

    pub fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        self.encoder.begin_encoding()
    }

    pub fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        self.encoder.new_fmt_chunk()
    }

    pub fn get_bitrate(&self) -> u32 {
        self.encoder.get_bitrate()
    }

    pub fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        self.encoder.update_fmt_chunk(fmt)
    }

    pub fn finish(&mut self) -> Result<(), AudioWriteError> {
        self.encoder.finish()
    }

    pub fn write_samples<S>(&mut self, samples: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_samples__i8(&sample_conv(samples)),
            "i16" => self.encoder.write_samples_i16(&sample_conv(samples)),
            "i24" => self.encoder.write_samples_i24(&sample_conv(samples)),
            "i32" => self.encoder.write_samples_i32(&sample_conv(samples)),
            "i64" => self.encoder.write_samples_i64(&sample_conv(samples)),
            "u8"  => self.encoder.write_samples__u8(&sample_conv(samples)),
            "u16" => self.encoder.write_samples_u16(&sample_conv(samples)),
            "u24" => self.encoder.write_samples_u24(&sample_conv(samples)),
            "u32" => self.encoder.write_samples_u32(&sample_conv(samples)),
            "u64" => self.encoder.write_samples_u64(&sample_conv(samples)),
            "f32" => self.encoder.write_samples_f32(&sample_conv(samples)),
            "f64" => self.encoder.write_samples_f64(&sample_conv(samples)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_frame__i8(&sample_conv(frame)),
            "i16" => self.encoder.write_frame_i16(&sample_conv(frame)),
            "i24" => self.encoder.write_frame_i24(&sample_conv(frame)),
            "i32" => self.encoder.write_frame_i32(&sample_conv(frame)),
            "i64" => self.encoder.write_frame_i64(&sample_conv(frame)),
            "u8"  => self.encoder.write_frame__u8(&sample_conv(frame)),
            "u16" => self.encoder.write_frame_u16(&sample_conv(frame)),
            "u24" => self.encoder.write_frame_u24(&sample_conv(frame)),
            "u32" => self.encoder.write_frame_u32(&sample_conv(frame)),
            "u64" => self.encoder.write_frame_u64(&sample_conv(frame)),
            "f32" => self.encoder.write_frame_f32(&sample_conv(frame)),
            "f64" => self.encoder.write_frame_f64(&sample_conv(frame)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_frames<S>(&mut self, frames: &[Vec<S>], channels: u16) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() { // 希望编译器能做到优化，省区字符串比对的过程。
            "i8"  => self.encoder.write_frames__i8(&sample_conv_batch(frames), channels),
            "i16" => self.encoder.write_frames_i16(&sample_conv_batch(frames), channels),
            "i24" => self.encoder.write_frames_i24(&sample_conv_batch(frames), channels),
            "i32" => self.encoder.write_frames_i32(&sample_conv_batch(frames), channels),
            "i64" => self.encoder.write_frames_i64(&sample_conv_batch(frames), channels),
            "u8"  => self.encoder.write_frames__u8(&sample_conv_batch(frames), channels),
            "u16" => self.encoder.write_frames_u16(&sample_conv_batch(frames), channels),
            "u24" => self.encoder.write_frames_u24(&sample_conv_batch(frames), channels),
            "u32" => self.encoder.write_frames_u32(&sample_conv_batch(frames), channels),
            "u64" => self.encoder.write_frames_u64(&sample_conv_batch(frames), channels),
            "f32" => self.encoder.write_frames_f32(&sample_conv_batch(frames), channels),
            "f64" => self.encoder.write_frames_f64(&sample_conv_batch(frames), channels),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_sample<S>(&mut self, mono: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_sample__i8(mono.to_i8() ),
            "i16" => self.encoder.write_sample_i16(mono.to_i16()),
            "i24" => self.encoder.write_sample_i24(mono.to_i24()),
            "i32" => self.encoder.write_sample_i32(mono.to_i32()),
            "i64" => self.encoder.write_sample_i64(mono.to_i64()),
            "u8"  => self.encoder.write_sample__u8(mono.to_u8() ),
            "u16" => self.encoder.write_sample_u16(mono.to_u16()),
            "u24" => self.encoder.write_sample_u24(mono.to_u24()),
            "u32" => self.encoder.write_sample_u32(mono.to_u32()),
            "u64" => self.encoder.write_sample_u64(mono.to_u64()),
            "f32" => self.encoder.write_sample_f32(mono.to_f32()),
            "f64" => self.encoder.write_sample_f64(mono.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_mono_channel<S>(&mut self, monos: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_samples__i8(&sample_conv(monos)),
            "i16" => self.encoder.write_samples_i16(&sample_conv(monos)),
            "i24" => self.encoder.write_samples_i24(&sample_conv(monos)),
            "i32" => self.encoder.write_samples_i32(&sample_conv(monos)),
            "i64" => self.encoder.write_samples_i64(&sample_conv(monos)),
            "u8"  => self.encoder.write_samples__u8(&sample_conv(monos)),
            "u16" => self.encoder.write_samples_u16(&sample_conv(monos)),
            "u24" => self.encoder.write_samples_u24(&sample_conv(monos)),
            "u32" => self.encoder.write_samples_u32(&sample_conv(monos)),
            "u64" => self.encoder.write_samples_u64(&sample_conv(monos)),
            "f32" => self.encoder.write_samples_f32(&sample_conv(monos)),
            "f64" => self.encoder.write_samples_f64(&sample_conv(monos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_dual_mono<S>(&mut self, mono1: S, mono2: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_dual_sample__i8(mono1.to_i8() , mono2.to_i8() ),
            "i16" => self.encoder.write_dual_sample_i16(mono1.to_i16(), mono2.to_i16()),
            "i24" => self.encoder.write_dual_sample_i24(mono1.to_i24(), mono2.to_i24()),
            "i32" => self.encoder.write_dual_sample_i32(mono1.to_i32(), mono2.to_i32()),
            "i64" => self.encoder.write_dual_sample_i64(mono1.to_i64(), mono2.to_i64()),
            "u8"  => self.encoder.write_dual_sample__u8(mono1.to_u8() , mono2.to_u8() ),
            "u16" => self.encoder.write_dual_sample_u16(mono1.to_u16(), mono2.to_u16()),
            "u24" => self.encoder.write_dual_sample_u24(mono1.to_u24(), mono2.to_u24()),
            "u32" => self.encoder.write_dual_sample_u32(mono1.to_u32(), mono2.to_u32()),
            "u64" => self.encoder.write_dual_sample_u64(mono1.to_u64(), mono2.to_u64()),
            "f32" => self.encoder.write_dual_sample_f32(mono1.to_f32(), mono2.to_f32()),
            "f64" => self.encoder.write_dual_sample_f64(mono1.to_f64(), mono2.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_dual_monos<S>(&mut self, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_dual_monos__i8(&sample_conv(mono1), &sample_conv(mono2)),
            "i16" => self.encoder.write_dual_monos_i16(&sample_conv(mono1), &sample_conv(mono2)),
            "i24" => self.encoder.write_dual_monos_i24(&sample_conv(mono1), &sample_conv(mono2)),
            "i32" => self.encoder.write_dual_monos_i32(&sample_conv(mono1), &sample_conv(mono2)),
            "i64" => self.encoder.write_dual_monos_i64(&sample_conv(mono1), &sample_conv(mono2)),
            "u8"  => self.encoder.write_dual_monos__u8(&sample_conv(mono1), &sample_conv(mono2)),
            "u16" => self.encoder.write_dual_monos_u16(&sample_conv(mono1), &sample_conv(mono2)),
            "u24" => self.encoder.write_dual_monos_u24(&sample_conv(mono1), &sample_conv(mono2)),
            "u32" => self.encoder.write_dual_monos_u32(&sample_conv(mono1), &sample_conv(mono2)),
            "u64" => self.encoder.write_dual_monos_u64(&sample_conv(mono1), &sample_conv(mono2)),
            "f32" => self.encoder.write_dual_monos_f32(&sample_conv(mono1), &sample_conv(mono2)),
            "f64" => self.encoder.write_dual_monos_f64(&sample_conv(mono1), &sample_conv(mono2)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_monos<S>(&mut self, monos: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_monos__i8(&sample_conv_batch(monos)),
            "i16" => self.encoder.write_monos_i16(&sample_conv_batch(monos)),
            "i24" => self.encoder.write_monos_i24(&sample_conv_batch(monos)),
            "i32" => self.encoder.write_monos_i32(&sample_conv_batch(monos)),
            "i64" => self.encoder.write_monos_i64(&sample_conv_batch(monos)),
            "u8"  => self.encoder.write_monos__u8(&sample_conv_batch(monos)),
            "u16" => self.encoder.write_monos_u16(&sample_conv_batch(monos)),
            "u24" => self.encoder.write_monos_u24(&sample_conv_batch(monos)),
            "u32" => self.encoder.write_monos_u32(&sample_conv_batch(monos)),
            "u64" => self.encoder.write_monos_u64(&sample_conv_batch(monos)),
            "f32" => self.encoder.write_monos_f32(&sample_conv_batch(monos)),
            "f64" => self.encoder.write_monos_f64(&sample_conv_batch(monos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_stereo<S>(&mut self, stereo: (S, S)) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_stereo__i8(stereo_conv(stereo)),
            "i16" => self.encoder.write_stereo_i16(stereo_conv(stereo)),
            "i24" => self.encoder.write_stereo_i24(stereo_conv(stereo)),
            "i32" => self.encoder.write_stereo_i32(stereo_conv(stereo)),
            "i64" => self.encoder.write_stereo_i64(stereo_conv(stereo)),
            "u8"  => self.encoder.write_stereo__u8(stereo_conv(stereo)),
            "u16" => self.encoder.write_stereo_u16(stereo_conv(stereo)),
            "u24" => self.encoder.write_stereo_u24(stereo_conv(stereo)),
            "u32" => self.encoder.write_stereo_u32(stereo_conv(stereo)),
            "u64" => self.encoder.write_stereo_u64(stereo_conv(stereo)),
            "f32" => self.encoder.write_stereo_f32(stereo_conv(stereo)),
            "f64" => self.encoder.write_stereo_f64(stereo_conv(stereo)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_stereos<S>(&mut self, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_stereos__i8(&stereos_conv(stereos)),
            "i16" => self.encoder.write_stereos_i16(&stereos_conv(stereos)),
            "i24" => self.encoder.write_stereos_i24(&stereos_conv(stereos)),
            "i32" => self.encoder.write_stereos_i32(&stereos_conv(stereos)),
            "i64" => self.encoder.write_stereos_i64(&stereos_conv(stereos)),
            "u8"  => self.encoder.write_stereos__u8(&stereos_conv(stereos)),
            "u16" => self.encoder.write_stereos_u16(&stereos_conv(stereos)),
            "u24" => self.encoder.write_stereos_u24(&stereos_conv(stereos)),
            "u32" => self.encoder.write_stereos_u32(&stereos_conv(stereos)),
            "u64" => self.encoder.write_stereos_u64(&stereos_conv(stereos)),
            "f32" => self.encoder.write_stereos_f32(&stereos_conv(stereos)),
            "f64" => self.encoder.write_stereos_f64(&stereos_conv(stereos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }
}

// `PcmEncoderFrom<S>`: Transcodes samples from type `S` into the target format.
#[derive(Debug, Clone, Copy)]
struct PcmEncoderFrom<S>
where S: SampleType {
    write_fn: fn(&mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>,
}

impl<S> PcmEncoderFrom<S>
where S: SampleType {
    pub fn new(target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        Ok(Self {
            write_fn: match target_sample{
                S8  => Self::write_sample_to::<i8 >,
                S16 => Self::write_sample_to::<i16>,
                S24 => Self::write_sample_to::<i24>,
                S32 => Self::write_sample_to::<i32>,
                S64 => Self::write_sample_to::<i64>,
                U8  => Self::write_sample_to::<u8 >,
                U16 => Self::write_sample_to::<u16>,
                U24 => Self::write_sample_to::<u24>,
                U32 => Self::write_sample_to::<u32>,
                U64 => Self::write_sample_to::<u64>,
                F32 => Self::write_sample_to::<f32>,
                F64 => Self::write_sample_to::<f64>,
                other => return Err(AudioWriteError::InvalidArguments(format!("Unknown target sample type: \"{:?}\"", other))),
            },
        })
    }

    // S: The input format provided to us (external source).
    // T: The target format to be written into the WAV file.
    fn write_sample_to<T>(writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>
    where T: SampleType {
        for sample in frame.iter() {
            T::from(*sample).write_le(writer)?;
        }
        Ok(())
    }

    pub fn write_frame(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError> {
        (self.write_fn)(writer, frame)
    }

    pub fn write_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError> {
        for frame in frames.iter() {
            (self.write_fn)(writer, frame)?;
        }
        Ok(())
    }

    pub fn write_samples(&mut self, writer: &mut dyn Writer, samples: &[S]) -> Result<(), AudioWriteError> {
        (self.write_fn)(writer, samples)
    }
}

#[derive(Debug)]
pub struct PcmEncoder<'a> {
    spec: Spec,
    sample_type: WaveSampleType,
    writer: &'a mut dyn Writer,
    writer_from__i8: PcmEncoderFrom< i8>,
    writer_from_i16: PcmEncoderFrom<i16>,
    writer_from_i24: PcmEncoderFrom<i24>,
    writer_from_i32: PcmEncoderFrom<i32>,
    writer_from_i64: PcmEncoderFrom<i64>,
    writer_from__u8: PcmEncoderFrom< u8>,
    writer_from_u16: PcmEncoderFrom<u16>,
    writer_from_u24: PcmEncoderFrom<u24>,
    writer_from_u32: PcmEncoderFrom<u32>,
    writer_from_u64: PcmEncoderFrom<u64>,
    writer_from_f32: PcmEncoderFrom<f32>,
    writer_from_f64: PcmEncoderFrom<f64>,
}

impl<'a> PcmEncoder<'a> {
    // target_sample: The specific PCM format (e.g., bit depth, signedness) to encode into the WAV file.
    pub fn new(writer: &'a mut dyn Writer, spec: Spec) -> Result<Self, AudioWriteError> {
        if spec.is_channel_mask_valid() == false {
            return Err(AudioWriteError::InvalidArguments(format!("Number of bits of channel mask 0x{:08x} does not match {} channels", spec.channel_mask, spec.channels)));
        }
        let target_sample = spec.get_sample_type();
        Ok(Self {
            spec,
            sample_type: target_sample,
            writer,
            writer_from__i8: PcmEncoderFrom::< i8>::new(target_sample)?,
            writer_from_i16: PcmEncoderFrom::<i16>::new(target_sample)?,
            writer_from_i24: PcmEncoderFrom::<i24>::new(target_sample)?,
            writer_from_i32: PcmEncoderFrom::<i32>::new(target_sample)?,
            writer_from_i64: PcmEncoderFrom::<i64>::new(target_sample)?,
            writer_from__u8: PcmEncoderFrom::< u8>::new(target_sample)?,
            writer_from_u16: PcmEncoderFrom::<u16>::new(target_sample)?,
            writer_from_u24: PcmEncoderFrom::<u24>::new(target_sample)?,
            writer_from_u32: PcmEncoderFrom::<u32>::new(target_sample)?,
            writer_from_u64: PcmEncoderFrom::<u64>::new(target_sample)?,
            writer_from_f32: PcmEncoderFrom::<f32>::new(target_sample)?,
            writer_from_f64: PcmEncoderFrom::<f64>::new(target_sample)?,
        })
    }
}

impl<'a> EncoderToImpl for PcmEncoder<'_> {
    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }
    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64, Unknown};
        let bytes_per_sample = self.spec.bits_per_sample / 8;
        let byte_rate = self.spec.sample_rate * self.spec.channels as u32 * bytes_per_sample as u32;
        let extensible = match self.spec.channel_mask {
            0 => None,
            channel_mask => Some(FmtExtension::new_extensible(ExtensibleData {
                valid_bits_per_sample: self.spec.bits_per_sample,
                channel_mask,
                sub_format: match self.sample_type {
                    U8 | S16 | S24 | S32 | S64 => GUID_PCM_FORMAT,
                    F32 | F64 => GUID_IEEE_FLOAT_FORMAT,
                    other => return Err(AudioWriteError::Unsupported(format!("\"{:?}\" was given for the extensible format PCM to specify the sample format", other))),
                },
            })),
        };
        Ok(FmtChunk {
            format_tag: if extensible.is_some() {
                0xFFFE
            } else {
                match self.sample_type {
                    S8 | U16 | U24 | U32 | U64 => return Err(AudioWriteError::Unsupported(format!("PCM format does not support {} samples.", self.sample_type))),
                    U8 | S16 | S24 | S32 | S64 => 1,
                    F32 | F64 => 3,
                    Unknown => panic!("Can't encode \"Unknown\" format to PCM."),
                }
            },
            channels: self.spec.channels,
            sample_rate: self.spec.sample_rate,
            byte_rate,
            block_align: bytes_per_sample * self.spec.channels,
            bits_per_sample: self.spec.bits_per_sample,
            extension: extensible,
        })
    }

    fn get_bitrate(&self) -> u32 {
        self.spec.channels as u32 * self.spec.sample_rate * self.sample_type.sizeof() as u32 * 8
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn finish(&mut self, ) -> Result<(), AudioWriteError> {
        Ok(self.writer.flush()?)
    }

    fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.writer_from__i8.write_samples(self.writer, samples)}
    fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.writer_from_i16.write_samples(self.writer, samples)}
    fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.writer_from_i24.write_samples(self.writer, samples)}
    fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.writer_from_i32.write_samples(self.writer, samples)}
    fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.writer_from_i64.write_samples(self.writer, samples)}
    fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.writer_from__u8.write_samples(self.writer, samples)}
    fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.writer_from_u16.write_samples(self.writer, samples)}
    fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.writer_from_u24.write_samples(self.writer, samples)}
    fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.writer_from_u32.write_samples(self.writer, samples)}
    fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.writer_from_u64.write_samples(self.writer, samples)}
    fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.writer_from_f32.write_samples(self.writer, samples)}
    fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.writer_from_f64.write_samples(self.writer, samples)}
}

#[derive(Debug)]
pub struct AdpcmEncoderWrap<'a, E>
where E: adpcm::AdpcmEncoder {
    writer: &'a mut dyn Writer,
    channels: u16,
    sample_rate: u32,
    bytes_written: u64,
    encoder: E,
    nibbles: Vec<u8>,
}

const MAX_BUFFER_USAGE: usize = 1024;

impl<'a, E> AdpcmEncoderWrap<'a, E>
where E: adpcm::AdpcmEncoder {
    pub fn new(writer: &'a mut dyn Writer, spec: Spec) -> Result<Self, AudioWriteError> {
        Ok(Self {
            writer,
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            bytes_written: 0,
            encoder: E::new(spec.channels)?,
            nibbles: Vec::<u8>::with_capacity(MAX_BUFFER_USAGE),
        })
    }

    fn flush_buffers(&mut self) -> Result<(), AudioWriteError> {
        self.writer.write_all(&self.nibbles)?;

        // Avoid using `clear()`. If a user writes a large batch of samples once, 
        // `clear()` retains the original capacity without shrinking it, leading to persistent memory usage.
        self.nibbles = Vec::<u8>::with_capacity(MAX_BUFFER_USAGE);
        Ok(())
    }

    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {
        let mut iter = samples.iter().copied();
        self.encoder.encode(|| -> Option<i16> { iter.next()}, |byte: u8|{ self.nibbles.push(byte); })?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers()?;
        }
        Ok(())
    }

    pub fn write_stereos(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {
        if self.channels != 2 {
            return Err(AudioWriteError::Unsupported(format!("This encoder only accepts {} channel audio data", self.channels)));
        }
        let mut iter = utils::stereos_to_interleaved_samples(stereos).into_iter();
        self.encoder.encode(|| -> Option<i16> {iter.next()}, |byte: u8|{ self.nibbles.push(byte);})?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers()?;
        }
        Ok(())
    }
}

impl<'a, E> EncoderToImpl for AdpcmEncoderWrap<'_, E>
where E: adpcm::AdpcmEncoder {
    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        Ok(self.encoder.new_fmt_chunk(self.channels, self.sample_rate, 4)?)
    }

    fn get_bitrate(&self) -> u32 {
        self.sample_rate * self.channels as u32 * 4
    }

    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(self.encoder.modify_fmt_chunk(fmt)?)
    }

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        self.encoder.flush(|nibble: u8|{ self.nibbles.push(nibble);})?;
        self.flush_buffers()?;
        Ok(self.writer.flush()?)
    }

    fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}

    fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
    fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
}

#[derive(Debug)]
pub struct PcmXLawEncoderWrap<'a> {
    writer: &'a mut dyn Writer,
    enc: PcmXLawEncoder,
    channels: u16,
    sample_rate: u32,
}

impl<'a> PcmXLawEncoderWrap<'a> {
    pub fn new(writer: &'a mut dyn Writer, spec: Spec, which_law: XLaw) -> Self {
        Self {
            writer,
            enc: PcmXLawEncoder::new(which_law),
            channels: spec.channels,
            sample_rate: spec.sample_rate,
        }
    }

    pub fn write_samples(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {
        self.writer.write_all(&samples.iter().map(|sample| -> u8 {self.enc.encode(*sample)}).collect::<Vec<u8>>())?;
        Ok(())
    }
}

impl<'a> EncoderToImpl for PcmXLawEncoderWrap<'_> {
    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        let bits_per_sample = 8u16;
        let block_align = self.channels;
        Ok(FmtChunk {
            format_tag: match self.enc.get_which_law() {
                XLaw::ALaw => 0x0006,
                XLaw::MuLaw => 0x0007,
            },
            channels: self.channels,
            sample_rate: self.sample_rate,
            byte_rate: self.sample_rate * bits_per_sample as u32 * self.channels as u32 / 8,
            block_align,
            bits_per_sample,
            extension: None,
        })
    }

    fn get_bitrate(&self) -> u32 {
        self.sample_rate * self.channels as u32 * 8
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        Ok(self.writer.flush()?)
    }

    fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
    fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
}

pub mod mp3 {
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mp3Channels {
        Mono = 3,
        Stereo = 0,
        JointStereo = 1,
        DualChannel = 2,
        NotSet = 4,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mp3Quality {
        Best = 0,
        SecondBest = 1,
        NearBest = 2,
        VeryNice = 3,
        Nice = 4,
        Good = 5,
        Decent = 6,
        Ok = 7,
        SecondWorst = 8,
        Worst = 9,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u16)]
    pub enum Mp3Bitrate {
        Kbps8 = 8,
        Kbps16 = 16,
        Kbps24 = 24,
        Kbps32 = 32,
        Kbps40 = 40,
        Kbps48 = 48,
        Kbps64 = 64,
        Kbps80 = 80,
        Kbps96 = 96,
        Kbps112 = 112,
        Kbps128 = 128,
        Kbps160 = 160,
        Kbps192 = 192,
        Kbps224 = 224,
        Kbps256 = 256,
        Kbps320 = 320,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mp3VbrMode {
        Off = 0,
        Mt = 1,
        Rh = 2,
        Abr = 3,
        Mtrh = 4,
    }

    const ID3_FIELD_LENGTH: usize = 250;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Mp3Id3Tag {
        pub title: [u8; ID3_FIELD_LENGTH],
        pub artist: [u8; ID3_FIELD_LENGTH],
        pub album: [u8; ID3_FIELD_LENGTH],
        pub album_art: [u8; ID3_FIELD_LENGTH],
        pub year: [u8; ID3_FIELD_LENGTH],
        pub comment: [u8; ID3_FIELD_LENGTH],
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Mp3EncoderOptions {
        pub channels: Mp3Channels,
        pub quality: Mp3Quality,
        pub bitrate: Mp3Bitrate,
        pub vbr_mode: Mp3VbrMode,
        pub id3tag: Option<Mp3Id3Tag>,
    }

    impl Mp3EncoderOptions {
        pub fn new() -> Self {
            Self {
                channels: Mp3Channels::NotSet,
                quality: Mp3Quality::Best,
                bitrate: Mp3Bitrate::Kbps320,
                vbr_mode: Mp3VbrMode::Off,
                id3tag: None,
            }
        }

        pub fn new_mono() -> Self {
            Self {
                channels: Mp3Channels::Mono,
                quality: Mp3Quality::Best,
                bitrate: Mp3Bitrate::Kbps320,
                vbr_mode: Mp3VbrMode::Off,
                id3tag: None,
            }
        }

        pub fn new_stereo() -> Self {
            Self {
                channels: Mp3Channels::JointStereo,
                quality: Mp3Quality::Best,
                bitrate: Mp3Bitrate::Kbps320,
                vbr_mode: Mp3VbrMode::Off,
                id3tag: None,
            }
        }

        pub fn get_channels(&self) -> u16 {
            match self.channels {
                Mp3Channels::Mono => 1,
                Mp3Channels::Stereo | Mp3Channels::DualChannel | Mp3Channels::JointStereo => 2,
                Mp3Channels::NotSet => 0,
            }
        }

        pub fn get_bitrate(&self) -> u32 {
            self.bitrate as u32 * 1000
        }
    }

    impl Default for Mp3EncoderOptions {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(feature = "mp3enc")]
    use super::EncoderToImpl;

    #[cfg(feature = "mp3enc")]
    pub mod impl_mp3 {
        use std::{any::type_name, fmt::{self, Debug, Formatter}, sync::{Arc, Mutex}, ops::DerefMut};
        use super::*;
        use crate::Writer;
        use crate::AudioWriteError;
        use crate::{SampleType, i24, u24};
        use crate::wavcore::{Spec, FmtChunk, FmtExtension, Mp3Data};
        use crate::utils::{self, sample_conv, stereos_conv};
        use crate::hacks;
        use mp3lame_encoder::{Builder, Encoder, MonoPcm, DualPcm, FlushNoGap};
        use mp3lame_encoder::{Mode, Quality, Bitrate, VbrMode, Id3Tag};

        const MAX_SAMPLES_TO_ENCODE: usize = 1024;

        #[derive(Clone)]
        pub struct SharedMp3Encoder(Arc<Mutex<Encoder>>);

        impl SharedMp3Encoder{
            pub fn new(encoder: Encoder) -> Self {
                Self(Arc::new(Mutex::new(encoder)))
            }

            pub fn escorted_encode<T, F, E>(&self, mut action: F) -> Result<T, E>
            where F: FnMut(&mut Encoder) -> Result<T, E> {
                let mut guard = self.0.lock().unwrap();
                let encoder = guard.deref_mut();
                (action)(encoder)
            }
        }

        impl Mp3Channels {
            pub fn to_lame_mode(&self) -> Mode {
                match self {
                    Self::Mono => Mode::Mono,
                    Self::Stereo => Mode::Stereo,
                    Self::JointStereo => Mode::JointStereo,
                    Self::DualChannel => Mode::DaulChannel,
                    Self::NotSet => Mode::NotSet,
                }
            }
        }

        impl Mp3Quality {
            pub fn to_lame_quality(&self) -> Quality {
                match self {
                    Self::Best => Quality::Best,
                    Self::SecondBest => Quality::SecondBest,
                    Self::NearBest => Quality::NearBest,
                    Self::VeryNice => Quality::VeryNice,
                    Self::Nice => Quality::Nice,
                    Self::Good => Quality::Good,
                    Self::Decent => Quality::Decent,
                    Self::Ok => Quality::Ok,
                    Self::SecondWorst => Quality::SecondWorst,
                    Self::Worst => Quality::Worst,
                }
            }
        }

        impl Mp3Bitrate {
            pub fn to_lame_bitrate(&self) -> Bitrate {
                match self {
                    Self::Kbps8 => Bitrate::Kbps8,
                    Self::Kbps16 => Bitrate::Kbps16,
                    Self::Kbps24 => Bitrate::Kbps24,
                    Self::Kbps32 => Bitrate::Kbps32,
                    Self::Kbps40 => Bitrate::Kbps40,
                    Self::Kbps48 => Bitrate::Kbps48,
                    Self::Kbps64 => Bitrate::Kbps64,
                    Self::Kbps80 => Bitrate::Kbps80,
                    Self::Kbps96 => Bitrate::Kbps96,
                    Self::Kbps112 => Bitrate::Kbps112,
                    Self::Kbps128 => Bitrate::Kbps128,
                    Self::Kbps160 => Bitrate::Kbps160,
                    Self::Kbps192 => Bitrate::Kbps192,
                    Self::Kbps224 => Bitrate::Kbps224,
                    Self::Kbps256 => Bitrate::Kbps256,
                    Self::Kbps320 => Bitrate::Kbps320,
                }
            }
        }

        impl Mp3VbrMode {
            pub fn to_lame_vbr_mode(&self) -> VbrMode {
                match self {
                    Self::Off => VbrMode::Off,
                    Self::Mt => VbrMode::Mt,
                    Self::Rh => VbrMode::Rh,
                    Self::Abr => VbrMode::Abr,
                    Self::Mtrh => VbrMode::Mtrh,
                }
            }
        }

        #[derive(Clone)]
        pub struct Mp3EncoderLameOptions {
            channels: Mode,
            quality: Quality,
            bitrate: Bitrate,
            vbr_mode: VbrMode,
            id3tag: Option<Mp3Id3Tag>,
        }

        impl Mp3EncoderOptions {
            pub fn to_lame_options(&self) -> Mp3EncoderLameOptions{
                Mp3EncoderLameOptions {
                    channels: self.channels.to_lame_mode(),
                    quality:  self.quality.to_lame_quality(),
                    bitrate:  self.bitrate.to_lame_bitrate(),
                    vbr_mode: self.vbr_mode.to_lame_vbr_mode(),
                    id3tag: self.id3tag,
                }
            }
        }

        #[derive(Debug)]
        pub struct Mp3Encoder<'a, S>
        where S: SampleType {
            channels: u16,
            sample_rate: u32,
            bitrate: u32,
            encoder: SharedMp3Encoder,
            encoder_options: Mp3EncoderOptions,
            buffers: ChannelBuffers<'a, S>,
        }

        impl<'a, S> Mp3Encoder<'a, S>
        where S: SampleType {
            pub fn new(mut writer: &'a mut dyn Writer, spec: Spec, mp3_options: &Mp3EncoderOptions) -> Result<Self, AudioWriteError> {
                if spec.channels != mp3_options.get_channels() {
                    return Err(AudioWriteError::InvalidArguments(format!("The number of channels from `spec` is {}, but from `mp3_options` is {}", spec.channels, mp3_options.get_channels())));
                }

                let mp3_builder = Builder::new();
                let mut mp3_builder = match mp3_builder {
                    Some(mp3_builder) => mp3_builder,
                    None => return Err(AudioWriteError::OtherReason("`lame_init()` somehow failed.".to_owned())),
                };
                let options = mp3_options.to_lame_options();

                mp3_builder.set_mode(options.channels)?;
                mp3_builder.set_sample_rate(spec.sample_rate)?;
                mp3_builder.set_brate(options.bitrate)?;
                mp3_builder.set_quality(options.quality)?;
                mp3_builder.set_vbr_mode(options.vbr_mode)?;

                match options.vbr_mode {
                    VbrMode::Off => mp3_builder.set_to_write_vbr_tag(false)?,
                    _ => {
                        mp3_builder.set_to_write_vbr_tag(true)?;
                        mp3_builder.set_vbr_quality(options.quality)?;
                    }
                }

                if let Some(id3tag) = options.id3tag {
                    mp3_builder.set_id3_tag(Id3Tag{
                        title: &id3tag.title,
                        artist: &id3tag.artist,
                        album: &id3tag.album,
                        album_art: &id3tag.album_art,
                        year: &id3tag.year,
                        comment: &id3tag.comment,
                    })?;
                }

                let encoder = SharedMp3Encoder::new(mp3_builder.build()?);

                let channels = mp3_options.get_channels();
                Ok(Self {
                    channels,
                    sample_rate: spec.sample_rate,
                    bitrate: mp3_options.get_bitrate(),
                    encoder: encoder.clone(),
                    encoder_options: *mp3_options,
                    buffers: match channels {
                        1 | 2 => ChannelBuffers::<'a, S>::new(hacks::force_borrow!(writer, dyn Writer), encoder.clone(), MAX_SAMPLES_TO_ENCODE, channels)?,
                        o => return Err(AudioWriteError::Unsupported(format!("Bad channel number: {o}"))),
                    },
                })
            }

            pub fn write_samples<T>(&mut self, samples: &[T]) -> Result<(), AudioWriteError>
            where T: SampleType {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels {
                    1 => self.buffers.add_monos(&sample_conv::<T, S>(samples)),
                    2 => self.buffers.add_stereos(&utils::interleaved_samples_to_stereos(&sample_conv::<T, S>(samples))?),
                    o => Err(AudioWriteError::Unsupported(format!("Bad channels number: {o}"))),
                }
            }

            pub fn write_stereos<T>(&mut self, stereos: &[(T, T)]) -> Result<(), AudioWriteError>
            where T: SampleType {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels{
                    1 => self.buffers.add_monos(&utils::stereos_to_monos(&stereos_conv::<T, S>(stereos))),
                    2 => self.buffers.add_stereos(&stereos_conv::<T, S>(stereos)),
                    o => Err(AudioWriteError::InvalidArguments(format!("Bad channels number: {o}"))),
                }
            }

            pub fn write_dual_monos<T>(&mut self, mono_l: &[T], mono_r: &[T]) -> Result<(), AudioWriteError>
            where T: SampleType {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels{
                    1 => self.buffers.add_monos(&sample_conv::<T, S>(&utils::dual_monos_to_monos(&(mono_l.to_vec(), mono_r.to_vec()))?)),
                    2 => self.buffers.add_dual_monos(&sample_conv::<T, S>(mono_l), &sample_conv::<T, S>(mono_r)),
                    o => Err(AudioWriteError::InvalidArguments(format!("Bad channels number: {o}"))),
                }
            }

            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.buffers.finish()
            }
        }

        #[derive(Debug, Clone)]
        enum Channels<S>
        where S: SampleType {
            Mono(Vec<S>),
            Stereo((Vec<S>, Vec<S>)),
        }

        struct ChannelBuffers<'a, S>
        where S: SampleType {
            writer: &'a mut dyn Writer,
            encoder: SharedMp3Encoder,
            channels: Channels<S>,
            max_frames: usize,
        }

        impl<S> Channels<S>
        where S: SampleType {
            pub fn new_mono(max_frames: usize) -> Self {
                Self::Mono(Vec::<S>::with_capacity(max_frames))
            }
            pub fn new_stereo(max_frames: usize) -> Self {
                Self::Stereo((Vec::<S>::with_capacity(max_frames), Vec::<S>::with_capacity(max_frames)))
            }
            pub fn add_mono(&mut self, frame: S) {
                match self {
                    Self::Mono(m) => m.push(frame),
                    Self::Stereo((l, r)) => {
                        l.push(frame);
                        r.push(frame);
                    },
                }
            }
            pub fn add_stereo(&mut self, frame: (S, S)) {
                match self {
                    Self::Mono(m) => m.push(S::average(frame.0, frame.1)),
                    Self::Stereo((l, r)) => {
                        l.push(frame.0);
                        r.push(frame.1);
                    },
                }
            }
            pub fn add_monos(&mut self, frames: &[S]) {
                match self {
                    Self::Mono(m) => m.extend(frames),
                    Self::Stereo((l, r)) => {
                        l.extend(frames);
                        r.extend(frames);
                    },
                }
            }
            pub fn add_stereos(&mut self, frames: &[(S, S)]) {
                match self {
                    Self::Mono(m) => m.extend(utils::stereos_to_monos(frames)),
                    Self::Stereo((l, r)) => {
                        let (il, ir) = utils::stereos_to_dual_monos(frames);
                        l.extend(il);
                        r.extend(ir);
                    },
                }
            }
            pub fn add_dual_monos(&mut self, monos_l: &[S], monos_r: &[S]) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(m) => m.extend(utils::dual_monos_to_monos(&(monos_l.to_vec(), monos_r.to_vec()))?),
                    Self::Stereo((l, r)) => {
                        if monos_l.len() != monos_r.len() {
                            return Err(AudioWriteError::MultipleMonosAreNotSameSize);
                        }
                        l.extend(monos_l);
                        r.extend(monos_r);
                    },
                }
                Ok(())
            }
            pub fn len(&self) -> usize {
                match self {
                    Self::Mono(m) => m.len(),
                    Self::Stereo((l, r)) => {
                        assert_eq!(l.len(), r.len());
                        l.len()
                    },
                }
            }
            pub fn is_empty(&self) -> bool {
                match self {
                    Self::Mono(m) => m.is_empty(),
                    Self::Stereo((l, r)) => l.is_empty() && r.is_empty(),
                }
            }
            pub fn get_channels(&self) -> u16 {
                match self {
                    Self::Mono(_) => 1,
                    Self::Stereo(_) => 2,
                }
            }
            pub fn clear(&mut self, max_frames: usize) {
                match self {
                    Self::Mono(ref mut m) => *m = Vec::<S>::with_capacity(max_frames),
                    Self::Stereo(ref mut s) => *s = (Vec::<S>::with_capacity(max_frames), Vec::<S>::with_capacity(max_frames)),
                }
            }
        }

        impl<'a, S> ChannelBuffers<'a, S>
        where S: SampleType{
            pub fn new(writer: &'a mut dyn Writer, encoder: SharedMp3Encoder, max_frames: usize, channels: u16) -> Result<Self, AudioWriteError> {
                Ok(Self {
                    writer,
                    encoder,
                    channels: match channels {
                        1 => Channels::<S>::new_mono(max_frames),
                        2 => Channels::<S>::new_stereo(max_frames),
                        o => return Err(AudioWriteError::InvalidArguments(format!("Invalid channels: {o}. Only 1 and 2 are accepted."))),
                    },
                    max_frames,
                })
            }

            pub fn is_full(&self) -> bool {
                self.channels.len() >= self.max_frames
            }

            pub fn add_monos(&mut self, monos: &[S]) -> Result<(), AudioWriteError> {
                self.channels.add_monos(monos);
                if self.is_full() {
                    self.flush()?;
                }
                Ok(())
            }

            pub fn add_stereos(&mut self, stereos: &[(S, S)]) -> Result<(), AudioWriteError> {
                self.channels.add_stereos(stereos);
                if self.is_full() {
                    self.flush()?;
                }
                Ok(())
            }

            pub fn add_dual_monos(&mut self, monos_l: &[S], monos_r: &[S]) -> Result<(), AudioWriteError> {
                self.channels.add_dual_monos(monos_l, monos_r)?;
                if self.is_full() {
                    self.flush()?;
                }
                Ok(())
            }

            fn channel_to_type<T>(mono: &[S]) -> Vec<T>
            where T: SampleType {
                mono.iter().map(|s|{T::from(*s)}).collect()
            }

            fn encode_to_vec(&self, encoder: &mut Encoder, out_buf :&mut Vec<u8>) -> Result<usize, AudioWriteError> {
                // Explicitly converts all samples (even natively supported i16/u16/i32/f32/f64) for pipeline uniformity.
                match &self.channels {
                    Channels::Mono(pcm) => {
                        match std::any::type_name::<S>() {
                            "i16" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<i16>(pcm)), out_buf)?),
                            "u16" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<u16>(pcm)), out_buf)?),
                            "i32" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<i32>(pcm)), out_buf)?),
                            "f32" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<f32>(pcm)), out_buf)?),
                            "f64" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<f64>(pcm)), out_buf)?),
                            "i8"  => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<i16>(pcm)), out_buf)?),
                            "u8"  => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<u16>(pcm)), out_buf)?),
                            "i24" | "u24" | "u32" => Ok(encoder.encode_to_vec(MonoPcm(&Self::channel_to_type::<i32>(pcm)), out_buf)?),
                            other => Err(AudioWriteError::Unsupported(format!("\"{other}\""))),
                        }
                    },
                    Channels::Stereo(pcm) => {
                        match std::any::type_name::<S>() {
                            "i16" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<i16>(&pcm.0), right: &Self::channel_to_type::<i16>(&pcm.1)}, out_buf)?),
                            "u16" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<u16>(&pcm.0), right: &Self::channel_to_type::<u16>(&pcm.1)}, out_buf)?),
                            "i32" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<i32>(&pcm.0), right: &Self::channel_to_type::<i32>(&pcm.1)}, out_buf)?),
                            "f32" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<f32>(&pcm.0), right: &Self::channel_to_type::<f32>(&pcm.1)}, out_buf)?),
                            "f64" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<f64>(&pcm.0), right: &Self::channel_to_type::<f64>(&pcm.1)}, out_buf)?),
                            "i8"  => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<i16>(&pcm.0), right: &Self::channel_to_type::<i16>(&pcm.1)}, out_buf)?),
                            "u8"  => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<u16>(&pcm.0), right: &Self::channel_to_type::<u16>(&pcm.1)}, out_buf)?),
                            "i24" | "u24" | "u32" => Ok(encoder.encode_to_vec(DualPcm{left: &Self::channel_to_type::<i32>(&pcm.0), right: &Self::channel_to_type::<i32>(&pcm.1)}, out_buf)?),
                            other => Err(AudioWriteError::Unsupported(format!("\"{other}\""))),
                        }
                    },
                }
            }

            pub fn flush(&mut self) -> Result<(), AudioWriteError> {
                if self.channels.is_empty() {
                    return Ok(())
                }
                let to_save = self.encoder.escorted_encode(|encoder| -> Result<Vec<u8>, AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.channels.len()));
                    self.encode_to_vec(encoder, &mut to_save)?;
                    Ok(to_save)
                })?;
                self.writer.write_all(&to_save)?;
                self.channels.clear(self.max_frames);
                Ok(())
            }

            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.flush()?;
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_frames));
                    encoder.flush_to_vec::<FlushNoGap>(&mut to_save)?;
                    self.writer.write_all(&to_save)?;
                    Ok(())
                })?;
                self.channels.clear(self.max_frames);
                Ok(())
            }
        }

        impl<'a, S> EncoderToImpl for Mp3Encoder<'_, S>
        where S: SampleType {
            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                Ok(FmtChunk{
                    format_tag: 0x0055,
                    channels: self.channels,
                    sample_rate: self.sample_rate,
                    byte_rate: self.bitrate / 8,
                    block_align: 1,
                    bits_per_sample: 0,
                    extension: Some(FmtExtension::new_mp3(Mp3Data::new(self.bitrate, self.sample_rate))),
                })
            }

            fn get_bitrate(&self) -> u32 {
                self.bitrate * self.channels as u32
            }

            fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.finish()
            }

            fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(samples)}
            fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(samples)}

            fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}
            fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_stereos(stereos)}

            fn write_dual_monos__i8(&mut self, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_i16(&mut self, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_i24(&mut self, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_i32(&mut self, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_i64(&mut self, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos__u8(&mut self, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_u16(&mut self, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_u24(&mut self, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_u32(&mut self, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_u64(&mut self, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_f32(&mut self, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
            fn write_dual_monos_f64(&mut self, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_dual_monos(mono1, mono2)}
        }

        impl Debug for SharedMp3Encoder {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct("SharedMp3Encoder")
                    .finish_non_exhaustive()
            }
        }

        impl<'a, S> Debug for ChannelBuffers<'_, S>
        where S: SampleType {
            fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.debug_struct(&format!("ChannelBuffers<{}>", type_name::<S>()))
                    .field("encoder", &self.encoder)
                    .field("channels", &format_args!("{}", match self.channels {
                        Channels::Mono(_) => "Mono",
                        Channels::Stereo(_) => "Stereo",
                    }))
                    .field("max_frames", &self.max_frames)
                    .finish()
            }
        }
    }

    #[cfg(feature = "mp3enc")]
    pub use impl_mp3::*;
}

pub mod opus {
    const OPUS_ALLOWED_SAMPLE_RATES: [u32; 5] = [8000, 12000, 16000, 24000, 48000];
    const OPUS_MIN_SAMPLE_RATE: u32 = 8000;
    const OPUS_MAX_SAMPLE_RATE: u32 = 48000;

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u32)]
    pub enum OpusEncoderSampleDuration {
        MilliSec2_5 = 25,
        MilliSec5 = 50,
        MilliSec10 = 100,
        MilliSec20 = 200,
        MilliSec40 = 400,
        MilliSec60 = 600,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub enum OpusBitrate {
        Bits(i32),
        Max,
        Auto,
    }

    impl OpusEncoderSampleDuration {
        pub fn get_num_samples(&self, channels: u16, sample_rate: u32) -> usize {
            let ms_m_10 = match self {
                Self::MilliSec2_5 => 25,
                Self::MilliSec5 => 50,
                Self::MilliSec10 => 100,
                Self::MilliSec20 => 200,
                Self::MilliSec40 => 400,
                Self::MilliSec60 => 600,
            };
            (sample_rate as usize * ms_m_10 as usize) * channels as usize / 10000
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct OpusEncoderOptions {
        pub bitrate: OpusBitrate,
        pub encode_vbr: bool,
        pub samples_cache_duration: OpusEncoderSampleDuration,
    }

    impl OpusEncoderOptions {
        pub fn new() -> Self {
            Self {
                bitrate: OpusBitrate::Max,
                encode_vbr: false,
                samples_cache_duration: OpusEncoderSampleDuration::MilliSec60,
            }
        }

        pub fn get_allowed_sample_rates(&self) -> [u32; OPUS_ALLOWED_SAMPLE_RATES.len()] {
            OPUS_ALLOWED_SAMPLE_RATES
        }

        pub fn get_rounded_up_sample_rate(&self, sample_rate: u32) -> u32 {
            if sample_rate <= OPUS_MIN_SAMPLE_RATE {
                OPUS_MIN_SAMPLE_RATE
            } else if sample_rate >= OPUS_MAX_SAMPLE_RATE {
                OPUS_MAX_SAMPLE_RATE
            } else {
                for (l, h) in OPUS_ALLOWED_SAMPLE_RATES[..OPUS_ALLOWED_SAMPLE_RATES.len() - 1].iter().zip(OPUS_ALLOWED_SAMPLE_RATES[1..].iter()) {
                    if sample_rate > *l && sample_rate <= *h {
                        return *h;
                    }
                }
                OPUS_MAX_SAMPLE_RATE
            }
        }
    }

    impl Default for OpusEncoderOptions {
        fn default() -> Self {
            Self::new()
        }
    }

    #[cfg(feature = "opus")]
    use super::EncoderToImpl;

    #[cfg(feature = "opus")]
    pub mod impl_opus {
        use std::{mem, fmt::{self, Debug, Formatter}};

        use super::*;
        use crate::Writer;
        use crate::wavcore::{Spec, FmtChunk};
        use crate::AudioWriteError;
        use crate::{i24, u24};
        use crate::utils::sample_conv;

        use opus::{Encoder, Application, Channels, Bitrate};

        impl OpusBitrate {
            pub fn to_opus_bitrate(&self) -> Bitrate {
                match self {
                    Self::Bits(bitrate) => Bitrate::Bits(*bitrate),
                    Self::Max => Bitrate::Max,
                    Self::Auto => Bitrate::Auto,
                }
            }
        }

        pub struct OpusEncoder<'a> {
            writer: &'a mut dyn Writer,
            encoder: Encoder,
            channels: u16,
            sample_rate: u32,
            cache_duration: OpusEncoderSampleDuration,
            num_samples_per_encode: usize,
            sample_cache: Vec<f32>,
            samples_written: u64,
            bytes_written: u64,
        }

        impl<'a> OpusEncoder<'a> {
            pub fn new(writer: &'a mut dyn Writer, spec: Spec, options: &OpusEncoderOptions) -> Result<Self, AudioWriteError> {
                let opus_channels = match spec.channels {
                    1 => Channels::Mono,
                    2 => Channels::Stereo,
                    o => return Err(AudioWriteError::InvalidArguments(format!("Bad channels: {o} for the opus encoder."))),
                };
                if !OPUS_ALLOWED_SAMPLE_RATES.contains(&spec.sample_rate) {
                    return Err(AudioWriteError::InvalidArguments(format!("Bad sample rate: {} for the opus encoder. The sample rate must be one of {}",
                        spec.sample_rate, OPUS_ALLOWED_SAMPLE_RATES.iter().map(|s|{format!("{s}")}).collect::<Vec<String>>().join(", ")
                    )));
                }
                let mut encoder = Encoder::new(spec.sample_rate, opus_channels, Application::Audio)?;
                encoder.set_bitrate(options.bitrate.to_opus_bitrate())?;
                encoder.set_vbr(options.encode_vbr)?;
                let num_samples_per_encode = options.samples_cache_duration.get_num_samples(spec.channels, spec.sample_rate);
                Ok(Self {
                    writer,
                    encoder,
                    channels: spec.channels,
                    sample_rate: spec.sample_rate,
                    cache_duration: options.samples_cache_duration,
                    num_samples_per_encode,
                    sample_cache: Vec::<f32>::new(),
                    samples_written: 0,
                    bytes_written: 0,
                })
            }

            pub fn set_cache_duration(&mut self, samples_cache_duration: OpusEncoderSampleDuration) {
                self.cache_duration = samples_cache_duration;
                self.num_samples_per_encode = samples_cache_duration.get_num_samples(self.channels, self.sample_rate);
            }

            pub fn write_samples(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {
                self.sample_cache.extend(samples);
                let mut cached_length = self.sample_cache.len();
                let mut iter = mem::take(&mut self.sample_cache).into_iter();
                while cached_length >= self.num_samples_per_encode {

                    // Extract `self.num_samples_per_encode` samples to encode
                    let samples_to_write: Vec<f32> = iter.by_ref().take(self.num_samples_per_encode).collect();
                    if samples_to_write.is_empty() {break;}

                    // Allocates a buffer of sufficient size, reserving one byte per sample.
                    let mut buf = vec![0u8; self.num_samples_per_encode];

                    // Do encode. The output size should be the same as the input samples a.k.a. block size.
                    let size = self.encoder.encode_float(&samples_to_write, &mut buf)?;
                    assert_eq!(size, buf.len());
                    self.writer.write_all(&buf)?;

                    // Update statistics
                    cached_length -= self.num_samples_per_encode;
                    self.samples_written += self.num_samples_per_encode as u64;
                    self.bytes_written += buf.len() as u64;
                }
                self.sample_cache = iter.collect();
                Ok(())
            }

            pub fn flush(&mut self) -> Result<(), AudioWriteError> {
                if !self.sample_cache.is_empty() {
                    let pad = (self.num_samples_per_encode - self.sample_cache.len() % self.num_samples_per_encode) % self.num_samples_per_encode;

                    // Pad to the block size to trigger it to write.
                    self.write_samples(&vec![0.0f32; pad])?;
                }
                Ok(())
            }
        }

        impl<'a> Debug for OpusEncoder<'_> {
            fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.debug_struct("OpusEncoder")
                    .field("writer", &self.writer)
                    .field("encoder", &self.encoder)
                    .field("channels", &self.channels)
                    .field("sample_rate", &self.sample_rate)
                    .field("cache_duration", &self.cache_duration)
                    .field("num_samples_per_encode", &self.num_samples_per_encode)
                    .field("sample_cache", &format_args!("[f32; {}]", self.sample_cache.len()))
                    .field("samples_written", &self.samples_written)
                    .field("bytes_written", &self.bytes_written)
                    .finish()
            }
        }

        impl<'a> EncoderToImpl for OpusEncoder<'_> {
            fn get_bitrate(&self) -> u32 {
                if self.samples_written != 0 {
                    (self.sample_rate as u64 * self.bytes_written / self.samples_written * self.channels as u64 * 8) as u32
                } else {
                    self.sample_rate * self.channels as u32 * 8 // Fake data
                }
            }
            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                Ok(FmtChunk{
                    format_tag: 0x704F,
                    channels: self.channels,
                    sample_rate: self.sample_rate,
                    byte_rate: self.get_bitrate() / 8,
                    block_align: self.num_samples_per_encode as u16,
                    bits_per_sample: 0,
                    extension: None,
                })
            }
            fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
                fmt.byte_rate = self.get_bitrate() / 8;
                fmt.block_align = self.num_samples_per_encode as u16;
                Ok(())
            }
            fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.flush()?;
                self.writer.flush()?;
                Ok(())
            }

            fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
            fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(&sample_conv(samples))}
        }
    }

    #[cfg(feature = "opus")]
    pub use impl_opus::*;
}

#[cfg(feature = "flac")]
pub mod flac {
    use std::{io::{self, SeekFrom}};

    use super::EncoderToImpl;

    use crate::Writer;
    use crate::{i24, u24};
    use crate::AudioWriteError;
    use crate::wavcore::FmtChunk;
    use crate::flac::*;
    use crate::hacks;
    use crate::utils::{sample_conv, stereos_conv, sample_conv_batch};

    #[derive(Debug)]
    pub struct FlacEncoderWrap<'a> {
        encoder: FlacEncoder<'a>,
        write_offset: Box<u64>,
        frames_written: u64,
        bytes_written: Box<u64>,
    }
    
    impl<'a> FlacEncoderWrap<'a> {
        pub fn new(mut writer: &'a mut dyn Writer, params: &FlacEncoderParams) -> Result<Self, AudioWriteError> {
            let mut write_offset = Box::new(writer.stream_position()?);
            let write_offset_ptr = (&mut *write_offset) as *mut u64;
            let mut bytes_written = Box::new(0u64);
            let bytes_written_ptr = (&mut *bytes_written) as *mut u64;
            Ok(Self{
                encoder: FlacEncoder::new(
                    hacks::force_borrow!(writer, dyn WriteSeek),
                    Box::new(move |writer: &mut dyn WriteSeek, data: &[u8]| -> Result<(), io::Error> {
                        unsafe{*bytes_written_ptr += data.len() as u64};
                        writer.write_all(data)
                    }),
                    Box::new(move |writer: &mut dyn WriteSeek, position: u64| -> Result<(), io::Error> {
                        let write_offset = unsafe{*write_offset_ptr};
                        writer.seek(SeekFrom::Start(write_offset + position))?;
                        Ok(())
                    }),
                    Box::new(move |writer: &mut dyn WriteSeek| -> Result<u64, io::Error> {
                        let write_offset = unsafe{*write_offset_ptr};
                        Ok(write_offset + writer.stream_position()?)
                    }),
                    params
                )?,
                write_offset,
                frames_written: 0,
                bytes_written,
            })
        }

        pub fn get_channels(&self) -> u16 {
            self.encoder.get_params().channels
        }

        pub fn get_sample_rate(&self) -> u32 {
            self.encoder.get_params().sample_rate
        }

        pub fn write_interleaved_samples(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {
            match self.encoder.write_interleaved_samples(samples) {
                Ok(_) => Ok(self.frames_written += samples.len() as u64 / self.get_channels() as u64),
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_mono_channel(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {
            match self.encoder.write_mono_channel(monos) {
                Ok(_) => Ok(self.frames_written += monos.len() as u64),
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {
            match self.encoder.write_stereos(stereos) {
                Ok(_) => Ok(self.frames_written += stereos.len() as u64),
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_monos(&mut self, monos: &[Vec<i32>]) -> Result<(), AudioWriteError> {
            match self.encoder.write_monos(monos) {
                Ok(_) => Ok(self.frames_written += monos[0].len() as u64),
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {
            match self.encoder.write_frames(frames) {
                Ok(_) => Ok(self.frames_written += frames.len() as u64),
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }
    }

    impl<'a> EncoderToImpl for FlacEncoderWrap<'_> {
        fn get_bitrate(&self) -> u32 {
            if self.frames_written != 0 {
                (*self.bytes_written * self.get_sample_rate() as u64 * self.get_channels() as u64 * 8 / self.frames_written) as u32
            } else {
                self.get_sample_rate() as u32 * self.get_channels() as u32 * 8 // Fake data
            }
        }

        fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
            self.encoder.initialize()?;
            Ok(())
        }

        fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
            Ok(FmtChunk{
                format_tag: 0xF1AC,
                channels: self.get_channels(),
                sample_rate: self.get_sample_rate(),
                byte_rate: self.get_bitrate() / 8,
                block_align: 1,
                bits_per_sample: 0,
                extension: None,
            })
        }

        fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
            fmt.byte_rate = self.get_bitrate() / 8;
            Ok(())
        }

        fn finish(&mut self) -> Result<(), AudioWriteError> {
            Ok(self.encoder.finish()?)
        }

        fn write_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}

        fn write_mono_channel__i8(&mut self, monos: &[i8 ]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_i16(&mut self, monos: &[i16]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_i24(&mut self, monos: &[i24]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_i32(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_i64(&mut self, monos: &[i64]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel__u8(&mut self, monos: &[u8 ]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_u16(&mut self, monos: &[u16]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_u24(&mut self, monos: &[u24]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_u32(&mut self, monos: &[u32]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_u64(&mut self, monos: &[u64]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_f32(&mut self, monos: &[f32]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}
        fn write_mono_channel_f64(&mut self, monos: &[f64]) -> Result<(), AudioWriteError> {self.write_mono_channel(&sample_conv(monos))}

        fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}
        fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_stereos(&stereos_conv(stereos))}

        fn write_monos__i8(&mut self, monos_array: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_i16(&mut self, monos_array: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_i24(&mut self, monos_array: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_i32(&mut self, monos_array: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_i64(&mut self, monos_array: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos__u8(&mut self, monos_array: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_u16(&mut self, monos_array: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_u24(&mut self, monos_array: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_u32(&mut self, monos_array: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_u64(&mut self, monos_array: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_f32(&mut self, monos_array: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}
        fn write_monos_f64(&mut self, monos_array: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_monos(&sample_conv_batch(monos_array))}

        fn write_frames__i8(&mut self, frames: &[Vec<i8 >], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_i16(&mut self, frames: &[Vec<i16>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_i24(&mut self, frames: &[Vec<i24>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_i32(&mut self, frames: &[Vec<i32>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_i64(&mut self, frames: &[Vec<i64>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames__u8(&mut self, frames: &[Vec<u8 >], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_u16(&mut self, frames: &[Vec<u16>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_u24(&mut self, frames: &[Vec<u24>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_u32(&mut self, frames: &[Vec<u32>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_u64(&mut self, frames: &[Vec<u64>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_f32(&mut self, frames: &[Vec<f32>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
        fn write_frames_f64(&mut self, frames: &[Vec<f64>], channels: u16) -> Result<(), AudioWriteError> {if channels != self.encoder.get_params().channels {Err(AudioWriteError::WrongChannels(format!("The encoder channels is {} but {channels} channels audio data are asked to be written.", self.encoder.get_params().channels)))} else {self.write_frames(&sample_conv_batch(frames))}}
    }
}

