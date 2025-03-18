use std::{fs::File, {path::Path}, io::{Write, BufWriter, Seek, SeekFrom}, error::Error};

use crate::structwrite::StructWrite;
use crate::sampleutils::SampleUtils;
use crate::audiocore::{SampleFormat, Spec, Frame};
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
        writer.write_le_u32(16)?;
        writer.write_le_i16(match (spec.bits_per_sample, spec.sample_format) {
            (8, SampleFormat::Int) => 1,
            (16, SampleFormat::Int) => 1,
            (24, SampleFormat::Int) => -2,
            (32, SampleFormat::Int) => -2,
            (32, SampleFormat::Float) => 3,
            (64, SampleFormat::Float) => 3,
            _ => return Err(AudioWriteError::UnsupportedFormat.into()),
        })?;
        writer.write_le_u16(spec.channels)?;
        writer.write_le_u32(spec.sample_rate)?;
        writer.write_le_u32(spec.sample_rate * frame_size as u32)?;
        writer.write_le_u16(frame_size)?;
        writer.write_le_u16(spec.bits_per_sample)?;
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
            packer: match (spec.bits_per_sample, spec.channels, spec.sample_format) {
                (8, 1, SampleFormat::Int) => Box::new(PackerU8M{}),
                (8, 2, SampleFormat::Int) => Box::new(PackerU8S{}),
                (16, 1, SampleFormat::Int) => Box::new(PackerS16M{}),
                (16, 2, SampleFormat::Int) => Box::new(PackerS16S{}),
                (24, 1, SampleFormat::Int) => Box::new(PackerS24M{}),
                (24, 2, SampleFormat::Int) => Box::new(PackerS24S{}),
                (32, 1, SampleFormat::Int) => Box::new(PackerS32M{}),
                (32, 2, SampleFormat::Int) => Box::new(PackerS32S{}),
                (32, 1, SampleFormat::Float) => Box::new(PackerF32M{}),
                (32, 2, SampleFormat::Float) => Box::new(PackerF32S{}),
                (64, 1, SampleFormat::Float) => Box::new(PackerF64M{}),
                (64, 2, SampleFormat::Float) => Box::new(PackerF64S{}),
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
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>>;
}

struct PackerU8M;

impl<W> SamplePacker<W> for PackerU8M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_u8(SampleUtils::f32_to_u8(frame.0))?;
        Ok(())
    }
}

struct PackerU8S;

impl<W> SamplePacker<W> for PackerU8S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_u8(SampleUtils::f32_to_u8(frame.0))?;
        writer.write_le_u8(SampleUtils::f32_to_u8(frame.1))?;
        Ok(())
    }
}

struct PackerS16M;

impl<W> SamplePacker<W> for PackerS16M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_i16(SampleUtils::f32_to_i16(frame.0))?;
        Ok(())
    }
}

struct PackerS16S;

impl<W> SamplePacker<W> for PackerS16S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_i16(SampleUtils::f32_to_i16(frame.0))?;
        writer.write_le_i16(SampleUtils::f32_to_i16(frame.1))?;
        Ok(())
    }
}

struct PackerS24M;

impl<W> SamplePacker<W> for PackerS24M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        let mono = SampleUtils::f32_to_i32(frame.0).to_be_bytes(); mono.to_vec().remove(0);
        writer.write_bytes(&mono)?;
        Ok(())
    }
}

struct PackerS24S;

impl<W> SamplePacker<W> for PackerS24S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        let chnl1 = SampleUtils::f32_to_i32(frame.0).to_be_bytes(); chnl1.to_vec().remove(0);
        let chnl2 = SampleUtils::f32_to_i32(frame.1).to_be_bytes(); chnl2.to_vec().remove(0);
        writer.write_bytes(&chnl1)?;
        writer.write_bytes(&chnl2)?;
        Ok(())
    }
}

struct PackerS32M;

impl<W> SamplePacker<W> for PackerS32M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_i32(SampleUtils::f32_to_i32(frame.0))?;
        Ok(())
    }
}

struct PackerS32S;

impl<W> SamplePacker<W> for PackerS32S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_i32(SampleUtils::f32_to_i32(frame.0))?;
        writer.write_le_i32(SampleUtils::f32_to_i32(frame.1))?;
        Ok(())
    }
}

struct PackerF32M;

impl<W> SamplePacker<W> for PackerF32M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_f32(frame.0)?;
        Ok(())
    }
}

struct PackerF32S;

impl<W> SamplePacker<W> for PackerF32S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_f32(frame.0)?;
        writer.write_le_f32(frame.1)?;
        Ok(())
    }
}

struct PackerF64M;

impl<W> SamplePacker<W> for PackerF64M where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_f64(frame.0 as f64)?;
        Ok(())
    }
}

struct PackerF64S;

impl<W> SamplePacker<W> for PackerF64S where W: Write + Seek {
    fn save_sample(&self, writer: &mut StructWrite::<W>, frame: &Frame) -> Result<(), Box<dyn Error>> {
        writer.write_le_f64(frame.0 as f64)?;
        writer.write_le_f64(frame.1 as f64)?;
        Ok(())
    }
}


