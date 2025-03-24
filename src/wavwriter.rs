use std::{any::TypeId, fs::File, {path::Path}, io::{self, Write, Seek, SeekFrom, BufWriter}, error::Error};

#[allow(unused_imports)]
pub use crate::errors::*;

#[allow(unused_imports)]
pub use crate::wavcore::*;

#[allow(unused_imports)]
pub use crate::audiocore::*;

use crate::readwrite::*;
use crate::sampleutils::*;

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
            packer__u8: Packer::<DynWriter,  u8>::new(&spec)?,
            packer_s16: Packer::<DynWriter, i16>::new(&spec)?,
            packer_s24: Packer::<DynWriter, i24>::new(&spec)?,
            packer_s32: Packer::<DynWriter, i32>::new(&spec)?,
            packer_f32: Packer::<DynWriter, f32>::new(&spec)?,
            packer_f64: Packer::<DynWriter, f64>::new(&spec)?,
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        use SampleFormat::{Int, UInt, Float};
        let mut writer = self.writer;
        writer.write_all(b"RIFF")?;
        self.riff_offset = writer.stream_position()?;
        0u32.write_le(&mut writer)?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
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
        } as u32).write_le(&mut writer)?;
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
        } as u16).write_le(&mut writer)?;
        (self.spec.channels as u16).write_le(&mut writer)?;
        (self.spec.sample_rate as u32).write_le(&mut writer)?;
        (self.spec.sample_rate * self.frame_size as u32).write_le(&mut writer)?;
        (self.frame_size as u16).write_le(&mut writer)?;
        (self.spec.bits_per_sample as u16).write_le(&mut writer)?;
        if ext == true {
            22u16.write_le(&mut writer)?; // 额外数据大小
            (self.spec.bits_per_sample as u16).write_le(&mut writer)?;
            (self.spec.channel_mask as u32).write_le(&mut writer)?; // 声道掩码

            // 写入具体格式的 GUID
            match self.spec.sample_format {
                Int => guid_pcm_format.write(&mut writer)?,
                Float => guid_ieee_float_format.write(&mut writer)?,
                _ => return Err(AudioWriteError::InvalidArguments.into()),
            }
        };
        writer.write_all(b"data")?;
        self.datalen_offset = writer.stream_position()?;
        0u32.write_le(&mut writer)?;
        self.data_offset = writer.stream_position()?;
        Ok(())
    }

    pub fn save_frame<S>(&self, frame: &Vec<S>, sample_type: WaveSampleType) -> Result<(), Box<dyn Error>>
    where S: SampleType {

        // 用户输入的样本格式是 S，而我们要存储的格式则是由我们的 spec 决定的。
        // 根据 spec 要求，我们需要转换为我们自己的格式，再保存到文件里。
        // 在这里，我们的不同类型的 packer 能把 S 转换为各自的类型再存储。
        // 但似乎光从泛型参数 S 来判断输入的格式不太合理，因为 Rust 编译器的能力有限，它会很轴，认为 S 一定不等于 u8，我是服了。
        //     if TypeId::of::<S>() == TypeId::of::<u8 >() { self.packer_u8 .save_sample(&mut self.writer, frame)?;}
        //else if TypeId::of::<S>() == TypeId::of::<i16>() { self.packer_s16.save_sample(&mut self.writer, frame)?;}
        //else if TypeId::of::<S>() == TypeId::of::<i24>() { self.packer_s24.save_sample(&mut self.writer, frame)?;}
        //else if TypeId::of::<S>() == TypeId::of::<i32>() { self.packer_s32.save_sample(&mut self.writer, frame)?;}
        //else if TypeId::of::<S>() == TypeId::of::<f32>() { self.packer_f32.save_sample(&mut self.writer, frame)?;}
        //else if TypeId::of::<S>() == TypeId::of::<f64>() { self.packer_f64.save_sample(&mut self.writer, frame)?;}
        //else { return Err(AudioWriteError::UnsupportedFormat.into);}

        use WaveSampleType::{U8, S16, S24, S32, F32, F64};
        match sample_type {
            U8  => self.packer__u8.save_sample(&mut self.writer, frame)?,
            S16 => self.packer_s16.save_sample(&mut self.writer, frame)?,
            S24 => self.packer_s24.save_sample(&mut self.writer, frame)?,
            S32 => self.packer_s32.save_sample(&mut self.writer, frame)?,
            F32 => self.packer_f32.save_sample(&mut self.writer, frame)?,
            F64 => self.packer_f64.save_sample(&mut self.writer, frame)?,
            _ => return Err(AudioWriteError::UnsupportedFormat.into),
        }

        self.num_frames += 1;
    }

    // 一旦调用，WaveWriter 就失去了对 DynWriter 的所有权
    pub fn get_writer(&self) -> DynWriter {
        self.writer
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
        const header_size: u32 = 44;
        let mut writer = self.writer;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        writer.seek(SeekFrom::Start(self.riff_offset))?;
        (header_size + all_sample_size - 8).write_le(&mut writer)?;
        writer.seek(SeekFrom::Start(self.datalen_offset))?;
        all_sample_size.write_le(&mut writer)?;
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
            sample.to_u8().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_i16(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i16().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_i24(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i24().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_i32(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i32().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_f32(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f32().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_f64(writer: &mut W, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f64().write_le(&mut writer)?;
        }
        Ok(())
    }
}
