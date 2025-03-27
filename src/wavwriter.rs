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
    allow_larger_than_4gb: bool,
    num_frames: u64,
    frame_size: u16,
    data_offset: u64,
    sample_type: WaveSampleType,
    sample_packer_from: SamplePacker,
    riff_chunk: Option<ChunkWriter>,
    data_chunk: Option<ChunkWriter>,
    pub bext_chunk: Option<BextChunk>,
    pub smpl_chunk: Option<SmplChunk>,
    pub inst_chunk: Option<InstChunk>,
    pub cue__chunk: Option<Cue_Chunk>,
    pub axml_chunk: Option<String>,
    pub ixml_chunk: Option<String>,
    pub list_chunk: Option<ListChunk>,
    pub acid_chunk: Option<AcidChunk>,
    pub trkn_chunk: Option<String>,
    pub junk_chunks: Vec<Vec<u8>>,
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, allow_larger_than_4gb: bool) -> Result<WaveWriter, Box<dyn Error>> {
        Self::new(Arc::new(Mutex::new(BufWriter::new(File::create(filename)?))), spec, allow_larger_than_4gb)
    }

    pub fn new(writer: Arc<Mutex<dyn Writer>>, spec: &Spec, allow_larger_than_4gb: bool) -> Result<WaveWriter, Box<dyn Error>> {
        let spec = spec.clone();
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let mut ret = Self{
            writer: writer.clone(),
            spec,
            allow_larger_than_4gb,
            num_frames: 0,
            frame_size,
            data_offset: 0,
            sample_type: spec.get_sample_type()?,
            sample_packer_from: SamplePacker::new(writer.clone()),
            riff_chunk: Some(ChunkWriter::begin(writer.clone(), b"RIFF")?),
            data_chunk: None,
            bext_chunk: None,
            smpl_chunk: None,
            inst_chunk: None,
            cue__chunk: None,
            axml_chunk: None,
            ixml_chunk: None,
            list_chunk: None,
            acid_chunk: None,
            trkn_chunk: None,
            junk_chunks: Vec::<Vec<u8>>::new(),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        use SampleFormat::{Int, UInt, Float};
        peel_arc_mutex!(self.writer, writer, writer_guard);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        writer.write_all(b"WAVE")?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        if self.allow_larger_than_4gb {
            let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
            writer.write_all(&[0u8; 28])?;
            cw.end()?;
        }

        // 准备写入 fmt 块
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

        let fmt_chunk = fmt_Chunk {
            format_tag: match ext {
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
            },
            channels: self.spec.channels,
            sample_rate: self.spec.sample_rate,
            byte_rate: self.spec.sample_rate * self.frame_size as u32,
            block_align: self.frame_size,
            bits_per_sample: self.spec.bits_per_sample,
            extension: match ext {
                false => None,
                true => Some(fmt_Chunk_Extension {
                    ext_len: 22,
                    bits_per_sample: self.spec.bits_per_sample,
                    channel_mask: self.spec.channel_mask,
                    sub_format: match self.spec.sample_format {
                        Int => GUID_PCM_FORMAT,
                        Float => GUID_IEEE_FLOAT_FORMAT,
                        _ => return Err(AudioWriteError::InvalidArguments(String::from("\"Unknown\" was given for specifying the sample format")).into()),
                    },
                }),
            },
        };

        fmt_chunk.write(self.writer.clone())?;

        self.data_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"data")?);
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_sample<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType + Clone {
        if self.data_chunk.is_some() {
            self.sample_packer_from.write_sample(frame, self.sample_type)?;
            self.num_frames += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")).into())
        }
    }

    pub fn spec(&self) -> &Spec{
        &self.spec
    }
    pub fn get_num_frames(&self) -> u64 {
        self.num_frames
    }
    pub fn get_frame_size(&self) -> u16 {
        self.frame_size
    }
    pub fn set_bext_chunk(&mut self, chunk: &BextChunk) {
        self.bext_chunk = Some(chunk.clone());
    }
    pub fn set_smpl_chunk(&mut self, chunk: &SmplChunk) {
        self.smpl_chunk = Some(chunk.clone());
    }
    pub fn set_inst_chunk(&mut self, chunk: &InstChunk) {
        self.inst_chunk = Some(chunk.clone());
    }
    pub fn set_cue__chunk(&mut self, chunk: &Cue_Chunk) {
        self.cue__chunk = Some(chunk.clone());
    }
    pub fn set_axml_chunk(&mut self, chunk: &String) {
        self.axml_chunk = Some(chunk.clone());
    }
    pub fn set_ixml_chunk(&mut self, chunk: &String) {
        self.ixml_chunk = Some(chunk.clone());
    }
    pub fn set_list_chunk(&mut self, chunk: &ListChunk) {
        self.list_chunk = Some(chunk.clone());
    }
    pub fn set_acid_chunk(&mut self, chunk: &AcidChunk) {
        self.acid_chunk = Some(chunk.clone());
    }
    pub fn set_trkn_chunk(&mut self, chunk: &String) {
        self.trkn_chunk = Some(chunk.clone());
    }
    pub fn add_junk_chunk(&mut self, chunk: &Vec<u8>) {
        self.junk_chunks.push(chunk.clone());
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // 结束对 data 块的写入
        self.data_chunk = None;

        peel_arc_mutex!(self.writer, writer, writer_guard);

        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk {
            chunk.write(self.writer.clone())?;
        }
        if let Some(chunk) = &self.smpl_chunk {
            chunk.write(self.writer.clone())?;
        }
        if let Some(chunk) = &self.inst_chunk {
            chunk.write(self.writer.clone())?;
        }
        if let Some(chunk) = &self.cue__chunk {
            chunk.write(self.writer.clone())?;
        }
        if let Some(chunk) = &self.list_chunk {
            chunk.write(self.writer.clone())?;
        }
        if let Some(chunk) = &self.acid_chunk {
            chunk.write(self.writer.clone())?;
        }

        // 写入其它全部的字符串块
        let mut string_chunks_to_write = Vec::<([u8; 4], &String)>::new();
        if let Some(chunk) = &self.axml_chunk {
            string_chunks_to_write.push((*b"axml", &chunk));
        }
        if let Some(chunk) = &self.ixml_chunk {
            string_chunks_to_write.push((*b"ixml", &chunk));
        }
        if let Some(chunk) = &self.trkn_chunk {
            string_chunks_to_write.push((*b"Trkn", &chunk));
        }
        for (flag, chunk) in string_chunks_to_write.iter() {
            let mut cw = ChunkWriter::begin(self.writer.clone(), flag)?;
            write_str(writer, chunk)?;
            cw.end()?;
        }

        // 写入所有的 JUNK 块
        for junk in self.junk_chunks.iter() {
            let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
            writer.write_all(&junk)?;
            cw.end()?;
        }

        // 接下来是重点：判断文件大小是不是超过了 4GB，是的话，把文件头改为 RF64，然后在之前留坑的地方填入 RF64 的信息表
        self.riff_chunk = None;
        let file_end_pos = writer.stream_position()?;
        if file_end_pos > 0xFFFFFFFFu64 {
            writer.seek(SeekFrom::Start(0))?;
            writer.write_all(b"RF64")?;
            0xFFFFFFFFu32.write_le(writer)?;
            writer.write_all(b"WAVE")?;
            let mut cw = ChunkWriter::begin(self.writer.clone(), b"ds64")?;
            let riff_size = file_end_pos - 8;
            let data_size = self.num_frames * self.frame_size as u64;
            let sample_count = self.num_frames / self.spec.channels as u64;
            riff_size.write_le(writer)?;
            data_size.write_le(writer)?;
            sample_count.write_le(writer)?;
            0u32.write_le(writer)?; // table length
            cw.end()?;
        }
        Ok(())
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        self.finalize().unwrap();
    }
}

