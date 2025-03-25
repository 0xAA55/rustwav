use std::{fs::File, error::Error};

use flac::{StreamReader, ReadStream, stream::Iter, SampleSize, ErrorKind};

use crate::waveform::WaveForm;
use crate::audioreader::{AudioReader, Spec, SampleFormat, AudioReadError};

pub struct FlacReader {
	reader: Box<StreamReader::<File>>,
	spec: Spec,
    chunk_size: usize,
    cur_position: usize,
}

impl AudioReader for FlacReader {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> {
    	let mut reader = {
    		match StreamReader::<File>::from_file(input_file) {
	    		Ok(reader) => Box::new(reader),
	    		Err(ErrorKind::IO(ErrorKind)) => return Err(AudioReadError::IOError(format!("{ErrorKind}")).into()),
	    		_ => return Err(AudioReadError::FormatError.into()),
	    	}
	    };
    	let info = reader.info();
    	Ok(Self {
    		reader,
    		spec: Spec {
	    		channels: info.channels as u16,
	    		sample_rate: info.sample_rate,
	    		bits_per_sample: info.bits_per_sample as u16,
	    		sample_format: SampleFormat::Int,
	    	},
    		chunk_size: 0,
    		cur_position: 0,
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

impl Iterator for FlacReader {
    type Item = WaveForm;

    // 被当作迭代器使用时，返回一块块的音频数据
    fn next(&mut self) -> Option<Self::Item> {
        let chunk_size = self.get_chunk_size();
        if chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        // FLAC 不会解码出浮点数，因此值需要做整数处理。
        match self.spec.channels {
        	1 => None,
        	2 => None,
        	other => panic!("Unsupported channel number: {}", other),
        }

        // 分浮点数格式和整数格式分别处理（整数）
        // match self.spec.sample_format {
        //     SampleFormat::Int => {
        //         // 整数要转换为浮点数，并且不同长度的整数要标准化到相同的长度
        //         match self.spec.channels {
        //             1 => {
        //                 let mono: Vec<i32> = self.reader.samples::<i32>().take(chunk_size).flatten().collect();
        //                 if mono.is_empty() { return None; }
        //                 let mono = SampleUtils::integers_upcast_to_floats(&mono, self.spec.bits_per_sample);
        //                 Some(Self::Item::Mono(mono))
        //             },
        //             2 => {
        //                 let stereo: Vec<i32> = self.reader.samples::<i32>().take(chunk_size * 2).flatten().collect();
        //                 if stereo.is_empty() { return None; }
        //                 let stereo = SampleUtils::integers_upcast_to_floats(&stereo, self.spec.bits_per_sample);
        //                 Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
        //             },
        //             other => panic!("Unsupported channel number: {}", other),
        //         }
        //     },
        //     SampleFormat::Float => {
        //         // 浮点数不用转换
        //         match self.spec().channels {
        //             1 => {
        //                 let mono: Vec<f32> = self.reader.samples::<f32>().take(chunk_size).flatten().collect();
        //                 if mono.is_empty() { return None; }
        //                 Some(Self::Item::Mono(mono))
        //             },
        //             2 => {
        //                 let stereo: Vec<f32> = self.reader.samples::<f32>().take(chunk_size * 2).flatten().collect();
        //                 if stereo.is_empty() { return None; }
        //                 Some(Self::Item::Stereo(SampleUtils::unzip_samples(&stereo)))
        //             },
        //             other => panic!("Unsupported channel number: {}", other),
        //         }
        //     },
        // }
    }
}


