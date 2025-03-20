use std::{fs::File, {path::Path}, io::{Write, BufWriter, Seek, SeekFrom}, error::Error};

use crate::structwrite::StructWrite;
use crate::sampleutils::SampleUtils;
use crate::audiocore::{SampleFormat::{Int, UInt, Float}, Spec, Frame};
use crate::audiowriter::{AudioWriter, AudioWriteError};

pub struct WaveWriter<W> {
    writer: StructWrite<W>,
    spec: Spec,
    num_frames: u32,
    frame_size: u16,
    riff_offset: u64,
    data_offset: u64,
    sample_offset: u64,
    packer: Box<dyn SamplePacker<W>>, // 快速样本转换器
}

impl<W> WaveWriter<W> where W: Write + Seek {
    pub fn new(writer: W, spec: &Spec) -> Result<WaveWriter<W>, Box<dyn Error>> {
        let mut writer = StructWrite::new(writer);
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        writer.write_bytes(b"RIFF")?;
        let riff_offset = writer.stream_position()?;
        writer.write_le_u32(0)?;
        writer.write_bytes(b"WAVE")?;
        writer.write_bytes(b"fmt ")?;
        // 如果格式类型是 0xFFFE 则需要单独对待
        let ext = match (spec.bits_per_sample, spec.sample_format) {
            (24, Int) | (32, Int) => true,
            _ => false
        };
        // fmt 块的大小
        writer.write_bytes(match ext {
            true => 40,
            false => 16,
        })?;
        writer.write_le_u16(match ext {
            true => 0xFFFE,
            false => {
                match (spec.bits_per_sample, spec.sample_format) {
                    (8, UInt) => 1,
                    (16, Int) => 1,
                    (32, Float) => 3,
                    (64, Float) => 3,
                    _ => return Err(AudioWriteError::UnsupportedFormat.into()),
                }
            }
        }?;
        writer.write_le_u16(spec.channels)?;
        writer.write_le_u32(spec.sample_rate)?;
        writer.write_le_u32(spec.sample_rate * frame_size as u32)?;
        writer.write_le_u16(frame_size)?;
        writer.write_le_u16(spec.bits_per_sample)?;
        if ext == true {
            writer.write_le_u16(22)?; // 额外数据大小
            writer.write_le_u16(spec.bits_per_sample)?;
            writer.write_le_u32(spec.channel_mask)?;

            // 写 GUID，这个 GUID 的意思是它是 PCM 数据哈哈哈
            writer.write_le_u32(0x00000001)?;
            writer.write_le_u16(0x0000)?;
            writer.write_le_u16(0x0010)?;
            writer.write_bytes(&[0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71])?
        }
        writer.write_bytes(b"data")?;
        let data_offset = writer.stream_position()?;
        writer.write_le_u32(0)?;
        let sample_offset = writer.stream_position()?;
        Ok(Self{
            writer,
            spec: spec.clone(),
            num_frames: 0,
            frame_size,
            riff_offset,
            data_offset,
            sample_offset,
            packer: match (spec.bits_per_sample, spec.sample_format) {
                (8, UInt) => Box::new(PackerU8{}),
                (16, Int) => Box::new(PackerS16{}),
                (24, Int) => Box::new(PackerS24{}),
                (32, Int) => Box::new(PackerS32{}),
                (32, Float) => Box::new(PackerF32{}),
                (64, Float) => Box::new(PackerF64{}),
                _ => return Err(AudioWriteError::UnsupportedFormat.into()),
            },
        })
    }
}

// 用指定写入文件的方式，自动套上 BufWriter 来提升读取效率
impl WaveWriter<BufWriter<File>> {
    pub fn open<P: AsRef<Path>>(filename: P, spec: &Spec) -> Result<WaveWriter<BufWriter<File>>, Box<dyn Error>> {
        let file = File::open(filename)?;
        let buf_writer = BufWriter::new(file);
        WaveWriter::new(buf_writer, spec)
    }
}

impl<W> AudioWriter for WaveWriter<W> where W: Write + Seek {
    fn spec(&self) -> Spec {
        self.spec.clone()
    }

    fn write(&mut self, frame: &Frame) -> Result<(), Box<dyn Error>> {
        self.packer.save_sample(&mut self.writer, frame)?;
        self.num_frames += 1;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        const header_size: u32 = 44;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        self.writer.seek(SeekFrom::Start(self.riff_offset))?;
        self.writer.write_le_u32(header_size + all_sample_size - 8)?;
        self.writer.seek(SeekFrom::Start(self.data_offset))?;
        self.writer.write_le_u32(all_sample_size)?;
        Ok(())
    }
}

trait SamplePacker<W> where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>>;
}

struct PackerU8;

impl<W> SamplePacker<W> for PackerU8 where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_le_u8(SampleUtils::f32_to_u8(sample))?;
        }
        Ok(())
    }
}

struct PackerS16;

impl<W> SamplePacker<W> for PackerS16 where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_le_s16(SampleUtils::f32_to_i16(sample))?;
        }
        Ok(())
    }
}


struct PackerS24;

impl<W> SamplePacker<W> for PackerS24 where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_bytes(SampleUtils::f32_to_i24(sample))?;
        }
        Ok(())
    }
}

struct PackerS32;

impl<W> SamplePacker<W> for PackerS32 where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_le_i32(SampleUtils::f32_to_i32(sample))?;
        }
        Ok(())
    }
}

struct PackerF32;

impl<W> SamplePacker<W> for PackerF32M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_le_f32(SampleUtils::f32_to_f32(sample))?;
        }
        Ok(())
    }
}

struct PackerF64;

impl<W> SamplePacker<W> for PackerF64M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        for sample in frame.iter() {
            writer.write_le_f64(SampleUtils::f32_to_f64(sample))?;
        }
        Ok(())
    }
}

