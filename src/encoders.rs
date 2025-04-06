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

    fn get_bit_rate(&mut self) -> u32;
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError>;
}

// 提供默认实现。无论用户输入的是什么格式，默认用 f32 传递给编码器。
impl EncoderToImpl for () {
    // 这个方法用户必须实现
    fn write_frame_f32(&mut self, _writer: &mut dyn Writer, _frame: &Vec<f32>) -> Result<(), AudioWriteError> {
        panic!("Must implement `write_frame_f32()` for your encoder to get samples.");
    }

    fn get_bit_rate(&mut self) -> u32 {
        panic!("Must implement `get_bit_rate()` for your encoder.");
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

    fn get_bit_rate(&mut self) -> u32 {
        self.channels as u32 * self.sample_rate * self.sample_type.sizeof() as u32 * 8
    }
    fn finalize(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        Ok(writer.flush()?)
    }
}

const ADPCM_ENCODE_BUFFER: usize = 128;

#[derive(Clone)]
pub struct AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    sample_rate: u32,
    samples_written: u64,
    bytes_written: u64,
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
            .field("encoder_l", &self.encoder_l)
            .field("encoder_r", &self.encoder_r)
            .field("buffer_l", &format_args!("[{}u8;...]", self.buffer_l.len()))
            .field("buffer_r", &format_args!("[{}u8;...]", self.buffer_r.len()))
            .finish()
    }
}

impl<E> AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples_written: 0,
            bytes_written: 0,
            encoder_l: E::new(),
            encoder_r: E::new(),
            buffer_l: Vec::<u8>::with_capacity(ADPCM_ENCODE_BUFFER),
            buffer_r: Vec::<u8>::with_capacity(ADPCM_ENCODE_BUFFER),
        }
    }

    fn flush_stereo(&mut self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let mut interleaved = Vec::<u8>::with_capacity(ADPCM_ENCODE_BUFFER * 2);
        let min_len = cmp::min(self.buffer_l.len(), self.buffer_r.len());
        if min_len > 0 {
            for i in 0..min_len {
                interleaved.push(self.buffer_l[i]);
                interleaved.push(self.buffer_r[i]);
            }
            writer.write_all(&interleaved)?;
            self.bytes_written += interleaved.len() as u64;
            if self.buffer_l.len() > min_len {
                let byte = *self.buffer_l.last().unwrap();
                self.buffer_l.clear();
                self.buffer_l.push(byte);
            } else {
                self.buffer_l.clear();
            }
            if self.buffer_r.len() > min_len {
                let byte = *self.buffer_r.last().unwrap();
                self.buffer_r.clear();
                self.buffer_r.push(byte);
            } else {
                self.buffer_r.clear();
            }
        } else {
            self.buffer_l.clear();
            self.buffer_r.clear();
        }
        Ok(())
    }

    pub fn write_frame(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError> {
        let mut sample_sent_l = false;
        let mut sample_sent_r = false;
        let feeder_l = || -> Option<i16> {
            if sample_sent_l == false {
                sample_sent_l = true;
                Some(frame[0])
            } else {
                None
            }
        };
        let feeder_r = || -> Option<i16> {
            if sample_sent_r == false {
                sample_sent_r = true;
                Some(frame[1])
            } else {
                None
            }
        };
        let receiver_l = |byte: u8|{ self.buffer_l.push(byte); };
        let receiver_r = |byte: u8|{ self.buffer_r.push(byte); };
        match frame.len() {
            1 => {
                self.encoder_l.encode(feeder_l, receiver_l)?;
                self.samples_written += 1;

                writer.write_all(&self.buffer_l)?;
                self.bytes_written += self.buffer_l.len() as u64;
                self.buffer_l.clear();

                Ok(())
            },
            2 => {
                self.encoder_l.encode(feeder_l, receiver_l)?;
                self.encoder_r.encode(feeder_r, receiver_r)?;
                self.samples_written += 1;
                self.samples_written += 1;

                self.flush_stereo(writer)?;
                
                Ok(())
            }
            other => Err(AudioWriteError::InvalidArguments(format!("Wrong channel number {other}"))),
        }
    }

    pub fn write_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {
        let (lv, rv) = stereo_to_dual_mono(frames)?;
        let (mut li, mut ri) = (lv.iter(), rv.iter());
        let feeder_l = || -> Option<i16> { li.next().copied() };
        let feeder_r = || -> Option<i16> { ri.next().copied() };
        let receiver_l = |byte: u8|{ self.buffer_l.push(byte); };
        let receiver_r = |byte: u8|{ self.buffer_r.push(byte); };
        match (lv.len() > 0, rv.len() > 0) {
            (true, false) => {
                self.encoder_l.encode(feeder_l, receiver_l)?;
                self.samples_written += lv.len() as u64;
                writer.write_all(&self.buffer_l)?;
                self.bytes_written += self.buffer_l.len() as u64;
                self.buffer_l.clear();
                Ok(())
            },
            (true, true) => {
                self.encoder_l.encode(feeder_l, receiver_l)?;
                self.encoder_r.encode(feeder_r, receiver_r)?;
                self.samples_written += lv.len() as u64;
                self.samples_written += rv.len() as u64;
                self.flush_stereo(writer)?;
                Ok(())
            },
            _ => panic!("The developer must have changed the behavior of the function `stereo_to_dual_mono()`, it should only feed the `left` channel for mono."),
        }
    }
}

impl<E> EncoderToImpl for AdpcmEncoderWrap<E>
where E: adpcm::AdpcmEncoder {
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &Vec<f32>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), AudioWriteError> {self.write_frame(writer, &sample_conv(frame))}

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), AudioWriteError> {self.write_multiple_frames(writer, &sample_conv_batch(frames))}

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
                bitrate: bitrate as u32 * 1000,
                encoder: encoder.clone(),
                buffers: match channels {
                    1 => ChannelBuffers::<S>::Mono(BufferMono::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
                    2 => ChannelBuffers::<S>::Stereo(BufferStereo::<S>::new(encoder.clone(), MAX_SAMPLES_TO_ENCODE)),
                    other => return Err(AudioWriteError::InvalidArguments(format!("Bad channel number: {}", other))),
                },
            })
        }

        pub fn write_frame<T>(&mut self, writer: &mut dyn Writer, frame: &[T]) -> Result<(), AudioWriteError>
        where T: SampleType {
            if self.buffers.is_full() {
                self.buffers.flush(writer)?;
            }
            match frame.len() {
                1 => self.buffers.add_sample_m(S::from(frame[0])),
                2 => self.buffers.add_sample_s((S::from(frame[0]), S::from(frame[1]))),
                other => return Err(AudioWriteError::InvalidArguments(format!("Bad frame channels: {}", other)))
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






