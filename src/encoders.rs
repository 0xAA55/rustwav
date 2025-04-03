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

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError>;
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

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        Ok(writer.flush()?)
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

    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> { Ok(writer.flush()?) }
}

#[cfg(feature = "mp3enc")]
pub mod MP3 {
    use std::{any::type_name, fmt::Debug, error::Error};
    use crate::Writer;
    use crate::{SampleType, i24, u24};
    use crate::AudioWriteError;
    use crate::EncoderToImpl;
    use mp3lame_encoder::{Builder, Encoder, Mode, EncoderInput, MonoPcm, DualPcm, FlushNoGap};
    pub use mp3lame_encoder::{Bitrate, VbrMode, Quality, Id3Tag};

    const MAX_SAMPLES_TO_ENCODE: usize = 1024;

    #[derive(PartialEq)]
    enum LastUsedBuf {
        NotUsed,
        MonoI16,
        MonoU16,
        MonoI32,
        MonoI64,
        MonoF32,
        MonoF64,
        StereoI16,
        StereoU16,
        StereoI32,
        StereoI64,
        StereoF32,
        StereoF64,
    }

    pub struct Mp3Encoder<'a> {
        encoder: Encoder,
        last_used_buf: LastUsedBuf,
        mono_buf_i16: EncoderBufMono<'a, i16>,
        mono_buf_u16: EncoderBufMono<'a, u16>,
        mono_buf_i32: EncoderBufMono<'a, i32>,
        mono_buf_i64: EncoderBufMono<'a, i64>,
        mono_buf_f32: EncoderBufMono<'a, f32>,
        mono_buf_f64: EncoderBufMono<'a, f64>,
        stereo_buf_i16: EncoderBufStereo<'a, i16>,
        stereo_buf_u16: EncoderBufStereo<'a, u16>,
        stereo_buf_i32: EncoderBufStereo<'a, i32>,
        stereo_buf_i64: EncoderBufStereo<'a, i64>,
        stereo_buf_f32: EncoderBufStereo<'a, f32>,
        stereo_buf_f64: EncoderBufStereo<'a, f64>,
    }

    impl<'a> Mp3Encoder<'a> {
        fn new(channels: u8, sample_rate: u32, bit_rate: Option<Bitrate>, quality: Option<Quality>, vbr_mode: Option<VbrMode>, id3tag: Option<Id3Tag>) -> Result<Self, AudioWriteError> {
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
                encoder: mp3_builder.build()?,
                last_used_buf: LastUsedBuf::NotUsed,
                mono_buf_i16: EncoderBufMono::<'a, i16>::new(),
                mono_buf_u16: EncoderBufMono::<'a, u16>::new(),
                mono_buf_i32: EncoderBufMono::<'a, i32>::new(),
                mono_buf_i64: EncoderBufMono::<'a, i64>::new(),
                mono_buf_f32: EncoderBufMono::<'a, f32>::new(),
                mono_buf_f64: EncoderBufMono::<'a, f64>::new(),
                stereo_buf_i16: EncoderBufStereo::<'a, i16>::new(),
                stereo_buf_u16: EncoderBufStereo::<'a, u16>::new(),
                stereo_buf_i32: EncoderBufStereo::<'a, i32>::new(),
                stereo_buf_i64: EncoderBufStereo::<'a, i64>::new(),
                stereo_buf_f32: EncoderBufStereo::<'a, f32>::new(),
                stereo_buf_f64: EncoderBufStereo::<'a, f64>::new(),
            })
        }

        pub fn flush_buffers(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
            use LastUsedBuf::{ NotUsed, MonoI16, MonoU16, MonoI32, MonoI64, MonoF32, MonoF64, StereoI16, StereoU16, StereoI32, StereoI64, StereoF32, StereoF64 };
            let ret = match self.last_used_buf {
                NotUsed => Ok(()),
                MonoI16 => self.mono_buf_i16.flush(&mut self.encoder, writer),
                MonoU16 => self.mono_buf_u16.flush(&mut self.encoder, writer),
                MonoI32 => self.mono_buf_i32.flush(&mut self.encoder, writer),
                MonoI64 => self.mono_buf_i64.flush(&mut self.encoder, writer),
                MonoF32 => self.mono_buf_f32.flush(&mut self.encoder, writer),
                MonoF64 => self.mono_buf_f64.flush(&mut self.encoder, writer),
                StereoI16 => self.stereo_buf_i16.flush(&mut self.encoder, writer),
                StereoU16 => self.stereo_buf_u16.flush(&mut self.encoder, writer),
                StereoI32 => self.stereo_buf_i32.flush(&mut self.encoder, writer),
                StereoI64 => self.stereo_buf_i64.flush(&mut self.encoder, writer),
                StereoF32 => self.stereo_buf_f32.flush(&mut self.encoder, writer),
                StereoF64 => self.stereo_buf_f64.flush(&mut self.encoder, writer),
            };
            self.last_used_buf = NotUsed;
            ret
        }

        pub fn encode_mono<S>(&mut self, writer: &mut dyn Writer, frame: S) -> Result<(), AudioWriteError>
        where S: SampleType {
            use LastUsedBuf::{ NotUsed, MonoI16, MonoU16, MonoI32, MonoI64, MonoF32, MonoF64, StereoI16, StereoU16, StereoI32, StereoI64, StereoF32, StereoF64 };

            // 判断当前要用的缓冲区类型
            let cur_buf = match type_name::<S>() {
                "i16" => MonoI16,
                "u16" => MonoU16,
                "i32" => MonoI32,
                "i64" => MonoI64,
                "f32" => MonoF32,
                "f64" => MonoF64,

                // 如果不是 encoder 支持的格式则选择一个看起来最合适的格式来做转换
                "i8"  => return self.encode_mono::<i16>(writer, <i16 as SampleType>::from(frame)),
                "u8"  => return self.encode_mono::<u16>(writer, <u16 as SampleType>::from(frame)),
                "i24" => return self.encode_mono::<i32>(writer, <i32 as SampleType>::from(frame)),
                "u24" => return self.encode_mono::<i32>(writer, <i32 as SampleType>::from(frame)),
                "u32" => return self.encode_mono::<i32>(writer, <i32 as SampleType>::from(frame)),
                "u64" => return self.encode_mono::<i64>(writer, <i64 as SampleType>::from(frame)),
            };
            if self.last_used_buf != cur_buf {
                self.flush_buffers(writer)?;
            }
            match cur_buf {
                MonoI16 => match self.mono_buf_i16.add_sample(<i16 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_i16.flush(&mut self.encoder, writer)?, }
                MonoU16 => match self.mono_buf_u16.add_sample(<u16 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_u16.flush(&mut self.encoder, writer)?, }
                MonoI32 => match self.mono_buf_i32.add_sample(<i32 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_i32.flush(&mut self.encoder, writer)?, }
                MonoI64 => match self.mono_buf_i64.add_sample(<i64 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_i64.flush(&mut self.encoder, writer)?, }
                MonoF32 => match self.mono_buf_f32.add_sample(<f32 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_f32.flush(&mut self.encoder, writer)?, }
                MonoF64 => match self.mono_buf_f64.add_sample(<f64 as SampleType>::from(frame)) {Ok(_) => (), Err(_) => self.mono_buf_f64.flush(&mut self.encoder, writer)?, }
            }
            self.last_used_buf = cur_buf;
            Ok(())
        }

        pub fn encode_stereo<S>(&mut self, writer: &mut dyn Writer, frames: (S, S)) -> Result<(), AudioWriteError>
        where S: SampleType {
            use LastUsedBuf::{ NotUsed, MonoI16, MonoU16, MonoI32, MonoI64, MonoF32, MonoF64, StereoI16, StereoU16, StereoI32, StereoI64, StereoF32, StereoF64 };
            let cur_buf = match type_name::<S>() {
                "i16" => StereoI16,
                "u16" => StereoU16,
                "i32" => StereoI32,
                "i64" => StereoI64,
                "f32" => StereoF32,
                "f64" => StereoF64,

                // 如果不是 encoder 支持的格式则选择一个看起来最合适的格式来做转换
                "i8"  => return self.encode_stereo::<i16>(writer, (<i16 as SampleType>::from(frames.0), <i16 as SampleType>::from(frames.1))),
                "u8"  => return self.encode_stereo::<u16>(writer, (<u16 as SampleType>::from(frames.0), <u16 as SampleType>::from(frames.1))),
                "i24" => return self.encode_stereo::<i32>(writer, (<i32 as SampleType>::from(frames.0), <i32 as SampleType>::from(frames.1))),
                "u24" => return self.encode_stereo::<i32>(writer, (<i32 as SampleType>::from(frames.0), <i32 as SampleType>::from(frames.1))),
                "u32" => return self.encode_stereo::<i32>(writer, (<i32 as SampleType>::from(frames.0), <i32 as SampleType>::from(frames.1))),
                "u64" => return self.encode_stereo::<i64>(writer, (<i64 as SampleType>::from(frames.0), <i64 as SampleType>::from(frames.1))),
            };
            if self.last_used_buf != cur_buf {
                self.flush_buffers(writer)?;
            }
            match cur_buf {
                StereoI16 => match self.stereo_buf_i16.add_sample((<i16 as SampleType>::from(frames.0), <i16 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_i16.flush(&mut self.encoder, writer)?, }
                StereoU16 => match self.stereo_buf_u16.add_sample((<u16 as SampleType>::from(frames.0), <u16 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_u16.flush(&mut self.encoder, writer)?, }
                StereoI32 => match self.stereo_buf_i32.add_sample((<i32 as SampleType>::from(frames.0), <i32 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_i32.flush(&mut self.encoder, writer)?, }
                StereoI64 => match self.stereo_buf_i64.add_sample((<i64 as SampleType>::from(frames.0), <i64 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_i64.flush(&mut self.encoder, writer)?, }
                StereoF32 => match self.stereo_buf_f32.add_sample((<f32 as SampleType>::from(frames.0), <f32 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_f32.flush(&mut self.encoder, writer)?, }
                StereoF64 => match self.stereo_buf_f64.add_sample((<f64 as SampleType>::from(frames.0), <f64 as SampleType>::from(frames.1))) {Ok(_) => (), Err(_) => self.stereo_buf_f64.flush(&mut self.encoder, writer)?, }
            }
            self.last_used_buf = cur_buf;
            Ok(())
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

        fn sample_conv_2<S, D>(frame: &[(S, S)]) -> Vec<(D, D)>
        where S: SampleType,
              D: SampleType {

            let mut ret = Vec::<(D, D)>::with_capacity(frame.len());
            for f in frame.iter() {
                let (l, r) = *f;
                ret.push((D::from(l), D::from(r)));
            }
            ret
        }

        pub fn encode_multiple_mono<S>(&mut self, writer: &mut dyn Writer, frames: &[S]) -> Result<(), AudioWriteError>
        where S: SampleType {
            use LastUsedBuf::{ NotUsed, MonoI16, MonoU16, MonoI32, MonoI64, MonoF32, MonoF64, StereoI16, StereoU16, StereoI32, StereoI64, StereoF32, StereoF64 };

            // 判断当前要用的缓冲区类型
            let cur_buf = match type_name::<S>() {
                "i16" => MonoI16,
                "u16" => MonoU16,
                "i32" => MonoI32,
                "i64" => MonoI64,
                "f32" => MonoF32,
                "f64" => MonoF64,

                // 如果不是 encoder 支持的格式则选择一个看起来最合适的格式来做转换
                "i8"  => return self.encode_multiple_mono::<i16>(writer, &frames.iter().map(|s| <i16 as SampleType>::from(*s)).collect::<Vec<i16>>()),
                "u8"  => return self.encode_multiple_mono::<u16>(writer, &frames.iter().map(|s| <u16 as SampleType>::from(*s)).collect::<Vec<u16>>()),
                "i24" => return self.encode_multiple_mono::<i32>(writer, &frames.iter().map(|s| <i32 as SampleType>::from(*s)).collect::<Vec<i32>>()),
                "u24" => return self.encode_multiple_mono::<i32>(writer, &frames.iter().map(|s| <i32 as SampleType>::from(*s)).collect::<Vec<i32>>()),
                "u32" => return self.encode_multiple_mono::<i32>(writer, &frames.iter().map(|s| <i32 as SampleType>::from(*s)).collect::<Vec<i32>>()),
                "u64" => return self.encode_multiple_mono::<i64>(writer, &frames.iter().map(|s| <i64 as SampleType>::from(*s)).collect::<Vec<i64>>()),
            };
            if self.last_used_buf != cur_buf {
                self.flush_buffers(writer)?;
            }
            match cur_buf {
                MonoI16 => { let mut to_save: Vec<i16> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_i16.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_i16.flush(&mut self.encoder, writer)?, }}}
                MonoU16 => { let mut to_save: Vec<u16> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_u16.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_u16.flush(&mut self.encoder, writer)?, }}}
                MonoI32 => { let mut to_save: Vec<i32> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_i32.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_i32.flush(&mut self.encoder, writer)?, }}}
                MonoI64 => { let mut to_save: Vec<i64> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_i64.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_i64.flush(&mut self.encoder, writer)?, }}}
                MonoF32 => { let mut to_save: Vec<f32> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_f32.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_f32.flush(&mut self.encoder, writer)?, }}}
                MonoF64 => { let mut to_save: Vec<f64> = Self::sample_conv(frames); while to_save.len() > 0 { match self.mono_buf_f64.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.mono_buf_f64.flush(&mut self.encoder, writer)?, }}}
            }
            self.last_used_buf = cur_buf;
            Ok(())
        }

        pub fn encode_multiple_stereo<S>(&mut self, writer: &mut dyn Writer, frames: &[(S, S)]) -> Result<(), AudioWriteError>
        where S: SampleType {
            use LastUsedBuf::{ NotUsed, MonoI16, MonoU16, MonoI32, MonoI64, MonoF32, MonoF64, StereoI16, StereoU16, StereoI32, StereoI64, StereoF32, StereoF64 };
            let cur_buf = match type_name::<S>() {
                "i16" => StereoI16,
                "u16" => StereoU16,
                "i32" => StereoI32,
                "i64" => StereoI64,
                "f32" => StereoF32,
                "f64" => StereoF64,

                // 如果不是 encoder 支持的格式则选择一个看起来最合适的格式来做转换
                "i8"  => return self.encode_multiple_stereo::<i16>(writer, &frames.iter().map(|(l, r)| (<i16 as SampleType>::from(*l), <i16 as SampleType>::from(*r))).collect::<Vec<(i16, i16)>>()),
                "u8"  => return self.encode_multiple_stereo::<u16>(writer, &frames.iter().map(|(l, r)| (<u16 as SampleType>::from(*l), <u16 as SampleType>::from(*r))).collect::<Vec<(u16, u16)>>()),
                "i24" => return self.encode_multiple_stereo::<i32>(writer, &frames.iter().map(|(l, r)| (<i32 as SampleType>::from(*l), <i32 as SampleType>::from(*r))).collect::<Vec<(i32, i32)>>()),
                "u24" => return self.encode_multiple_stereo::<i32>(writer, &frames.iter().map(|(l, r)| (<i32 as SampleType>::from(*l), <i32 as SampleType>::from(*r))).collect::<Vec<(i32, i32)>>()),
                "u32" => return self.encode_multiple_stereo::<i32>(writer, &frames.iter().map(|(l, r)| (<i32 as SampleType>::from(*l), <i32 as SampleType>::from(*r))).collect::<Vec<(i32, i32)>>()),
                "u64" => return self.encode_multiple_stereo::<i64>(writer, &frames.iter().map(|(l, r)| (<i64 as SampleType>::from(*l), <i64 as SampleType>::from(*r))).collect::<Vec<(i64, i64)>>()),
            };
            if self.last_used_buf != cur_buf {
                self.flush_buffers(writer)?;
            }
            match cur_buf {
                StereoI16 => { let mut to_save: Vec<(i16, i16)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_i16.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_i16.flush(&mut self.encoder, writer)?, }}}
                StereoU16 => { let mut to_save: Vec<(u16, u16)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_u16.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_u16.flush(&mut self.encoder, writer)?, }}}
                StereoI32 => { let mut to_save: Vec<(i32, i32)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_i32.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_i32.flush(&mut self.encoder, writer)?, }}}
                StereoI64 => { let mut to_save: Vec<(i64, i64)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_i64.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_i64.flush(&mut self.encoder, writer)?, }}}
                StereoF32 => { let mut to_save: Vec<(f32, f32)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_f32.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_f32.flush(&mut self.encoder, writer)?, }}}
                StereoF64 => { let mut to_save: Vec<(f64, f64)> = Self::sample_conv_2(frames); while to_save.len() > 0 { match self.stereo_buf_f64.add_multiple_sample(&to_save) {Ok(remain) => to_save = remain, Err(_) => self.stereo_buf_f64.flush(&mut self.encoder, writer)?, }}}
            }
            self.last_used_buf = cur_buf;
            Ok(())
        }

        pub fn write_frame<S>(&mut self, writer: &mut dyn Writer, frame: &Vec<S>) -> Result<(), AudioWriteError>
        where S: SampleType {
            match frame.len() {
                1 => self.encode_mono::<S>(writer, frame[0]),
                2 => self.encode_stereo::<S>(writer, (frame[0], frame[1])),
                other => return Err(AudioWriteError::InvalidArguments(format!("Bad channels to write: {} channels", other)))
            }
        }

        pub fn write_multiple_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
        where S: SampleType {
            let mut mono = Vec::<S>::new();
            let mut stereo = Vec::<(S, S)>::new();
            for frame in frames.iter() {
                match frame.len() {
                    1 => mono.push(frame[0]),
                    2 => stereo.push((frame[0], frame[1])),
                    other => return Err(AudioWriteError::InvalidArguments(format!("Bad channels to write: {} channels", other)))
                }
            }
            if mono.len() > 0 {
                self.encode_multiple_mono::<S>(writer, &mono)?;
            }
            if stereo.len() > 0 {
                self.encode_multiple_stereo::<S>(writer, &stereo)?;
            }
            Ok(())
        }
    }

    struct EncoderBufMono<'a, S>
    where S: SampleType {
        mono_pcm: MonoPcm<'a, S>,
        cur_samples: usize,
        max_samples: usize,
    }

    struct EncoderBufStereo<'a, S>
    where S: SampleType{
        dual_pcm: DualPcm<'a, S>,
        cur_samples: usize,
        max_samples: usize,
    }

    impl<'a, S> EncoderBufMono<'a, S>
    where S: SampleType , MonoPcm<'a, S>: EncoderInput{
        pub fn new() -> Self {
            Self {
                mono_pcm: MonoPcm(&[S::from(0); MAX_SAMPLES_TO_ENCODE]),
                cur_samples: 0,
                max_samples: MAX_SAMPLES_TO_ENCODE,
            }
        }

        pub fn add_sample(&mut self, frame: S) -> Result<(), AudioWriteError> {
            if self.cur_samples < self.max_samples {
                self.mono_pcm.0[self.cur_samples] = frame;
                self.cur_samples += 1;
                Ok(())
            } else {
                Err(AudioWriteError::BufferIsFull)
            }
        }

        // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
        pub fn add_multiple_sample(&mut self, frame: &[S]) -> Result<Vec<S>, AudioWriteError> {
            let size = frame.len();
            let space = self.max_samples - self.cur_samples;
            if space == 0 {
                Err(AudioWriteError::BufferIsFull) 
            } else {
                for i in 0..size {
                    self.mono_pcm.0[self.cur_samples] = frame[i];
                    self.cur_samples += 1;
                    if self.max_samples - self.cur_samples == 0 {
                        return Ok(frame[i + 1..].to_vec());
                    }
                }
                return Ok(Vec::<S>::new())
            }
        }

        pub fn flush(&mut self, encoder: &mut Encoder, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
            if self.cur_samples == 0 {
                return Ok(())
            }
            let to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples));
            encoder.encode_to_vec(self.mono_pcm, &mut to_save)?;
            writer.write_all(&to_save)?;
            self.cur_samples = 0;
            Ok(())
        }
    }

    impl<'a, S> EncoderBufStereo<'a, S>
    where S: SampleType, DualPcm<'a, S>: EncoderInput{
        pub fn new() -> Self {
            Self {
                dual_pcm: DualPcm{
                    left:  &[S::from(0); MAX_SAMPLES_TO_ENCODE],
                    right: &[S::from(0); MAX_SAMPLES_TO_ENCODE],
                },
                cur_samples: 0,
                max_samples: MAX_SAMPLES_TO_ENCODE,
            }
        }

        pub fn add_sample(&mut self, frame: (S, S)) -> Result<(), AudioWriteError> {
            if self.cur_samples < self.max_samples {
                self.dual_pcm.left [self.cur_samples] = frame.0;
                self.dual_pcm.right[self.cur_samples] = frame.1;
                self.cur_samples += 1;
                Ok(())
            } else {
                Err(AudioWriteError::BufferIsFull)
            }
        }

        // 将一批音频数据写入缓冲区，如果缓冲区已满就报错；否则一直填；如果数据足够填充到满；返回 Ok(剩下的数据)
        pub fn add_multiple_sample(&mut self, frame: &[(S, S)]) -> Result<Vec<(S, S)>, AudioWriteError> {
            let size = frame.len();
            let space = self.max_samples - self.cur_samples;
            if space == 0 {
                Err(AudioWriteError::BufferIsFull) 
            } else {
                for i in 0..size {
                    self.dual_pcm.left [self.cur_samples] = frame[i].0;
                    self.dual_pcm.right[self.cur_samples] = frame[i].1;
                    self.cur_samples += 1;
                    if self.max_samples - self.cur_samples == 0 {
                        return Ok(frame[i + 1..].to_vec());
                    }
                }
                return Ok(Vec::<(S, S)>::new())
            }
        }

        pub fn flush(&mut self, encoder: &mut Encoder, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
            if self.cur_samples == 0 {
                return Ok(())
            }
            let to_save = Vec::<u8>::with_capacity(mp3lame_encoder::max_required_buffer_size(self.max_samples));
            encoder.encode_to_vec(self.dual_pcm, &mut to_save)?;
            writer.write_all(&to_save)?;
            self.cur_samples = 0;
            Ok(())
        }
    }

    impl EncoderToImpl for Mp3Encoder<'_> {
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

        fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> { self.flush_buffers(writer); writer.flush() }
    }

    impl Debug for Mp3Encoder<'_> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Encoder<'_>")
                .field("writer", &self.writer)
                .field("encoder", &format_args!("Encoder"))
                .field("mono_buf_i16", &self.mono_buf_i16)
                .field("mono_buf_u16", &self.mono_buf_u16)
                .field("mono_buf_i32", &self.mono_buf_i32)
                .field("mono_buf_i64", &self.mono_buf_i64)
                .field("mono_buf_f32", &self.mono_buf_f32)
                .field("mono_buf_f64", &self.mono_buf_f64)
                .field("stereo_buf_i16", &self.stereo_buf_i16)
                .field("stereo_buf_u16", &self.stereo_buf_u16)
                .field("stereo_buf_i32", &self.stereo_buf_i32)
                .field("stereo_buf_i64", &self.stereo_buf_i64)
                .field("stereo_buf_f32", &self.stereo_buf_f32)
                .field("stereo_buf_f64", &self.stereo_buf_f64)
                .finish()
        }
    }

    impl<S> Debug for EncoderBufMono<'_, S>
    where S: SampleType{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct(&format!("EncoderBufMono<'_, {}>", type_name::<S>()))
                .field("mono_pcm", &format_args!("mono_pcm"))
                .field("cur_samples", &self.cur_samples)
                .field("max_samples", &self.max_samples)
                .finish()
        }
    }

    impl<S> Debug for EncoderBufStereo<'_, S>
    where S: SampleType{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct(&format!("EncoderBufStereo<'_, {}>", type_name::<S>()))
                .field("dual_pcm", &format_args!("dual_pcm"))
                .field("cur_samples", &self.cur_samples)
                .field("max_samples", &self.max_samples)
                .finish()
        }
    }

}




