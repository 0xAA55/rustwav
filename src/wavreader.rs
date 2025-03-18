use std::{fs::File, {path::Path}, io::{Read, Seek, SeekFrom, BufReader}, error::Error};

use crate::structread::StructRead;
use crate::sampleutils::SampleUtils;
use crate::audioreader::{Spec, SampleFormat, AudioReader, Frame, AudioReadError};

pub struct WaveReader<R> {
    reader: StructRead<R>,
    first_sample_offset: u64,
    spec: Spec,
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u32, // 总帧数
    sampler: Box<dyn WaveSampler<R>>, // 快速取样器
}

trait WaveSampler<R> where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>>;
}

struct SamplerU8M;

impl<R> WaveSampler<R> for SamplerU8M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        Ok(Frame(mono, mono))
    }
}

struct SamplerU8S;

impl<R> WaveSampler<R> for SamplerU8S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        let chnl2 = SampleUtils::u8_to_f32(reader.read_le_u8()?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct SamplerS16M;

impl<R> WaveSampler<R> for SamplerS16M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i16_to_f32(reader.read_le_i16()?);
        Ok(Frame(mono, mono))
    }
}

struct SamplerS16S;

impl<R> WaveSampler<R> for SamplerS16S where R: Read + Seek {
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

struct SamplerS24M;

impl<R> WaveSampler<R> for SamplerS24M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        Ok(Frame(mono, mono))
    }
}

struct SamplerS24S;

impl<R> WaveSampler<R> for SamplerS24S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        let chnl2 = SampleUtils::i32_to_f32(read_i24_as_i32(reader)?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct SamplerS32M;

impl<R> WaveSampler<R> for SamplerS32M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        Ok(Frame(mono, mono))
    }
}

struct SamplerS32S;

impl<R> WaveSampler<R> for SamplerS32S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        let chnl2 = SampleUtils::i32_to_f32(reader.read_le_i32()?);
        Ok(Frame(chnl1, chnl2))
    }
}

struct SamplerF32M;

impl<R> WaveSampler<R> for SamplerF32M where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead::<R>) -> Result<Frame, Box<dyn Error>> {
        let mono = reader.read_le_f32()?;
        Ok(Frame(mono, mono))
    }
}

struct SamplerF32S;

impl<R> WaveSampler<R> for SamplerF32S where R: Read + Seek {
    fn get_sample(&self, reader: &mut StructRead<R>) -> Result<Frame, Box<dyn Error>> {
        let chnl1 = reader.read_le_f32()?;
        let chnl2 = reader.read_le_f32()?;
        Ok(Frame(chnl1, chnl2))
    }
}

