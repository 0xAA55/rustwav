use std::{fs::File, {path::Path}, io::{Read, BufReader, Seek, SeekFrom}, error::Error};

use crate::structread::StructRead;
use crate::sampleutils::SampleUtils;
use crate::audiocore::{SampleFormat, Spec, Frame};
use crate::audioreader::{AudioReader, AudioReadError};

pub struct WaveReader<R> {
    reader: StructRead<R>,
    first_sample_offset: u64,
    spec: Spec,
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u32, // 总帧数
    sampler: Box<dyn SampleUnpacker<R>>, // 快速取样器
}

impl<R> WaveReader<R> where R: Read + Seek {
    pub fn new(reader: R) -> Result<WaveReader<R>, Box<dyn Error>> {
        use SampleFormat::{Int, Float, Unknown};
        let mut reader = StructRead::new(reader);
        reader.expect_flag(b"RIFF")?;
        let _remain_len = reader.read_le_u32()?;
        reader.expect_flag(b"WAVE")?;
        reader.expect_flag(b"fmt ")?;
        let fmt_size = reader.read_le_u32()?;
        let cur_pos = reader.stream_position()?;
        let format = reader.read_le_i16()?;
        let channels = reader.read_le_u16()?;
        let sample_rate = reader.read_le_u32()?;
        let _byte_rate = reader.read_le_u32()?;
        let block_align = reader.read_le_u16()?;
        let bits_per_sample = reader.read_le_u16()?;
        reader.seek(SeekFrom::Start(cur_pos + fmt_size as u64))?;
        let data_or_fact = reader.read_flag(4)?;
        if data_or_fact == b"fact" {
            let fact_size = reader.read_le_u32()?;
            reader.seek(SeekFrom::Current(fact_size as i64))?;
            reader.expect_flag(b"data")?;
        }
        let data_size = reader.read_le_u32()?;
        let first_sample_offset = reader.stream_position()?;
        let frame_size = block_align;
        let num_frames = data_size / block_align as u32;
        let sample_format = match (format, bits_per_sample) {
            (1, 8) => Int,
            (1, 16) => Int,
            (-2, 24) => Int,
            (-2, 32) => Int,
            (3, 32) => Float,
            (3, 64) => Float,
            _ => return Err(AudioReadError::Unimplemented.into()),
        };
        Ok(Self {
            reader,
            first_sample_offset,
            spec: Spec {
                channels,
                sample_rate,
                bits_per_sample,
                sample_format,
            },
            frame_size,
            num_frames,
            sampler: match (bits_per_sample, channels) {
                (8, 1) => Box::new(UnpackerU8M{}),
                (8, 2) => Box::new(UnpackerU8S{}),
                (16, 1) => Box::new(UnpackerS16M{}),
                (16, 2) => Box::new(UnpackerS16S{}),
                (24, 1) => Box::new(UnpackerS24M{}),
                (24, 2) => Box::new(UnpackerS24S{}),
                (32, 1) => match sample_format { Int => Box::new(UnpackerS32M{}), Float => Box::new(UnpackerF32M{}), Unknown => return Err(AudioReadError::Unimplemented.into()), },
                (32, 2) => match sample_format { Int => Box::new(UnpackerS32S{}), Float => Box::new(UnpackerF32S{}), Unknown => return Err(AudioReadError::Unimplemented.into()), },
                (64, 1) => Box::new(UnpackerF64M{}),
                (64, 2) => Box::new(UnpackerF64S{}),
                _ => return Err(AudioReadError::Unimplemented.into()),
            }
        })
    }

    pub fn seek_to_frame(&mut self, position: u64) -> Result<(), Box<dyn Error>> {
        self.reader.reader.seek(SeekFrom::Start(self.first_sample_offset + position * self.frame_size as u64))?;
        Ok(())
    }
}

// 用文件来读取的方式，自动套上 BufReader 来提升读取效率
impl WaveReader<BufReader<File>> {
    pub fn open<P: AsRef<Path>>(filename: P) -> Result<WaveReader<BufReader<File>>, Box<dyn Error>> {
        let file = File::open(filename)?;
        let buf_reader = BufReader::new(file);
        WaveReader::new(buf_reader)
    }
}

impl<R> AudioReader for WaveReader<R> where R: Read + Seek {
    fn spec(&self) -> Spec {
        self.spec.clone()
    }

    fn get_sample(&mut self, position: u64) -> Result<Frame, Box<dyn Error>> {
        self.seek_to_frame(position)?;
        self.sampler.get_sample(&mut self.reader)
    }
}

trait SampleUnpacker<R> where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>>;
}

struct UnpackerU8M;

impl<R> SampleUnpacker<R> for UnpackerU8M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        Ok(Frame(mono, mono))
    }
}

struct UnpackerU8S;

impl<R> SampleUnpacker<R> for UnpackerU8S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        let chnl2 = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct UnpackerS16M;

impl<R> SampleUnpacker<R> for UnpackerS16M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i16_to_f32(reader.read_le_i16()?);
        Ok(Frame(mono, mono))
    }
}

struct UnpackerS16S;

impl<R> SampleUnpacker<R> for UnpackerS16S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::i16_to_f32(reader.read_le_i16()?);
        let chnl2 = SampleUtils::i16_to_f32(reader.read_le_i16()?);
        Ok(Frame(chnl1, chnl2))
    }
}

fn read_i24_as_i32<R>(reader: &mut StructRead::<R>) -> Result<i32, Box<dyn Error>> where R: Read + Seek {
    let mut v = reader.read_flag(3)?;
    v.push(v[0]);
    let mut buf = [0u8; 4];
    for i in 0..4 {buf[i] = v[i];}
    Ok(i32::from_le_bytes(buf))
}

struct UnpackerS24M;

impl<R> SampleUnpacker<R> for UnpackerS24M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        Ok(Frame(mono, mono))
    }
}

struct UnpackerS24S;

impl<R> SampleUnpacker<R> for UnpackerS24S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        let chnl2 = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct UnpackerS32M;

impl<R> SampleUnpacker<R> for UnpackerS32M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        Ok(Frame(mono, mono))
    }
}

struct UnpackerS32S;

impl<R> SampleUnpacker<R> for UnpackerS32S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        let chnl2 = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct UnpackerF32M;

impl<R> SampleUnpacker<R> for UnpackerF32M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = reader.read_le_f32()?;
        Ok(Frame(mono, mono))
    }
}

struct UnpackerF32S;

impl<R> SampleUnpacker<R> for UnpackerF32S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = reader.read_le_f32()?;
        let chnl2 = reader.read_le_f32()?;
        Ok(Frame(chnl1, chnl2))
    }
}

struct UnpackerF64M;

impl<R> SampleUnpacker<R> for UnpackerF64M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = reader.read_le_f64()? as f32;
        Ok(Frame(mono, mono))
    }
}

struct UnpackerF64S;

impl<R> SampleUnpacker<R> for UnpackerF64S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = reader.read_le_f64()? as f32;
        let chnl2 = reader.read_le_f64()? as f32;
        Ok(Frame(chnl1, chnl2))
    }
}
