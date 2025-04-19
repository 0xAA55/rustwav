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

// 编码器，接收样本格式 S，编码为文件要的格式
// 因为 trait 不准用泛型参数，所以每一种函数都给我实现一遍。
pub trait EncoderToImpl: Debug {
    fn get_bitrate(&self, channels: u16) -> u32;
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16, channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError>;
    fn update_fmt_chunk(&self, fmt: &mut FmtChunk) -> Result<(), AudioWriteError>;
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError>;

    // 这些是最底层的函数，不关声道数，直接写样本，其中除了 f32 版以外都有默认实现。
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

    // 这些是用于方便写音频帧的接口，每个音频帧是个数组，对应每个声道的样本。有默认的实现。
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

    // 这些是用于方便写多个音频帧而设计的接口，有默认的实现。
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

    // 这些是用来写单声道音频帧的接口，有默认的实现。
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

    // 这些是用来写多个单声道音频帧的接口，有默认的实现。
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

    // 这些是用来写立体声音频的接口，立体声由两个单独的声道数据组成，有默认的实现。
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

    // 这些是用来写多个立体声音频的接口，立体声由两个单独的声道数据组成，有默认的实现。
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

    // 这些是用来写立体声音频的接口，使用 tuple 存储立体声音频帧，有默认的实现。
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

    // 这些是用来写立体声音频的接口，使用 tuple 数组存储立体声音频帧，有默认的实现。
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

// 提供默认实现。无论用户输入的是什么格式，默认用 f32 传递给编码器。
impl EncoderToImpl for () {

    // 这个方法用户必须实现
    fn get_bitrate(&self, _channels: u16) -> u32 {
        panic!("Must implement `get_bitrate()` for your encoder.");
    }

    // 这个方法用户必须实现
    fn new_fmt_chunk(&mut self, _channels: u16, _sample_rate: u32, _bits_per_sample: u16, _channel_mask: Option<u32>) -> Result<FmtChunk, AudioWriteError> {
        panic!("Must implement `new_fmt_chunk()` for your encoder.");
    }

    // 这个方法用户必须实现
    fn update_fmt_chunk(&self, _fmt: &mut FmtChunk) -> Result<(), AudioWriteError> {
        panic!("Must implement `update_fmt_chunk()` for your encoder.");
    }

    // 这个方法用户必须实现
    fn finalize(&mut self, _writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        panic!("Must implement `finalize()` for your encoder to flush the data.");
    }

    // 这个方法用户必须实现
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
pub struct Encoder { // 它就只是负责帮存储一个 `EncoderToImpl`，然后提供具有泛型参数的函数便于调用者使用。
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

// PcmEncoderFrom<S>：样本从 S 类型打包到目标类型
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

    // S：别人给我们的格式
    // T：我们要写入到 WAV 中的格式
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
    // target_sample: 要编码进 WAV 的 PCM 具体格式
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
        // 不能用 clear，否则万一有一次用户写入大量的 sample 的时候，用 clear 后它自己的容量是不会缩小的，一直占着内存。
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
    pub mod true_mp3 {
        use std::{any::type_name, fmt::Debug, sync::{Arc, Mutex}, ops::DerefMut};
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
                    encoder_options: mp3_options.clone(),
                    buffers: match channels {
                        1 => ChannelBuffers::<S>::Mono(BufferMono::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
                        2 => ChannelBuffers::<S>::Stereo(BufferStereo::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
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
                    1 => self.buffers.add_samples_m(writer, &sample_conv::<T, S>(samples)),
                    2 => self.buffers.add_samples_s(writer, &utils::interleaved_samples_to_stereos(&sample_conv::<T, S>(samples))?),
                    o => Err(AudioWriteError::Unsupported(format!("Bad channels number: {o}"))),
                }
            }

            pub fn write_stereos<T>(&mut self, writer: &mut dyn Writer, stereos: &[(T, T)]) -> Result<(), AudioWriteError>
            where T: SampleType {
                match self.channels{
                    1 => self.buffers.add_samples_m(writer, &utils::stereos_to_monos(&stereos_conv::<T, S>(stereos))),
                    2 => self.buffers.add_samples_s(writer, &stereos_conv::<T, S>(stereos)),
                    o => Err(AudioWriteError::InvalidArguments(format!("Bad channels number: {o}"))),
                }
            }

            pub fn finish(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.buffers.finish(writer)
            }
        }

