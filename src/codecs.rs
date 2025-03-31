#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, io::BufReader, fmt::Debug};

use crate::wavcore::*;
use crate::readwrite::*;
use crate::adpcm::*;

// 解码器，解码出来的样本格式是 S
pub trait Decoder<S>: Debug
    where S: SampleType {
    fn decode(&mut self) -> Result<S, io::Error>;
}

// 编码器，接收样本格式 S，编码为文件要的格式
// 因为 trait 不准用泛型参数，所以每一种函数都给我实现一遍。
pub trait EncoderBasic: Debug {
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frames: &Vec<i8 >) -> Result<(), Box<dyn Error>>;
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frames: &Vec<i16>) -> Result<(), Box<dyn Error>>;
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frames: &Vec<i24>) -> Result<(), Box<dyn Error>>;
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frames: &Vec<i32>) -> Result<(), Box<dyn Error>>;
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frames: &Vec<i64>) -> Result<(), Box<dyn Error>>;
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frames: &Vec<u8 >) -> Result<(), Box<dyn Error>>;
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frames: &Vec<u16>) -> Result<(), Box<dyn Error>>;
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frames: &Vec<u24>) -> Result<(), Box<dyn Error>>;
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frames: &Vec<u32>) -> Result<(), Box<dyn Error>>;
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frames: &Vec<u64>) -> Result<(), Box<dyn Error>>;
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frames: &Vec<f32>) -> Result<(), Box<dyn Error>>;
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frames: &Vec<f64>) -> Result<(), Box<dyn Error>>;

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), Box<dyn Error>>;
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]
pub struct Encoder {
    encoder: Box<dyn EncoderBasic>,
}

impl Encoder {
    pub fn new(encoder: Box<dyn EncoderBasic>) -> Self {
        Self {
            encoder,
        }
    }

    pub fn write_frame<S>(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_frame__i8(writer, &Self::generic_type_conv(frame)),
            "i16" => self.encoder.write_frame_i16(writer, &Self::generic_type_conv(frame)),
            "i24" => self.encoder.write_frame_i24(writer, &Self::generic_type_conv(frame)),
            "i32" => self.encoder.write_frame_i32(writer, &Self::generic_type_conv(frame)),
            "i64" => self.encoder.write_frame_i64(writer, &Self::generic_type_conv(frame)),
            "u8"  => self.encoder.write_frame__u8(writer, &Self::generic_type_conv(frame)),
            "u16" => self.encoder.write_frame_u16(writer, &Self::generic_type_conv(frame)),
            "u24" => self.encoder.write_frame_u24(writer, &Self::generic_type_conv(frame)),
            "u32" => self.encoder.write_frame_u32(writer, &Self::generic_type_conv(frame)),
            "u64" => self.encoder.write_frame_u64(writer, &Self::generic_type_conv(frame)),
            "f32" => self.encoder.write_frame_f32(writer, &Self::generic_type_conv(frame)),
            "f64" => self.encoder.write_frame_f64(writer, &Self::generic_type_conv(frame)),
            other => Err(AudioWriteError::WrongSampleFormat(other.to_owned()).into()),
        }
    }

    pub fn write_multiple_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.encoder.write_multiple_frames__i8(writer, &Self::generics_type_conv(frames)),
            "i16" => self.encoder.write_multiple_frames_i16(writer, &Self::generics_type_conv(frames)),
            "i24" => self.encoder.write_multiple_frames_i24(writer, &Self::generics_type_conv(frames)),
            "i32" => self.encoder.write_multiple_frames_i32(writer, &Self::generics_type_conv(frames)),
            "i64" => self.encoder.write_multiple_frames_i64(writer, &Self::generics_type_conv(frames)),
            "u8"  => self.encoder.write_multiple_frames__u8(writer, &Self::generics_type_conv(frames)),
            "u16" => self.encoder.write_multiple_frames_u16(writer, &Self::generics_type_conv(frames)),
            "u24" => self.encoder.write_multiple_frames_u24(writer, &Self::generics_type_conv(frames)),
            "u32" => self.encoder.write_multiple_frames_u32(writer, &Self::generics_type_conv(frames)),
            "u64" => self.encoder.write_multiple_frames_u64(writer, &Self::generics_type_conv(frames)),
            "f32" => self.encoder.write_multiple_frames_f32(writer, &Self::generics_type_conv(frames)),
            "f64" => self.encoder.write_multiple_frames_f64(writer, &Self::generics_type_conv(frames)),
            other => Err(AudioWriteError::WrongSampleFormat(other.to_owned()).into()),
        }
    }
}

