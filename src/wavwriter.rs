#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{any::TypeId, fs::File, {path::Path}, io::{self, Write, Seek, SeekFrom, BufWriter}, error::Error};

use crate::errors::{AudioWriteError};
use crate::wavcore::*;

pub struct WaveWriter {
    writer: DynWriter,
    spec: Spec,
    num_frames: u32,
    frame_size: u16,
    riff_offset: u64,
    datalen_offset: u64,
    data_offset: u64,
    packer__u8: Packer<DynWriter,  u8>,
    packer_s16: Packer<DynWriter, i16>,
    packer_s24: Packer<DynWriter, i24>,
    packer_s32: Packer<DynWriter, i32>,
    packer_f32: Packer<DynWriter, f32>,
    packer_f64: Packer<DynWriter, f64>,
    typeid__u8: TypeId,
    typeid_s16: TypeId,
    typeid_s24: TypeId,
    typeid_s32: TypeId,
    typeid_f32: TypeId,
    typeid_f64: TypeId,
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec) -> Result<WaveWriter, Box<dyn Error>> {
        let writer = Box::new(BufWriter::new(File::create(filename)?));
        Self::new(writer, spec)
    }

    pub fn new(writer: Box<dyn Writer>, spec: &Spec) -> Result<WaveWriter, Box<dyn Error>> {
        let spec = spec.clone();
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let mut ret = Self{
            writer: DynWriter::new(writer),
            spec,
            num_frames: 0,
            frame_size,
            riff_offset: 0,
            datalen_offset: 0,
            data_offset: 0,
            packer__u8: Packer::<DynWriter,  u8>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            packer_s16: Packer::<DynWriter, i16>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            packer_s24: Packer::<DynWriter, i24>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            packer_s32: Packer::<DynWriter, i32>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            packer_f32: Packer::<DynWriter, f32>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            packer_f64: Packer::<DynWriter, f64>::new(&spec)?, // 所有这些东西都是接收泛型参数的类型再根据 spec 的要求存储样本
            typeid__u8: TypeId::of::<u8 >(),
            typeid_s16: TypeId::of::<i16>(),
            typeid_s24: TypeId::of::<i24>(),
            typeid_s32: TypeId::of::<i32>(),
            typeid_f32: TypeId::of::<f32>(),
            typeid_f64: TypeId::of::<f64>(),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        use SampleFormat::{Int, UInt, Float};
        self.writer.write_all(b"RIFF")?;
        self.riff_offset = self.writer.stream_position()?;
        0u32.write_le(&mut self.writer)?;
        self.writer.write_all(b"WAVE")?;
        self.writer.write_all(b"fmt ")?;
        // 如果格式类型是 0xFFFE 则需要单独对待
        let mut ext = match (self.spec.bits_per_sample, self.spec.sample_format) {
            (24, Int) | (32, Int) => true,
            _ => false
        };
        // 如果有针对声道的特殊要求，则需要扩展数据
        ext |= match self.spec.channels {
            1 => {
                if self.spec.channel_mask != SpeakerPosition::FrontCenter as u32 {
                    true
                } else {
                    false
                }
            },
            2 => {
                if self.spec.channel_mask != SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32 {
                    true
                } else {
                    false
                }
            },
            _ => true, // 否则就需要额外的数据了
        };
        // fmt 块的大小
        (match ext {
            true => 40,
            false => 16,
        } as u32).write_le(&mut self.writer)?;
        (match ext {
            true => 0xFFFE,
            false => {
                match (self.spec.bits_per_sample, self.spec.sample_format) {
                    (8, UInt) => 1,
                    (16, Int) => 1,
                    (32, Float) => 3,
                    (64, Float) => 3,
                    _ => return Err(AudioWriteError::UnsupportedFormat.into()),
                }
            }
        } as u16).write_le(&mut self.writer)?;
        (self.spec.channels as u16).write_le(&mut self.writer)?;
        (self.spec.sample_rate as u32).write_le(&mut self.writer)?;
        (self.spec.sample_rate * self.frame_size as u32).write_le(&mut self.writer)?;
        (self.frame_size as u16).write_le(&mut self.writer)?;
        (self.spec.bits_per_sample as u16).write_le(&mut self.writer)?;
        if ext == true {
            22u16.write_le(&mut self.writer)?; // 额外数据大小
            (self.spec.bits_per_sample as u16).write_le(&mut self.writer)?;
            (self.spec.channel_mask as u32).write_le(&mut self.writer)?; // 声道掩码

            // 写入具体格式的 GUID
            match self.spec.sample_format {
                Int => GUID_PCM_FORMAT.write(&mut self.writer)?,
                Float => GUID_IEEE_FLOAT_FORMAT.write(&mut self.writer)?,
                _ => return Err(AudioWriteError::InvalidArguments.into()),
            }
        };
        self.writer.write_all(b"data")?;
        self.datalen_offset = self.writer.stream_position()?;
        0u32.write_le(&mut self.writer)?;
        self.data_offset = self.writer.stream_position()?;
        Ok(())
    }

    // 将一种格式的样本转换为另一种格式的样本。
    // 这个代码的实际作用是忽悠编译器。
    fn frame_conv<T1, T2>(frame: &Vec<T1>) -> Vec<T2>
    where T1: SampleType,
          T2: SampleType {
        let mut ret = Vec::<T2>::with_capacity(frame.len());
        for sample in frame.iter() {
            ret.push(T2::from(sample.clone()));
        }
        ret
    }

    pub fn save_frame<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType + Clone {

        // 用户输入的样本格式是 S，而我们要存储的格式则是由我们的 spec 决定的。
        // 根据 spec 要求，我们需要转换为我们自己的格式，再保存到文件里。
        // 在这里，我们的不同类型的 packer 能把 S 转换为各自的类型再存储。
        // 但似乎光从泛型参数 S 来判断输入的格式不太合理，因为 Rust 编译器的能力有限，它会很轴，认为 S 一定不等于 u8，我是服了。
        if TypeId::of::<S>() == self.typeid__u8 {self.packer__u8.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        if TypeId::of::<S>() == self.typeid_s16 {self.packer_s16.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        if TypeId::of::<S>() == self.typeid_s24 {self.packer_s24.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        if TypeId::of::<S>() == self.typeid_s32 {self.packer_s32.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        if TypeId::of::<S>() == self.typeid_f32 {self.packer_f32.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        if TypeId::of::<S>() == self.typeid_f64 {self.packer_f64.save_sample(&mut self.writer, &Self::frame_conv(frame))?;} else
        {return Err(AudioWriteError::UnsupportedFormat.into());}

        self.num_frames += 1;
        Ok(())
    }

    pub fn spec(&self) -> &Spec{
        &self.spec
    }
    pub fn get_num_frames(&self) -> u32 {
        self.num_frames
    }
    pub fn get_frame_size(&self) -> u16 {
        self.frame_size
    }
    pub fn get_data_offset(&self) -> u64 {
        self.data_offset
    }

    pub fn update_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        const HEADER_SIZE: u32 = 44;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        self.writer.seek(SeekFrom::Start(self.riff_offset))?;
        (HEADER_SIZE + all_sample_size - 8).write_le(&mut self.writer)?;
        self.writer.seek(SeekFrom::Start(self.datalen_offset))?;
        all_sample_size.write_le(&mut self.writer)?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        self.update_header()
    }
}

struct Packer<W, S>
where W: Writer,
      S: SampleType {
    save_sample_func: fn(&mut W, &Vec<S>) -> Result<(), io::Error>,
}

impl<W, S> Packer<W, S>
where W: Writer,
      S: SampleType {

    // 根据自己的音频格式，挑选合适的函数指针来写入正确的样本类型。
    pub fn new(writer_spec: &Spec) -> Result<Self, Box<dyn Error>> {
        use WaveSampleType::{U8, S16, S24, S32, F32, F64};
        Ok(Self {
            save_sample_func: match get_sample_type(writer_spec.bits_per_sample, writer_spec.sample_format)? {
                U8  => Self::save__u8,
                S16 => Self::save_i16,
                S24 => Self::save_i24,
                S32 => Self::save_i32,
                F32 => Self::save_f32,
                F64 => Self::save_f64,
            }
        })
    }

    pub fn save_sample(&self, writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        (self.save_sample_func)(writer, frame)
    }

    fn save__u8(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_u8().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i16(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i16().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i24(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i24().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i32(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i32().write_le(writer)?;
        }
        Ok(())
    }

    fn save_f32(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f32().write_le(writer)?;
        }
        Ok(())
    }

    fn save_f64(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f64().write_le(writer)?;
        }
        Ok(())
    }
}