        #[derive(Debug, Clone)]
        enum ChannelBuffers<S>
        where S: SampleType {
            Mono(BufferMono<S>),
            Stereo(BufferStereo<S>),
        }

        #[derive(Clone)]
        struct BufferMono<S>
        where S: SampleType {
            encoder: SharedMp3Encoder,
            mono_pcm: Vec<S>,
            cur_samples: usize,
            max_samples: usize,
        }

        #[derive(Clone)]
        struct BufferStereo<S>
        where S: SampleType{
            encoder: SharedMp3Encoder,
            dual_pcm: Vec<(S, S)>,
            cur_samples: usize,
            max_samples: usize,
        }

        impl<S> ChannelBuffers<S>
        where S: SampleType {
            pub fn is_full(&self) -> bool {
                match self {
                    Self::Mono(mbuf) => mbuf.is_full(),
                    Self::Stereo(sbuf) => sbuf.is_full(),
                }
            }
            pub fn add_sample_m(&mut self, frame: S) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.add_sample(frame),
                    Self::Stereo(sbuf) => sbuf.add_sample((frame, frame)),
                }
            }
            pub fn add_sample_s(&mut self, frame: (S, S)) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.add_sample(S::average(frame.0, frame.1)),
                    Self::Stereo(sbuf) => sbuf.add_sample(frame),
                }
            }
            pub fn add_samples_m(&mut self, writer: &mut dyn Writer, frames: &[S]) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.add_samples(writer, frames),
                    Self::Stereo(sbuf) => sbuf.add_samples(writer, &utils::monos_to_stereos(frames)),
                }
            }
            pub fn add_samples_s(&mut self, writer: &mut dyn Writer, frames: &[(S, S)]) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.add_samples(writer, &utils::stereos_to_monos(frames)),
                    Self::Stereo(sbuf) => sbuf.add_samples(writer, frames),
                }
            }
            pub fn flush(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.flush(writer),
                    Self::Stereo(sbuf) => sbuf.flush(writer),
                }
            }
            pub fn finish(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                match self {
                    Self::Mono(mbuf) => mbuf.finish(writer),
                    Self::Stereo(sbuf) => sbuf.finish(writer),
                }
            }
        }

        impl<S> BufferMono<S>
        where S: SampleType{
            pub fn new(encoder: SharedMp3Encoder, max_samples: usize) -> Self {
                let mut mono_pcm = Vec::<S>::new();
                mono_pcm.resize(max_samples, <S as SampleType>::from(0));
                Self {
                    encoder,
                    mono_pcm,
                    cur_samples: 0,
                    max_samples,
                }
            }

            pub fn is_full(&self) -> bool {
                self.cur_samples >= self.max_samples 
            }

            pub fn add_sample(&mut self, frame: S) -> Result<(), AudioWriteError> {
                if self.cur_samples < self.max_samples {
                    self.mono_pcm[self.cur_samples] = frame;
                    self.cur_samples += 1;
                    Ok(())
                } else {
                    Err(AudioWriteError::BufferIsFull(format!("The buffer is full (max = {} samples)", self.max_samples)))
                }
            }

            // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
            pub fn add_samples(&mut self, writer: &mut dyn Writer, frames: &[S]) -> Result<(), AudioWriteError> {
                for frame in frames.iter() {
                    if self.is_full() {
                        self.flush(writer)?;
                    }
                    self.mono_pcm[self.cur_samples] = *frame;
                    self.cur_samples += 1;
                }
                Ok(())
            }

            fn convert_mono_pcm<T>(&self) -> Vec<T>
            where T: SampleType {
                let mut mono_pcm = Vec::<T>::with_capacity(self.mono_pcm.len());
                for s in self.mono_pcm.iter() {
                    mono_pcm.push(T::from(*s));
                }
                mono_pcm
            }

            fn encode_to_vec(&self, encoder: &mut Encoder, out_buf :&mut Vec<u8>) -> Result<usize, AudioWriteError> {
                match std::any::type_name::<S>() { // i16, u16, i32, f32, f64 是它本来就支持的功能，然而这里需要假装转换一下。
                    "i16" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<i16>()), out_buf)?)
                    },
                    "u16" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<u16>()), out_buf)?)
                    },
                    "i32" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<i32>()), out_buf)?)
                    },
                    "f32" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<f32>()), out_buf)?)
                    },
                    "f64" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<f64>()), out_buf)?)
                    },
                    "i8" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<i16>()), out_buf)?)
                    },
                    "u8" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<u16>()), out_buf)?)
                    },
                    "i24" | "u24" | "u32" => {
                        Ok(encoder.encode_to_vec(MonoPcm(&self.convert_mono_pcm::<i32>()), out_buf)?)
                    },
                    other => Err(AudioWriteError::Unsupported(format!("\"{other}\""))),
                }
            }

            pub fn flush(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                if self.cur_samples == 0 {
                    return Ok(())
                }
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples));
                    self.encode_to_vec(encoder, &mut to_save)?;
                    writer.write_all(&to_save)?;
                    Ok(())
                })?;
                self.cur_samples = 0;
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
                Ok(())
            }
        }

        impl<S> BufferStereo<S>
        where S: SampleType{
            pub fn new(encoder: SharedMp3Encoder, max_samples: usize) -> Self {
                let s0 = <S as SampleType>::from(0);
                Self {
                    encoder,
                    dual_pcm: vec![(s0, s0); max_samples],
                    cur_samples: 0,
                    max_samples,
                }
            }

            pub fn is_full(&self) -> bool {
                self.cur_samples >= self.max_samples 
            }

            pub fn add_sample(&mut self, frame: (S, S)) -> Result<(), AudioWriteError> {
                if self.cur_samples < self.max_samples {
                    self.dual_pcm[self.cur_samples] = frame;
                    self.cur_samples += 1;
                    Ok(())
                } else {
                    Err(AudioWriteError::BufferIsFull(format!("The buffer is full (max = {} samples)", self.max_samples)))
                }
            }

            // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
            pub fn add_samples(&mut self, writer: &mut dyn Writer, frames: &[(S, S)]) -> Result<(), AudioWriteError> {
                for frame in frames.iter() {
                    if self.is_full() {
                        self.flush(writer)?;
                    }
                    self.dual_pcm[self.cur_samples] = *frame;
                    self.cur_samples += 1;
                }
                Ok(())
            }

            fn to_left_right(&self) -> (Vec<S>, Vec<S>) {
                utils::stereos_to_dual_monos(&self.dual_pcm)
            }

            fn convert_dual_pcm<T>(&self) -> (Vec<T>, Vec<T>)
            where T: SampleType {
                utils::stereos_to_dual_monos(&stereos_conv(&self.dual_pcm))
            }

            fn encode_to_vec(&self, encoder: &mut Encoder, out_buf :&mut Vec<u8>) -> Result<usize, AudioWriteError> {
                match std::any::type_name::<S>() { // i16, u16, i32, f32, f64 是它本来就支持的功能，然而这里需要假装转换一下。
                    "i16" => {
                        let (l, r) = self.convert_dual_pcm::<i16>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "u16" => {
                        let (l, r) = self.convert_dual_pcm::<u16>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "i32" => {
                        let (l, r) = self.convert_dual_pcm::<i32>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "f32" => {
                        let (l, r) = self.convert_dual_pcm::<f32>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "f64" => {
                        let (l, r) = self.convert_dual_pcm::<f64>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "i8" => {
                        let (l, r) = self.convert_dual_pcm::<i16>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "u8" => {
                        let (l, r) = self.convert_dual_pcm::<u16>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    "i24" | "u24" | "u32" => {
                        let (l, r) = self.convert_dual_pcm::<i32>();
                        Ok(encoder.encode_to_vec(DualPcm{left: &l, right: &r}, out_buf)?)
                    },
                    other => Err(AudioWriteError::Unsupported(format!("\"{other}\""))),
                }
            }

            pub fn flush(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                if self.cur_samples == 0 {
                    return Ok(())
                }
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples * 2));
                    self.encode_to_vec(encoder, &mut to_save)?;
                    writer.write_all(&to_save)?;
                    Ok(())
                })?;
                self.cur_samples = 0;
                Ok(())
            }

            pub fn finish(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
                self.flush(writer)?;
                self.encoder.escorted_encode(|encoder| -> Result<(), AudioWriteError> {
                    let mut to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples * 2));
                    encoder.flush_to_vec::<FlushNoGap>(&mut to_save)?;
                    writer.write_all(&to_save)?;
                    Ok(())
                })?;
                writer.flush()?;
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

        impl<S> Debug for BufferMono<S>
        where S: SampleType {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(&format!("BufferMono<{}>", type_name::<S>()))
                    .field("mono_pcm", &format_args!("mono_pcm"))
                    .field("cur_samples", &self.cur_samples)
                    .field("max_samples", &self.max_samples)
                    .finish()
            }
        }

        impl<S> Debug for BufferStereo<S>
        where S: SampleType {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(&format!("BufferStereo<{}>", type_name::<S>()))
                    .field("dual_pcm", &format_args!("dual_pcm"))
                    .field("cur_samples", &self.cur_samples)
                    .field("max_samples", &self.max_samples)
                    .finish()
            }
        }
    }

    #[cfg(feature = "mp3enc")]
    pub use true_mp3::*;
}

