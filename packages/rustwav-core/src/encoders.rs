#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

use xlaw::{PcmXLawEncoder, XLaw};
use io_utils::Writer;
use audioutils::{sample_conv, sample_conv_batch, stereo_conv, stereos_conv};
use sampletypes::{SampleType, i24, u24};
use crate::adpcm;
use crate::errors::AudioWriteError;
use crate::format_specs::format_tags::*;
use crate::format_specs::guids::*;
use crate::wavcore::{ExtensibleData, FmtChunk, FmtExtension};
use crate::wavcore::{Spec, WaveSampleType};

/// An encoder that accepts samples of type `S` and encodes them into the file's target format.
/// Due to trait bounds prohibiting generic parameters, each function must be explicitly
/// implemented for every supported type.
pub trait EncoderToImpl: Debug {
    fn get_channels(&self) -> u16;
    fn get_max_channels(&self) -> u16;
    fn get_bitrate(&self) -> u32;
    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError>;
    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError>;
    fn begin_encoding(&mut self) -> Result<(), AudioWriteError>;
    fn finish(&mut self) -> Result<(), AudioWriteError>;

    // Write interleaved samples
    fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError>;
    fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError>;

    // Convenience interfaces for writing audio frames. Each frame is an array of samples per channel. Default implementations are provided.
    fn write_frame__i8(&mut self, frame: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_i16(&mut self, frame: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_i24(&mut self, frame: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_i32(&mut self, frame: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_i64(&mut self, frame: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame__u8(&mut self, frame: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_u16(&mut self, frame: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_u24(&mut self, frame: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_u32(&mut self, frame: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_u64(&mut self, frame: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_f32(&mut self, frame: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}
    fn write_frame_f64(&mut self, frame: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&audioutils::frames_to_interleaved_samples(&[frame.to_vec()])?)}

    // Convenience interfaces for writing multiple audio frames. Default implementations are provided.
    fn write_frames__i8(&mut self, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_i16(&mut self, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_i24(&mut self, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_i32(&mut self, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_i64(&mut self, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames__u8(&mut self, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_u16(&mut self, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_u24(&mut self, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_u32(&mut self, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_u64(&mut self, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_f32(&mut self, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&audioutils::frames_to_interleaved_samples(frames)?)}
    fn write_frames_f64(&mut self, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&audioutils::frames_to_interleaved_samples(frames)?)}

    // Interfaces for writing mono audio frames (single-channel). Default implementations are provided.
    fn write_mono__i8(&mut self, sample: i8 ) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&(0..self.get_channels()).map(|_|sample).collect::<Vec<i8 >>())}
    fn write_mono_i16(&mut self, sample: i16) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&(0..self.get_channels()).map(|_|sample).collect::<Vec<i16>>())}
    fn write_mono_i24(&mut self, sample: i24) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&(0..self.get_channels()).map(|_|sample).collect::<Vec<i24>>())}
    fn write_mono_i32(&mut self, sample: i32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&(0..self.get_channels()).map(|_|sample).collect::<Vec<i32>>())}
    fn write_mono_i64(&mut self, sample: i64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&(0..self.get_channels()).map(|_|sample).collect::<Vec<i64>>())}
    fn write_mono__u8(&mut self, sample: u8 ) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&(0..self.get_channels()).map(|_|sample).collect::<Vec<u8 >>())}
    fn write_mono_u16(&mut self, sample: u16) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&(0..self.get_channels()).map(|_|sample).collect::<Vec<u16>>())}
    fn write_mono_u24(&mut self, sample: u24) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&(0..self.get_channels()).map(|_|sample).collect::<Vec<u24>>())}
    fn write_mono_u32(&mut self, sample: u32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&(0..self.get_channels()).map(|_|sample).collect::<Vec<u32>>())}
    fn write_mono_u64(&mut self, sample: u64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&(0..self.get_channels()).map(|_|sample).collect::<Vec<u64>>())}
    fn write_mono_f32(&mut self, sample: f32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&(0..self.get_channels()).map(|_|sample).collect::<Vec<f32>>())}
    fn write_mono_f64(&mut self, sample: f64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&(0..self.get_channels()).map(|_|sample).collect::<Vec<f64>>())}

    // Interfaces for writing batched mono audio frames. Default implementations are provided.
    fn write_mono_channel__i8(&mut self, monos: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i8 >>()).collect::<Vec<Vec<i8 >>>().into_iter().flatten().collect::<Vec<i8 >>())}
    fn write_mono_channel_i16(&mut self, monos: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i16>>()).collect::<Vec<Vec<i16>>>().into_iter().flatten().collect::<Vec<i16>>())}
    fn write_mono_channel_i24(&mut self, monos: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i24>>()).collect::<Vec<Vec<i24>>>().into_iter().flatten().collect::<Vec<i24>>())}
    fn write_mono_channel_i32(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i32>>()).collect::<Vec<Vec<i32>>>().into_iter().flatten().collect::<Vec<i32>>())}
    fn write_mono_channel_i64(&mut self, monos: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i64>>()).collect::<Vec<Vec<i64>>>().into_iter().flatten().collect::<Vec<i64>>())}
    fn write_mono_channel__u8(&mut self, monos: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u8 >>()).collect::<Vec<Vec<u8 >>>().into_iter().flatten().collect::<Vec<u8 >>())}
    fn write_mono_channel_u16(&mut self, monos: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u16>>()).collect::<Vec<Vec<u16>>>().into_iter().flatten().collect::<Vec<u16>>())}
    fn write_mono_channel_u24(&mut self, monos: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u24>>()).collect::<Vec<Vec<u24>>>().into_iter().flatten().collect::<Vec<u24>>())}
    fn write_mono_channel_u32(&mut self, monos: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u32>>()).collect::<Vec<Vec<u32>>>().into_iter().flatten().collect::<Vec<u32>>())}
    fn write_mono_channel_u64(&mut self, monos: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u64>>()).collect::<Vec<Vec<u64>>>().into_iter().flatten().collect::<Vec<u64>>())}
    fn write_mono_channel_f32(&mut self, monos: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<f32>>()).collect::<Vec<Vec<f32>>>().into_iter().flatten().collect::<Vec<f32>>())}
    fn write_mono_channel_f64(&mut self, monos: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&monos.iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<f64>>()).collect::<Vec<Vec<f64>>>().into_iter().flatten().collect::<Vec<f64>>())}

    // Interfaces for writing stereo audio (composed of two separate channel buffers). Default implementations are provided.
    fn write_dual_sample__i8(&mut self, mono1: i8 , mono2: i8 ) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&[mono1, mono2])}
    fn write_dual_sample_i16(&mut self, mono1: i16, mono2: i16) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&[mono1, mono2])}
    fn write_dual_sample_i24(&mut self, mono1: i24, mono2: i24) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&[mono1, mono2])}
    fn write_dual_sample_i32(&mut self, mono1: i32, mono2: i32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&[mono1, mono2])}
    fn write_dual_sample_i64(&mut self, mono1: i64, mono2: i64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&[mono1, mono2])}
    fn write_dual_sample__u8(&mut self, mono1: u8 , mono2: u8 ) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&[mono1, mono2])}
    fn write_dual_sample_u16(&mut self, mono1: u16, mono2: u16) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&[mono1, mono2])}
    fn write_dual_sample_u24(&mut self, mono1: u24, mono2: u24) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&[mono1, mono2])}
    fn write_dual_sample_u32(&mut self, mono1: u32, mono2: u32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&[mono1, mono2])}
    fn write_dual_sample_u64(&mut self, mono1: u64, mono2: u64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&[mono1, mono2])}
    fn write_dual_sample_f32(&mut self, mono1: f32, mono2: f32) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&[mono1, mono2])}
    fn write_dual_sample_f64(&mut self, mono1: f64, mono2: f64) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&[mono1, mono2])}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_dual_monos__i8(&mut self, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i16(&mut self, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i24(&mut self, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i32(&mut self, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i64(&mut self, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos__u8(&mut self, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u16(&mut self, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u24(&mut self, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u32(&mut self, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u64(&mut self, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f32(&mut self, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f64(&mut self, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&audioutils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_monos__i8(&mut self, monos_array: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i16(&mut self, monos_array: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i24(&mut self, monos_array: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i32(&mut self, monos_array: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i64(&mut self, monos_array: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos__u8(&mut self, monos_array: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u16(&mut self, monos_array: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u24(&mut self, monos_array: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u32(&mut self, monos_array: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u64(&mut self, monos_array: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f32(&mut self, monos_array: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&audioutils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f64(&mut self, monos_array: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&audioutils::monos_to_interleaved_samples(monos_array)?)}

    // Interfaces for writing stereo audio frames using tuples (L, R). Default implementations are provided.
    fn write_stereo__i8(&mut self, stereo: (i8 , i8 )) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&[stereo.0, stereo.1])}
    fn write_stereo_i16(&mut self, stereo: (i16, i16)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&[stereo.0, stereo.1])}
    fn write_stereo_i24(&mut self, stereo: (i24, i24)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&[stereo.0, stereo.1])}
    fn write_stereo_i32(&mut self, stereo: (i32, i32)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&[stereo.0, stereo.1])}
    fn write_stereo_i64(&mut self, stereo: (i64, i64)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&[stereo.0, stereo.1])}
    fn write_stereo__u8(&mut self, stereo: (u8 , u8 )) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&[stereo.0, stereo.1])}
    fn write_stereo_u16(&mut self, stereo: (u16, u16)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&[stereo.0, stereo.1])}
    fn write_stereo_u24(&mut self, stereo: (u24, u24)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&[stereo.0, stereo.1])}
    fn write_stereo_u32(&mut self, stereo: (u32, u32)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&[stereo.0, stereo.1])}
    fn write_stereo_u64(&mut self, stereo: (u64, u64)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&[stereo.0, stereo.1])}
    fn write_stereo_f32(&mut self, stereo: (f32, f32)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&[stereo.0, stereo.1])}
    fn write_stereo_f64(&mut self, stereo: (f64, f64)) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&[stereo.0, stereo.1])}

    // Interfaces for writing stereo audio frames using arrays of tuples. Default implementations are provided.
    fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&audioutils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&audioutils::stereos_to_interleaved_samples(stereos))}
}

/// * The `DummyEncoder` is not for you to use, it allows me to implement `Default` for `Encoder<'a>`
#[derive(Debug, Clone, Copy)]
pub struct DummyEncoder;

/// * Default implementations: all functions are `panic!`
impl EncoderToImpl for DummyEncoder {
    fn get_channels(&self) -> u16 {
        panic!("Encoder creation failed.");
    }

    fn get_max_channels(&self) -> u16 {
        panic!("Encoder creation failed.");
    }

    fn get_bitrate(&self) -> u32 {
        panic!("Encoder creation failed.");
    }

    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        panic!("Encoder creation failed.");
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        panic!("Encoder creation failed.");
    }

    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        panic!("Encoder creation failed.");
    }

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        panic!("Encoder creation failed.");
    }

    fn write_interleaved_samples_f32(&mut self, _samples: &[f32]) -> Result<(), AudioWriteError> {
        panic!("Encoder creation failed.");
    }

    fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
    fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&sample_conv(samples))}
}

