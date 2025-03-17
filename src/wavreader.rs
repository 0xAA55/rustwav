use std::{fs::File, io::{BufReader}, error::Error};

use hound::WavReader;

use crate::waveform::WaveForm;
use crate::sampleutils::SampleUtils;
use crate::audioreader::{Spec, SampleFormat, AudioReader};

pub struct WaveReaderSimple {
    reader: WavReader<BufReader<File>>,
    spec: Spec,
    chunk_size: usize,
}

impl AudioReader for WaveReaderSimple {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> {
        let reader = WavReader::open(input_file)?;
        let spec = reader.spec();
        Ok( Self {
            reader,
            spec: Spec {
                channels: spec.channels,
                sample_rate: spec.sample_rate,
                bits_per_sample: spec.bits_per_sample,
                sample_format: match spec.sample_format {
                    hound::SampleFormat::Float => SampleFormat::Float,
                    hound::SampleFormat::Int => SampleFormat::Int,
                },
            },
            chunk_size: 0,
        })
    }

    fn spec(&self) -> Spec {
        self.spec.clone()
    }

    fn set_chunk_size(&mut self, chunk_size: usize) {
        self.chunk_size = chunk_size;
    }

    fn get_chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl Iterator for WaveReaderSimple {
    type Item = WaveForm;

    // 被当作迭代器使用时，返回一块块的音频数据
    fn next(&mut self) -> Option<Self::Item> {
        let chunk_size = self.get_chunk_size();
        if chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        // 分浮点数格式和整数格式分别处理（整数）
        match self.spec.sample_format {
            SampleFormat::Int => {
                // 整数要转换为浮点数，并且不同长度的整数要标准化到相同的长度
                match self.spec.channels {
                    1 => {
                        let mono: Vec<i32> = self.reader.samples::<i32>().take(chunk_size).flatten().collect();
                        if mono.is_empty() { return None; }
                        let mono = SampleUtils::integers_upcast_to_floats(&mono, self.spec.bits_per_sample);
                        Some(Self::Item::Mono(mono))
                    },
                    2 => {
                        let stereo: Vec<i32> = self.reader.samples::<i32>().take(chunk_size * 2).flatten().collect();
                        if stereo.is_empty() { return None; }
                        let stereo = SampleUtils::integers_upcast_to_floats(&stereo, self.spec.bits_per_sample);
                        Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
                    },
                    other => panic!("Unsupported channel number: {}", other)
                }
            },
            SampleFormat::Float => {
                // 浮点数不用转换
                match self.spec().channels {
                    1 => {
                        let mono: Vec<f32> = self.reader.samples::<f32>().take(chunk_size).flatten().collect();
                        if mono.is_empty() { return None; }
                        Some(Self::Item::Mono(mono))
                    },
                    2 => {
                        let stereo: Vec<f32> = self.reader.samples::<f32>().take(chunk_size * 2).flatten().collect();
                        if stereo.is_empty() { return None; }
                        Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
                    },
                    other => panic!("Unsupported channel number: {}", other)
                }
            },
        }
    }
}