impl<R> WaveReader<R> where R: Read + Seek {
    pub fn new(reader: R) -> Result<WaveReader<R>, Box<dyn Error>> {
        let mut reader = StructRead::new(reader);
        reader.expect_flag(b"RIFF")?;
        let _remain_len = reader.read_le_u32()?;
        reader.expect_flag(b"WAVE")?;
        reader.expect_flag(b"fmt ")?;
        let fmt_size = reader.read_le_u32()?;
        let cur_pos = reader.reader.stream_position()?;
        let format = reader.read_le_i16()?;
        let channels = reader.read_le_u16()?;
        let sample_rate = reader.read_le_u32()?;
        let _byte_rate = reader.read_le_u32()?;
        let block_align = reader.read_le_u16()?;
        let bits_per_sample = reader.read_le_u16()?;
        reader.reader.seek(SeekFrom::Start(cur_pos + fmt_size as u64))?;
        let data_or_fact = reader.read_flag(4)?;
        if data_or_fact == b"fact" {
            let fact_size = reader.read_le_u32()?;
            reader.reader.seek(SeekFrom::Current(fact_size as i64))?;
            reader.expect_flag(b"data")?;
        }
        let data_size = reader.read_le_u32()?;
        let first_sample_offset = reader.reader.stream_position()?;
        let frame_size = block_align;
        let num_frames = data_size / block_align as u32;
        let sample_format = match format {
            1 => {
                match bits_per_sample {
                    8 => SampleFormat::Int,
                    16 => SampleFormat::Int,
                    _ => return Err(AudioReadError::DataCorrupted.into()),
                }
            },
            -2 => {
                match bits_per_sample {
                    24 => SampleFormat::Int,
                    32 => SampleFormat::Int,
                    _ => return Err(AudioReadError::DataCorrupted.into()),
                }
            },
            3 => {
                match bits_per_sample {
                    32 => SampleFormat::Float,
                    64 => SampleFormat::Float,
                    _ => return Err(AudioReadError::DataCorrupted.into()),
                }
            },
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
            sampler: match bits_per_sample {
                8 => {
                    match channels {
                        1 => Box::new(SamplerU8M{}),
                        2 => Box::new(SamplerU8S{}),
                        _ => {return Err(AudioReadError::Unimplemented.into())},
                    }
                },
                16 => {
                    match channels {
                        1 => Box::new(SamplerS16M{}),
                        2 => Box::new(SamplerS16S{}),
                        _ => {return Err(AudioReadError::Unimplemented.into())},
                    }
                },
                24 => {
                    match channels {
                        1 => Box::new(SamplerS24M{}),
                        2 => Box::new(SamplerS24S{}),
                        _ => {return Err(AudioReadError::Unimplemented.into())},
                    }
                },
                32 => {
                    match sample_format {
                        SampleFormat::Int => {
                            match channels {
                                1 => Box::new(SamplerS32M{}),
                                2 => Box::new(SamplerS32S{}),
                                _ => {return Err(AudioReadError::Unimplemented.into())},
                            }
                        },
                        SampleFormat::Float => {
                            match channels {
                                1 => Box::new(SamplerF32M{}),
                                2 => Box::new(SamplerF32S{}),
                                _ => {return Err(AudioReadError::Unimplemented.into())},
                            }
                        },
                        _ => {return Err(AudioReadError::Unimplemented.into())},
                    }
                },
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

//impl Iterator for Iter {
//    type Item = WaveForm;
//
//    // 被当作迭代器使用时，返回一块块的音频数据
//    fn next(&mut self) -> Option<Self::Item> {
//        let chunk_size = self.get_chunk_size();
//        if chunk_size == 0 {panic!("Must set chunk size before iterations.")}
//
//        // 分浮点数格式和整数格式分别处理（整数）
//        match self.spec.sample_format {
//            SampleFormat::Int => {
//                // 整数要转换为浮点数，并且不同长度的整数要标准化到相同的长度
//                match self.spec.channels {
//                    1 => {
//                        let mono: Vec<i32> = self.reader.samples::<i32>().take(chunk_size).flatten().collect();
//                        if mono.is_empty() { return None; }
//                        let mono = SampleUtils::integers_upcast_to_floats(&mono, self.spec.bits_per_sample);
//                        Some(Self::Item::Mono(mono))
//                    },
//                    2 => {
//                        let stereo: Vec<i32> = self.reader.samples::<i32>().take(chunk_size * 2).flatten().collect();
//                        if stereo.is_empty() { return None; }
//                        let stereo = SampleUtils::integers_upcast_to_floats(&stereo, self.spec.bits_per_sample);
//                        Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
//                    },
//                    other => panic!("Unsupported channel number: {}", other)
//                }
//            },
//            SampleFormat::Float => {
//                // 浮点数不用转换
//                match self.spec().channels {
//                    1 => {
//                        let mono: Vec<f32> = self.reader.samples::<f32>().take(chunk_size).flatten().collect();
//                        if mono.is_empty() { return None; }
//                        Some(Self::Item::Mono(mono))
//                    },
//                    2 => {
//                        let stereo: Vec<f32> = self.reader.samples::<f32>().take(chunk_size * 2).flatten().collect();
//                        if stereo.is_empty() { return None; }
//                        Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
//                    },
//                    other => panic!("Unsupported channel number: {}", other)
//                }
//            },
//        }
//    }
//}

