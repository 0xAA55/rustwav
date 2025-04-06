#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{cmp, fmt::Debug};

use crate::adpcm;
use crate::AudioWriteError;
use crate::WaveSampleType;
use crate::{SampleType, i24, u24};
use crate::Writer;
use crate::utils::{self, sample_conv, stereo_conv, sample_conv_batch};

// 编码器，接收样本格式 S，编码为文件要的格式
// 因为 trait 不准用泛型参数，所以每一种函数都给我实现一遍。
pub trait EncoderToImpl: Debug {
    fn get_bit_rate(&mut self) -> u32;
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
    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >], channels: u16) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>], channels: u16) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::multiple_frames_to_interleaved_samples(frames, Some(channels))?)}

    // 这些是用来写单声道音频帧的接口，有默认的实现。
    fn write_mono__i8(&mut self, writer: &mut dyn Writer, frame: i8 ) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_i16(&mut self, writer: &mut dyn Writer, frame: i16) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_i24(&mut self, writer: &mut dyn Writer, frame: i24) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_i32(&mut self, writer: &mut dyn Writer, frame: i32) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_i64(&mut self, writer: &mut dyn Writer, frame: i64) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono__u8(&mut self, writer: &mut dyn Writer, frame: u8 ) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_u16(&mut self, writer: &mut dyn Writer, frame: u16) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_u24(&mut self, writer: &mut dyn Writer, frame: u24) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_u32(&mut self, writer: &mut dyn Writer, frame: u32) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_u64(&mut self, writer: &mut dyn Writer, frame: u64) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_f32(&mut self, writer: &mut dyn Writer, frame: f32) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}
    fn write_mono_f64(&mut self, writer: &mut dyn Writer, frame: f64) -> Result<(), AudioWriteError> {Ok(frame.write_le(writer)?)}

    // 这些是用来写多个单声道音频帧的接口，有默认的实现。
    fn write_multiple_mono__i8(&mut self, writer: &mut dyn Writer, frames: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, frames)}
    fn write_multiple_mono_i16(&mut self, writer: &mut dyn Writer, frames: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, frames)}
    fn write_multiple_mono_i24(&mut self, writer: &mut dyn Writer, frames: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, frames)}
    fn write_multiple_mono_i32(&mut self, writer: &mut dyn Writer, frames: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, frames)}
    fn write_multiple_mono_i64(&mut self, writer: &mut dyn Writer, frames: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, frames)}
    fn write_multiple_mono__u8(&mut self, writer: &mut dyn Writer, frames: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, frames)}
    fn write_multiple_mono_u16(&mut self, writer: &mut dyn Writer, frames: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, frames)}
    fn write_multiple_mono_u24(&mut self, writer: &mut dyn Writer, frames: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, frames)}
    fn write_multiple_mono_u32(&mut self, writer: &mut dyn Writer, frames: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, frames)}
    fn write_multiple_mono_u64(&mut self, writer: &mut dyn Writer, frames: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, frames)}
    fn write_multiple_mono_f32(&mut self, writer: &mut dyn Writer, frames: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, frames)}
    fn write_multiple_mono_f64(&mut self, writer: &mut dyn Writer, frames: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, frames)}

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
    fn write_multiple_dual_mono__i8(&mut self, writer: &mut dyn Writer, mono1: &[i8 ], mono2: &[i8 ]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_i16(&mut self, writer: &mut dyn Writer, mono1: &[i16], mono2: &[i16]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_i24(&mut self, writer: &mut dyn Writer, mono1: &[i24], mono2: &[i24]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_i32(&mut self, writer: &mut dyn Writer, mono1: &[i32], mono2: &[i32]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_i64(&mut self, writer: &mut dyn Writer, mono1: &[i64], mono2: &[i64]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono__u8(&mut self, writer: &mut dyn Writer, mono1: &[u8 ], mono2: &[u8 ]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_u16(&mut self, writer: &mut dyn Writer, mono1: &[u16], mono2: &[u16]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_u24(&mut self, writer: &mut dyn Writer, mono1: &[u24], mono2: &[u24]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_u32(&mut self, writer: &mut dyn Writer, mono1: &[u32], mono2: &[u32]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_u64(&mut self, writer: &mut dyn Writer, mono1: &[u64], mono2: &[u64]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_f32(&mut self, writer: &mut dyn Writer, mono1: &[f32], mono2: &[f32]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}
    fn write_multiple_dual_mono_f64(&mut self, writer: &mut dyn Writer, mono1: &[f64], mono2: &[f64]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::multiple_monos_to_interleaved_samples(&[mono1.to_vec(), mono2.to_vec()])?)}

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
    fn write_multiple_stereos__i8(&mut self, writer: &mut dyn Writer, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_samples__i8(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_i16(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_samples_i16(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_i24(&mut self, writer: &mut dyn Writer, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_samples_i24(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_i32(&mut self, writer: &mut dyn Writer, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_samples_i32(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_i64(&mut self, writer: &mut dyn Writer, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_samples_i64(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos__u8(&mut self, writer: &mut dyn Writer, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_samples__u8(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_u16(&mut self, writer: &mut dyn Writer, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_samples_u16(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_u24(&mut self, writer: &mut dyn Writer, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_samples_u24(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_u32(&mut self, writer: &mut dyn Writer, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_samples_u32(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_u64(&mut self, writer: &mut dyn Writer, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_samples_u64(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_f32(&mut self, writer: &mut dyn Writer, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_samples_f32(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
    fn write_multiple_stereos_f64(&mut self, writer: &mut dyn Writer, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_samples_f64(writer, &utils::multiple_stereos_to_interleaved_samples(stereos))}
}

// 提供默认实现。无论用户输入的是什么格式，默认用 f32 传递给编码器。
impl EncoderToImpl for () {

    // 这个方法用户必须实现
    fn get_bit_rate(&mut self) -> u32 {
        panic!("Must implement `get_bit_rate()` for your encoder.");
    }

    // 这个方法用户必须实现
    fn write_samples_f32(&mut self, _writer: &mut dyn Writer, _samples: &[f32]) -> Result<(), AudioWriteError> {
        panic!("Must atlease implement `write_samples_f32()` for your encoder to get samples.");
    }

    // 这个方法用户必须实现
    fn finalize(&mut self, _writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        panic!("Must implement `finalize()` for your encoder to flush the data.");
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
    pub fn new(encoder: impl EncoderToImpl + 'static) -> Self {
        Self {
            encoder: Box::new(encoder),
        }
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

    pub fn write_multiple_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>], channels: u16) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() { // 希望编译器能做到优化，省区字符串比对的过程。
            "i8"  => self.encoder.write_multiple_frames__i8(writer, &sample_conv_batch(frames), channels),
            "i16" => self.encoder.write_multiple_frames_i16(writer, &sample_conv_batch(frames), channels),
            "i24" => self.encoder.write_multiple_frames_i24(writer, &sample_conv_batch(frames), channels),
            "i32" => self.encoder.write_multiple_frames_i32(writer, &sample_conv_batch(frames), channels),
            "i64" => self.encoder.write_multiple_frames_i64(writer, &sample_conv_batch(frames), channels),
            "u8"  => self.encoder.write_multiple_frames__u8(writer, &sample_conv_batch(frames), channels),
            "u16" => self.encoder.write_multiple_frames_u16(writer, &sample_conv_batch(frames), channels),
            "u24" => self.encoder.write_multiple_frames_u24(writer, &sample_conv_batch(frames), channels),
            "u32" => self.encoder.write_multiple_frames_u32(writer, &sample_conv_batch(frames), channels),
            "u64" => self.encoder.write_multiple_frames_u64(writer, &sample_conv_batch(frames), channels),
            "f32" => self.encoder.write_multiple_frames_f32(writer, &sample_conv_batch(frames), channels),
            "f64" => self.encoder.write_multiple_frames_f64(writer, &sample_conv_batch(frames), channels),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_mono<S>(&mut self, writer: &mut dyn Writer, mono: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        Ok(mono.write_le(writer)?)
    }

    pub fn write_multiple_mono<S>(&mut self, writer: &mut dyn Writer, monos: &[S]) -> Result<(), AudioWriteError>
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
        mono1.write_le(writer)?;
        mono2.write_le(writer)?;
        Ok(())
    }

    pub fn write_multiple_dual_mono<S>(&mut self, writer: &mut dyn Writer, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_multiple_dual_mono__i8(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i16" => self.encoder.write_multiple_dual_mono_i16(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i24" => self.encoder.write_multiple_dual_mono_i24(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i32" => self.encoder.write_multiple_dual_mono_i32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "i64" => self.encoder.write_multiple_dual_mono_i64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u8"  => self.encoder.write_multiple_dual_mono__u8(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u16" => self.encoder.write_multiple_dual_mono_u16(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u24" => self.encoder.write_multiple_dual_mono_u24(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u32" => self.encoder.write_multiple_dual_mono_u32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "u64" => self.encoder.write_multiple_dual_mono_u64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "f32" => self.encoder.write_multiple_dual_mono_f32(writer, &sample_conv(mono1), &sample_conv(mono2)),
            "f64" => self.encoder.write_multiple_dual_mono_f64(writer, &sample_conv(mono1), &sample_conv(mono2)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn write_stereo<S>(&mut self, writer: &mut dyn Writer, stereo: (S, S)) -> Result<(), AudioWriteError>
    where S: SampleType {
        stereo.0.write_le(writer)?;
        stereo.1.write_le(writer)?;
        Ok(())
    }

    pub fn write_multiple_stereos<S>(&mut self, writer: &mut dyn Writer, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_multiple_stereos__i8(writer, &stereo_conv(stereos)),
            "i16" => self.encoder.write_multiple_stereos_i16(writer, &stereo_conv(stereos)),
            "i24" => self.encoder.write_multiple_stereos_i24(writer, &stereo_conv(stereos)),
            "i32" => self.encoder.write_multiple_stereos_i32(writer, &stereo_conv(stereos)),
            "i64" => self.encoder.write_multiple_stereos_i64(writer, &stereo_conv(stereos)),
            "u8"  => self.encoder.write_multiple_stereos__u8(writer, &stereo_conv(stereos)),
            "u16" => self.encoder.write_multiple_stereos_u16(writer, &stereo_conv(stereos)),
            "u24" => self.encoder.write_multiple_stereos_u24(writer, &stereo_conv(stereos)),
            "u32" => self.encoder.write_multiple_stereos_u32(writer, &stereo_conv(stereos)),
            "u64" => self.encoder.write_multiple_stereos_u64(writer, &stereo_conv(stereos)),
            "f32" => self.encoder.write_multiple_stereos_f32(writer, &stereo_conv(stereos)),
            "f64" => self.encoder.write_multiple_stereos_f64(writer, &stereo_conv(stereos)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }

    pub fn get_bit_rate(&mut self) -> u32 {
        self.encoder.get_bit_rate()
    }

    pub fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.encoder.finalize(writer)
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

    pub fn write_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError> {
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
    channels: u16,
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
    pub fn new(channels: u16, sample_rate: u32, target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        Ok(Self {
            channels,
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

    fn get_bit_rate(&mut self) -> u32 {
        self.channels as u32 * self.sample_rate * self.sample_type.sizeof() as u32 * 8
    }
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        Ok(writer.flush()?)
    }
}

#[derive(Clone)]
pub struct AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    sample_rate: u32,
    samples_written: u64,
    bytes_written: u64,
    is_stereo: bool,
    encoder_l: E,
    encoder_r: E,
    buffer_l: Vec<u8>,
    buffer_r: Vec<u8>,
}

impl<E> Debug for AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct(&format!("AdpcmEncoderWrap<{}>", std::any::type_name::<E>()))
            .field("sample_rate", &self.sample_rate)
            .field("samples_written", &self.samples_written)
            .field("bytes_written", &self.bytes_written)
            .field("is_stereo", &self.is_stereo)
            .field("encoder_l", &self.encoder_l)
            .field("encoder_r", &self.encoder_r)
            .field("buffer_l", &format_args!("[{}u8;...]", self.buffer_l.len()))
            .field("buffer_r", &format_args!("[{}u8;...]", self.buffer_r.len()))
            .finish()
    }
}

impl<E> AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    pub fn new(sample_rate: u32, is_stereo: bool) -> Self {
        Self {
            sample_rate,
            samples_written: 0,
            bytes_written: 0,
            is_stereo,
            encoder_l: E::new(),
            encoder_r: E::new(),
            buffer_l: Vec::<u8>::new(),
            buffer_r: Vec::<u8>::new(),
        }
    }

    fn flush_stereo(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let mut interleaved = Vec::<u8>::new();
        if self.is_stereo {
            let min_len = cmp::min(self.buffer_l.len(), self.buffer_r.len());
            if min_len > 0 {
                for i in 0..min_len {
                    interleaved.push(self.buffer_l[i]);
                    interleaved.push(self.buffer_r[i]);
                }
                writer.write_all(&interleaved)?;
                self.bytes_written += interleaved.len() as u64;
                if self.buffer_l.len() > min_len {
                    self.buffer_l = self.buffer_l.clone().into_iter().skip(min_len).collect();
                } else {
                    self.buffer_l.clear();
                }
                if self.buffer_r.len() > min_len {
                    self.buffer_r = self.buffer_r.clone().into_iter().skip(min_len).collect();
                } else {
                    self.buffer_r.clear();
                }
            }
        } else {
            writer.write_all(&self.buffer_l)?;
            self.buffer_l.clear();
        }
        Ok(())
    }

    pub fn write_samples(&mut self, writer: &mut dyn Writer, samples: &[i16]) -> Result<(), AudioWriteError> {
        if self.is_stereo {
            let monos = utils::interleaved_samples_to_multiple_monos(samples, 2)?;
            let (mono_l, mono_r) = (&monos[0], &monos[1]);
            let mut mono_l = mono_l.into_iter();
            let mut mono_r = mono_r.into_iter();
            self.encoder_l.encode(|| -> Option<i16> { mono_l.next().copied() }, |byte: u8|{ self.buffer_l.push(byte); })?;
            self.encoder_r.encode(|| -> Option<i16> { mono_r.next().copied() }, |byte: u8|{ self.buffer_r.push(byte); })?;
        } else {
            let mut iter = samples.iter();
            self.encoder_l.encode(|| -> Option<i16> { iter.next().copied() }, |byte: u8|{ self.buffer_l.push(byte); })?;
        }
        self.samples_written += samples.len() as u64;
        self.flush_stereo(writer)?;
        Ok(())
    }

    pub fn write_multiple_stereos(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {
        if self.is_stereo == false {
            return Err(AudioWriteError::InvalidArguments("This encoder is not for stereo audio".to_owned()));
        }
        let (lv, rv) = utils::stereos_to_dual_mono(stereos);
        let (ll, rl) = (lv.len(), rv.len());
        let (mut li, mut ri) = (lv.into_iter(), rv.into_iter());
        self.encoder_l.encode(|| -> Option<i16> { li.next() }, |byte: u8|{ self.buffer_l.push(byte); })?;
        self.encoder_r.encode(|| -> Option<i16> { ri.next() }, |byte: u8|{ self.buffer_r.push(byte); })?;
        self.samples_written += ll as u64;
        self.samples_written += rl as u64;
        self.flush_stereo(writer)?;
        Ok(())
    }
}

impl<E> EncoderToImpl for AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
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

    fn write_multiple_stereos__i8(&mut self, writer: &mut dyn Writer, stereos: &[(i8 , i8 )]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_i16(&mut self, writer: &mut dyn Writer, stereos: &[(i16, i16)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_i24(&mut self, writer: &mut dyn Writer, stereos: &[(i24, i24)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_i32(&mut self, writer: &mut dyn Writer, stereos: &[(i32, i32)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_i64(&mut self, writer: &mut dyn Writer, stereos: &[(i64, i64)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos__u8(&mut self, writer: &mut dyn Writer, stereos: &[(u8 , u8 )]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_u16(&mut self, writer: &mut dyn Writer, stereos: &[(u16, u16)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_u24(&mut self, writer: &mut dyn Writer, stereos: &[(u24, u24)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_u32(&mut self, writer: &mut dyn Writer, stereos: &[(u32, u32)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_u64(&mut self, writer: &mut dyn Writer, stereos: &[(u64, u64)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_f32(&mut self, writer: &mut dyn Writer, stereos: &[(f32, f32)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}
    fn write_multiple_stereos_f64(&mut self, writer: &mut dyn Writer, stereos: &[(f64, f64)]) -> Result<(), AudioWriteError> {self.write_multiple_stereos(writer, &stereo_conv(stereos))}

    fn get_bit_rate(&mut self) -> u32 {
        if self.samples_written == 0 {
            self.sample_rate * 8 // 估算
        } else {
            (self.bytes_written / (self.samples_written / (self.sample_rate as u64))) as u32 * 8
        }
    }
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.flush_stereo(writer)?;
        Ok(writer.flush()?)
    }
}

#[cfg(feature = "mp3enc")]
pub mod MP3 {
    use std::{any::type_name, fmt::Debug, sync::{Arc, Mutex}, ops::DerefMut};
    use crate::Writer;
    use crate::{SampleType, i24, u24};
    use crate::AudioWriteError;
    use crate::EncoderToImpl;
    use crate::utils::{self, sample_conv, stereo_conv};
    use mp3lame_encoder::{Builder, Encoder, Mode, MonoPcm, DualPcm, FlushNoGap};
    pub use mp3lame_encoder::{Bitrate, VbrMode, Quality, Id3Tag};

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
            let mut encoder = guard.deref_mut();
            (action)(&mut encoder)
        }
    }

    #[derive(Debug, Clone)]
    pub struct Mp3Encoder<S>
    where S: SampleType {
        channels: u8,
        bitrate: u32,
        encoder: SharedMp3Encoder,
        buffers: ChannelBuffers<S>,
    }

    impl<S> Mp3Encoder<S>
    where S: SampleType {
        pub fn new(channels: u8, sample_rate: u32, bit_rate: Option<Bitrate>, quality: Option<Quality>, vbr_mode: Option<VbrMode>, id3tag: Option<Id3Tag>) -> Result<Self, AudioWriteError> {
            let mp3_builder = Builder::new();
            let mut mp3_builder = match mp3_builder {
                Some(mp3_builder) => mp3_builder,
                None => return Err(AudioWriteError::OtherReason("`lame_init()` somehow failed.".to_owned())),
            };

            match channels {
                1 => mp3_builder.set_mode(Mode::Mono)?,
                2 => mp3_builder.set_mode(Mode::JointStereo)?,
                other => return Err(AudioWriteError::InvalidArguments(format!("Bad channel number: {}", other))),
            }

            mp3_builder.set_num_channels(channels)?;
            mp3_builder.set_sample_rate(sample_rate)?;

            // 设置位率，直接决定音质。如果没有提供位率，就使用 320 kbps 位率。
            let bitrate = match bit_rate {
                Some(bit_rate) => bit_rate,
                None => Bitrate::Kbps320,
            };
            mp3_builder.set_brate(bitrate)?;

            // 设置品质，如果没设置那就是最佳品质
            let quality = match quality {
                Some(quality) => quality,
                None => Quality::Best,
            };

            // 此处决定这个 MP3 是 VBR 还是 CBR 模式
            if let Some(vbr_mode) = vbr_mode {
                mp3_builder.set_to_write_vbr_tag(true)?;
                mp3_builder.set_vbr_mode(vbr_mode)?;
                mp3_builder.set_vbr_quality(quality)?;
            } else {
                mp3_builder.set_to_write_vbr_tag(false)?;
                mp3_builder.set_quality(quality)?;
            }

            // 如果提供了 id3 信息则设置
            if let Some(id3tag) = id3tag {
                mp3_builder.set_id3_tag(id3tag)?;
            }

            let encoder = SharedMp3Encoder::new(mp3_builder.build()?);

            // 创建编码器
            Ok(Self {
                channels,
                bitrate: bitrate as u32 * 1000,
                encoder: encoder.clone(),
                buffers: match channels {
                    1 => ChannelBuffers::<S>::Mono(BufferMono::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
                    2 => ChannelBuffers::<S>::Stereo(BufferStereo::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
                    other => return Err(AudioWriteError::InvalidArguments(format!("Bad channel number: {}", other))),
                },
            })
        }

        pub fn write_samples<T>(&mut self, writer: &mut dyn Writer, samples: &[T]) -> Result<(), AudioWriteError>
        where T: SampleType {
            if self.buffers.is_full() {
                self.buffers.flush(writer)?;
            }
            match self.channels {
                1 => Ok(self.buffers.add_multiple_samples_m(writer, &sample_conv::<T, S>(samples))?),
                2 => Ok(self.buffers.add_multiple_samples_s(writer, &utils::interleaved_samples_to_stereos(&sample_conv::<T, S>(samples))?)?),
                other => return Err(AudioWriteError::InvalidArguments(format!("Bad channels number: {other}"))),
            }
        }

        pub fn write_multiple_frames<T>(&mut self, writer: &mut dyn Writer, frames: &[Vec<T>]) -> Result<(), AudioWriteError>
        where T: SampleType {
            match self.buffers {
                ChannelBuffers::Mono(ref mut mbuf) => {
                    let mut buf = Vec::<S>::with_capacity(frames.len());
                    for frame in frames.iter() {
                        if frame.len() != 1 {
                            return Err(AudioWriteError::InvalidArguments(format!("Bad frame channels: {}, should be 1", frame.len())))
                        } else {
                            buf.push(S::from(frame[0]));
                        }
                    }
                    mbuf.add_multiple_samples(writer, &buf)?;
                },
                ChannelBuffers::Stereo(ref mut sbuf) => {
                    let mut buf = Vec::<(S, S)>::with_capacity(frames.len());
                    for frame in frames.iter() {
                        if frame.len() != 2 {
                            return Err(AudioWriteError::InvalidArguments(format!("Bad frame channels: {}, should be 2", frame.len())))
                        } else {
                            buf.push((S::from(frame[0]), S::from(frame[1])));
                        }
                    }
                    sbuf.add_multiple_samples(writer, &buf)?;
                },
            }
            Ok(())
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
                Self::Stereo(_) => Err(AudioWriteError::InvalidArguments("The buffer is stereo, can't add mono sample".to_owned())),
            }
        }
        pub fn add_sample_s(&mut self, frame: (S, S)) -> Result<(), AudioWriteError> {
            match self {
                Self::Mono(_) => Err(AudioWriteError::InvalidArguments("The buffer is mono, can't add stereo sample".to_owned())),
                Self::Stereo(sbuf) => sbuf.add_sample(frame),
            }
        }
        pub fn add_multiple_samples_m(&mut self, writer: &mut dyn Writer, frames: &[S]) -> Result<(), AudioWriteError> {
            match self {
                Self::Mono(mbuf) => mbuf.add_multiple_samples(writer, frames),
                Self::Stereo(_) => Err(AudioWriteError::InvalidArguments("The buffer is stereo, can't add mono samples".to_owned())),
            }
        }
        pub fn add_multiple_samples_s(&mut self, writer: &mut dyn Writer, frames: &[(S, S)]) -> Result<(), AudioWriteError> {
            match self {
                Self::Mono(_) => Err(AudioWriteError::InvalidArguments("The buffer is mono, can't add stereo samples".to_owned())),
                Self::Stereo(sbuf) => sbuf.add_multiple_samples(writer, frames),
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
                Err(AudioWriteError::BufferIsFull)
            }
        }

        // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
        pub fn add_multiple_samples(&mut self, writer: &mut dyn Writer, frames: &[S]) -> Result<(), AudioWriteError> {
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
            let mut dual_pcm = Vec::<(S, S)>::new();
            let s0 = <S as SampleType>::from(0);
            dual_pcm.resize(max_samples, (s0, s0));
            Self {
                encoder,
                dual_pcm,
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
                Err(AudioWriteError::BufferIsFull)
            }
        }

        // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
        pub fn add_multiple_samples(&mut self, writer: &mut dyn Writer, frames: &[(S, S)]) -> Result<(), AudioWriteError> {
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
            let mut vl = Vec::<S>::with_capacity(self.max_samples);
            let mut vr = Vec::<S>::with_capacity(self.max_samples);
            for (l, r) in self.dual_pcm.iter() {
                vl.push(*l);
                vr.push(*r);
            }
            (vl, vr)
        }

        fn convert_dual_pcm<T>(&self) -> (Vec<T>, Vec<T>)
        where T: SampleType {
            let mut vl = Vec::<T>::with_capacity(self.max_samples);
            let mut vr = Vec::<T>::with_capacity(self.max_samples);
            for s in self.dual_pcm.iter() {
                let (l, r) = *s;
                let l = T::from(l);
                let r = T::from(r);
                vl.push(l);
                vr.push(r);
            }
            (vl, vr)
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
        fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &Vec<f32>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}
        fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), AudioWriteError> {self.write_frame(writer, frame)}

        fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}
        fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, frames)}

        fn get_bit_rate(&mut self) -> u32 {
            self.bitrate as u32
        }

        fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
            Ok(self.finish(writer)?)
        }
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