#[derive(Clone)]
struct SamplePackerFrom<S>
where S : SampleType {
    writer: Arc<Mutex<dyn Writer>>,
    funcmap: HashMap<WaveSampleType, fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), Box<dyn Error>>>,
    last_used_target_format: WaveSampleType,
    last_used_func: fn(writer: &mut Arc<Mutex<dyn Writer>>, &Vec<S>) -> Result<(), Box<dyn Error>>,
}

impl<S> std::fmt::Debug for SamplePackerFrom<S>
where S : SampleType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.last_used_target_format == WaveSampleType::Unknown {
            fmt.debug_struct(&format!("SamplePackerFrom<{}>", type_name::<S>()))
                .finish_non_exhaustive()
        } else {
            fmt.debug_struct(&format!("SamplePackerFrom<{}>", type_name::<S>()))
                .field("writer", &self.writer)
                .field("funcmap", &self.funcmap)
                .field("last_used_target_format", &self.last_used_target_format)
                .field("last_used_func", &self.last_used_func)
                .finish()
        }
    }
}

impl<S> SamplePackerFrom<S>
where S : SampleType {
    fn new(writer: Arc<Mutex<dyn Writer>>) -> Self {
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
    fn new(writer: Arc<Mutex<dyn Writer>>) -> Self {
        let sample_packer_from__u8 = SamplePackerFrom::< u8>::new(writer.clone());
        let sample_packer_from_i16 = SamplePackerFrom::<i16>::new(writer.clone());
        let sample_packer_from_i24 = SamplePackerFrom::<i24>::new(writer.clone());
        let sample_packer_from_i32 = SamplePackerFrom::<i32>::new(writer.clone());
        let sample_packer_from_f32 = SamplePackerFrom::<f32>::new(writer.clone());
        let sample_packer_from_f64 = SamplePackerFrom::<f64>::new(writer.clone());
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

