#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

use crate::AudioWriteError;
use crate::Writer;
use crate::adpcm;
use crate::utils::{self, sample_conv, sample_conv_batch, stereo_conv, stereos_conv};
use crate::wavcore::format_tags::*;
use crate::wavcore::guids::*;
use crate::wavcore::{ExtensibleData, FmtChunk, FmtExtension};
use crate::wavcore::{Spec, WaveSampleType};
use crate::xlaw::{PcmXLawEncoder, XLaw};
use crate::{SampleType, i24, u24};

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
    fn write_frame__i8(&mut self, frame: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_i16(&mut self, frame: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_i24(&mut self, frame: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_i32(&mut self, frame: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_i64(&mut self, frame: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame__u8(&mut self, frame: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_u16(&mut self, frame: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_u24(&mut self, frame: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_u32(&mut self, frame: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_u64(&mut self, frame: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_f32(&mut self, frame: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}
    fn write_frame_f64(&mut self, frame: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&utils::frames_to_interleaved_samples(&[frame.to_vec()], Some(self.get_channels()))?)}

    // Convenience interfaces for writing multiple audio frames. Default implementations are provided.
    fn write_frames__i8(&mut self, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_i16(&mut self, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_i24(&mut self, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_i32(&mut self, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_i64(&mut self, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames__u8(&mut self, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_u16(&mut self, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_u24(&mut self, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_u32(&mut self, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_u64(&mut self, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_f32(&mut self, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}
    fn write_frames_f64(&mut self, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&utils::frames_to_interleaved_samples(frames, Some(self.get_channels()))?)}

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
    fn write_mono_channel__i8(&mut self, monos: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i8 >>()).collect::<Vec<Vec<i8 >>>().into_iter().flatten().collect::<Vec<i8 >>())}
    fn write_mono_channel_i16(&mut self, monos: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i16>>()).collect::<Vec<Vec<i16>>>().into_iter().flatten().collect::<Vec<i16>>())}
    fn write_mono_channel_i24(&mut self, monos: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i24>>()).collect::<Vec<Vec<i24>>>().into_iter().flatten().collect::<Vec<i24>>())}
    fn write_mono_channel_i32(&mut self, monos: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i32>>()).collect::<Vec<Vec<i32>>>().into_iter().flatten().collect::<Vec<i32>>())}
    fn write_mono_channel_i64(&mut self, monos: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<i64>>()).collect::<Vec<Vec<i64>>>().into_iter().flatten().collect::<Vec<i64>>())}
    fn write_mono_channel__u8(&mut self, monos: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u8 >>()).collect::<Vec<Vec<u8 >>>().into_iter().flatten().collect::<Vec<u8 >>())}
    fn write_mono_channel_u16(&mut self, monos: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u16>>()).collect::<Vec<Vec<u16>>>().into_iter().flatten().collect::<Vec<u16>>())}
    fn write_mono_channel_u24(&mut self, monos: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u24>>()).collect::<Vec<Vec<u24>>>().into_iter().flatten().collect::<Vec<u24>>())}
    fn write_mono_channel_u32(&mut self, monos: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u32>>()).collect::<Vec<Vec<u32>>>().into_iter().flatten().collect::<Vec<u32>>())}
    fn write_mono_channel_u64(&mut self, monos: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<u64>>()).collect::<Vec<Vec<u64>>>().into_iter().flatten().collect::<Vec<u64>>())}
    fn write_mono_channel_f32(&mut self, monos: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<f32>>()).collect::<Vec<Vec<f32>>>().into_iter().flatten().collect::<Vec<f32>>())}
    fn write_mono_channel_f64(&mut self, monos: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&monos.into_iter().map(|mono|(0..self.get_channels()).map(|_|*mono).collect::<Vec<f64>>()).collect::<Vec<Vec<f64>>>().into_iter().flatten().collect::<Vec<f64>>())}

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
    fn write_dual_monos__i8(&mut self, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i16(&mut self, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i24(&mut self, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i32(&mut self, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_i64(&mut self, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos__u8(&mut self, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u16(&mut self, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u24(&mut self, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u32(&mut self, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_u64(&mut self, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f32(&mut self, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_dual_monos_f64(&mut self, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&utils::monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}

    // Interfaces for writing batched stereo audio (two separate channel buffers). Default implementations are provided.
    fn write_monos__i8(&mut self, monos_array: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i16(&mut self, monos_array: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i24(&mut self, monos_array: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i32(&mut self, monos_array: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_i64(&mut self, monos_array: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos__u8(&mut self, monos_array: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u16(&mut self, monos_array: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u24(&mut self, monos_array: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u32(&mut self, monos_array: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_u64(&mut self, monos_array: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f32(&mut self, monos_array: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&utils::monos_to_interleaved_samples(monos_array)?)}
    fn write_monos_f64(&mut self, monos_array: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&utils::monos_to_interleaved_samples(monos_array)?)}

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
    fn write_stereos__i8(&mut self, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__i8(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i16(&mut self, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i16(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i24(&mut self, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i24(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i32(&mut self, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_i64(&mut self, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_i64(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos__u8(&mut self, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_interleaved_samples__u8(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u16(&mut self, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u16(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u24(&mut self, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u24(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u32(&mut self, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_u64(&mut self, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_u64(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f32(&mut self, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f32(&utils::stereos_to_interleaved_samples(stereos))}
    fn write_stereos_f64(&mut self, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_interleaved_samples_f64(&utils::stereos_to_interleaved_samples(stereos))}
}

/// ## The `DummyEncoder` is not for you to use, it allows me to implement `Default` for `Encoder<'a>`
#[derive(Debug, Clone, Copy)]
pub struct DummyEncoder;

/// ## Default implementations: all functions are `panic!`
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

/// ## The `Encoder` struct contains all of the encoder types and provides convenient functions that have generic type parameters.
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

/// ## `PcmEncoderFrom<S>`: Transcodes samples from type `S` into the target format.
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

/// ## `PcmEncoder`: convert various formats of PCM samples to the WAV file specific sample type
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

/// ## `AdpcmEncoderWrap<E>`: encode `i16` audio samples to ADPCM nibbles
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
        let mut iter = utils::stereos_to_interleaved_samples(stereos).into_iter();
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

/// ## `PcmXLawEncoderWrap`: encode `i16` audio samples to bytes
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

/// ## The MP3 encoder for `WaveWriter`
pub mod mp3 {
    /// * MP3 supports two channels in multiple ways.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mp3Channels {
        /// * Mono audio
        Mono = 3,

        /// * Stereo audio (not telling how it is)
        Stereo = 0,

        /// * Joint stereo (most commonly used, for better compression)
        JointStereo = 1,

        /// * Dual channel stereo (for better audio quality)
        DualChannel = 2,

        /// * Not set
        NotSet = 4,
    }

    /// * MP3 quality. Affects the speed of encoding.
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

    /// * The tier 1 factor for MP3 audio quality, bigger bitrate means better audio quality.
    /// * Most of the music website provides 128 kbps music for free, and 320 kbps music for the purchased subscribed members.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u16)]
    pub enum Mp3Bitrate {
        Kbps8 = 8,
        Kbps16 = 16,
        Kbps24 = 24,
        Kbps32 = 32,
        Kbps40 = 40,
        Kbps48 = 48,

        /// * The bitrate for audio chatting.
        Kbps64 = 64,
        Kbps80 = 80,
        Kbps96 = 96,
        Kbps112 = 112,

        /// * The bitrate for free users.
        Kbps128 = 128,
        Kbps160 = 160,
        Kbps192 = 192,
        Kbps224 = 224,

        /// * Laboratories uses this bitrate.
        Kbps256 = 256,

        /// * The bitrate for VIP users who pay for it.
        Kbps320 = 320,
    }

    /// * The VBR mode for MP3. If you turn VBR on, the audio quality for MP3 will be a little bit worse.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u8)]
    pub enum Mp3VbrMode {
        /// * This option disables the VBR mode.
        Off = 0,

        Mt = 1,
        Rh = 2,
        Abr = 3,

        /// * This option is used most commonly.
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

    /// ## The encoder options for MP3
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Mp3EncoderOptions {
        /// * MP3 channels, not just mono and stereo. MP3 supports two channels in multiple ways.
        pub channels: Mp3Channels,

        /// * MP3 quality. Affects the speed of encoding.
        pub quality: Mp3Quality,

        /// * MP3 bitrate. Affects the audio quality and file size, bigger is better.
        pub bitrate: Mp3Bitrate,

        /// * VBR mode for better compression, turn it off to get better audio quality.
        pub vbr_mode: Mp3VbrMode,

        /// * ID3 tags, if you have, fill this field.
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
        use super::*;
        use crate::AudioWriteError;
        use crate::Writer;
        use crate::utils::{self, sample_conv, stereos_conv};
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
                    id3tag: self.id3tag,
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
            encoder_options: Mp3EncoderOptions,
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
                    encoder_options: *mp3_options,
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
                        .add_stereos(&utils::interleaved_samples_to_stereos(
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
                        .add_mono_channel(&utils::stereos_to_mono_channel(&stereos_conv::<T, S>(stereos))),
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
                        .add_mono_channel(&sample_conv::<T, S>(&utils::dual_monos_to_monos(&(
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
                    Self::Mono(m) => m.extend(utils::stereos_to_mono_channel(frames)),
                    Self::Stereo((l, r)) => {
                        let (il, ir) = utils::stereos_to_dual_monos(frames);
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
                    Self::Mono(m) => m.extend(utils::dual_monos_to_monos(&(
                        monos_l.to_vec(),
                        monos_r.to_vec(),
                    ))?),
                    Self::Stereo((l, r)) => {
                        if monos_l.len() != monos_r.len() {
                            return Err(AudioWriteError::MultipleMonosAreNotSameSize);
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
    }

    #[cfg(feature = "mp3enc")]
    pub use impl_mp3::*;
}

pub use mp3::{Mp3Bitrate, Mp3Channels, Mp3EncoderOptions, Mp3Quality, Mp3VbrMode};

/// ## The Opus encoder for `WaveWriter`
pub mod opus {
    const OPUS_ALLOWED_SAMPLE_RATES: [u32; 5] = [8000, 12000, 16000, 24000, 48000];
    const OPUS_MIN_SAMPLE_RATE: u32 = 8000;
    const OPUS_MAX_SAMPLE_RATE: u32 = 48000;

    /// * The opus encoder only eats these durations of the samples to encode.
    /// * Longer duration means better quality and compression.
    /// * If longer than or equal to 10ms, the compression algorithm could be able to use some advanced technology.
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[repr(u32)]
    pub enum OpusEncoderSampleDuration {
        MilliSec2_5,
        MilliSec5,
        MilliSec10,
        MilliSec20,
        MilliSec40,
        MilliSec60,
    }

    /// ## The bitrate option for the Opus encoder, the higher the better for audio quality.
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

    /// ## The encoder options for Opus
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct OpusEncoderOptions {
        /// * The tier 1 factor for Opus audio quality, bigger bitrate means better audio quality.
        pub bitrate: OpusBitrate,

        /// * VBR mode for better compression, turn it off to get better audio quality.
        pub encode_vbr: bool,

        /// * The opus encoder only eats these durations of the samples to encode.
        /// * Longer duration means better quality and compression.
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
                for (l, h) in OPUS_ALLOWED_SAMPLE_RATES[..OPUS_ALLOWED_SAMPLE_RATES.len() - 1]
                    .iter()
                    .zip(OPUS_ALLOWED_SAMPLE_RATES[1..].iter())
                {
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
        use std::{
            fmt::{self, Debug, Formatter},
            mem,
        };

        use super::*;
        use crate::AudioWriteError;
        use crate::Writer;
        use crate::utils::sample_conv;
        use crate::wavcore::format_tags::*;
        use crate::wavcore::{FmtChunk, Spec};
        use crate::{i24, u24};

        use opus::{Application, Bitrate, Channels, Encoder};

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
                unsafe {
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
    }

    #[cfg(feature = "opus")]
    pub use impl_opus::*;
}

pub use opus::{OpusBitrate, OpusEncoderOptions, OpusEncoderSampleDuration};

/// ## The FLAC encoder for `WaveWriter`
#[cfg(feature = "flac")]
pub mod flac_enc {
    use std::{
        borrow::Cow,
        io::{self, Seek, SeekFrom, Write},
    };

    use super::EncoderToImpl;

    use crate::AudioWriteError;
    use crate::Writer;
    use crate::readwrite::WriteBridge;
    use crate::utils::{sample_conv, sample_conv_batch, stereos_conv};
    use crate::wavcore::format_tags::*;
    use crate::wavcore::{FmtChunk, ListChunk, get_listinfo_flacmeta};
    use crate::{i24, u24};
    use flac::{FlacEncoderParams, FlacEncoderUnmovable};

    #[derive(Debug)]
    pub struct FlacEncoderWrap<'a> {
        encoder: Box<FlacEncoderUnmovable<'a, WriteBridge<'a>>>,
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
            let write_offset = writer.stream_position()?;
            let mut bytes_written = Box::new(0u64);
            let bytes_written_ptr = (&mut *bytes_written) as *mut u64;
            // Let the closures capture the pointer of the boxed variables, then use these pointers to update the variables.
            Ok(Self {
                encoder: Box::new(FlacEncoderUnmovable::new(
                    WriteBridge::new(writer),
                    Box::new(
                        move |writer: &mut WriteBridge, data: &[u8]| -> Result<(), io::Error> {
                            unsafe { *bytes_written_ptr += data.len() as u64 };
                            writer.write_all(data)
                        },
                    ),
                    Box::new(
                        move |writer: &mut WriteBridge, position: u64| -> Result<(), io::Error> {
                            writer.seek(SeekFrom::Start(write_offset + position))?;
                            Ok(())
                        },
                    ),
                    Box::new(move |writer: &mut WriteBridge| -> Result<u64, io::Error> {
                        Ok(write_offset + writer.stream_position()?)
                    }),
                    params,
                )?),
                params: *params,
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
}

/// ## The Ogg Vorbis encoder for `format_tag = 0x674f`
/// * Microsoft says this should be supported: see <https://github.com/tpn/winsdk-10/blob/master/Include/10.0.14393.0/shared/mmreg.h#L2321>
/// * FFmpeg does not support this format: see <https://git.ffmpeg.org/gitweb/ffmpeg.git/blob/refs/heads/release/7.1:/libavformat/riff.c>
pub mod vorbis_enc {
    /// ## Vorbis encoder parameters, NOTE: Most of the comments or documents were copied from `vorbis_rs`
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct VorbisEncoderParams {
        /// Num channels
        pub channels: u16,

        /// Sample rate
        pub sample_rate: u32,

        /// The serials for the generated Ogg Vorbis streams will be randomly generated, as dictated by the Ogg specification. If this behavior is not desirable, set this field to `Some(your_serial_number)`.
        pub stream_serial: Option<i32>,

        /// Vorbis bitrate strategy represents a bitrate management strategy that a Vorbis encoder can use.
        pub bitrate: Option<VorbisBitrateStrategy>,

        /// Specifies the minimum size of Vorbis stream data to put into each Ogg page, except for some header pages,
        /// which have to be cut short to conform to the Ogg Vorbis specification.
        /// This value controls the tradeoff between Ogg encapsulation overhead and ease of seeking and packet loss concealment.
        /// By default, it is set to None, which lets the encoder decide.
        pub minimum_page_data_size: Option<u16>,
    }

    /// ## Vorbis bitrate strategy represents a bitrate management strategy that a Vorbis encoder can use.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum VorbisBitrateStrategy {
        /// Pure VBR quality mode, selected by a target bitrate (in bit/s).
        /// The bitrate management engine is not enabled.
        /// The average bitrate will usually be close to the target, but there are no guarantees.
        /// Easier or harder than expected to encode audio may be encoded at a significantly different bitrate.
        Vbr(u32),

        /// Similar to `Vbr`, this encoding strategy fixes the output subjective quality level,
        /// but lets Vorbis vary the target bitrate depending on the qualities of the input signal.
        /// An upside of this approach is that Vorbis can automatically increase or decrease the target bitrate according to how difficult the signal is to encode,
        /// which guarantees perceptually-consistent results while using an optimal bitrate.
        /// Another upside is that there always is some mode to encode audio at a given quality level.
        /// The downside is that the output bitrate is harder to predict across different types of audio signals.
        QualityVbr(f32),

        /// ABR mode, selected by an average bitrate (in bit/s).
        /// The bitrate management engine is enabled to ensure that the instantaneous bitrate does not divert significantly from the specified average over time,
        /// but no hard bitrate limits are imposed. Any bitrate fluctuations are guaranteed to be minor and short.
        Abr(u32),

        /// Constrained ABR mode, selected by a hard maximum bitrate (in bit/s).
        /// The bitrate management engine is enabled to ensure that the instantaneous bitrate never exceeds the specified maximum bitrate,
        /// which is a hard limit. Internally, the encoder will target an average bitrate that’s slightly lower than the specified maximum bitrate.
        /// The stream is guaranteed to never go above the specified bitrate, at the cost of a lower bitrate,
        /// and thus lower audio quality, on average.
        ConstrainedAbr(u32),
    }

    #[cfg(feature = "vorbis")]
    use super::EncoderToImpl;

    #[cfg(feature = "vorbis")]
    mod impl_vorbis {
        use std::{
            collections::BTreeMap,
            fmt::{self, Debug, Formatter},
            io::Seek,
            num::NonZero,
        };

        use super::*;
        use vorbis_rs::*;

        use crate::AudioWriteError;
        use crate::SharedWriter;
        use crate::Writer;
        use crate::utils::{self, sample_conv, sample_conv_batch};
        use crate::wavcore::FmtChunk;
        use crate::wavcore::format_tags::*;
        use crate::{i24, u24};

        impl VorbisBitrateStrategy {
            /// ## Convert to the `VorbisBitrateManagementStrategy` from `vorbis_rs` crate
            fn to(self) -> VorbisBitrateManagementStrategy {
                match self {
                    Self::Vbr(bitrate) => VorbisBitrateManagementStrategy::Vbr {
                        target_bitrate: NonZero::new(bitrate).unwrap(),
                    },
                    Self::QualityVbr(quality) => VorbisBitrateManagementStrategy::QualityVbr {
                        target_quality: quality,
                    },
                    Self::Abr(bitrate) => VorbisBitrateManagementStrategy::Abr {
                        average_bitrate: NonZero::new(bitrate).unwrap(),
                    },
                    Self::ConstrainedAbr(bitrate) => {
                        VorbisBitrateManagementStrategy::ConstrainedAbr {
                            maximum_bitrate: NonZero::new(bitrate).unwrap(),
                        }
                    }
                }
            }
        }

        impl Default for VorbisBitrateStrategy {
            fn default() -> Self {
                Self::Vbr(320_000) // I don't know which is best for `default()`.
            }
        }

        /// ## The Vorbis encoder or builder enum, the builder one has metadata to put in the builder.
        enum VorbisEncoderOrBuilder<'a> {
            /// The Vorbis encoder builder
            Builder {
                /// The builder that has our shared writer.
                builder: VorbisEncoderBuilder<SharedWriter<'a>>,

                /// The metadata to be added to the Vorbis file. Before the encoder was built, add all of the comments here.
                metadata: BTreeMap<String, String>,
            },

            /// The built encoder. It has our shared writer. Use this to encode PCM waveform to Ogg Vorbis format.
            Encoder(VorbisEncoder<SharedWriter<'a>>),

            /// When the encoding has finished, set this enum to `Finished`
            Finished,
        }

        impl Debug for VorbisEncoderOrBuilder<'_> {
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

        /// ## Vorbis encoder wrap for `WaveWriter`
        #[derive(Debug)]
        pub struct VorbisEncoderWrap<'a> {
            /// The writer for the encoder. Since the encoder only asks for a `Write` trait, we can control where should it write data to by using `seek()`
            writer: SharedWriter<'a>,

            /// The parameters for the encoder
            params: VorbisEncoderParams,

            /// The Vorbis encoder builder or the built encoder.
            encoder: VorbisEncoderOrBuilder<'a>,

            /// The data offset. The Ogg Vorbis data should be written after here.
            data_offset: u64,

            /// How many bytes were written. This field is for calculating the bitrate of the Ogg stream.
            bytes_written: u64,

            /// How many audio frames were written. This field is for calculating the bitrate of the Ogg stream.
            frames_written: u64,
        }

        impl<'a> VorbisEncoderWrap<'a> {
            pub fn new(
                writer: &'a mut dyn Writer,
                params: VorbisEncoderParams,
            ) -> Result<Self, AudioWriteError> {
                let mut writer = SharedWriter::new(writer);
                let data_offset = writer.stream_position()?;

                let sample_rate = NonZero::new(params.sample_rate).unwrap();
                let channels = NonZero::new(params.channels as u8).unwrap();

                let mut builder = VorbisEncoderBuilder::new(sample_rate, channels, writer.clone())?;
                if let Some(serial) = params.stream_serial {
                    builder.stream_serial(serial);
                }
                if let Some(bitrate) = params.bitrate {
                    builder.bitrate_management_strategy(bitrate.to());
                }
                builder.minimum_page_data_size(params.minimum_page_data_size);
                Ok(Self {
                    writer,
                    params,
                    encoder: VorbisEncoderOrBuilder::Builder {
                        builder,
                        metadata: BTreeMap::new(),
                    },
                    data_offset,
                    bytes_written: 0,
                    frames_written: 0,
                })
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
                    VorbisEncoderOrBuilder::Builder{builder: _, ref mut metadata} => {
                        metadata.insert(key, value);
                        Ok(())
                    },
                    _ => Err(AudioWriteError::InvalidArguments("The encoder has already entered encoding mode, why are you just starting to add metadata to it?".to_string())),
                }
            }

            /// When you call this method, the builder will build the encoder, and the enum `self.encoder` will be changed to the encoder.
            /// After this point, you can not add metadata anymore, and then the encoding starts.
            pub fn begin_to_encode(&mut self) -> Result<(), AudioWriteError> {
                match self.encoder {
                    VorbisEncoderOrBuilder::Builder {
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
                        self.encoder = VorbisEncoderOrBuilder::Encoder(builder.build()?);
                        Ok(())
                    }
                    VorbisEncoderOrBuilder::Encoder(_) => Ok(()),
                    VorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The Vorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Write the interleaved samples to the encoder. The interleaved samples were interleaved by channels.
            /// The encoder actually takes the waveform array. Conversion performed during this function.
            pub fn write_interleaved_samples(&mut self, samples: &[f32]) -> Result<(), AudioWriteError> {
                let channels = self.get_channels();
                match self.encoder {
                    VorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    VorbisEncoderOrBuilder::Encoder(ref mut encoder) => {
                        let frames = utils::interleaved_samples_to_monos(samples, channels)?;
                        encoder.encode_audio_block(&frames)?;
                        self.bytes_written = self.writer.stream_position()? - self.data_offset;
                        self.frames_written += frames[0].len() as u64;
                        Ok(())
                    }
                    VorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The Vorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Write multiple mono waveforms to the encoder.
            pub fn write_monos(&mut self, monos: &[Vec<f32>]) -> Result<(), AudioWriteError> {
                match self.encoder {
                    VorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    VorbisEncoderOrBuilder::Encoder(ref mut encoder) => {
                        encoder.encode_audio_block(monos)?;
                        self.bytes_written = self.writer.stream_position()? - self.data_offset;
                        self.frames_written += monos.len() as u64;
                        Ok(())
                    }
                    VorbisEncoderOrBuilder::Finished => Err(AudioWriteError::AlreadyFinished(
                        "The Vorbis encoder has been sealed. No more encoding accepted."
                            .to_string(),
                    )),
                }
            }

            /// Finish encoding audio.
            pub fn finish(&mut self) -> Result<(), AudioWriteError> {
                match self.encoder {
                    VorbisEncoderOrBuilder::Builder {
                        builder: _,
                        metadata: _,
                    } => Err(AudioWriteError::InvalidArguments(
                        "Must call `begin_to_encode()` before encoding.".to_string(),
                    )),
                    VorbisEncoderOrBuilder::Encoder(ref mut _encoder) => {
                        self.encoder = VorbisEncoderOrBuilder::Finished;
                        Ok(())
                    }
                    VorbisEncoderOrBuilder::Finished => Ok(()),
                }
            }
        }

        impl EncoderToImpl for VorbisEncoderWrap<'_> {
            fn get_channels(&self) -> u16 {
                self.params.channels
            }

            fn get_max_channels(&self) -> u16 {
                255
            }

            fn begin_encoding(&mut self) -> Result<(), AudioWriteError> {
                self.begin_to_encode()
            }

            fn get_bitrate(&self) -> u32 {
                if self.frames_written != 0 {
                    (self.bytes_written * 8 * self.get_sample_rate() as u64 / self.frames_written)
                        as u32
                } else {
                    320_000
                }
            }

            fn new_fmt_chunk(&mut self) -> Result<FmtChunk, AudioWriteError> {
                Ok(FmtChunk {
                    format_tag: FORMAT_TAG_VORBIS,
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
    }

    #[cfg(feature = "vorbis")]
    pub use impl_vorbis::*;
}

pub use vorbis_enc::{VorbisBitrateStrategy, VorbisEncoderParams};