/// * The `Encoder` struct contains all of the encoder types and provides convenient functions that have generic type parameters.
/// * It just translates the API to the inner encoder API.
#[derive(Debug)]
pub struct Encoder<'a> {
    encoder: Box<dyn EncoderToImpl + 'a>,
}

impl Default for Encoder<'_> {
    fn default() -> Self {
        Self::new(DummyEncoder)
    }
}

impl<'a> Encoder<'a> {
    pub fn new<T>(encoder: T) -> Self
    where
        T: EncoderToImpl + 'a,
    {
        Self {
            encoder: Box::new(encoder),
        }
    }

    pub fn get_channels(&self) -> u16 {
        self.encoder.get_channels()
    }

    pub fn get_max_channels(&self) -> u16 {
        self.encoder.get_max_channels()
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

    /// * Write samples regardless of channels
    pub fn write_interleaved_samples<S>(&mut self, samples: &[S]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_interleaved_samples__i8(&sample_conv(samples)),
            "i16" => self.encoder.write_interleaved_samples_i16(&sample_conv(samples)),
            "i24" => self.encoder.write_interleaved_samples_i24(&sample_conv(samples)),
            "i32" => self.encoder.write_interleaved_samples_i32(&sample_conv(samples)),
            "i64" => self.encoder.write_interleaved_samples_i64(&sample_conv(samples)),
            "u8"  => self.encoder.write_interleaved_samples__u8(&sample_conv(samples)),
            "u16" => self.encoder.write_interleaved_samples_u16(&sample_conv(samples)),
            "u24" => self.encoder.write_interleaved_samples_u24(&sample_conv(samples)),
            "u32" => self.encoder.write_interleaved_samples_u32(&sample_conv(samples)),
            "u64" => self.encoder.write_interleaved_samples_u64(&sample_conv(samples)),
            "f32" => self.encoder.write_interleaved_samples_f32(&sample_conv(samples)),
            "f64" => self.encoder.write_interleaved_samples_f64(&sample_conv(samples)),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write an audio frame, each frame contains one sample for all channels
    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
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
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write audio frames, each frame contains one sample for all channels
    pub fn write_frames<S>(&mut self, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_frames__i8(&sample_conv_batch(frames)),
            "i16" => self.encoder.write_frames_i16(&sample_conv_batch(frames)),
            "i24" => self.encoder.write_frames_i24(&sample_conv_batch(frames)),
            "i32" => self.encoder.write_frames_i32(&sample_conv_batch(frames)),
            "i64" => self.encoder.write_frames_i64(&sample_conv_batch(frames)),
            "u8"  => self.encoder.write_frames__u8(&sample_conv_batch(frames)),
            "u16" => self.encoder.write_frames_u16(&sample_conv_batch(frames)),
            "u24" => self.encoder.write_frames_u24(&sample_conv_batch(frames)),
            "u32" => self.encoder.write_frames_u32(&sample_conv_batch(frames)),
            "u64" => self.encoder.write_frames_u64(&sample_conv_batch(frames)),
            "f32" => self.encoder.write_frames_f32(&sample_conv_batch(frames)),
            "f64" => self.encoder.write_frames_f64(&sample_conv_batch(frames)),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write only one sample regardless of channels
    pub fn write_sample<S>(&mut self, mono: S) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_mono__i8(mono.to_i8 ()),
            "i16" => self.encoder.write_mono_i16(mono.to_i16()),
            "i24" => self.encoder.write_mono_i24(mono.to_i24()),
            "i32" => self.encoder.write_mono_i32(mono.to_i32()),
            "i64" => self.encoder.write_mono_i64(mono.to_i64()),
            "u8"  => self.encoder.write_mono__u8(mono.to_u8 ()),
            "u16" => self.encoder.write_mono_u16(mono.to_u16()),
            "u24" => self.encoder.write_mono_u24(mono.to_u24()),
            "u32" => self.encoder.write_mono_u32(mono.to_u32()),
            "u64" => self.encoder.write_mono_u64(mono.to_u64()),
            "f32" => self.encoder.write_mono_f32(mono.to_f32()),
            "f64" => self.encoder.write_mono_f64(mono.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write a single channel of audio to the encoder
    pub fn write_mono<S>(&mut self, monos: S) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_mono__i8(monos.as_i8 ()),
            "i16" => self.encoder.write_mono_i16(monos.as_i16()),
            "i24" => self.encoder.write_mono_i24(monos.as_i24()),
            "i32" => self.encoder.write_mono_i32(monos.as_i32()),
            "i64" => self.encoder.write_mono_i64(monos.as_i64()),
            "u8"  => self.encoder.write_mono__u8(monos.as_u8 ()),
            "u16" => self.encoder.write_mono_u16(monos.as_u16()),
            "u24" => self.encoder.write_mono_u24(monos.as_u24()),
            "u32" => self.encoder.write_mono_u32(monos.as_u32()),
            "u64" => self.encoder.write_mono_u64(monos.as_u64()),
            "f32" => self.encoder.write_mono_f32(monos.as_f32()),
            "f64" => self.encoder.write_mono_f64(monos.as_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write a single channel of audio to the encoder
    pub fn write_mono_channel<S>(&mut self, monos: &[S]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_mono_channel__i8(&sample_conv(monos)),
            "i16" => self.encoder.write_mono_channel_i16(&sample_conv(monos)),
            "i24" => self.encoder.write_mono_channel_i24(&sample_conv(monos)),
            "i32" => self.encoder.write_mono_channel_i32(&sample_conv(monos)),
            "i64" => self.encoder.write_mono_channel_i64(&sample_conv(monos)),
            "u8"  => self.encoder.write_mono_channel__u8(&sample_conv(monos)),
            "u16" => self.encoder.write_mono_channel_u16(&sample_conv(monos)),
            "u24" => self.encoder.write_mono_channel_u24(&sample_conv(monos)),
            "u32" => self.encoder.write_mono_channel_u32(&sample_conv(monos)),
            "u64" => self.encoder.write_mono_channel_u64(&sample_conv(monos)),
            "f32" => self.encoder.write_mono_channel_f32(&sample_conv(monos)),
            "f64" => self.encoder.write_mono_channel_f64(&sample_conv(monos)),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write double samples of audio to the encoder
    pub fn write_dual_mono<S>(&mut self, mono1: S, mono2: S) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
            "i8"  => self.encoder.write_dual_sample__i8(mono1.to_i8 (), mono2.to_i8 ()),
            "i16" => self.encoder.write_dual_sample_i16(mono1.to_i16(), mono2.to_i16()),
            "i24" => self.encoder.write_dual_sample_i24(mono1.to_i24(), mono2.to_i24()),
            "i32" => self.encoder.write_dual_sample_i32(mono1.to_i32(), mono2.to_i32()),
            "i64" => self.encoder.write_dual_sample_i64(mono1.to_i64(), mono2.to_i64()),
            "u8"  => self.encoder.write_dual_sample__u8(mono1.to_u8 (), mono2.to_u8 ()),
            "u16" => self.encoder.write_dual_sample_u16(mono1.to_u16(), mono2.to_u16()),
            "u24" => self.encoder.write_dual_sample_u24(mono1.to_u24(), mono2.to_u24()),
            "u32" => self.encoder.write_dual_sample_u32(mono1.to_u32(), mono2.to_u32()),
            "u64" => self.encoder.write_dual_sample_u64(mono1.to_u64(), mono2.to_u64()),
            "f32" => self.encoder.write_dual_sample_f32(mono1.to_f32(), mono2.to_f32()),
            "f64" => self.encoder.write_dual_sample_f64(mono1.to_f64(), mono2.to_f64()),
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write double channels of audio to the encoder
    pub fn write_dual_monos<S>(&mut self, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
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
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write multiple channels of audio to the encoder
    pub fn write_monos<S>(&mut self, monos: &[Vec<S>]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
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
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write only one stereo sample to the encoder
    pub fn write_stereo<S>(&mut self, stereo: (S, S)) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
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
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }

    /// * Write stereo samples to the encoder
    pub fn write_stereos<S>(&mut self, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where
        S: SampleType,
    {
        match S::TYPE_NAME {
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
            other => Err(AudioWriteError::InvalidArguments(format!(
                "Bad sample type: {}",
                other
            ))),
        }
    }
}

/// * `PcmEncoderFrom<S>`: Transcodes samples from type `S` into the target format.
/// * This is a component for the `PcmEncoder`
#[derive(Debug, Clone, Copy)]
struct PcmEncoderFrom<S>
where
    S: SampleType,
{
    write_fn: fn(&mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>,
}

impl<S> PcmEncoderFrom<S>
where
    S: SampleType,
{
    pub fn new(target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64};
        Ok(Self {
            write_fn: match target_sample {
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
                other => {
                    return Err(AudioWriteError::InvalidArguments(format!(
                        "Unknown target sample type: \"{:?}\"",
                        other
                    )));
                }
            },
        })
    }

    /// S: The input format provided to us (external source).
    /// T: The target format to be written into the WAV file.
    fn write_sample_to<T>(writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>
    where
        T: SampleType,
    {
        for sample in frame.iter() {
            T::scale_from(*sample).write_le(writer)?;
        }
        Ok(())
    }

    pub fn write_frame(
        &mut self,
        writer: &mut dyn Writer,
        frame: &[S],
    ) -> Result<(), AudioWriteError> {
        (self.write_fn)(writer, frame)
    }

    pub fn write_frames(
        &mut self,
        writer: &mut dyn Writer,
        frames: &[Vec<S>],
    ) -> Result<(), AudioWriteError> {
        for frame in frames.iter() {
            (self.write_fn)(writer, frame)?;
        }
        Ok(())
    }

    pub fn write_interleaved_samples(
        &mut self,
        writer: &mut dyn Writer,
        samples: &[S],
    ) -> Result<(), AudioWriteError> {
        (self.write_fn)(writer, samples)
    }
}

/// * `PcmEncoder`: convert various formats of PCM samples to the WAV file specific sample type
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
    /// * target_sample: The specific PCM format (e.g., bit depth, signedness) to encode into the WAV file.
    pub fn new(writer: &'a mut dyn Writer, spec: Spec) -> Result<Self, AudioWriteError> {
        if !spec.is_channel_mask_valid() {
            return Err(AudioWriteError::InvalidArguments(format!(
                "Number of bits of channel mask 0x{:08x} does not match {} channels",
                spec.channel_mask, spec.channels
            )));
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

impl EncoderToImpl for PcmEncoder<'_> {
    fn get_channels(&self) -> u16 {
        self.spec.channels
    }

    fn get_max_channels(&self) -> u16 {
        18
    }

    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, Unknown};
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
                    other => {
                        return Err(AudioWriteError::Unsupported(format!(
                            "\"{:?}\" was given for the extensible format PCM to specify the sample format",
                            other
                        )));
                    }
                },
            })),
        };
        Ok(FmtChunk {
            format_tag: if extensible.is_some() {
                FORMAT_TAG_EXTENSIBLE
            } else {
                match self.sample_type {
                    S8 | U16 | U24 | U32 | U64 => {
                        return Err(AudioWriteError::Unsupported(format!(
                            "PCM format does not support {} samples.",
                            self.sample_type
                        )));
                    }
                    U8 | S16 | S24 | S32 | S64 => FORMAT_TAG_PCM,
                    F32 | F64 => FORMAT_TAG_PCM_IEEE,
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

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        Ok(self.writer.flush()?)
    }

    fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.writer_from__i8.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.writer_from_i16.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.writer_from_i24.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.writer_from_i32.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.writer_from_i64.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.writer_from__u8.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.writer_from_u16.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.writer_from_u24.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.writer_from_u32.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.writer_from_u64.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.writer_from_f32.write_interleaved_samples(self.writer, samples)}
    fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.writer_from_f64.write_interleaved_samples(self.writer, samples)}
}

/// * `AdpcmEncoderWrap<E>`: encode `i16` audio samples to ADPCM nibbles
#[derive(Debug)]
pub struct AdpcmEncoderWrap<'a, E>
where
    E: adpcm::AdpcmEncoder,
{
    writer: &'a mut dyn Writer,
    channels: u16,
    sample_rate: u32,
    bytes_written: u64,
    encoder: E,
    nibbles: Vec<u8>,
}

const MAX_BUFFER_USAGE: usize = 1024;

impl<'a, E> AdpcmEncoderWrap<'a, E>
where
    E: adpcm::AdpcmEncoder,
{
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

    pub fn write_interleaved_samples(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {
        let mut iter = samples.iter().copied();
        self.encoder.encode(
            || -> Option<i16> { iter.next() },
            |byte: u8| {
                self.nibbles.push(byte);
            },
        )?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers()?;
        }
        Ok(())
    }

    pub fn write_stereos(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {
        if self.channels != 2 {
            return Err(AudioWriteError::Unsupported(format!(
                "This encoder only accepts {} channel audio data",
                self.channels
            )));
        }
        let mut iter = audioutils::stereos_to_interleaved_samples(stereos).into_iter();
        self.encoder.encode(
            || -> Option<i16> { iter.next() },
            |byte: u8| {
                self.nibbles.push(byte);
            },
        )?;
        if self.nibbles.len() >= MAX_BUFFER_USAGE {
            self.flush_buffers()?;
        }
        Ok(())
    }
}

impl<E> EncoderToImpl for AdpcmEncoderWrap<'_, E>
where
    E: adpcm::AdpcmEncoder,
{
    fn get_channels(&self) -> u16 {
        self.channels
    }

    fn get_max_channels(&self) -> u16 {
        2
    }

    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        Ok(self
            .encoder
            .new_fmt_chunk(self.channels, self.sample_rate, 4)?)
    }

    fn get_bitrate(&self) -> u32 {
        self.sample_rate * self.channels as u32 * 4
    }

    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        Ok(self.encoder.modify_fmt_chunk(fmt)?)
    }

    fn finish(&mut self) -> Result<(), AudioWriteError> {
        self.encoder.flush(|nibble: u8| {
            self.nibbles.push(nibble);
        })?;
        self.flush_buffers()?;
        Ok(self.writer.flush()?)
    }

    fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}

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

/// * `PcmXLawEncoderWrap`: encode `i16` audio samples to bytes
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

    pub fn write_interleaved_samples(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {
        self.writer.write_all(
            &samples
                .iter()
                .map(|sample| -> u8 { self.enc.encode(*sample) })
                .collect::<Vec<u8>>(),
        )?;
        Ok(())
    }
}

impl EncoderToImpl for PcmXLawEncoderWrap<'_> {
    fn get_channels(&self) -> u16 {
        self.channels
    }

    fn get_max_channels(&self) -> u16 {
        2
    }

    fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
        Ok(())
    }

    fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
        let bits_per_sample = 8u16;
        let block_align = self.channels;
        Ok(FmtChunk {
            format_tag: match self.enc.get_which_law() {
                XLaw::ALaw => FORMAT_TAG_ALAW,
                XLaw::MuLaw => FORMAT_TAG_MULAW,
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

    fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
    fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
}

/// * The MP3 encoder for `WaveWriter`
pub mod mp3 {
    use crate::wavcore::mp3::*;

    #[cfg(feature = "mp3enc")]
    use super::EncoderToImpl;

    #[cfg(feature = "mp3enc")]
    pub mod impl_mp3 {
        use super::*;
        use crate::errors::AudioWriteError;
        use crate::io_utils::Writer;
        use crate::audioutils::{self, sample_conv, stereos_conv};
        use crate::wavcore::format_tags::*;
        use crate::wavcore::{FmtChunk, FmtExtension, Mp3Data, Spec};
        use crate::{SampleType, i24, u24};
        use std::{
            any::type_name,
            fmt::{self, Debug, Formatter},
            ops::DerefMut,
            sync::{Arc, Mutex},
        };

        use mp3lame_encoder::{Bitrate, Id3Tag, Mode, Quality, VbrMode};
        use mp3lame_encoder::{Builder, DualPcm, Encoder, FlushNoGap, MonoPcm};

        const MAX_SAMPLES_TO_ENCODE: usize = 1024;

        #[derive(Clone)]
        pub struct SharedMp3Encoder(Arc<Mutex<Encoder>>);

        impl SharedMp3Encoder {
            pub fn new(encoder: Encoder) -> Self {
                Self(Arc::new(Mutex::new(encoder)))
            }

            pub fn escorted_encode<T, F, E>(&self, mut action: F) -> Result<T, E>
            where
                F: FnMut(&mut Encoder) -> Result<T, E>,
            {
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
            pub fn to_lame_options(&self) -> Mp3EncoderLameOptions {
                Mp3EncoderLameOptions {
                    channels: self.channels.to_lame_mode(),
                    quality: self.quality.to_lame_quality(),
                    bitrate: self.bitrate.to_lame_bitrate(),
                    vbr_mode: self.vbr_mode.to_lame_vbr_mode(),
                    id3tag: self.id3tag.clone(),
                }
            }
        }

        #[derive(Debug)]
        pub struct Mp3Encoder<'a, S>
        where
            S: SampleType,
        {
            channels: u16,
            sample_rate: u32,
            bitrate: u32,
            encoder: SharedMp3Encoder,
            options: Mp3EncoderOptions,
            buffers: ChannelBuffers<'a, S>,
        }

        impl<'a, S> Mp3Encoder<'a, S>
        where
            S: SampleType,
        {
            pub fn new(
                writer: &'a mut dyn Writer,
                spec: Spec,
                mp3_options: &Mp3EncoderOptions,
            ) -> Result<Self, AudioWriteError> {
                if spec.channels != mp3_options.get_channels() {
                    return Err(AudioWriteError::InvalidArguments(format!(
                        "The number of channels from `spec` is {}, but from `mp3_options` is {}",
                        spec.channels,
                        mp3_options.get_channels()
                    )));
                }

                let mp3_builder = Builder::new();
                let mut mp3_builder = match mp3_builder {
                    Some(mp3_builder) => mp3_builder,
                    None => {
                        return Err(AudioWriteError::OtherReason(
                            "`lame_init()` somehow failed.".to_owned(),
                        ));
                    }
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
                    mp3_builder.set_id3_tag(Id3Tag {
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
                    options: mp3_options.clone(),
                    buffers: match channels {
                        1 | 2 => ChannelBuffers::<'a, S>::new(
                            writer,
                            encoder.clone(),
                            MAX_SAMPLES_TO_ENCODE,
                            channels,
                        )?,
                        o => {
                            return Err(AudioWriteError::Unsupported(format!(
                                "Bad channel number: {o}"
                            )));
                        }
                    },
                })
            }

            pub fn write_interleaved_samples<T>(&mut self, samples: &[T]) -> Result<(), AudioWriteError>
            where
                T: SampleType,
            {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels {
                    1 => self.buffers.add_mono_channel(&sample_conv::<T, S>(samples)),
                    2 => self
                        .buffers
                        .add_stereos(&audioutils::interleaved_samples_to_stereos(
                            &sample_conv::<T, S>(samples),
                        )?),
                    o => Err(AudioWriteError::Unsupported(format!(
                        "Bad channels number: {o}"
                    ))),
                }
            }

            pub fn write_stereos<T>(&mut self, stereos: &[(T, T)]) -> Result<(), AudioWriteError>
            where
                T: SampleType,
            {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels {
                    1 => self
                        .buffers
                        .add_mono_channel(&audioutils::stereos_to_mono_channel(&stereos_conv::<T, S>(stereos))),
                    2 => self.buffers.add_stereos(&stereos_conv::<T, S>(stereos)),
                    o => Err(AudioWriteError::InvalidArguments(format!(
                        "Bad channels number: {o}"
                    ))),
                }
            }

            pub fn write_dual_monos<T>(
                &mut self,
                mono_l: &[T],
                mono_r: &[T],
            ) -> Result<(), AudioWriteError>
            where
                T: SampleType,
            {
                if self.buffers.is_full() {
                    self.buffers.flush()?;
                }
                match self.channels {
                    1 => self
                        .buffers
                        .add_mono_channel(&sample_conv::<T, S>(&audioutils::dual_monos_to_monos(&(
                            mono_l.to_vec(),
                            mono_r.to_vec(),
                        ))?)),
                    2 => self
                        .buffers
                        .add_dual_monos(&sample_conv::<T, S>(mono_l), &sample_conv::<T, S>(mono_r)),
                    o => Err(AudioWriteError::InvalidArguments(format!(
                        "Bad channels number: {o}"
                    ))),
                }
            }

            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.buffers.finish()
            }
        }

        #[derive(Debug, Clone)]
        enum Channels<S>
        where
            S: SampleType,
        {
            Mono(Vec<S>),
            Stereo((Vec<S>, Vec<S>)),
        }

        struct ChannelBuffers<'a, S>
        where
            S: SampleType,
        {
            writer: &'a mut dyn Writer,
            encoder: SharedMp3Encoder,
            channels: Channels<S>,
            max_frames: usize,
        }

        impl<S> Channels<S>
        where
            S: SampleType,
        {
            pub fn new_mono(max_frames: usize) -> Self {
                Self::Mono(Vec::<S>::with_capacity(max_frames))
            }
            pub fn new_stereo(max_frames: usize) -> Self {
                Self::Stereo((
                    Vec::<S>::with_capacity(max_frames),
                    Vec::<S>::with_capacity(max_frames),
                ))
            }
            pub fn add_mono(&mut self, frame: S) {
                match self {
                    Self::Mono(m) => m.push(frame),
                    Self::Stereo((l, r)) => {
                        l.push(frame);
                        r.push(frame);
                    }
                }
            }
            pub fn add_stereo(&mut self, frame: (S, S)) {
                match self {
                    Self::Mono(m) => m.push(S::average(frame.0, frame.1)),
                    Self::Stereo((l, r)) => {
                        l.push(frame.0);
                        r.push(frame.1);
                    }
                }
            }
            pub fn add_mono_channel(&mut self, frames: &[S]) {
                match self {
                    Self::Mono(m) => m.extend(frames),
                    Self::Stereo((l, r)) => {
                        l.extend(frames);
                        r.extend(frames);
                    }
                }
            }
            pub fn add_stereos(&mut self, frames: &[(S, S)]) {
                match self {
                    Self::Mono(m) => m.extend(audioutils::stereos_to_mono_channel(frames)),
                    Self::Stereo((l, r)) => {
                        let (il, ir) = audioutils::stereos_to_dual_monos(frames);
                        l.extend(il);
                        r.extend(ir);
                    }
                }
            }
            pub fn add_dual_monos(
                &mut self,
                monos_l: &[S],
                monos_r: &[S],
            ) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(m) => m.extend(audioutils::dual_monos_to_monos(&(
                        monos_l.to_vec(),
                        monos_r.to_vec(),
                    ))?),
                    Self::Stereo((l, r)) => {
                        if monos_l.len() != monos_r.len() {
                            return Err(AudioWriteError::ChannelsNotInSameSize);
                        }
                        l.extend(monos_l);
                        r.extend(monos_r);
                    }
                }
                Ok(())
            }
            pub fn len(&self) -> usize {
                match self {
                    Self::Mono(m) => m.len(),
                    Self::Stereo((l, r)) => {
                        assert_eq!(l.len(), r.len());
                        l.len()
                    }
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
                    Self::Mono(m) => *m = Vec::<S>::with_capacity(max_frames),
                    Self::Stereo(s) => {
                        *s = (
                            Vec::<S>::with_capacity(max_frames),
                            Vec::<S>::with_capacity(max_frames),
                        )
                    }
                }
            }
        }

        impl<'a, S> ChannelBuffers<'a, S>
        where
            S: SampleType,
        {
            pub fn new(
                writer: &'a mut dyn Writer,
                encoder: SharedMp3Encoder,
                max_frames: usize,
                channels: u16,
            ) -> Result<Self, AudioWriteError> {
                Ok(Self {
                    writer,
                    encoder,
                    channels: match channels {
                        1 => Channels::<S>::new_mono(max_frames),
                        2 => Channels::<S>::new_stereo(max_frames),
                        o => {
                            return Err(AudioWriteError::InvalidArguments(format!(
                                "Invalid channels: {o}. Only 1 and 2 are accepted."
                            )));
                        }
                    },
                    max_frames,
                })
            }

            pub fn is_full(&self) -> bool {
                self.channels.len() >= self.max_frames
            }

            pub fn add_mono_channel(&mut self, monos: &[S]) -> Result<(), AudioWriteError> {
                self.channels.add_mono_channel(monos);
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

            pub fn add_dual_monos(
                &mut self,
                monos_l: &[S],
                monos_r: &[S],
            ) -> Result<(), AudioWriteError> {
                self.channels.add_dual_monos(monos_l, monos_r)?;
                if self.is_full() {
                    self.flush()?;
                }
                Ok(())
            }

            fn channel_to_type<T>(mono: &[S]) -> Vec<T>
            where
                T: SampleType,
            {
                mono.iter().map(|s| T::scale_from(*s)).collect()
            }

            fn encode_to_vec(
                &self,
                encoder: &mut Encoder,
                out_buf: &mut Vec<u8>,
            ) -> Result<usize, AudioWriteError> {
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
                    return Ok(());
                }
                let to_save = self.encoder.escorted_encode(
                    |encoder| -> Result<Vec<u8>, AudioWriteError> {
                        let mut to_save = Vec::<u8>::with_capacity(
                            mp3lame_encoder::max_required_buffer_size(self.channels.len()),
                        );
                        self.encode_to_vec(encoder, &mut to_save)?;
                        Ok(to_save)
                    },
                )?;
                self.writer.write_all(&to_save)?;
                self.channels.clear(self.max_frames);
                Ok(())
            }

            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.flush()?;
                self.encoder
                    .escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                        let mut to_save = Vec::<u8>::with_capacity(
                            mp3lame_encoder::max_required_buffer_size(self.max_frames),
                        );
                        encoder.flush_to_vec::<FlushNoGap>(&mut to_save)?;
                        self.writer.write_all(&to_save)?;
                        Ok(())
                    })?;
                self.channels.clear(self.max_frames);
                Ok(())
            }
        }

        impl<S> EncoderToImpl for Mp3Encoder<'_, S>
        where
            S: SampleType,
        {
            fn get_channels(&self) -> u16 {
                self.channels
            }

            fn get_max_channels(&self) -> u16 {
                2
            }

            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                Ok(FmtChunk {
                    format_tag: FORMAT_TAG_MP3,
                    channels: self.channels,
                    sample_rate: self.sample_rate,
                    byte_rate: self.bitrate / 8,
                    block_align: if self.sample_rate <= 28000 {
                        576
                    } else {
                        576 * 2
                    },
                    bits_per_sample: 0,
                    extension: Some(FmtExtension::new_mp3(Mp3Data::new(
                        self.bitrate,
                        self.sample_rate,
                    ))),
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

            fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}
            fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(samples)}

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
                fmt.debug_struct("SharedMp3Encoder").finish_non_exhaustive()
            }
        }