fn sample_conv<S, D>(frame: &[S]) -> Vec<D>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<D>::with_capacity(frame.len());
    for f in frame.iter() {
        ret.push(D::from(*f));
    }
    ret
}

fn sample_conv_batch<S, D>(frames: &[Vec<S>]) -> Vec<Vec<D>>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<Vec<D>>::with_capacity(frames.len());
    for f in frames.iter() {
        ret.push(sample_conv::<S, D>(f));
    }
    ret
}

impl EncoderBasic for PcmEncoder {
    fn write_frame__i8(&mut self, writer: &mut dyn Writer, frame: &Vec<i8 >) -> Result<(), Box<dyn Error>> {self.writer_from__i8.write_frame(writer, frame)}
    fn write_frame_i16(&mut self, writer: &mut dyn Writer, frame: &Vec<i16>) -> Result<(), Box<dyn Error>> {self.writer_from_i16.write_frame(writer, frame)}
    fn write_frame_i24(&mut self, writer: &mut dyn Writer, frame: &Vec<i24>) -> Result<(), Box<dyn Error>> {self.writer_from_i24.write_frame(writer, frame)}
    fn write_frame_i32(&mut self, writer: &mut dyn Writer, frame: &Vec<i32>) -> Result<(), Box<dyn Error>> {self.writer_from_i32.write_frame(writer, frame)}
    fn write_frame_i64(&mut self, writer: &mut dyn Writer, frame: &Vec<i64>) -> Result<(), Box<dyn Error>> {self.writer_from_i64.write_frame(writer, frame)}
    fn write_frame__u8(&mut self, writer: &mut dyn Writer, frame: &Vec<u8 >) -> Result<(), Box<dyn Error>> {self.writer_from__u8.write_frame(writer, frame)}
    fn write_frame_u16(&mut self, writer: &mut dyn Writer, frame: &Vec<u16>) -> Result<(), Box<dyn Error>> {self.writer_from_u16.write_frame(writer, frame)}
    fn write_frame_u24(&mut self, writer: &mut dyn Writer, frame: &Vec<u24>) -> Result<(), Box<dyn Error>> {self.writer_from_u24.write_frame(writer, frame)}
    fn write_frame_u32(&mut self, writer: &mut dyn Writer, frame: &Vec<u32>) -> Result<(), Box<dyn Error>> {self.writer_from_u32.write_frame(writer, frame)}
    fn write_frame_u64(&mut self, writer: &mut dyn Writer, frame: &Vec<u64>) -> Result<(), Box<dyn Error>> {self.writer_from_u64.write_frame(writer, frame)}
    fn write_frame_f32(&mut self, writer: &mut dyn Writer, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {self.writer_from_f32.write_frame(writer, frame)}
    fn write_frame_f64(&mut self, writer: &mut dyn Writer, frame: &Vec<f64>) -> Result<(), Box<dyn Error>> {self.writer_from_f64.write_frame(writer, frame)}

    fn write_multiple_frames__i8(&mut self, writer: &mut dyn Writer, frames: &[Vec<i8 >]) -> Result<(), Box<dyn Error>> {self.writer_from__i8.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i16(&mut self, writer: &mut dyn Writer, frames: &[Vec<i16>]) -> Result<(), Box<dyn Error>> {self.writer_from_i16.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i24(&mut self, writer: &mut dyn Writer, frames: &[Vec<i24>]) -> Result<(), Box<dyn Error>> {self.writer_from_i24.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i32(&mut self, writer: &mut dyn Writer, frames: &[Vec<i32>]) -> Result<(), Box<dyn Error>> {self.writer_from_i32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_i64(&mut self, writer: &mut dyn Writer, frames: &[Vec<i64>]) -> Result<(), Box<dyn Error>> {self.writer_from_i64.write_multiple_frames(writer, frames)}
    fn write_multiple_frames__u8(&mut self, writer: &mut dyn Writer, frames: &[Vec<u8 >]) -> Result<(), Box<dyn Error>> {self.writer_from__u8.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u16(&mut self, writer: &mut dyn Writer, frames: &[Vec<u16>]) -> Result<(), Box<dyn Error>> {self.writer_from_u16.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u24(&mut self, writer: &mut dyn Writer, frames: &[Vec<u24>]) -> Result<(), Box<dyn Error>> {self.writer_from_u24.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u32(&mut self, writer: &mut dyn Writer, frames: &[Vec<u32>]) -> Result<(), Box<dyn Error>> {self.writer_from_u32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_u64(&mut self, writer: &mut dyn Writer, frames: &[Vec<u64>]) -> Result<(), Box<dyn Error>> {self.writer_from_u64.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_f32(&mut self, writer: &mut dyn Writer, frames: &[Vec<f32>]) -> Result<(), Box<dyn Error>> {self.writer_from_f32.write_multiple_frames(writer, frames)}
    fn write_multiple_frames_f64(&mut self, writer: &mut dyn Writer, frames: &[Vec<f64>]) -> Result<(), Box<dyn Error>> {self.writer_from_f64.write_multiple_frames(writer, frames)}
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    frame_size: u16,
    spec: Spec,
    decoder: fn(&mut dyn Reader) -> Result<S, io::Error>,
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn decode(&mut self) -> Result<S, io::Error>
    where S: SampleType {
        self.decode()
    }
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &fmt__Chunk) -> Result<Self, Box<dyn Error>> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioReadError::Unimplemented(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other)).into()),
        }
        let wave_sample_type = get_sample_type(spec.bits_per_sample, spec.sample_format)?;
        Ok(Self {
            reader,
            data_offset,
            data_length,
            frame_size: wave_sample_type.sizeof() * spec.channels,
            spec: spec.clone(),
            decoder: Self::get_decoder(wave_sample_type)?,
        })
    }

    pub fn decode(&mut self) -> Result<S, io::Error> {
        (self.decoder)(&mut self.reader)
    }

    fn decode_to<T>(r: &mut dyn Reader) -> Result<S, io::Error>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }

    fn get_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader) -> Result<S, io::Error>, Box<dyn Error>> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::decode_to::<i8 >),
            S16 => Ok(Self::decode_to::<i16>),
            S24 => Ok(Self::decode_to::<i24>),
            S32 => Ok(Self::decode_to::<i32>),
            S64 => Ok(Self::decode_to::<i64>),
            U8 =>  Ok(Self::decode_to::<u8 >),
            U16 => Ok(Self::decode_to::<u16>),
            U24 => Ok(Self::decode_to::<u24>),
            U32 => Ok(Self::decode_to::<u32>),
            U64 => Ok(Self::decode_to::<u64>),
            F32 => Ok(Self::decode_to::<f32>),
            F64 => Ok(Self::decode_to::<f64>),
            Unknown => return Err(AudioError::UnknownSampleType.into()),
        }
    }
}