#[cfg(feature = "opus")]
pub mod opus {
    use std::mem;

    use super::EncoderToImpl;
    use crate::Writer;
    use crate::wavcore::{FmtChunk, SpeakerPosition};
    use crate::AudioWriteError;
    use crate::{i24, u24};
    use crate::utils::sample_conv;

    use opus::{Encoder, Application, Channels, Bitrate};

    #[derive(Debug, Clone, Copy)]
    #[repr(u32)]
    pub enum OpusEncoderSampleDuration {
        MilliSec2_5 = 25,
        MilliSec5 = 50,
        MilliSec10 = 100,
        MilliSec20 = 200,
        MilliSec40 = 400,
        MilliSec60 = 600,
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
        pub fn new(channels: u16, sample_rate: u32, bitrate: Option<Bitrate>, encode_vbr: Option<bool>, samples_cache_duration: Option<OpusEncoderSampleDuration>) -> Result<Self, AudioWriteError> {
            let opus_channels = match channels {
                1 => Channels::Mono,
                2 => Channels::Stereo,
                o => return Err(AudioWriteError::InvalidArguments(format!("Bad channels: {o} for the opus encoder."))),
            };
            match sample_rate {
                8000 | 12000 | 16000 | 24000 | 48000 => (),
                o => return Err(AudioWriteError::InvalidArguments(format!("Bad sample_rate: {o}. Must be one of 8000, 12000, 16000, 24000, 48000"))),
            }
            let mut encoder = Encoder::new(sample_rate, opus_channels, Application::Audio)?;
            // 所有的可选参数不输入的话默认把音质、带宽等全部拉满
            let bitrate = if let Some(bitrate) = bitrate {
                bitrate
            } else {
                Bitrate::Max
            };
            let encode_vbr = encode_vbr.unwrap_or(false);
            encoder.set_bitrate(bitrate)?;
            encoder.set_vbr(encode_vbr)?;
            let cache_duration = if let Some(cache_duration) = samples_cache_duration{
                cache_duration
            } else {
                OpusEncoderSampleDuration::MilliSec60
            };
            let num_samples_per_encode = cache_duration.get_num_samples(channels, sample_rate);
            Ok(Self {
                encoder,
                channels,
                sample_rate,
                cache_duration,
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
                let mut buf = vec![0u8; self.num_samples_per_encode]; // 给定足够大小的缓冲区。每个样本给它一个字节。
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




