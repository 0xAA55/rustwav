#![allow(dead_code)]
#![allow(non_snake_case)]

use std::fmt::Debug;

// use crate::adpcm::*;
use crate::AudioWriteError;
use crate::WaveSampleType;
use crate::{SampleType, i24, u24};
use crate::Writer;

// 编码器，接收样本格式 S，编码为文件要的格式
// 因为 trait 不准用泛型参数，所以每一种函数都给我实现一遍。
pub trait EncoderToImpl: Debug {
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), AudioWriteError>;
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError>;
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), AudioWriteError>;
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), AudioWriteError>;
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), AudioWriteError>;
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), AudioWriteError>;
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), AudioWriteError>;
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), AudioWriteError>;
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), AudioWriteError>;
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), AudioWriteError>;
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &Vec<f32>) -> Result<(), AudioWriteError>;
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), AudioWriteError>;

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), AudioWriteError>;
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), AudioWriteError>;
}

// 提供默认实现。无论用户输入的是什么格式，默认用 f32 传递给编码器。
impl EncoderToImpl for () {
    // 这个方法用户必须实现
    fn write_frame_f32(&mut self, _writer: &mut dyn Writer, _frame: &Vec<f32>) -> Result<(), AudioWriteError> {
        panic!("Must implement `write_frame_f32()` for your encoder to get samples.");
    }

    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), AudioWriteError> {self.write_frame_f32(writer, &sample_conv(frame))}

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames_f32(writer, &sample_conv_batch(frames))}

    // 这个东西可以帮用户实现
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {
        for frame in frames.iter() {
            self.write_frame_f32(writer, frame)?;
        }
        Ok(())
    }
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

    pub fn write_frame<S>(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() { // 希望编译器能做到优化，省区字符串比对的过程。
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

    pub fn write_multiple_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        match std::any::type_name::<S>() { // 希望编译器能做到优化，省区字符串比对的过程。
            "i8"  => self.encoder.write_multiple_frames__i8(writer, &sample_conv_batch(frames)),
            "i16" => self.encoder.write_multiple_frames_i16(writer, &sample_conv_batch(frames)),
            "i24" => self.encoder.write_multiple_frames_i24(writer, &sample_conv_batch(frames)),
            "i32" => self.encoder.write_multiple_frames_i32(writer, &sample_conv_batch(frames)),
            "i64" => self.encoder.write_multiple_frames_i64(writer, &sample_conv_batch(frames)),
            "u8"  => self.encoder.write_multiple_frames__u8(writer, &sample_conv_batch(frames)),
            "u16" => self.encoder.write_multiple_frames_u16(writer, &sample_conv_batch(frames)),
            "u24" => self.encoder.write_multiple_frames_u24(writer, &sample_conv_batch(frames)),
            "u32" => self.encoder.write_multiple_frames_u32(writer, &sample_conv_batch(frames)),
            "u64" => self.encoder.write_multiple_frames_u64(writer, &sample_conv_batch(frames)),
            "f32" => self.encoder.write_multiple_frames_f32(writer, &sample_conv_batch(frames)),
            "f64" => self.encoder.write_multiple_frames_f64(writer, &sample_conv_batch(frames)),
            other => Err(AudioWriteError::InvalidArguments(format!("Bad sample type: {}", other))),
        }
    }
}

// 样本类型缩放转换
// 根据样本的存储值范围大小的不同，进行缩放使适应目标样本类型。
fn sample_conv<S, D>(frame: &[S]) -> Vec<D>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<D>::with_capacity(frame.len());
    for f in frame.iter() {
        ret.push(D::from(*f));
    }
    ret
}
// 样本类型缩放转换批量版
fn sample_conv_batch<S, D>(frames: &[Vec<S>]) -> Vec<Vec<D>>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<Vec<D>>::with_capacity(frames.len());
    for f in frames.iter() {
        ret.push(sample_conv(f));
    }
    ret
}

// PcmEncoderFrom<S>：样本从 S 类型打包到目标类型
#[derive(Debug)]
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
}

#[derive(Debug)]
pub struct PcmEncoder {
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
    pub fn new(target_sample: WaveSampleType) -> Result<Self, AudioWriteError> {
        Ok(Self {
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
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), AudioWriteError> {self.writer_from__i8.write_frame(writer, frame)}
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError> {self.writer_from_i16.write_frame(writer, frame)}
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), AudioWriteError> {self.writer_from_i24.write_frame(writer, frame)}
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), AudioWriteError> {self.writer_from_i32.write_frame(writer, frame)}
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), AudioWriteError> {self.writer_from_i64.write_frame(writer, frame)}
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), AudioWriteError> {self.writer_from__u8.write_frame(writer, frame)}
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), AudioWriteError> {self.writer_from_u16.write_frame(writer, frame)}
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), AudioWriteError> {self.writer_from_u24.write_frame(writer, frame)}
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), AudioWriteError> {self.writer_from_u32.write_frame(writer, frame)}
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), AudioWriteError> {self.writer_from_u64.write_frame(writer, frame)}
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &Vec<f32>) -> Result<(), AudioWriteError> {self.writer_from_f32.write_frame(writer, frame)}
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), AudioWriteError> {self.writer_from_f64.write_frame(writer, frame)}

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.writer_from__i8.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.writer_from_i16.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.writer_from_i24.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.writer_from_i32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.writer_from_i64.write_multiple_frames(writer, frames)}
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.writer_from__u8.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.writer_from_u16.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.writer_from_u24.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.writer_from_u32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.writer_from_u64.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.writer_from_f32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.writer_from_f64.write_multiple_frames(writer, frames)}
}

#[cfg(feature = "mp3enc")]
pub mod MP3 {
    use std::{fmt::Debug, error::Error};
    use crate::Writer;
    use crate::AudioWriteError;
    use mp3lame_encoder::{Builder, Encoder, Mode, DualPcm, FlushNoGap};
    pub use mp3lame_encoder::{Bitrate, VbrMode, Quality, Id3Tag};

    pub struct Mp3Encoder {
        writer: Box<dyn Writer>,
        encoder: Encoder,
    }

    impl Mp3Encoder {
        fn new(writer: Box<dyn Writer>, channels: u8, sample_rate: u32, bit_rate: Option<Bitrate>, quality: Option<Quality>, vbr_mode: Option<VbrMode>, id3tag: Option<Id3Tag>) -> Result<Self, AudioWriteError> {
            let mp3_builder = Builder::new();
            let mut mp3_builder = match mp3_builder {
                Some(mp3_builder) => mp3_builder,
                None => return Err(AudioWriteError::OtherReason("`lame_init()` somehow failed.".to_owned())),
            };

            match channels {
                1 => mp3_builder.set_mode(Mode::Mono)?,
                2 => mp3_builder.set_mode(Mode::JointStereo)?,
                other => return Err(AudioWriteError::InvalidArguments(format!("Bad channel number: {}", other)))
            }

            mp3_builder.set_num_channels(channels)?;
            mp3_builder.set_sample_rate(sample_rate)?;

            // 设置位率，直接决定音质。如果没有提供位率，就使用 320 kbps 位率。
            mp3_builder.set_brate(match bit_rate {
                Some(bit_rate) => bit_rate,
                None => Bitrate::Kbps320,
            })?;

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

            // 创建编码器
            Ok(Self {
                writer,
                encoder: mp3_builder.build()?,
            })
        }
    }

    impl Debug for Mp3Encoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Encoder")
                .field("writer", &self.writer)
                .field("encoder", &format_args!("Encoder"))
                .finish()
        }
    }

}