        impl<S> Debug for ChannelBuffers<'_, S>
        where
            S: SampleType,
        {
            fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.debug_struct(&format!("ChannelBuffers<{}>", type_name::<S>()))
                    .field("encoder", &self.encoder)
                    .field(
                        "channels",
                        &format_args!(
                            "{}",
                            match self.channels {
                                Channels::Mono(_) => "Mono",
                                Channels::Stereo(_) => "Stereo",
                            }
                        ),
                    )
                    .field("max_frames", &self.max_frames)
                    .finish()
            }
        }

        impl From<mp3lame_encoder::BuildError> for AudioWriteError {
            fn from(err: mp3lame_encoder::BuildError) -> Self {
                match err {
                    mp3lame_encoder::BuildError::Generic => Self::OtherReason("Generic error".to_owned()),
                    mp3lame_encoder::BuildError::NoMem => Self::OtherReason("No enough memory".to_owned()),
                    mp3lame_encoder::BuildError::BadBRate => Self::InvalidInput("Bad bit rate".to_owned()),
                    mp3lame_encoder::BuildError::BadSampleFreq => Self::InvalidInput("Bad sample rate".to_owned()),
                    mp3lame_encoder::BuildError::InternalError => Self::OtherReason("Internal error".to_owned()),
                    mp3lame_encoder::BuildError::Other(c_int) => Self::OtherReason(format!("Other lame error code: {c_int}")),
                }
            }
        }

        impl From<mp3lame_encoder::Id3TagError> for AudioWriteError {
            fn from(err: mp3lame_encoder::Id3TagError) -> Self {
                match err {
                    mp3lame_encoder::Id3TagError::AlbumArtOverflow => {
                        Self::BufferIsFull("Specified Id3 tag buffer exceed limit of 128kb".to_owned())
                    }
                }
            }
        }

        impl From<mp3lame_encoder::EncodeError> for AudioWriteError {
            fn from(err: mp3lame_encoder::EncodeError) -> Self {
                match err {
                    mp3lame_encoder::EncodeError::BufferTooSmall => Self::BufferIsFull("Buffer is too small".to_owned()),
                    mp3lame_encoder::EncodeError::NoMem => Self::OtherReason("No enough memory".to_owned()),
                    mp3lame_encoder::EncodeError::InvalidState => Self::OtherReason("Invalid state".to_owned()),
                    mp3lame_encoder::EncodeError::PsychoAcoustic => Self::OtherReason("Psycho acoustic problems".to_owned()),
                    mp3lame_encoder::EncodeError::Other(c_int) => Self::OtherReason(format!("Other lame error code: {c_int}")),
                }
            }
        }
    }

    #[cfg(feature = "mp3enc")]
    pub use impl_mp3::*;
}

