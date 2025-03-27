#![allow(non_snake_case)]
#![allow(dead_code)]

use std::any::type_name;
use std::fs::File;
use std::path::Path;
use std::io::{SeekFrom, BufWriter};
use std::ops::DerefMut; 
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::collections::HashMap;

use crate::errors::{AudioWriteError};
pub use crate::wavcore::*;

#[derive(Debug, Clone)]
pub struct WaveWriter {
    writer: Arc<Mutex<dyn Writer>>,
    spec: Spec,
    num_frames: u32,
    frame_size: u16,
    riff_offset: u64,
    datalen_offset: u64,
    data_offset: u64,
    sample_type: WaveSampleType,
    sample_packer_from: SamplePacker,
}

// TODO
// 使 wavcores 里面的各种 Chunk 都能被写入到 WaveWriter 里面来。
// 修改接口，先接收用户提供的 Chunk，再使用 “BeginWriteSamples” 开始写入样本数据，写完后再调用 “EndWriteSamples” 。
// 不接收用户提供的 fmt Chunk，而是自行根据其他条件自己构建。
// 基本上所有的能允许用户提供的 Chunk 都被追加到 data Chunk 的末尾。

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec) -> Result<WaveWriter, Box<dyn Error>> {
        Self::new(Arc::new(Mutex::new(BufWriter::new(File::create(filename)?))), spec)
    }

    pub fn new(writer: Arc<Mutex<dyn Writer>>, spec: &Spec) -> Result<WaveWriter, Box<dyn Error>> {
        let spec = spec.clone();
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let mut ret = Self{
            writer: writer.clone(),
            spec,
            num_frames: 0,
            frame_size,
            riff_offset: 0,
            datalen_offset: 0,
            data_offset: 0,
            sample_type: spec.get_sample_type(),
            sample_packer_from: SamplePacker::new(&writer),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        use SampleFormat::{Int, UInt, Float};
        peel_arc_mutex!(self.writer, writer, writer_guard);
        writer.write_all(b"RIFF")?;
        self.riff_offset = writer.stream_position()?;
        0u32.write_le(writer)?;
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
        } as u32).write_le(writer)?;
        (match ext {
            true => 0xFFFE,
            false => {
                match (self.spec.bits_per_sample, self.spec.sample_format) {
                    (8, UInt) => 1,
                    (16, Int) => 1,
                    (32, Float) => 3,
                    (64, Float) => 3,
                    _ => return Err(AudioWriteError::UnsupportedFormat(format!("Don't know how to specify format tag")).into()),
                }
            }
        } as u16).write_le(writer)?;
        (self.spec.channels as u16).write_le(writer)?;
        (self.spec.sample_rate as u32).write_le(writer)?;
        (self.spec.sample_rate * self.frame_size as u32).write_le(writer)?;
        (self.frame_size as u16).write_le(writer)?;
        (self.spec.bits_per_sample as u16).write_le(writer)?;
        if ext == true {
            22u16.write_le(writer)?; // 额外数据大小
            (self.spec.bits_per_sample as u16).write_le(writer)?;
            (self.spec.channel_mask as u32).write_le(writer)?; // 声道掩码

            // 写入具体格式的 GUID
            match self.spec.sample_format {
                Int => GUID_PCM_FORMAT.write(writer)?,
                Float => GUID_IEEE_FLOAT_FORMAT.write(writer)?,
                _ => return Err(AudioWriteError::InvalidArguments(String::from("\"Unknown\" was given for specifying the sample format")).into()),
            }
        };
        writer.write_all(b"data")?;
        self.datalen_offset = writer.stream_position()?;
        0u32.write_le(writer)?;
        self.data_offset = writer.stream_position()?;
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_sample<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType + Clone {
        self.sample_packer_from.write_sample(frame, self.sample_type)?;
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
        peel_arc_mutex!(self.writer, writer, writer_guard);
        const HEADER_SIZE: u32 = 44;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        writer.seek(SeekFrom::Start(self.riff_offset))?;
        (HEADER_SIZE + all_sample_size - 8).write_le(writer)?;
        writer.seek(SeekFrom::Start(self.datalen_offset))?;
        all_sample_size.write_le(writer)?;
        Ok(())
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        self.update_header()
    }
}

#[derive(Debug, Clone)]
struct SamplePackerFrom<S>
where S : SampleType {
    writer: Arc<Mutex<dyn Writer>>,
    funcmap: HashMap<WaveSampleType, fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), Box<dyn Error>>>,
    last_used_target_format: WaveSampleType,
    last_used_func: fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), Box<dyn Error>>,
}

