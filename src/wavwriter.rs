#![allow(non_snake_case)]
#![allow(dead_code)]

use std::any::type_name;
use std::fs::File;
use std::path::Path;
use std::io::{self, SeekFrom, BufWriter};
use std::ops::DerefMut; 
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::collections::HashMap;

use crate::errors::{AudioWriteError};
use crate::wavcore::*;

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
            sample_type: spec.get_sample_type()?,
            sample_packer_from: SamplePacker::new(&writer),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        use SampleFormat::{Int, UInt, Float};
        make_guarded_writer!(self.writer, writer, writer_guard);
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
                    _ => return Err(AudioWriteError::UnsupportedFormat(format!("Don't know how to specify format tag")).into()),
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
                Int => GUID_PCM_FORMAT.write(&mut writer)?,
                Float => GUID_IEEE_FLOAT_FORMAT.write(&mut writer)?,
                _ => return Err(AudioWriteError::InvalidArguments(String::from("\"Unknown\" was given for specifying the sample format")).into()),
            }
        };
        writer.write_all(b"data")?;
        self.datalen_offset = writer.stream_position()?;
        0u32.write_le(&mut writer)?;
        self.data_offset = writer.stream_position()?;
        Ok(())
    }

    pub fn save_frame<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType + Clone {
        self.sample_packer_from.save_sample(frame, self.sample_type)?;
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
        make_guarded_writer!(self.writer, writer, writer_guard);
        const HEADER_SIZE: u32 = 44;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        writer.seek(SeekFrom::Start(self.riff_offset))?;
        (HEADER_SIZE + all_sample_size - 8).write_le(&mut writer)?;
        writer.seek(SeekFrom::Start(self.datalen_offset))?;
        all_sample_size.write_le(&mut writer)?;
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
    funcmap: HashMap<WaveSampleType, fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), io::Error>>,
}

impl<S> SamplePackerFrom<S>
where S : SampleType {
    fn new(writer: &Arc<Mutex<dyn Writer>>) -> Self {
        use WaveSampleType::{U8,S16,S24,S32,F32,F64};
        let mut funcmap = HashMap::<WaveSampleType, fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), io::Error>>::new();
        funcmap.insert(U8,  Self::save_sample_to__u8);
        funcmap.insert(S16, Self::save_sample_to_i16);
        funcmap.insert(S24, Self::save_sample_to_i24);
        funcmap.insert(S32, Self::save_sample_to_i32);
        funcmap.insert(F32, Self::save_sample_to_f32);
        funcmap.insert(F64, Self::save_sample_to_f64);
        Self {
            writer: writer.clone(),
            funcmap,
        }
    }

    fn save_sample_to(&mut self, frame: &Vec<S>, target_format: WaveSampleType) -> Result<(), io::Error> {
        self.funcmap.get(&target_format).unwrap()(&mut self.writer, frame)
    }

    fn save_sample_to__u8(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) -> Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<u8>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_i16(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i16>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_i24(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i24>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_i32(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i32>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_i64(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<i64>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_f32(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<f32>().write_le(&mut writer)?;
        }
        Ok(())
    }

    fn save_sample_to_f64(writer: &mut Arc<Mutex<dyn Writer>>, frame: &Vec<S>) ->Result<(), io::Error> {
        make_guarded_writer!(writer, writer, writer_guard);
        for sample in frame.iter() {
            sample.to::<f64>().write_le(&mut writer)?;
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

    fn save_sample<S>(&mut self, frame: &Vec<S>, to_format: WaveSampleType) -> Result<(), Box<dyn Error>> 
    where S: SampleType {
        match type_name::<S>() {
            "u8"  => self.sample_packer_from__u8.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i16" => self.sample_packer_from_i16.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i24" => self.sample_packer_from_i24.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "i32" => self.sample_packer_from_i32.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "f32" => self.sample_packer_from_f32.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            "f64" => self.sample_packer_from_f64.save_sample_to(&Self::frame_cvt(&frame), to_format)?,
            other => return Err(AudioWriteError::WrongSampleFormat(other.to_string()).into()),
        }
        Ok(())
    }
}