/// * The Opus encoder for `WaveWriter`
pub mod opus {
    use crate::wavcore::opus::*;

    #[cfg(feature = "opus")]
    use super::EncoderToImpl;

    #[cfg(feature = "opus")]
    pub mod impl_opus {
        use std::{
            fmt::{self, Debug, Formatter},
            mem,
        };

        use super::*;
        use io_utils::Writer;
        use audioutils::sample_conv;
        use crate::errors::AudioWriteError;
        use crate::wavcore::format_tags::*;
        use crate::wavcore::{FmtChunk, Spec};
        use crate::{i24, u24};

        use opus::{self, Application, Bitrate, Channels, Encoder, ErrorCode};

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
            pub fn new(
                writer: &'a mut dyn Writer,
                spec: Spec,
                options: &OpusEncoderOptions,
            ) -> Result<Self, AudioWriteError> {
                let mut opus_channels = Channels::Mono;
                unsafe { // See <https://github.com/SpaceManiac/opus-rs/blob/master/src/lib.rs#L52>
                    *(&mut opus_channels as *mut Channels as usize as *mut u8) = spec.channels as u8;
                };
                if !OPUS_ALLOWED_SAMPLE_RATES.contains(&spec.sample_rate) {
                    return Err(AudioWriteError::InvalidArguments(format!(
                        "Bad sample rate: {} for the opus encoder. The sample rate must be one of {}",
                        spec.sample_rate,
                        OPUS_ALLOWED_SAMPLE_RATES
                            .iter()
                            .map(|s| { format!("{s}") })
                            .collect::<Vec<String>>()
                            .join(", ")
                    )));
                }
                let mut encoder =
                    Encoder::new(spec.sample_rate, opus_channels, Application::Audio)?;
                encoder.set_bitrate(options.bitrate.to_opus_bitrate())?;
                encoder.set_vbr(options.encode_vbr)?;
                let num_samples_per_encode = options
                    .samples_cache_duration
                    .get_num_samples(spec.channels, spec.sample_rate);
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

            pub fn set_cache_duration(
                &mut self,
                samples_cache_duration: OpusEncoderSampleDuration,
            ) {
                self.cache_duration = samples_cache_duration;
                self.num_samples_per_encode =
                    samples_cache_duration.get_num_samples(self.channels, self.sample_rate);
            }

            pub fn write_interleaved_samples(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {
                self.sample_cache.extend(samples);
                let mut cached_length = self.sample_cache.len();
                let mut iter = mem::take(&mut self.sample_cache).into_iter();
                while cached_length >= self.num_samples_per_encode {
                    // Extract `self.num_samples_per_encode` samples to encode
                    let samples_to_write: Vec<f32> =
                        iter.by_ref().take(self.num_samples_per_encode).collect();
                    if samples_to_write.is_empty() {
                        break;
                    }

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
                    let pad = (self.num_samples_per_encode
                        - self.sample_cache.len() % self.num_samples_per_encode)
                        % self.num_samples_per_encode;

                    // Pad to the block size to trigger it to write.
                    self.write_interleaved_samples(&vec![0.0f32; pad])?;
                }
                Ok(())
            }
        }

        impl Debug for OpusEncoder<'_> {
            fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
                fmt.debug_struct("OpusEncoder")
                    .field("writer", &self.writer)
                    .field("encoder", &self.encoder)
                    .field("channels", &self.channels)
                    .field("sample_rate", &self.sample_rate)
                    .field("cache_duration", &self.cache_duration)
                    .field("num_samples_per_encode", &self.num_samples_per_encode)
                    .field(
                        "sample_cache",
                        &format_args!("[f32; {}]", self.sample_cache.len()),
                    )
                    .field("samples_written", &self.samples_written)
                    .field("bytes_written", &self.bytes_written)
                    .finish()
            }
        }