impl<S> SamplePackerFrom<S>
where S : SampleType {
    fn new(writer: &Arc<Mutex<dyn Writer>>) -> Self {
        use WaveSampleType::{Unknown,U8,S16,S24,S32,F32,F64};
        let mut funcmap = HashMap::<WaveSampleType, fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), Box<dyn Error>>>::new();
        funcmap.insert(Unknown, Self::write_sample_to__nothing);
        funcmap.insert(U8,  Self::write_sample_to__u8);
        funcmap.insert(S16, Self::write_sample_to_i16);
        funcmap.insert(S24, Self::write_sample_to_i24);
        funcmap.insert(S32, Self::write_sample_to_i32);
        funcmap.insert(F32, Self::write_sample_to_f32);
        funcmap.insert(F64, Self::write_sample_to_f64);
        Self {
            writer: writer.clone(),
            funcmap,
            last_used_target_format: Unknown,
            last_used_func: Self::write_sample_to__nothing,
        }
    }

    fn write_sample_to(&mut self, frame: &Vec<S>, target_format: WaveSampleType) -> Result<(), Box<dyn Error>> {
        if self.last_used_target_format != target_format {
            self.last_used_target_format = target_format;
            self.last_used_func = *self.funcmap.get(&target_format).unwrap();
        }
        (self.last_used_func)(&mut self.writer, frame)
    }

    fn write_sample_to__nothing(_writer: &mut Arc<Mutex<dyn Writer>>, _frame: &Vec<S>) -> Result<(), Box<dyn Error>> {
        Err(AudioError::UnknownSampleType.into())
    }

    fn write_sample_to__u8(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) -> Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<u8>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_i16(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i16>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_i24(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i24>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_i32(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i32>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_i64(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i64>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_f32(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<f32>().write_le(writer)?;
        }
        Ok(())
    }

    fn write_sample_to_f64(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), Box<dyn Error>> {
        peel_arc_mutex!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<f64>().write_le(writer)?;
        }
        Ok(())
    }
}


#[derive(Debug, Clone)]
struct SamplePacker {
    sample_packer_from__u8: SamplePackerFrom::< u8>,
    sample_packer_from_i16: SamplePackerFrom::<i16>,
    sample_packer_from_i24: SamplePackerFrom::<i24>,
    sample_packer_from_i32: SamplePackerFrom::<i32>,
    sample_packer_from_f32: SamplePackerFrom::<f32>,
    sample_packer_from_f64: SamplePackerFrom::<f64>,
}

impl SamplePacker {
    fn new(writer: &Arc<Mutex<dyn Writer>>) -> Self {
        let sample_packer_from__u8 = SamplePackerFrom::< u8>::new(&writer);
        let sample_packer_from_i16 = SamplePackerFrom::<i16>::new(&writer);
        let sample_packer_from_i24 = SamplePackerFrom::<i24>::new(&writer);
        let sample_packer_from_i32 = SamplePackerFrom::<i32>::new(&writer);
        let sample_packer_from_f32 = SamplePackerFrom::<f32>::new(&writer);
        let sample_packer_from_f64 = SamplePackerFrom::<f64>::new(&writer);
        Self {
            sample_packer_from__u8,
            sample_packer_from_i16,
            sample_packer_from_i24,
            sample_packer_from_i32,
            sample_packer_from_f32,
            sample_packer_from_f64,
        }
    }

    // 忽悠编译器用的
    fn frame_cvt<S, D>(frame: &Vec<S>) -> Vec<D>
    where S: SampleType,
          D: SampleType {
        let mut ret = Vec::<D>::with_capacity(frame.len());
        for sample in frame.iter(){
            ret.push(D::from(*sample));
        }
        ret
    }

    fn write_sample<S>(&mut self, frame: &Vec<S>, to_format: WaveSampleType) -> Result<(), Box<dyn Error>> 
    where S: SampleType {
        match type_name::<S>() { // 我打赌取泛型名字并用于匹配的过程不会发生运行时匹配，而是编译器会优化。
            "u8"  => self.sample_packer_from__u8.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i16" => self.sample_packer_from_i16.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i24" => self.sample_packer_from_i24.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i32" => self.sample_packer_from_i32.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "f32" => self.sample_packer_from_f32.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "f64" => self.sample_packer_from_f64.write_sample_to(&Self::frame_cvt(&frame), to_format)?,
            other => return Err(AudioWriteError::WrongSampleFormat(other.to_string()).into()),
        }
        Ok(())
    }
}