// PcmEncoderFrom<S>：样本从 S 类型打包到目标类型
#[derive(Debug)]
pub struct PcmEncoderFrom<S>
where S: SampleType {
    target_sample: WaveSampleType,
    writer: fn(&mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>,
}

impl<S> PcmEncoderFrom<S>
where S: SampleType {
    pub fn new(target_sample: WaveSampleType) -> Self {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        Self {
            target_sample,
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
                _ => Self::fake_write_sample,
            },
        }
    }

    pub fn get_target_sample(&self) -> WaveSampleType {
        self.target_sample
    }

    // S：别人给我们的格式
    // T：我们要写入到 WAV 中的格式
    fn write_sample_to<T>(writer: &mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        for sample in frame.iter() {
            T::from(*sample).write_le(writer)?;
        }
        Ok(())
    }

    fn fake_write_sample(_writer: &mut dyn Writer, _frame: &[S]) -> Result<(), Box<dyn Error>> {
        Err(AudioWriteError::WrongSampleFormat("Unknown".to_owned()).into())
    }

    pub fn write_frame(&mut self, writer: &mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>> {
        (self.writer)(writer, frame)
    }

    pub fn write_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>]) -> Result<(), Box<dyn Error>> {
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
    pub fn new(target_sample: WaveSampleType) -> Self {
        Self {
            writer_from__i8: PcmEncoderFrom::< i8>::new(target_sample),
            writer_from_i16: PcmEncoderFrom::<i16>::new(target_sample),
            writer_from_i24: PcmEncoderFrom::<i24>::new(target_sample),
            writer_from_i32: PcmEncoderFrom::<i32>::new(target_sample),
            writer_from_i64: PcmEncoderFrom::<i64>::new(target_sample),
            writer_from__u8: PcmEncoderFrom::< u8>::new(target_sample),
            writer_from_u16: PcmEncoderFrom::<u16>::new(target_sample),
            writer_from_u24: PcmEncoderFrom::<u24>::new(target_sample),
            writer_from_u32: PcmEncoderFrom::<u32>::new(target_sample),
            writer_from_u64: PcmEncoderFrom::<u64>::new(target_sample),
            writer_from_f32: PcmEncoderFrom::<f32>::new(target_sample),
            writer_from_f64: PcmEncoderFrom::<f64>::new(target_sample),
        }
    }
}