        impl EncoderToImpl for OpusEncoder<'_> {
            fn get_channels(&self) -> u16 {
                self.channels
            }

            fn get_max_channels(&self) -> u16 {
                255
            }

            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                Ok(())
            }

            fn get_bitrate(&self) -> u32 {
                if self.samples_written != 0 {
                    (self.sample_rate as u64 * self.bytes_written / self.samples_written
                        * self.channels as u64
                        * 8) as u32
                } else {
                    self.sample_rate * self.channels as u32 * 8 // Fake data
                }
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                Ok(FmtChunk {
                    format_tag: FORMAT_TAG_OPUS,
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

            fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        }


        impl From<opus::Error> for AudioWriteError {
            fn from(err: opus::Error) -> Self {
                match err.code() {
                    ErrorCode::BadArg => Self::InvalidArguments(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::BufferTooSmall => Self::BufferIsFull(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::InternalError => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::InvalidPacket => Self::InvalidData(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::Unimplemented => Self::Unimplemented(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::InvalidState => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::AllocFail => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
                    ErrorCode::Unknown => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
                }
            }
        }

    }

    #[cfg(feature = "opus")]
    pub use impl_opus::*;
}

/// * The FLAC encoder for `WaveWriter`
#[cfg(feature = "flac")]
pub mod flac_enc {
    use std::{
        borrow::Cow,
        io::{self, ErrorKind, Seek, SeekFrom, Write},
    };

    use super::EncoderToImpl;

    use flac::{FlacEncoderUnmovable, options::{FlacCompression as RealFlacCompression, FlacEncoderParams as RealFlacEncoderParams}};
    use io_utils::Writer;
    use sampletypes::{i24, u24};
    use audioutils::{sample_conv, sample_conv_batch, stereos_conv};
    use crate::errors::{AudioWriteError, IOErrorInfo};
    use crate::wavcore::{format_tags::*, FmtChunk, ListChunk, flac::{FlacCompression, FlacEncoderParams, get_listinfo_flacmeta}};

    impl Into<RealFlacCompression> for FlacCompression {
        fn into(self) -> RealFlacCompression {
            match self {
                Self::Level0 => RealFlacCompression::Level0,
                Self::Level1 => RealFlacCompression::Level1,
                Self::Level2 => RealFlacCompression::Level2,
                Self::Level3 => RealFlacCompression::Level3,
                Self::Level4 => RealFlacCompression::Level4,
                Self::Level5 => RealFlacCompression::Level5,
                Self::Level6 => RealFlacCompression::Level6,
                Self::Level7 => RealFlacCompression::Level7,
                Self::Level8 => RealFlacCompression::Level8,
            }
        }
    }

    impl Into<RealFlacEncoderParams> for FlacEncoderParams {
        fn into(self) -> RealFlacEncoderParams {
            RealFlacEncoderParams {
                verify_decoded: self.verify_decoded,
                compression: self.compression.into(),
                channels: self.channels,
                sample_rate: self.sample_rate,
                bits_per_sample: self.bits_per_sample,
                total_samples_estimate: self.total_samples_estimate,
            }
        }
    }

    #[derive(Debug)]
    pub struct FlacEncoderWrap<'a> {
        encoder: Box<FlacEncoderUnmovable<'a, &'a mut dyn Writer>>,
        params: FlacEncoderParams,
        write_offset: u64,
        frames_written: u64,
        bytes_written: Box<u64>,
    }

    impl<'a> FlacEncoderWrap<'a> {
        pub fn new(
            writer: &'a mut dyn Writer,
            params: &FlacEncoderParams,
        ) -> Result<Self, AudioWriteError> {
            let params = *params;
            let real_params: RealFlacEncoderParams = params.into();
            let write_offset = writer.stream_position()?;
            let mut bytes_written = Box::new(0u64);
            let bytes_written_ptr = (&mut *bytes_written) as *mut u64;
            // Let the closures capture the pointer of the boxed variables, then use these pointers to update the variables.
            Ok(Self {
                encoder: Box::new(FlacEncoderUnmovable::new(
                    writer,
                    Box::new(
                        move |writer: &mut &'a mut dyn Writer, data: &[u8]| -> io::Result<()> {
                            unsafe { *bytes_written_ptr += data.len() as u64 };
                            writer.write_all(data)
                        },
                    ),
                    Box::new(
                        move |writer: &mut &'a mut dyn Writer, position: u64| -> io::Result<()> {
                            writer.seek(SeekFrom::Start(write_offset + position))?;
                            Ok(())
                        },
                    ),
                    Box::new(move |writer: &mut &'a mut dyn Writer| -> io::Result<u64> {
                        Ok(write_offset + writer.stream_position()?)
                    }),
                    &real_params,
                )?),
                params,
                write_offset,
                frames_written: 0,
                bytes_written,
            })
        }

        // The input samples fill all the domains of the i32, so we should shrink the bits to `self.params.bits_per_sample` to achieve good compression.
        #[inline(always)]
        fn fit_32bit_to_bps(&self, sample: i32) -> i32 {
            sample >> (32 - self.params.bits_per_sample)
        }

        // Batch shrink
        fn fit_samples_to_bps<'b>(&self, samples: &'b [i32]) -> Cow<'b, [i32]> {
            if self.params.bits_per_sample == 32 {
                Cow::Borrowed(samples)
            } else {
                Cow::Owned(
                    samples
                        .iter()
                        .map(|sample| self.fit_32bit_to_bps(*sample))
                        .collect(),
                )
            }
        }

        // Shrink tuples
        fn fit_stereos_to_bps<'b>(&self, stereos: &'b [(i32, i32)]) -> Cow<'b, [(i32, i32)]> {
            if self.params.bits_per_sample == 32 {
                Cow::Borrowed(stereos)
            } else {
                Cow::Owned(
                    stereos
                        .iter()
                        .map(|(l, r)| (self.fit_32bit_to_bps(*l), self.fit_32bit_to_bps(*r)))
                        .collect::<Vec<(i32, i32)>>(),
                )
            }
        }

        // Shrink frames or multiple mono channels
        fn fit_2d_to_bps<'b>(&self, two_d: &'b [Vec<i32>]) -> Cow<'b, [Vec<i32>]> {
            if self.params.bits_per_sample == 32 {
                Cow::Borrowed(two_d)
            } else {
                Cow::Owned(
                    two_d
                        .iter()
                        .map(|mono| self.fit_samples_to_bps(mono).to_vec())
                        .collect(),
                )
            }
        }

        fn check_channels(&self, channels: u16) -> Result<(), AudioWriteError> {
            if channels != self.params.channels {
                Err(AudioWriteError::WrongChannels(format!(
                    "The encoder channels is {} but {channels} channels audio data are asked to be written.",
                    self.params.channels
                )))
            } else {
                Ok(())
            }
        }

        #[inline(always)]
        pub fn get_channels(&self) -> u16 {
            self.params.channels
        }

        #[inline(always)]
        pub fn get_sample_rate(&self) -> u32 {
            self.params.sample_rate
        }

        #[cfg(feature = "id3")]
        pub fn inherit_metadata_from_id3(
            &mut self,
            id3_tag: &id3::Tag,
        ) -> Result<(), AudioWriteError> {
            Ok(self.encoder.inherit_metadata_from_id3(id3_tag)?)
        }

        pub fn inherit_metadata_from_list(
            &mut self,
            list_chunk: &ListChunk,
        ) -> Result<(), AudioWriteError> {
            match list_chunk {
                ListChunk::Info(list) => {
                    for (list_key, flac_key) in get_listinfo_flacmeta().iter() {
                        if let Some(data) = list.get(list_key.to_owned()) {
                            self.encoder.insert_comments(flac_key, data).unwrap();
                        }
                    }
                }
                ListChunk::Adtl(_) => {
                    eprintln!("Don't have `INFO` data in the WAV file `LIST` chunk.");
                }
            }

            Ok(())
        }

        pub fn write_interleaved_samples(
            &mut self,
            samples: &[i32],
        ) -> Result<(), AudioWriteError> {
            match self
                .encoder
                .write_interleaved_samples(&self.fit_samples_to_bps(samples))
            {
                Ok(_) => {
                    self.frames_written += samples.len() as u64 / self.get_channels() as u64;
                    Ok(())
                }
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_mono_channel(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {
            match self
                .encoder
                .write_mono_channel(&self.fit_samples_to_bps(monos))
            {
                Ok(_) => {
                    self.frames_written += monos.len() as u64;
                    Ok(())
                }
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {
            match self
                .encoder
                .write_stereos(&self.fit_stereos_to_bps(stereos))
            {
                Ok(_) => {
                    self.frames_written += stereos.len() as u64;
                    Ok(())
                }
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_monos(&mut self, monos: &[Vec<i32>]) -> Result<(), AudioWriteError> {
            match self.encoder.write_monos(&self.fit_2d_to_bps(monos)) {
                Ok(_) => {
                    self.frames_written += monos[0].len() as u64;
                    Ok(())
                }
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }

        pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {
            match self.encoder.write_frames(&self.fit_2d_to_bps(frames)) {
                Ok(_) => {
                    self.frames_written += frames.len() as u64;
                    Ok(())
                }
                Err(e) => Err(AudioWriteError::from(e)),
            }
        }
    }

    impl EncoderToImpl for FlacEncoderWrap<'_> {
        fn get_channels(&self) -> u16 {
            self.params.channels
        }

        fn get_max_channels(&self) -> u16 {
            8
        }

        fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
            self.encoder.initialize()?;
            Ok(())
        }

        fn get_bitrate(&self) -> u32 {
            if self.frames_written != 0 {
                (*self.bytes_written * self.get_sample_rate() as u64 * 8 / self.frames_written)
                    as u32
            } else {
                self.get_sample_rate() * self.get_channels() as u32 * 8 // Fake data
            }
        }

        fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
            Ok(FmtChunk {
                format_tag: FORMAT_TAG_FLAC,
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

        fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
        fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}

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

        fn write_frames__i8(&mut self, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_i16(&mut self, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_i24(&mut self, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_i32(&mut self, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_i64(&mut self, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames__u8(&mut self, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_u16(&mut self, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_u24(&mut self, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_u32(&mut self, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_u64(&mut self, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_f32(&mut self, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
        fn write_frames_f64(&mut self, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_frames(&sample_conv_batch(frames))}
    }

    use flac::errors::*;
    impl From<FlacEncoderError> for AudioWriteError {
        fn from(err: FlacEncoderError) -> Self {
            let err_code = err.code;
            let err_func = err.function;
            let err_desc = err.message;
            use FlacEncoderErrorCode::*;
            let err_code = FlacEncoderErrorCode::from(err_code);
            let err_string = format!("On function `{err_func}`: {err_desc}: {err_code}");
            match err_code {
                StreamEncoderOk => Self::OtherReason(err_string),
                StreamEncoderUninitialized => Self::OtherReason(err_string),
                StreamEncoderOggError => Self::OtherReason(err_string),
                StreamEncoderVerifyDecoderError => Self::OtherReason(err_string),
                StreamEncoderVerifyMismatchInAudioData => Self::OtherReason(err_string),
                StreamEncoderClientError => Self::OtherReason(err_string),
                StreamEncoderIOError => Self::IOError(IOErrorInfo::new(ErrorKind::Other, err_string)),
                StreamEncoderFramingError => Self::InvalidInput(err_string),
                StreamEncoderMemoryAllocationError => Self::OtherReason(err_string),
            }
        }
    }

    impl From<FlacEncoderInitError> for AudioWriteError {
        fn from(err: FlacEncoderInitError) -> Self {
            let err_code = err.code;
            let err_func = err.function;
            let err_desc = err.message;
            use FlacEncoderInitErrorCode::*;
            let err_code = FlacEncoderInitErrorCode::from(err_code);
            let err_string = format!("On function `{err_func}`: {err_desc}: {err_code}");
            match err_code {
                StreamEncoderInitStatusOk => Self::OtherReason(err_string),
                StreamEncoderInitStatusEncoderError => Self::OtherReason(err_string),
                StreamEncoderInitStatusUnsupportedContainer => Self::OtherReason(err_string),
                StreamEncoderInitStatusInvalidCallbacks => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidNumberOfChannels => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidBitsPerSample => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidSampleRate => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidBlockSize => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidMaxLpcOrder => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusInvalidQlpCoeffPrecision => Self::InvalidArguments(err_string),
                StreamEncoderInitStatusBlockSizeTooSmallForLpcOrder => Self::BufferIsFull(err_string),
                StreamEncoderInitStatusNotStreamable => Self::OtherReason(err_string),
                StreamEncoderInitStatusInvalidMetadata => Self::InvalidInput(err_string),
                StreamEncoderInitStatusAlreadyInitialized => Self::InvalidArguments(err_string),
            }
        }
    }

    impl From<FlacDecoderError> for AudioWriteError {
        fn from(err: FlacDecoderError) -> Self {
            let err_code = err.code;
            let err_func = err.function;
            let err_desc = err.message;
            use FlacDecoderErrorCode::*;
            let err_code = FlacDecoderErrorCode::from(err_code);
            let err_string = format!("On function `{err_func}`: {err_desc}: {err_code}");
            match err_code {
                StreamDecoderSearchForMetadata => Self::OtherReason(err_string),
                StreamDecoderReadMetadata => Self::OtherReason(err_string),
                StreamDecoderSearchForFrameSync => Self::OtherReason(err_string),
                StreamDecoderReadFrame => Self::OtherReason(err_string),
                StreamDecoderEndOfStream => Self::OtherReason(err_string),
                StreamDecoderOggError => Self::OtherReason(err_string),
                StreamDecoderSeekError => Self::OtherReason(err_string),
                StreamDecoderAborted => Self::OtherReason(err_string),
                StreamDecoderMemoryAllocationError => Self::OtherReason(err_string),
                StreamDecoderUninitialized => Self::InvalidArguments(err_string),
            }
        }
    }

    impl From<FlacDecoderInitError> for AudioWriteError {
        fn from(err: FlacDecoderInitError) -> Self {
            let err_code = err.code;
            let err_func = err.function;
            let err_desc = err.message;
            use FlacDecoderInitErrorCode::*;
            let err_code = FlacDecoderInitErrorCode::from(err_code);
            let err_string = format!("On function `{err_func}`: {err_desc}: {err_code}");
            match err_code {
                StreamDecoderInitStatusOk => Self::OtherReason(err_string),
                StreamDecoderInitStatusUnsupportedContainer => Self::Unsupported(err_string),
                StreamDecoderInitStatusInvalidCallbacks => Self::InvalidArguments(err_string),
                StreamDecoderInitStatusMemoryAllocationError => Self::OtherReason(err_string),
                StreamDecoderInitStatusErrorOpeningFile => {
                    Self::IOError(IOErrorInfo::new(ErrorKind::Other, err_string))
                }
                StreamDecoderInitStatusAlreadyInitialized => Self::InvalidArguments(err_string),
            }
        }
    }

    impl From<&dyn FlacError> for AudioWriteError {
        fn from(err: &dyn FlacError) -> Self {
            let err_code = err.get_code();
            let err_func = err.get_function();
            let err_desc = err.get_message();
            if let Some(encoder_err) = err.as_any().downcast_ref::<FlacEncoderError>() {
                AudioWriteError::from(*encoder_err)
            } else if let Some(encoder_err) = err.as_any().downcast_ref::<FlacEncoderInitError>() {
                AudioWriteError::from(*encoder_err)
            } else if let Some(decoder_err) = err.as_any().downcast_ref::<FlacDecoderError>() {
                AudioWriteError::from(*decoder_err)
            } else if let Some(decoder_err) = err.as_any().downcast_ref::<FlacDecoderInitError>() {
                AudioWriteError::from(*decoder_err)
            } else {
                Self::OtherReason(format!(
                    "Unknown error type from `flac::FlacError`: `{err_func}`: {err_code}: {err_desc}"
                ))
            }
        }
    }
}

/// * The OggVorbis encoder
/// * Microsoft says this should be supported: see <https://github.com/tpn/winsdk-10/blob/master/Include/10.0.14393.0/shared/mmreg.h#L2321>
/// * FFmpeg does not support this format: see <https://git.ffmpeg.org/gitweb/ffmpeg.git/blob/refs/heads/release/7.1:/libavformat/riff.c>
pub mod oggvorbis_enc {
    use crate::wavcore::oggvorbis::*;

    #[cfg(any(feature = "vorbis", feature = "oggvorbis"))]
    use super::EncoderToImpl;

    #[cfg(any(feature = "vorbis", feature = "oggvorbis"))]
    mod impl_vorbis {
        use std::{
            collections::BTreeMap,
            fmt::{self, Debug, Formatter},
            io::{Seek, Write, ErrorKind},
            num::NonZero,
        };

        use super::*;
        use ogg::OggPacket;
        use vorbis_rs::*;

        use crate::errors::{AudioWriteError, IOErrorInfo};
        use crate::io_utils::{Reader, Writer, ReadWrite, CursorVecU8, SharedMultistreamIO, StreamType};
        use crate::audioutils::{self, sample_conv, sample_conv_batch};
        use crate::chunks::{FmtChunk, ext::{FmtExtension, VorbisHeaderData, OggVorbisData, OggVorbisWithHeaderData}};
        use crate::format_specs::format_tags::*;
        use crate::{i24, u24};

        type SharedAlterIO<'a> = SharedMultistreamIO<Box<dyn Reader>, &'a mut dyn Writer, &'a mut dyn ReadWrite>;
        type SharedIO<'a> = StreamType<Box<dyn Reader>, &'a mut dyn Writer, &'a mut dyn ReadWrite>;

        impl OggVorbisBitrateStrategy {
            fn is_vbr(&self) -> bool {
                match self {
                    Self::Vbr(_) => true,
                    Self::QualityVbr(_) => true,
                    Self::Abr(_) => false,
                    Self::ConstrainedAbr(_) => false,
                }
            }

            fn get_bitrate(&self, channels: u16, sample_rate: u32) -> Result<u32, AudioWriteError> {
                match self {
                    Self::Vbr(bitrate) => Ok(*bitrate),
                    Self::QualityVbr(quality) => {
                        let quality = ((quality * 10.0) as i32).clamp(0, 10);
                        match sample_rate {
                            48000 => {
                                Ok([
                                    [64000, 80000, 96000, 112000, 128000, 160000, 192000, 240000, 256000, 350000, 450000],
                                    [48000, 64000, 72000,  80000,  88000,  96000, 112000, 128000, 144000, 192000, 256000],
                                ][channels as usize][quality as usize])
                            }
                            44100 => {
                                Ok([
                                    [64000, 80000, 96000, 112000, 128000, 160000, 192000, 240000, 256000, 350000, 450000],
                                    [48000, 64000, 72000,  80000,  88000,  96000, 112000, 128000, 144000, 192000, 256000],
                                ][channels as usize][quality as usize])
                            }
                            22050 => {
                                Ok([
                                    [56000, 72000, 80000,  88000,  96000, 112000, 144000, 176000, 192000, 256000, 320000],
                                    [36000, 42000, 48000,  52000,  56000,  64000,  80000,  88000,  96000, 128000, 168000],
                                ][channels as usize][quality as usize])
                            }
                            11025 => {
                                Ok([
                                    [36000, 44000, 50000,  52000,  56000,  64000,  80000,  96000, 112000, 144000, 168000],
                                    [22000, 26000, 28000,  30000,  32000,  34000,  40000,  48000,  56000,  72000,  88000],
                                ][channels as usize][quality as usize])
                            }
                            o => Err(AudioWriteError::InvalidArguments(format!("Invalid sample rate {o}. For Vorbis encoding, sample rate must be 48000, 44100, 22050, 11025."))),
                        }
                    },
                    Self::Abr(bitrate) => Ok(*bitrate),
                    Self::ConstrainedAbr(bitrate) => Ok(*bitrate),
                }
            }
        }

        impl From<OggVorbisBitrateStrategy> for VorbisBitrateManagementStrategy {
            /// * Convert to the `VorbisBitrateManagementStrategy` from `vorbis_rs` crate
            fn from(val: OggVorbisBitrateStrategy) -> Self {
                match val {
                    OggVorbisBitrateStrategy::Vbr(bitrate) => VorbisBitrateManagementStrategy::Vbr {
                        target_bitrate: NonZero::new(bitrate).unwrap(),
                    },
                    OggVorbisBitrateStrategy::QualityVbr(quality) => VorbisBitrateManagementStrategy::QualityVbr {
                        target_quality: quality,
                    },
                    OggVorbisBitrateStrategy::Abr(bitrate) => VorbisBitrateManagementStrategy::Abr {
                        average_bitrate: NonZero::new(bitrate).unwrap(),
                    },
                    OggVorbisBitrateStrategy::ConstrainedAbr(bitrate) => VorbisBitrateManagementStrategy::ConstrainedAbr {
                        maximum_bitrate: NonZero::new(bitrate).unwrap(),
                    },
                }
            }
        }

        impl From<VorbisBitrateManagementStrategy> for OggVorbisBitrateStrategy {
            /// * Convert from `VorbisBitrateManagementStrategy` from `vorbis_rs` crate
            fn from(vbms: VorbisBitrateManagementStrategy) -> Self {
                match vbms {
                    VorbisBitrateManagementStrategy::Vbr{target_bitrate} => Self::Vbr(target_bitrate.into()),
                    VorbisBitrateManagementStrategy::QualityVbr{target_quality} => Self::QualityVbr(target_quality),
                    VorbisBitrateManagementStrategy::Abr{average_bitrate} => Self::Abr(average_bitrate.into()),
                    VorbisBitrateManagementStrategy::ConstrainedAbr{maximum_bitrate} => Self::ConstrainedAbr(maximum_bitrate.into()),
                }
            }
        }

        impl Default for OggVorbisBitrateStrategy {
            fn default() -> Self {
                Self::QualityVbr(1.0)
            }
        }

        /// * The OggVorbis encoder or builder enum, the builder one has metadata to put in the builder.
        pub enum OggVorbisEncoderOrBuilder<'a> {
            /// The OggVorbis encoder builder
            Builder {
                /// The builder that has our shared writer.
                builder: VorbisEncoderBuilder<SharedAlterIO<'a>>,

                /// The metadata to be added to the OggVorbis file. Before the encoder was built, add all of the comments here.
                metadata: BTreeMap<String, String>,
            },

            /// The built encoder. It has our shared writer. Use this to encode PCM waveform to OggVorbis format.
            Encoder(VorbisEncoder<SharedAlterIO<'a>>),

            /// When the encoding has finished, set this enum to `Finished`
            Finished,
        }

        impl Debug for OggVorbisEncoderOrBuilder<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                match self {
                    Self::Builder {
                        builder: _,
                        metadata,
                    } => write!(
                        f,
                        "Builder(builder: VorbisEncoderBuilder<WriteBridge>, metadata: {:?})",
                        metadata
                    ),
                    Self::Encoder(_encoder) => write!(f, "Encoder(VorbisEncoder<WriteBridge>)"),
                    Self::Finished => write!(f, "Finished"),
                }
            }
        }

        impl OggVorbisEncoderParams {
            pub fn create_vorbis_builder<W>(&self, writer: W) -> Result<VorbisEncoderBuilder<W>, AudioWriteError>
            where
                W: Write {
                let sample_rate = NonZero::new(self.sample_rate).unwrap();
                let channels = NonZero::new(self.channels as u8).unwrap();

                let mut builder = VorbisEncoderBuilder::new(sample_rate, channels, writer)?;

                match self.mode {
                    OggVorbisMode::HaveNoCodebookHeader => (),
                    _ => {
                        if let Some(serial) = self.stream_serial {
                            builder.stream_serial(serial);
                        }
                        if let Some(bitrate) = self.bitrate {
                            builder.bitrate_management_strategy(bitrate.into());
                        }
                    }
                }

                builder.minimum_page_data_size(self.minimum_page_data_size);
                Ok(builder)
            }

            pub fn get_bitrate(&self) -> u32 {
                self.bitrate.unwrap_or_default().get_bitrate(self.channels, self.sample_rate).unwrap()
            }
        }

        /// * OggVorbis encoder wrap for `WaveWriter`
        pub struct OggVorbisEncoderWrap<'a> {
            /// * The writer for the encoder. Since the encoder only asks for a `Write` trait, we can control where should it write data to by using `seek()`.
            /// * This `SharedWriterWithCursor` can switch to `cursor_mode` or `writer_mode`, on `cursor_mode`, call `write()` on it will write data into the `Cursor`
            writer: SharedAlterIO<'a>,

            /// * The parameters for the encoder
            params: OggVorbisEncoderParams,

            /// * The OggVorbis encoder builder or the built encoder.
            encoder: OggVorbisEncoderOrBuilder<'a>,

            /// * The data offset. The OggVorbis data should be written after here.
            data_offset: u64,

            /// * How many bytes were written. This field is for calculating the bitrate of the Ogg stream.
            bytes_written: u64,

            /// * How many audio frames were written. This field is for calculating the bitrate of the Ogg stream.
            frames_written: u64,

            /// * The header data that should be written in the `fmt ` chunk extension.
            vorbis_header: Vec<u8>,
        }

        impl Debug for OggVorbisEncoderWrap<'_> {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                f.debug_struct("OggVorbisEncoderWrap")
                .field("writer", &self.writer)
                .field("params", &self.params)
                .field("encoder", &self.encoder)
                .field("data_offset", &self.data_offset)
                .field("bytes_written", &self.bytes_written)
                .field("frames_written", &self.frames_written)
                .field("vorbis_header", &format_args!("[u8, {}]", self.vorbis_header.len()))
                .finish()
            }
        }

        impl<'a> OggVorbisEncoderWrap<'a> {
            pub fn new(
                writer: &'a mut dyn Writer,
                params: &OggVorbisEncoderParams,
            ) -> Result<Self, AudioWriteError> {
                let mut shared_writer = SharedAlterIO::default();
                shared_writer.push_stream(SharedIO::Writer(writer));
                shared_writer.push_stream(SharedIO::CursorU8(CursorVecU8::default()));
                let data_offset = shared_writer.stream_position()?;

                let mut ret = Self {
                    writer: shared_writer.clone(),
                    params: *params,
                    encoder: OggVorbisEncoderOrBuilder::Builder {
                        builder: params.create_vorbis_builder(shared_writer.clone())?,
                        metadata: BTreeMap::new(),
                    },
                    data_offset,
                    bytes_written: 0,
                    frames_written: 0,
                    vorbis_header: Vec::new(),
                };
                if ret.params.bitrate.is_none() {
                    ret.params.bitrate = Some(VorbisBitrateManagementStrategy::default().into());
                }
                if let OggVorbisEncoderOrBuilder::Builder{builder: _, ref mut metadata} = ret.encoder {
                    metadata.insert("ENCODER".to_string(), "rustwav".to_string());
                }
                Ok(ret)
            }

            /// Get num channels from the params
            pub fn get_channels(&self) -> u16 {
                self.params.channels
            }

            /// Get the sample rate from the params
            pub fn get_sample_rate(&self) -> u32 {
                self.params.sample_rate
            }

            /// Insert a comment to the metadata. NOTE: When the decoder was built, you can not add comments anymore.
            pub fn insert_comment(
                &mut self,
                key: String,
                value: String,
            ) -> Result<(), AudioWriteError> {
                match self.encoder {
                    OggVorbisEncoderOrBuilder::Builder{builder: _, ref mut metadata} => {
                        metadata.insert(key, value);
                        Ok(())
                    },
                    _ => Err(AudioWriteError::InvalidArguments("The encoder has already entered encoding mode, why are you just starting to add metadata to it?".to_string())),
                }
            }

            /// * When you call this method, the builder will build the encoder, and the enum `self.encoder` will be changed to the encoder.
            /// * After this point, you can not add metadata anymore, and then the encoding starts.
            pub fn begin_to_encode(&mut self) -> Result<(), AudioWriteError> {
                match self.encoder {
                    OggVorbisEncoderOrBuilder::Builder {
                        ref mut builder,
                        ref metadata,
                    } => {
                        for (tag, value) in metadata.iter() {
                            match builder.comment_tag(tag, value) {
                                Ok(_) => (),
                                Err(e) => {
                                    eprintln!("Set comment tag failed: {tag}: {value}: {:?}", e)
                                }
                            }
                        }
                        self.encoder = OggVorbisEncoderOrBuilder::Encoder(builder.build()?);
                        Ok(())
                    }
                    OggVorbisEncoderOrBuilder::Encoder(_) => Ok(()),
                    OggVorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The OggVorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Peel off the Ogg skin from the `Cursor` data, and write the naked data to the `Writer`
            fn peel_ogg(&mut self) -> Result<(), AudioWriteError> {
                let mut cursor = 0usize;
                let mut packet_length = 0usize;
                let data = self.writer.get_stream_mut(1).as_cursor().get_ref().clone();
                while cursor < data.len() {
                    match OggPacket::from_bytes(&data[cursor..], &mut packet_length) {
                        Ok(oggpacket) => {
                            self.writer.set_stream(0);
                            self.writer.write_all(&oggpacket.get_inner_data())?;
                            self.writer.set_stream(1);
                            cursor += packet_length;
                        }
                        Err(ioerr) => {
                            match ioerr.kind() {
                                ErrorKind::UnexpectedEof => {
                                    let remains = data[cursor..].to_vec();
                                    let _data = self.writer.get_stream_mut(1).take_cursor_data();
                                    self.writer.write_all(&remains)?;
                                    break;
                                }
                                _ => return Err(ioerr.into()),
                            }
                        }
                    }
                }
                Ok(())
            }

            /// * Write the interleaved samples to the encoder. The interleaved samples were interleaved by channels.
            /// * The encoder actually takes the waveform array. Conversion performed during this function.
            pub fn write_interleaved_samples(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {
                let channels = self.get_channels();
                match self.encoder {
                    OggVorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    OggVorbisEncoderOrBuilder::Encoder(ref mut encoder) => {
                        let frames = audioutils::interleaved_samples_to_monos(samples, channels)?;
                        encoder.encode_audio_block(&frames)?;
                        if self.params.mode == OggVorbisMode::NakedVorbis {
                            self.peel_ogg()?;
                        }
                        self.bytes_written = self.writer.stream_position()? - self.data_offset;
                        self.frames_written += frames[0].len() as u64;
                        Ok(())
                    }
                    OggVorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The OggVorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Write multiple mono waveforms to the encoder.
            pub fn write_monos(&mut self, monos: &[Vec<f32>]) -> Result<(), AudioWriteError> {
                match self.encoder {
                    OggVorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    OggVorbisEncoderOrBuilder::Encoder(ref mut encoder) => {
                        encoder.encode_audio_block(monos)?;
                        if self.params.mode == OggVorbisMode::NakedVorbis {
                            self.peel_ogg()?;
                        }
                        self.bytes_written = self.writer.stream_position()? - self.data_offset;
                        self.frames_written += monos.len() as u64;
                        Ok(())
                    }
                    OggVorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The OggVorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Finish encoding audio.
            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                match self.encoder {
                    OggVorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    OggVorbisEncoderOrBuilder::Encoder(ref mut _encoder) => {
                        self.encoder = OggVorbisEncoderOrBuilder::Finished;
                        Ok(())
                    }
                    OggVorbisEncoderOrBuilder::Finished => Ok(()),
                }
            }
        }

        impl EncoderToImpl for OggVorbisEncoderWrap<'_> {
            fn get_channels(&self) -> u16 {
                self.params.channels
            }

            fn get_max_channels(&self) -> u16 {
                255
            }

            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                match self.params.mode {
                    OggVorbisMode::OriginalStreamCompatible => self.begin_to_encode(),
                    OggVorbisMode::HaveIndependentHeader => Ok(()),
                    OggVorbisMode::HaveNoCodebookHeader => {
                        let _header = self.writer.get_stream_mut(1).take_cursor_data();
                        Ok(())
                    }
                    OggVorbisMode::NakedVorbis => Ok(())
                }
            }

            fn get_bitrate(&self) -> u32 {
                if self.frames_written != 0 {
                    (self.bytes_written * 8 * self.get_sample_rate() as u64 / self.frames_written)
                        as u32
                } else {
                    self.params.get_bitrate()
                }
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                match self.params.mode {
                    OggVorbisMode::OriginalStreamCompatible => (),
                    OggVorbisMode::HaveIndependentHeader => {
                        // Save the header to `fmt ` chunk
                        self.writer.set_stream(1);
                        self.begin_to_encode()?;
                        self.vorbis_header = self.writer.get_cur_stream_mut().take_cursor_data();
                        self.writer.set_stream(0);
                    }
                    OggVorbisMode::HaveNoCodebookHeader => {
                        self.writer.set_stream(1);
                        self.begin_to_encode()?;
                        // Discard the header. When to decode, the decoder generates the header by using an encoder.
                        let _header = self.writer.get_cur_stream_mut().take_cursor_data();
                        self.writer.set_stream(0);
                    }
                    OggVorbisMode::NakedVorbis => {
                        // Save the header to `fmt ` chunk
                        use revorbis::get_vorbis_headers_from_ogg_packet_bytes;
                        self.writer.set_stream(1);
                        self.begin_to_encode()?;
                        let header = self.writer.get_cur_stream_mut().take_cursor_data();
                        let mut _stream_id = 0u32;
                        let (identification_header, comments_header, setup_header) = get_vorbis_headers_from_ogg_packet_bytes(&header, &mut _stream_id)?;
                        self.vorbis_header.clear();
                        self.vorbis_header.push(2); // Two field of the header size
                        self.vorbis_header.push(identification_header.len() as u8);
                        self.vorbis_header.push(comments_header.len() as u8);
                        self.vorbis_header.extend(identification_header);
                        self.vorbis_header.extend(comments_header);
                        self.vorbis_header.extend(setup_header);
                    }
                }
                Ok(FmtChunk {
                    format_tag: match self.params.mode {
                        OggVorbisMode::OriginalStreamCompatible => {
                            if self.params.bitrate.unwrap().is_vbr() {
                                FORMAT_TAG_OGG_VORBIS1
                            } else {
                                FORMAT_TAG_OGG_VORBIS1P
                            }
                        }
                        OggVorbisMode::HaveIndependentHeader => {
                            if self.params.bitrate.unwrap().is_vbr() {
                                FORMAT_TAG_OGG_VORBIS2
                            } else {
                                FORMAT_TAG_OGG_VORBIS2P
                            }
                        }
                        OggVorbisMode::HaveNoCodebookHeader => {
                            if self.params.bitrate.unwrap().is_vbr() {
                                FORMAT_TAG_OGG_VORBIS3
                            } else {
                                FORMAT_TAG_OGG_VORBIS3P
                            }
                        }
                        OggVorbisMode::NakedVorbis => FORMAT_TAG_VORBIS,
                    },
                    channels: self.get_channels(),
                    sample_rate: self.get_sample_rate(),
                    byte_rate: self.params.get_bitrate() / 8,
                    block_align: 4,
                    bits_per_sample: 16,
                    extension: Some(if self.vorbis_header.is_empty() {
                        FmtExtension::new_oggvorbis(OggVorbisData::new())
                    } else if self.params.mode != OggVorbisMode::NakedVorbis {
                        FmtExtension::new_oggvorbis_with_header(OggVorbisWithHeaderData::new(&self.vorbis_header))
                    } else {
                        FmtExtension::new_vorbis(VorbisHeaderData::new(&self.vorbis_header))
                    }),
                })
            }

            fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
                fmt.byte_rate = self.get_bitrate() / 8;
                Ok(())
            }

            fn finish(&mut self) -> Result<(), AudioWriteError> {
                self.finish()?;
                Ok(())
            }

            fn write_interleaved_samples__i8(&mut self, samples: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i16(&mut self, samples: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i24(&mut self, samples: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i32(&mut self, samples: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_i64(&mut self, samples: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples__u8(&mut self, samples: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u16(&mut self, samples: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u24(&mut self, samples: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u32(&mut self, samples: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_u64(&mut self, samples: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_f32(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}
            fn write_interleaved_samples_f64(&mut self, samples: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples(&sample_conv(samples))}

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
        }

        impl From<vorbis_rs::VorbisError> for AudioWriteError {
            fn from(err: vorbis_rs::VorbisError) -> Self {
                use vorbis_rs::VorbisError::*;
                match err {
                    LibraryError(liberr) => {
                        let lib = liberr.library();
                        let func = liberr.function();
                        let kind = liberr.kind();
                        let message =
                            format!("OggVorbis library error: lib: {lib}, function: {func}, kind: {kind}");
                        use vorbis_rs::VorbisLibraryErrorKind::*;
                        match kind {
                            False | Hole | InternalFault | NotVorbis | BadHeader | BadVorbisVersion
                            | NotAudio | BadPacket | BadLink => Self::OtherReason(message),
                            Eof => Self::IOError(IOErrorInfo::new(ErrorKind::UnexpectedEof, message)),
                            Io => Self::IOError(IOErrorInfo::new(ErrorKind::Other, message)),
                            NotImplemented => Self::Unimplemented(message),
                            InvalidValue => Self::InvalidInput(message),
                            NotSeekable => Self::IOError(IOErrorInfo::new(ErrorKind::NotSeekable, message)),
                            Other { result_code: code } => Self::OtherReason(format!(
                                "OggVorbis library error: lib: {lib}, function: {func}, kind: {kind}, code: {code}"
                            )),
                            o => Self::OtherReason(format!(
                                "OggVorbis library error: lib: {lib}, function: {func}, kind: {kind}, error: {o}"
                            )),
                        }
                    }
                    InvalidAudioBlockChannelCount { expected, actual } => Self::WrongChannels(format!(
                        "Channel error: expected: {expected}, actual: {actual}"
                    )),
                    InvalidAudioBlockSampleCount { expected, actual } => Self::InvalidData(format!(
                        "Invalid audio block sample count: expected: {expected}, actual: {actual}"
                    )),
                    UnsupportedStreamChaining => {
                        Self::Unsupported("Unsupported stream chaining".to_string())
                    }
                    InvalidCommentString(err_char) => {
                        Self::InvalidInput(format!("Invalid comment string char {err_char}"))
                    }
                    RangeExceeded(try_error) => {
                        Self::InvalidInput(format!("Invalid parameters range exceeded: {try_error}"))
                    }
                    Io(ioerr) => Self::IOError(IOErrorInfo::new(ioerr.kind(), format!("{:?}", ioerr))),
                    Rng(rngerr) => Self::OtherReason(format!("Random number generator error: {rngerr}")),
                    ConsumedEncoderBuilderSink => {
                        Self::InvalidArguments("The `writer` was already consumed".to_string())
                    }
                    o => Self::OtherReason(format!("Unknown error: {o}")),
                }
            }
        }

    }

    #[cfg(any(feature = "vorbis", feature = "oggvorbis"))]
    pub use impl_vorbis::*;
}

