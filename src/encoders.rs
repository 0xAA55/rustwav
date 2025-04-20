#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

use crate::Writer;
use crate::{SampleType, i24, u24};
use crate::AudioWriteError;
use crate::adpcm;
use crate::wavcore::{WaveSampleType, SpeakerPosition};
use crate::wavcore::{FmtChunk, FmtExtension, ExtensibleData, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
use crate::utils::{self, sample_conv, stereo_conv, stereos_conv, sample_conv_batch};
use crate::xlaw::{XLaw, PcmXLawEncoder};

// An encoder that accepts samples of type `S` and encodes them into the file's target format.
// Due to trait bounds prohibiting generic parameters, each function must be explicitly 
// implemented for every supported type.
pub trait EncoderToImpl: Debug {
    fn get_bitrate(&self, channels: u16) -> u32;
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError>;
    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError>;
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError>;

    // Channel-agnostic low-level sample writers. Default impls exist except for `f32`.
    fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError>;
    fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError>;
    fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError>;
    fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError>;
    fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError>;
    fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError>;
    fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError>;
    fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError>;
    fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError>;
    fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError>;
    fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError>;
    fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError>;

    // Convenience interfaces for writing audio frames. Each frame is an array of samples per channel. Default implementations are provided.
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, frame)}
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, frame)}
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, frame)}
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, frame)}
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, frame)}
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, frame)}
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, frame)}
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, frame)}
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, frame)}
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, frame)}
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, frame)}
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, frame)}

    // Convenience interfaces for writing multiple audio frames. Default implementations are provided.
    fn write_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::frames_to_interleaved_samples(frames, Some(channels))?)}

    // Interfaces for writing mono audio frames (single-channel). Default implementations are provided.
    fn write_mono__i8(&mut self, writer: &mut dyn Writer, frame: i8 ) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &[frame])}
    fn write_mono_i16(&mut self, writer: &mut dyn Writer, frame: i16) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &[frame])}
    fn write_mono_i24(&mut self, writer: &mut dyn Writer, frame: i24) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &[frame])}
    fn write_mono_i32(&mut self, writer: &mut dyn Writer, frame: i32) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &[frame])}
    fn write_mono_i64(&mut self, writer: &mut dyn Writer, frame: i64) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &[frame])}
    fn write_mono__u8(&mut self, writer: &mut dyn Writer, frame: u8 ) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &[frame])}
    fn write_mono_u16(&mut self, writer: &mut dyn Writer, frame: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &[frame])}
    fn write_mono_u24(&mut self, writer: &mut dyn Writer, frame: u24) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &[frame])}
    fn write_mono_u32(&mut self, writer: &mut dyn Writer, frame: u32) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &[frame])}
    fn write_mono_u64(&mut self, writer: &mut dyn Writer, frame: u64) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &[frame])}
    fn write_mono_f32(&mut self, writer: &mut dyn Writer, frame: f32) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &[frame])}
    fn write_mono_f64(&mut self, writer: &mut dyn Writer, frame: f64) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &[frame])}

    // Interfaces for writing batched mono audio frames. Default implementations are provided.
    fn write_monos__i8(&mut self, writer: &mut dyn Writer, frames: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, frames)}
    fn write_monos_i16(&mut self, writer: &mut dyn Writer, frames: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, frames)}
    fn write_monos_i24(&mut self, writer: &mut dyn Writer, frames: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, frames)}
    fn write_monos_i32(&mut self, writer: &mut dyn Writer, frames: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, frames)}
    fn write_monos_i64(&mut self, writer: &mut dyn Writer, frames: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, frames)}
    fn write_monos__u8(&mut self, writer: &mut dyn Writer, frames: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, frames)}
    fn write_monos_u16(&mut self, writer: &mut dyn Writer, frames: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, frames)}
    fn write_monos_u24(&mut self, writer: &mut dyn Writer, frames: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, frames)}
    fn write_monos_u32(&mut self, writer: &mut dyn Writer, frames: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, frames)}
    fn write_monos_u64(&mut self, writer: &mut dyn Writer, frames: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, frames)}
    fn write_monos_f32(&mut self, writer: &mut dyn Writer, frames: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, frames)}
    fn write_monos_f64(&mut self, writer: &mut dyn Writer, frames: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, frames)}

    // Interfaces for writing stereo audio (composed of two separate channel buffers). Default implementations are provided.
    fn write_dual_mono__i8(&mut self, writer: &mut dyn Writer, mono1: i8 , mono2: i8 ) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &[mono1, mono2])}
    fn write_dual_mono_i16(&mut self, writer: &mut dyn Writer, mono1: i16, mono2: i16) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &[mono1, mono2])}
    fn write_dual_mono_i24(&mut self, writer: &mut dyn Writer, mono1: i24, mono2: i24) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &[mono1, mono2])}
    fn write_dual_mono_i32(&mut self, writer: &mut dyn Writer, mono1: i32, mono2: i32) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &[mono1, mono2])}
    fn write_dual_mono_i64(&mut self, writer: &mut dyn Writer, mono1: i64, mono2: i64) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &[mono1, mono2])}
    fn write_dual_mono__u8(&mut self, writer: &mut dyn Writer, mono1: u8 , mono2: u8 ) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &[mono1, mono2])}
    fn write_dual_mono_u16(&mut self, writer: &mut dyn Writer, mono1: u16, mono2: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &[mono1, mono2])}
    fn write_dual_mono_u24(&mut self, writer: &mut dyn Writer, mono1: u24, mono2: u24) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &[mono1, mono2])}
    fn write_dual_mono_u32(&mut self, writer: &mut dyn Writer, mono1: u32, mono2: u32) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &[mono1, mono2])}
    fn write_dual_mono_u64(&mut self, writer: &mut dyn Writer, mono1: u64, mono2: u64) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &[mono1, mono2])}
    fn write_dual_mono_f32(&mut self, writer: &mut dyn Writer, mono1: f32, mono2: f32) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &[mono1, mono2])}
    fn write_dual_mono_f64(&mut self, writer: &mut dyn Writer, mono1: f64, mono2: f64) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &[mono1, mono2])}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_dual_monos__i8(&mut self, writer: &mut dyn Writer, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i16(&mut self, writer: &mut dyn Writer, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i24(&mut self, writer: &mut dyn Writer, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i32(&mut self, writer: &mut dyn Writer, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i64(&mut self, writer: &mut dyn Writer, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos__u8(&mut self, writer: &mut dyn Writer, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u16(&mut self, writer: &mut dyn Writer, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u24(&mut self, writer: &mut dyn Writer, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u32(&mut self, writer: &mut dyn Writer, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u64(&mut self, writer: &mut dyn Writer, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f32(&mut self, writer: &mut dyn Writer, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f64(&mut self, writer: &mut dyn Writer, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}

    // Interfaces for writing stereo audio frames using tuples (L, R). Default implementations are provided.
    fn write_stereo__i8(&mut self, writer: &mut dyn Writer, stereo: (i8 , i8 )) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &[stereo.0, stereo.1])}
    fn write_stereo_i16(&mut self, writer: &mut dyn Writer, stereo: (i16, i16)) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &[stereo.0, stereo.1])}
    fn write_stereo_i24(&mut self, writer: &mut dyn Writer, stereo: (i24, i24)) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &[stereo.0, stereo.1])}
    fn write_stereo_i32(&mut self, writer: &mut dyn Writer, stereo: (i32, i32)) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &[stereo.0, stereo.1])}
    fn write_stereo_i64(&mut self, writer: &mut dyn Writer, stereo: (i64, i64)) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &[stereo.0, stereo.1])}
    fn write_stereo__u8(&mut self, writer: &mut dyn Writer, stereo: (u8 , u8 )) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &[stereo.0, stereo.1])}
    fn write_stereo_u16(&mut self, writer: &mut dyn Writer, stereo: (u16, u16)) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &[stereo.0, stereo.1])}
    fn write_stereo_u24(&mut self, writer: &mut dyn Writer, stereo: (u24, u24)) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &[stereo.0, stereo.1])}
    fn write_stereo_u32(&mut self, writer: &mut dyn Writer, stereo: (u32, u32)) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &[stereo.0, stereo.1])}
    fn write_stereo_u64(&mut self, writer: &mut dyn Writer, stereo: (u64, u64)) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &[stereo.0, stereo.1])}
    fn write_stereo_f32(&mut self, writer: &mut dyn Writer, stereo: (f32, f32)) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &[stereo.0, stereo.1])}
    fn write_stereo_f64(&mut self, writer: &mut dyn Writer, stereo: (f64, f64)) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &[stereo.0, stereo.1])}

    // Interfaces for writing stereo audio frames using arrays of tuples. Default implementations are provided.
    fn write_stereos__i8(&mut self, writer: &mut dyn Writer, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i16(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i24(&mut self, writer: &mut dyn Writer, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i32(&mut self, writer: &mut dyn Writer, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i64(&mut self, writer: &mut dyn Writer, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos__u8(&mut self, writer: &mut dyn Writer, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u16(&mut self, writer: &mut dyn Writer, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u24(&mut self, writer: &mut dyn Writer, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u32(&mut self, writer: &mut dyn Writer, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u64(&mut self, writer: &mut dyn Writer, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f32(&mut self, writer: &mut dyn Writer, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f64(&mut self, writer: &mut dyn Writer, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::stereos_to_interleaved_samples(stereos))}
}

// Default implementations: all input formats are normalized to `f32` for encoding.
impl EncoderToImpl for () {
    fn get_bitrate(&self, _channels: u16) -> u32 {
        panic!("Must implement `get_bitrate()` for your encoder.");
    }

    fn new_fmt_chunk(&mut self, _channels: u16, _sample_rate: u32, _bits_per_sample: u16, _channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        panic!("Must implement `new_fmt_chunk()` for your encoder.");
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        panic!("Must implement `update_fmt_chunk()` for your encoder.");
    }

    fn finalize(&mut self, _writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        panic!("Must implement `finalize()` for your encoder to flush the data.");
    }

    fn write_samples_f32(&mut self, _writer: &mut dyn Writer, _samples: &[f32]) -> Result<(), AudioWriteError> {
        panic!("Must atlease implement `write_samples_f32()` for your encoder to get samples.");
    }

    fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
    fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &sample_conv(samples))}
}

#[derive(Debug)]
pub struct Encoder { // Stores an `EncoderToImpl` and offers generic APIs for callers.
    encoder: Box<dyn EncoderToImpl>,
}

impl Encoder {
    pub fn new(encoder: Box<dyn EncoderToImpl>) -> Self {
        Self {
            encoder,
        }
    }

    pub fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        self.encoder.new_fmt_chunk(channels, sample_rate, bits_per_sample, channel_mask)
    }

    pub fn get_bitrate(&self, channels: u16) -> u32 {
        self.encoder.get_bitrate(channels)
    }

    pub fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        self.encoder.update_fmt_chunk(fmt)
    }

    pub fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.encoder.finalize(writer)
    }

    pub fn write_samples<S>(&mut self, writer: &mut dyn Writer, samples: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_samples__i8(writer, &sample_conv(samples)),
            "i16" => self.encoder.write_samples_i16(writer, &sample_conv(samples)),
            "i24" => self.encoder.write_samples_i24(writer, &sample_conv(samples)),
            "i32" => self.encoder.write_samples_i32(writer, &sample_conv(samples)),
            "i64" => self.encoder.write_samples_i64(writer, &sample_conv(samples)),
            "u8"  => self.encoder.write_samples__u8(writer, &sample_conv(samples)),
            "u16" => self.encoder.write_samples_u16(writer, &sample_conv(samples)),
            "u24" => self.encoder.write_samples_u24(writer, &sample_conv(samples)),
            "u32" => self.encoder.write_samples_u32(writer, &sample_conv(samples)),
            "u64" => self.encoder.write_samples_u64(writer, &sample_conv(samples)),
            "f32" => self.encoder.write_samples_f32(writer, &sample_conv(samples)),
            "f64" => self.encoder.write_samples_f64(writer, &sample_conv(samples)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_frame<S>(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_frame__i8(writer, &sample_conv(frame)),
            "i16" => self.encoder.write_frame_i16(writer, &sample_conv(frame)),
            "i24" => self.encoder.write_frame_i24(writer, &sample_conv(frame)),
            "i32" => self.encoder.write_frame_i32(writer, &sample_conv(frame)),
            "i64" => self.encoder.write_frame_i64(writer, &sample_conv(frame)),
            "u8"  => self.encoder.write_frame__u8(writer, &sample_conv(frame)),
            "u16" => self.encoder.write_frame_u16(writer, &sample_conv(frame)),
            "u24" => self.encoder.write_frame_u24(writer, &sample_conv(frame)),
            "u32" => self.encoder.write_frame_u32(writer, &sample_conv(frame)),
            "u64" => self.encoder.write_frame_u64(writer, &sample_conv(frame)),
            "f32" => self.encoder.write_frame_f32(writer, &sample_conv(frame)),
            "f64" => self.encoder.write_frame_f64(writer, &sample_conv(frame)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>], channels: u16) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() { // 希望编译器能做到优化，省区字符串比对的过程。
            "i8"  => self.encoder.write_frames__i8(writer, &sample_conv_batch(frames), channels),
            "i16" => self.encoder.write_frames_i16(writer, &sample_conv_batch(frames), channels),
            "i24" => self.encoder.write_frames_i24(writer, &sample_conv_batch(frames), channels),
            "i32" => self.encoder.write_frames_i32(writer, &sample_conv_batch(frames), channels),
            "i64" => self.encoder.write_frames_i64(writer, &sample_conv_batch(frames), channels),
            "u8"  => self.encoder.write_frames__u8(writer, &sample_conv_batch(frames), channels),
            "u16" => self.encoder.write_frames_u16(writer, &sample_conv_batch(frames), channels),
            "u24" => self.encoder.write_frames_u24(writer, &sample_conv_batch(frames), channels),
            "u32" => self.encoder.write_frames_u32(writer, &sample_conv_batch(frames), channels),
            "u64" => self.encoder.write_frames_u64(writer, &sample_conv_batch(frames), channels),
            "f32" => self.encoder.write_frames_f32(writer, &sample_conv_batch(frames), channels),
            "f64" => self.encoder.write_frames_f64(writer, &sample_conv_batch(frames), channels),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_mono<S>(&mut self, writer: &mut dyn Writer, mono: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_mono__i8(writer, mono.to_i8() ),
            "i16" => self.encoder.write_mono_i16(writer, mono.to_i16()),
            "i24" => self.encoder.write_mono_i24(writer, mono.to_i24()),
            "i32" => self.encoder.write_mono_i32(writer, mono.to_i32()),
            "i64" => self.encoder.write_mono_i64(writer, mono.to_i64()),
            "u8"  => self.encoder.write_mono__u8(writer, mono.to_u8() ),
            "u16" => self.encoder.write_mono_u16(writer, mono.to_u16()),
            "u24" => self.encoder.write_mono_u24(writer, mono.to_u24()),
            "u32" => self.encoder.write_mono_u32(writer, mono.to_u32()),
            "u64" => self.encoder.write_mono_u64(writer, mono.to_u64()),
            "f32" => self.encoder.write_mono_f32(writer, mono.to_f32()),
            "f64" => self.encoder.write_mono_f64(writer, mono.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_monos<S>(&mut self, writer: &mut dyn Writer, monos: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_samples__i8(writer, &sample_conv(monos)),
            "i16" => self.encoder.write_samples_i16(writer, &sample_conv(monos)),
            "i24" => self.encoder.write_samples_i24(writer, &sample_conv(monos)),
            "i32" => self.encoder.write_samples_i32(writer, &sample_conv(monos)),
            "i64" => self.encoder.write_samples_i64(writer, &sample_conv(monos)),
            "u8"  => self.encoder.write_samples__u8(writer, &sample_conv(monos)),
            "u16" => self.encoder.write_samples_u16(writer, &sample_conv(monos)),
            "u24" => self.encoder.write_samples_u24(writer, &sample_conv(monos)),
            "u32" => self.encoder.write_samples_u32(writer, &sample_conv(monos)),
            "u64" => self.encoder.write_samples_u64(writer, &sample_conv(monos)),
            "f32" => self.encoder.write_samples_f32(writer, &sample_conv(monos)),
            "f64" => self.encoder.write_samples_f64(writer, &sample_conv(monos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_dual_mono<S>(&mut self, writer: &mut dyn Writer, mono1: S, mono2: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_dual_mono__i8(writer, mono1.to_i8() , mono2.to_i8() ),
            "i16" => self.encoder.write_dual_mono_i16(writer, mono1.to_i16(), mono2.to_i16()),
            "i24" => self.encoder.write_dual_mono_i24(writer, mono1.to_i24(), mono2.to_i24()),
            "i32" => self.encoder.write_dual_mono_i32(writer, mono1.to_i32(), mono2.to_i32()),
            "i64" => self.encoder.write_dual_mono_i64(writer, mono1.to_i64(), mono2.to_i64()),
            "u8"  => self.encoder.write_dual_mono__u8(writer, mono1.to_u8() , mono2.to_u8() ),
            "u16" => self.encoder.write_dual_mono_u16(writer, mono1.to_u16(), mono2.to_u16()),
            "u24" => self.encoder.write_dual_mono_u24(writer, mono1.to_u24(), mono2.to_u24()),
            "u32" => self.encoder.write_dual_mono_u32(writer, mono1.to_u32(), mono2.to_u32()),
            "u64" => self.encoder.write_dual_mono_u64(writer, mono1.to_u64(), mono2.to_u64()),
            "f32" => self.encoder.write_dual_mono_f32(writer, mono1.to_f32(), mono2.to_f32()),
            "f64" => self.encoder.write_dual_mono_f64(writer, mono1.to_f64(), mono2.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_dual_monos<S>(&mut self, writer: &mut dyn Writer, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_dual_monos__i8(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i16" => self.encoder.write_dual_monos_i16(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i24" => self.encoder.write_dual_monos_i24(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i32" => self.encoder.write_dual_monos_i32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i64" => self.encoder.write_dual_monos_i64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u8"  => self.encoder.write_dual_monos__u8(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u16" => self.encoder.write_dual_monos_u16(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u24" => self.encoder.write_dual_monos_u24(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u32" => self.encoder.write_dual_monos_u32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u64" => self.encoder.write_dual_monos_u64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "f32" => self.encoder.write_dual_monos_f32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "f64" => self.encoder.write_dual_monos_f64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_stereo<S>(&mut self, writer: &mut dyn Writer, stereo: (S, S)) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_stereo__i8(writer, stereo_conv(stereo)),
            "i16" => self.encoder.write_stereo_i16(writer, stereo_conv(stereo)),
            "i24" => self.encoder.write_stereo_i24(writer, stereo_conv(stereo)),
            "i32" => self.encoder.write_stereo_i32(writer, stereo_conv(stereo)),
            "i64" => self.encoder.write_stereo_i64(writer, stereo_conv(stereo)),
            "u8"  => self.encoder.write_stereo__u8(writer, stereo_conv(stereo)),
            "u16" => self.encoder.write_stereo_u16(writer, stereo_conv(stereo)),
            "u24" => self.encoder.write_stereo_u24(writer, stereo_conv(stereo)),
            "u32" => self.encoder.write_stereo_u32(writer, stereo_conv(stereo)),
            "u64" => self.encoder.write_stereo_u64(writer, stereo_conv(stereo)),
            "f32" => self.encoder.write_stereo_f32(writer, stereo_conv(stereo)),
            "f64" => self.encoder.write_stereo_f64(writer, stereo_conv(stereo)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_stereos<S>(&mut self, writer: &mut dyn Writer, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_stereos__i8(writer, &stereos_conv(stereos)),
            "i16" => self.encoder.write_stereos_i16(writer, &stereos_conv(stereos)),
            "i24" => self.encoder.write_stereos_i24(writer, &stereos_conv(stereos)),
            "i32" => self.encoder.write_stereos_i32(writer, &stereos_conv(stereos)),
            "i64" => self.encoder.write_stereos_i64(writer, &stereos_conv(stereos)),
            "u8"  => self.encoder.write_stereos__u8(writer, &stereos_conv(stereos)),
            "u16" => self.encoder.write_stereos_u16(writer, &stereos_conv(stereos)),
            "u24" => self.encoder.write_stereos_u24(writer, &stereos_conv(stereos)),
            "u32" => self.encoder.write_stereos_u32(writer, &stereos_conv(stereos)),
            "u64" => self.encoder.write_stereos_u64(writer, &stereos_conv(stereos)),
            "f32" => self.encoder.write_stereos_f32(writer, &stereos_conv(stereos)),
            "f64" => self.encoder.write_stereos_f64(writer, &stereos_conv(stereos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }
}

// `PcmEncoderFrom<S>`: Transcodes samples from type `S` into the target format.
#[derive(Debug, Clone, Copy)]
struct PcmEncoderFrom<S>
where S: SampleType {
    writer: fn(&mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>,
}

impl<S> PcmEncoderFrom<S>
where S: SampleType {
    pub fn new(target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        Ok(Self {
            writer: match target_sample{
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
        (self.writer)(writer, frame)
    }

    pub fn write_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError> {
        for frame in frames.iter() {
            (self.writer)(writer, frame)?;
        }
        Ok(())
    }

    pub fn write_samples(&mut self, writer: &mut dyn Writer, samples: &[S]) -> Result<(), AudioWriteError> {
        (self.writer)(writer, samples)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PcmEncoder {
    sample_rate: u32,
    sample_type: WaveSampleType,
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

impl PcmEncoder {
    // target_sample: The specific PCM format (e.g., bit depth, signedness) to encode into the WAV file.
    pub fn new(sample_rate: u32, target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        Ok(Self {
            sample_rate,
            sample_type: target_sample,
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

impl EncoderToImpl for PcmEncoder {
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64, Unknown};
        let bytes_per_sample = bits_per_sample / 8;
        let byte_rate = sample_rate * channels as u32 * bytes_per_sample as u32;
        let extensible = match channel_mask {
            None => None,
            Some(channel_mask) => Some(FmtExtension::new_extensible(ExtensibleData {
                valid_bits_per_sample: bits_per_sample,
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
            channels,
            sample_rate,
            byte_rate,
            block_align: bytes_per_sample * channels,
            bits_per_sample,
            extension: extensible,
        })
    }

    fn get_bitrate(&self, channels: u16) -> u32 {
        channels as u32 * self.sample_rate * self.sample_type.sizeof() as u32 * 8
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        Ok(writer.flush()?)
    }

    fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.writer_from__i8.write_samples(writer, samples)}
    fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.writer_from_i16.write_samples(writer, samples)}
    fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.writer_from_i24.write_samples(writer, samples)}
    fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.writer_from_i32.write_samples(writer, samples)}
    fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.writer_from_i64.write_samples(writer, samples)}
    fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.writer_from__u8.write_samples(writer, samples)}
    fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.writer_from_u16.write_samples(writer, samples)}
    fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.writer_from_u24.write_samples(writer, samples)}
    fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.writer_from_u32.write_samples(writer, samples)}
    fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.writer_from_u64.write_samples(writer, samples)}
    fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {self.writer_from_f32.write_samples(writer, samples)}
    fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.writer_from_f64.write_samples(writer, samples)}
}

#[derive(Debug, Clone)]
pub struct AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    channels: u16,
    sample_rate: u32,
    bytes_written: u64,
    encoder: E,
    nibbles: Vec<u8>,
}

const MAX_BUFFER_USAGE: usize = 1024;

impl<E> AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    pub fn new(channels: u16, sample_rate: u32) -> Result<Self, AudioWriteError> {
        Ok(Self {
            channels,
            sample_rate,
            bytes_written: 0,
            encoder: E::new(channels)?,
            nibbles: Vec::<u8>::with_capacity(MAX_BUFFER_USAGE),
        })
    }

    fn flush_buffers(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        writer.write_all(&self.nibbles)?;
        // Avoid using `clear()`. If a user writes a large batch of samples once, 
        // `clear()` retains the original capacity without shrinking it, leading to persistent memory usage.
        self.nibbles = Vec::<u8>::with_capacity(MAX_BUFFER_USAGE);
        Ok(())
    }

    pub fn write_samples(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {
        let mut iter = samples.iter().copied();
        self.encoder.encode(|| -> Option<i16> { iter.next()}, |byte: u8|{ self.nibbles.push(byte); })?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers(writer)?;
        }
        Ok(())
    }

    pub fn write_stereos(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {
        if self.channels != 2 {
            return Err(AudioWriteError::Unsupported(format!("This encoder only accepts {} channel audio data", self.channels)));
        }
        let mut iter = utils::stereos_to_interleaved_samples(stereos).into_iter();
        self.encoder.encode(|| -> Option<i16> {iter.next()}, |byte: u8|{ self.nibbles.push(byte);})?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers(writer)?;
        }
        Ok(())
    }
}

impl<E> EncoderToImpl for AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, _bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        if let Some(channel_mask) = channel_mask {
            const MONO_MASK: u32 = SpeakerPosition::FrontCenter as u32;
            const STEREO_MASK: u32 = SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32;
            match (channels, channel_mask) {
                (1, MONO_MASK) => (),
                (2, STEREO_MASK) => (),
                _ => return Err(AudioWriteError::Unsupported(format!("Channel masks is not supported by the ADPCM format, and the mask is 0x{:08x} for {channels} channels.", channel_mask))),
            }
        }
        Ok(self.encoder.new_fmt_chunk(channels, sample_rate, 4)?)
    }

    fn get_bitrate(&self, channels: u16) -> u32 {
        self.sample_rate * channels as u32 * 4
    }

    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(self.encoder.modify_fmt_chunk(fmt)?)
    }

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.encoder.flush(|nibble: u8|{ self.nibbles.push(nibble);})?;
        self.flush_buffers(writer)?;
        Ok(writer.flush()?)
    }

    fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}

    fn write_stereos__i8(&mut self, writer: &mut dyn Writer, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_i16(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_i24(&mut self, writer: &mut dyn Writer, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_i32(&mut self, writer: &mut dyn Writer, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_i64(&mut self, writer: &mut dyn Writer, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos__u8(&mut self, writer: &mut dyn Writer, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_u16(&mut self, writer: &mut dyn Writer, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_u24(&mut self, writer: &mut dyn Writer, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_u32(&mut self, writer: &mut dyn Writer, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_u64(&mut self, writer: &mut dyn Writer, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_f32(&mut self, writer: &mut dyn Writer, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
    fn write_stereos_f64(&mut self, writer: &mut dyn Writer, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, &stereos_conv(stereos))}
}

#[derive(Debug, Clone, Copy)]
pub struct PcmXLawEncoderWrap {
    enc: PcmXLawEncoder,
    sample_rate: u32,
}

impl PcmXLawEncoderWrap {
    pub fn new(sample_rate: u32, which_law: XLaw) -> Self {
        Self {
            enc: PcmXLawEncoder::new(which_law),
            sample_rate,
        }
    }

    pub fn write_samples(&self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {
        writer.write_all(&samples.iter().map(|sample| -> u8 {self.enc.encode(*sample)}).collect::<Vec<u8>>())?;
        Ok(())
    }
}

impl EncoderToImpl for PcmXLawEncoderWrap {
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        if let Some(channel_mask) = channel_mask {
            const MONO_MASK: u32 = SpeakerPosition::FrontCenter as u32;
            const STEREO_MASK: u32 = SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32;
            match (channels, channel_mask) {
                (1, MONO_MASK) => (),
                (2, STEREO_MASK) => (),
                _ => return Err(AudioWriteError::Unsupported(format!("Channel masks is not supported by the ALaw PCM format, and the mask is 0x{:08x} for {channels} channels.", channel_mask))),
            }
        }
        if bits_per_sample != 8 {
            eprintln!("For {} PCM, bits_per_sample bust be 8, the value `{bits_per_sample}` is ignored.", self.enc.get_which_law());
        }
        let bits_per_sample = 8u16;
        let block_align = channels;
        Ok(FmtChunk {
            format_tag: match self.enc.get_which_law() {
                XLaw::ALaw => 0x0006,
                XLaw::MuLaw => 0x0007,
            },
            channels,
            sample_rate,
            byte_rate: sample_rate * bits_per_sample as u32 * channels as u32 / 8,
            block_align,
            bits_per_sample,
            extension: None,
        })
    }

    fn get_bitrate(&self, channels: u16) -> u32 {
        self.sample_rate * channels as u32 * 8
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        Ok(writer.flush()?)
    }

    fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
    fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
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
        use crate::wavcore::{FmtChunk, FmtExtension, Mp3Data, SpeakerPosition};
        use crate::utils::{self, sample_conv, stereos_conv};
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

        #[derive(Debug, Clone)]
        pub struct Mp3Encoder<S>
        where S: SampleType {
            channels: u16,
            bitrate: u32,
            encoder: SharedMp3Encoder,
            encoder_options: Mp3EncoderOptions,
            buffers: ChannelBuffers<S>,
        }

        impl<S> Mp3Encoder<S>
        where S: SampleType {
            pub fn new(sample_rate: u32, mp3_options: &Mp3EncoderOptions) -> Result<Self, AudioWriteError> {
                let mp3_builder = Builder::new();
                let mut mp3_builder = match mp3_builder {
                    Some(mp3_builder) => mp3_builder,
                    None => return Err(AudioWriteError::OtherReason("`lame_init()` somehow failed.".to_owned())),
                };
                let options = mp3_options.to_lame_options();

                mp3_builder.set_mode(options.channels)?;
                mp3_builder.set_sample_rate(sample_rate)?;
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
                    bitrate: mp3_options.get_bitrate(),
                    encoder: encoder.clone(),
                    encoder_options: *mp3_options,
                    buffers: match channels {
                        1 | 2 => ChannelBuffers::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE, channels)?,
                        o => return Err(AudioWriteError::Unsupported(format!("Bad channel number: {o}"))),
                    },
                })
            }

            pub fn write_samples<T>(&mut self, writer: &mut dyn Writer, samples: &[T]) -> Result<(), AudioWriteError>
            where T: SampleType {
                if self.buffers.is_full() {
                    self.buffers.flush(writer)?;
                }
                match self.channels {
                    1 => self.buffers.add_monos(writer, &sample_conv::<T, S>(samples)),
                    2 => self.buffers.add_stereos(writer, &utils::interleaved_samples_to_stereos(&sample_conv::<T, S>(samples))?),
                    o => Err(AudioWriteError::Unsupported(format!("Bad channels number: {o}"))),
                }
            }

            pub fn write_stereos<T>(&mut self, writer: &mut dyn Writer, stereos: &[(T, T)]) -> Result<(), AudioWriteError>
            where T: SampleType {
                if self.buffers.is_full() {
                    self.buffers.flush(writer)?;
                }
                match self.channels{
                    1 => self.buffers.add_monos(writer, &utils::stereos_to_monos(&stereos_conv::<T, S>(stereos))),
                    2 => self.buffers.add_stereos(writer, &stereos_conv::<T, S>(stereos)),
                    o => Err(AudioWriteError::InvalidArguments(format!("Bad channels number: {o}"))),
                }
            }

            pub fn finish(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.buffers.finish(writer)
            }
        }

        #[derive(Debug, Clone)]
        enum Channels<S>
        where S: SampleType {
            Mono(Vec<S>),
            Stereo((Vec<S>, Vec<S>)),
        }

        #[derive(Clone)]
        struct ChannelBuffers<S>
        where S: SampleType {
            encoder: SharedMp3Encoder,
            channels: Channels<S>,
            max_samples: usize,
        }

        impl<S> Channels<S>
        where S: SampleType {
            pub fn new_mono(max_samples: usize) -> Self {
                Self::Mono(Vec::<S>::with_capacity(max_samples))
            }
            pub fn new_stereo(max_samples: usize) -> Self {
                Self::Stereo((Vec::<S>::with_capacity(max_samples), Vec::<S>::with_capacity(max_samples)))
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
            pub fn clear(&mut self, max_samples: usize) {
                match self {
                    Self::Mono(ref mut m) => *m = Vec::<S>::with_capacity(max_samples),
                    Self::Stereo(ref mut s) => *s = (Vec::<S>::with_capacity(max_samples), Vec::<S>::with_capacity(max_samples)),
                }
            }
        }

        impl<S> ChannelBuffers<S>
        where S: SampleType{
            pub fn new(encoder: SharedMp3Encoder, max_samples: usize, channels: u16) -> Result<Self, AudioWriteError> {
                Ok(Self {
                    encoder,
                    channels: match channels {
                        1 => Channels::<S>::new_mono(max_samples),
                        2 => Channels::<S>::new_stereo(max_samples),
                        o => return Err(AudioWriteError::InvalidArguments(format!("Invalid channels: {o}. Only 1 and 2 are accepted."))),
                    },
                    max_samples,
                })
            }

            pub fn is_full(&self) -> bool {
                self.channels.len() >= self.max_samples 
            }

            pub fn add_monos(&mut self, writer: &mut dyn Writer, monos: &[S]) -> Result<(), AudioWriteError> {
                self.channels.add_monos(monos);
                if self.is_full() {
                    self.flush(writer)?;
                }
                Ok(())
            }

            pub fn add_stereos(&mut self, writer: &mut dyn Writer, stereos: &[(S, S)]) -> Result<(), AudioWriteError> {
                self.channels.add_stereos(stereos);
                if self.is_full() {
                    self.flush(writer)?;
                }
                Ok(())
            }

            pub fn add_dual_monos(&mut self, writer: &mut dyn Writer, monos_l: &[S], monos_r: &[S]) -> Result<(), AudioWriteError> {
                self.channels.add_dual_monos(monos_l, monos_r)?;
                if self.is_full() {
                    self.flush(writer)?;
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

            pub fn flush(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                if self.channels.is_empty() {
                    return Ok(())
                }
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.channels.len()));
                    self.encode_to_vec(encoder, &mut to_save)?;
                    writer.write_all(&to_save)?;
                    Ok(())
                })?;
                self.channels.clear(self.max_samples);
                Ok(())
            }

            pub fn finish(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.flush(writer)?;
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples));
                    encoder.flush_to_vec::<FlushNoGap>(&mut to_save)?;
                    writer.write_all(&to_save)?;
                    Ok(())
                })?;
                self.channels.clear(self.max_samples);
                Ok(())
            }
        }

        impl<S> EncoderToImpl for Mp3Encoder<S>
        where S: SampleType {
            fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, _bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
                if let Some(channel_mask) = channel_mask {
                    const MONO_MASK: u32 = SpeakerPosition::FrontCenter as u32;
                    const STEREO_MASK: u32 = SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32;
                    match (channels, channel_mask) {
                        (1, MONO_MASK) => (),
                        (2, STEREO_MASK) => (),
                        _ => return Err(AudioWriteError::Unsupported(format!("Channel masks is not supported by the MP3 format, and the mask is 0x{:08x} for {channels} channels.", channel_mask))),
                    }
                }
                Ok(FmtChunk{
                    format_tag: 0x0055,
                    channels,
                    sample_rate,
                    byte_rate: self.bitrate / 8,
                    block_align: 1,
                    bits_per_sample: 0,
                    extension: Some(FmtExtension::new_mp3(Mp3Data::new(self.bitrate, sample_rate))),
                })
            }

            fn get_bitrate(&self, channels: u16) -> u32 {
                self.bitrate * channels as u32
            }

            fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.finish(writer)
            }

            fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}
            fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(writer, samples)}

            fn write_stereos__i8(&mut self, writer: &mut dyn Writer, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_i16(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_i24(&mut self, writer: &mut dyn Writer, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_i32(&mut self, writer: &mut dyn Writer, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_i64(&mut self, writer: &mut dyn Writer, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos__u8(&mut self, writer: &mut dyn Writer, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_u16(&mut self, writer: &mut dyn Writer, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_u24(&mut self, writer: &mut dyn Writer, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_u32(&mut self, writer: &mut dyn Writer, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_u64(&mut self, writer: &mut dyn Writer, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_f32(&mut self, writer: &mut dyn Writer, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
            fn write_stereos_f64(&mut self, writer: &mut dyn Writer, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_stereos(writer, stereos)}
        }

        impl Debug for SharedMp3Encoder {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct("SharedMp3Encoder")
                    .finish_non_exhaustive()
            }
        }

        impl<S> Debug for ChannelBuffers<S>
        where S: SampleType {
            fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.debug_struct(&format!("ChannelBuffers<{}>", type_name::<S>()))
                    .field("encoder", &self.encoder)
                    .field("channels", &format_args!("{}", match self.channels {
                        Channels::Mono(_) => "Mono",
                        Channels::Stereo(_) => "Stereo",
                    }))
                    .field("max_samples", &self.max_samples)
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
        use std::mem;

        use super::*;
        use crate::Writer;
        use crate::wavcore::{FmtChunk, SpeakerPosition};
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

        #[derive(Debug)]
        pub struct OpusEncoder {
            encoder: Encoder,
            channels: u16,
            sample_rate: u32,
            cache_duration: OpusEncoderSampleDuration,
            num_samples_per_encode: usize,
            sample_cache: Vec<f32>,
            samples_written: u64,
            bytes_written: u64,
        }

        impl OpusEncoder {
            pub fn new(channels: u16, sample_rate: u32, options: &OpusEncoderOptions) -> Result<Self, AudioWriteError> {
                let opus_channels = match channels {
                    1 => Channels::Mono,
                    2 => Channels::Stereo,
                    o => return Err(AudioWriteError::InvalidArguments(format!("Bad channels: {o} for the opus encoder."))),
                };
                if !OPUS_ALLOWED_SAMPLE_RATES.contains(&sample_rate) {
                    return Err(AudioWriteError::InvalidArguments(format!("Bad sample rate: {sample_rate} for the opus encoder. The sample rate must be one of {}",
                        OPUS_ALLOWED_SAMPLE_RATES.iter().map(|s|{format!("{s}")}).collect::<Vec<String>>().join(", ")
                    )));
                }
                let mut encoder = Encoder::new(sample_rate, opus_channels, Application::Audio)?;
                encoder.set_bitrate(options.bitrate.to_opus_bitrate())?;
                encoder.set_vbr(options.encode_vbr)?;
                let num_samples_per_encode = options.samples_cache_duration.get_num_samples(channels, sample_rate);
                Ok(Self {
                    encoder,
                    channels,
                    sample_rate,
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

            pub fn write_samples(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {
                self.sample_cache.extend(samples);
                let mut cached_length = self.sample_cache.len();
                let mut iter = mem::take(&mut self.sample_cache).into_iter();
                while cached_length >= self.num_samples_per_encode {
                    let samples_to_write: Vec<f32> = iter.by_ref().take(self.num_samples_per_encode).collect();
                    if samples_to_write.is_empty() {break;}
                    let mut buf = vec![0u8; self.num_samples_per_encode]; // Allocates a buffer of sufficient size, reserving one byte per sample.
                    let size = self.encoder.encode_float(&samples_to_write, &mut buf)?;
                    buf.truncate(size);
                    writer.write_all(&buf)?;
                    cached_length -= self.num_samples_per_encode;
                    self.samples_written += self.num_samples_per_encode as u64;
                    self.bytes_written += buf.len() as u64;
                }
                self.sample_cache = iter.collect();
                Ok(())
            }

            pub fn flush(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                if !self.sample_cache.is_empty() {
                    let pad = (self.num_samples_per_encode - self.sample_cache.len() % self.num_samples_per_encode) % self.num_samples_per_encode;
                    self.write_samples(writer, &vec![0.0f32; pad])?;
                }
                Ok(())
            }
        }

        impl EncoderToImpl for OpusEncoder {
            fn get_bitrate(&self, channels: u16) -> u32 {
                if self.samples_written > 0 {
                    (self.sample_rate as u64 * self.bytes_written / self.samples_written * channels as u64 * 8) as u32
                } else {
                    0
                }
            }
            fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, _bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
                if let Some(channel_mask) = channel_mask {
                    const MONO_MASK: u32 = SpeakerPosition::FrontCenter as u32;
                    const STEREO_MASK: u32 = SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32;
                    match (channels, channel_mask) {
                        (1, MONO_MASK) => (),
                        (2, STEREO_MASK) => (),
                        _ => return Err(AudioWriteError::Unsupported(format!("Channel masks is not supported by the opus format, and the mask is 0x{:08x} for {channels} channels.", channel_mask))),
                    }
                }
                if channels != self.channels {
                    return Err(AudioWriteError::InvalidArguments(format!("The opus encoder has already configured the channels {}, which is different from the new channels {channels}", self.channels)));
                }
                if sample_rate != self.sample_rate {
                    return Err(AudioWriteError::InvalidArguments(format!("The opus encoder has already configured the sample rate {}, which is different from the new sample rate {sample_rate}", self.sample_rate)));
                }
                Ok(FmtChunk{
                    format_tag: 0x704F,
                    channels,
                    sample_rate,
                    byte_rate: self.get_bitrate(channels) / 8,
                    block_align: self.num_samples_per_encode as u16,
                    bits_per_sample: 0,
                    extension: None,
                })
            }
            fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
                fmt.byte_rate = self.get_bitrate(fmt.channels) / 8;
                fmt.block_align = self.num_samples_per_encode as u16;
                Ok(())
            }
            fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.flush(writer)?;
                writer.flush()?;
                Ok(())
            }

            fn write_samples__i8(&mut self, writer: &mut dyn Writer, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_i16(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_i24(&mut self, writer: &mut dyn Writer, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_i32(&mut self, writer: &mut dyn Writer, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_i64(&mut self, writer: &mut dyn Writer, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples__u8(&mut self, writer: &mut dyn Writer, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_u16(&mut self, writer: &mut dyn Writer, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_u24(&mut self, writer: &mut dyn Writer, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_u32(&mut self, writer: &mut dyn Writer, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_u64(&mut self, writer: &mut dyn Writer, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_f32(&mut self, writer: &mut dyn Writer, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
            fn write_samples_f64(&mut self, writer: &mut dyn Writer, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_samples(writer, &sample_conv(samples))}
        }
    }

    #[cfg(feature = "opus")]
    pub use impl_opus::*;
}


