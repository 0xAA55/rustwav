#![allow(non_snake_case)]
#![allow(dead_code)]

use std::fs::File;
use std::path::Path;
use std::io::{SeekFrom, BufWriter};
use std::sync::{Arc, Mutex};
use std::error::Error;

use crate::errors::{AudioWriteError};
pub use crate::wavcore::*;

#[derive(Debug)]
pub enum FileSizeOption{
    NeverLargerThan4GB,
    AllowLargerThan4GB,
    ForceUse4GBFormat,
}

#[derive(Debug)]
pub struct WaveWriter {
    writer: Arc<Mutex<dyn Writer>>,
    spec: Spec,
    file_size_option: FileSizeOption,
    num_frames: u64,
    frame_size: u16,
    data_offset: u64,
    sample_type: WaveSampleType,
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
    pub junk_chunks: Vec<JunkChunk>,
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, file_size_option: FileSizeOption) -> Result<WaveWriter, Box<dyn Error>> {
        let file_reader = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(Arc::new(Mutex::new(file_reader)), spec, file_size_option)?;
        Ok(wave_writer)
    }

    pub fn from(writer: Arc<Mutex<dyn Writer>>, spec: &Spec, file_size_option: FileSizeOption) -> Result<WaveWriter, Box<dyn Error>> {
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let sample_type = spec.get_sample_type()?;
        let mut ret = Self{
            writer: writer.clone(),
            spec: spec.clone(),
            file_size_option,
            num_frames: 0,
            frame_size,
            data_offset: 0,
            sample_type,
            riff_chunk: None,
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
            junk_chunks: Vec::<JunkChunk>::new(),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"RIFF")?);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            writer.write_all(b"WAVE")?;
            Ok(())
        })?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => (),
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
                use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                    writer.write_all(&[0u8; 28])?;
                    Ok(())
                })?;
                cw.end()?;
            },
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
                        other => return Err(AudioWriteError::InvalidArguments(format!("\"{}\" was given for specifying the sample format", other)).into()),
                    },
                }),
            },
        };

        fmt_chunk.write(self.writer.clone())?;

        self.data_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"data")?);
        Ok(())
    }

    // T：我们要写入到 WAV 中的格式
    fn write_sample_to<S, T>(writer: &mut dyn Writer, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType,
          T: SampleType {
        for sample in frame.iter() {
            T::from(*sample).write_le(writer)?;
        }
        Ok(())
    }
    fn write_multiple_sample_to<S, T>(writer: &mut dyn Writer, frames: &Vec<Vec<S>>) -> Result<(), Box<dyn Error>>
    where S: SampleType,
          T: SampleType {
        for frame in frames.iter() {
            for sample in frame.iter() {
                T::from(*sample).write_le(writer)?;
            }
        }
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_sample<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        if self.data_chunk.is_some() {
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                match self.sample_type {
                    S8  => Self::write_sample_to::<S, i8 >(writer, frame),
                    S16 => Self::write_sample_to::<S, i16>(writer, frame),
                    S24 => Self::write_sample_to::<S, i24>(writer, frame),
                    S32 => Self::write_sample_to::<S, i32>(writer, frame),
                    S64 => Self::write_sample_to::<S, i64>(writer, frame),
                    U8  => Self::write_sample_to::<S, u8 >(writer, frame),
                    U16 => Self::write_sample_to::<S, u16>(writer, frame),
                    U24 => Self::write_sample_to::<S, u24>(writer, frame),
                    U32 => Self::write_sample_to::<S, u32>(writer, frame),
                    U64 => Self::write_sample_to::<S, u64>(writer, frame),
                    F32 => Self::write_sample_to::<S, f32>(writer, frame),
                    F64 => Self::write_sample_to::<S, f64>(writer, frame),
                    other => Err(AudioWriteError::WrongSampleFormat(format!("{}", other)).into()),
                }
            })?;
            self.num_frames += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")).into())
        }
    }

    // 保存多个样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_multiple_sample<S>(&mut self, frames: &Vec<Vec<S>>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        if self.data_chunk.is_some() {
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                match self.sample_type {
                    S8  => Self::write_multiple_sample_to::<S, i8 >(writer, frames),
                    S16 => Self::write_multiple_sample_to::<S, i16>(writer, frames),
                    S24 => Self::write_multiple_sample_to::<S, i24>(writer, frames),
                    S32 => Self::write_multiple_sample_to::<S, i32>(writer, frames),
                    S64 => Self::write_multiple_sample_to::<S, i64>(writer, frames),
                    U8  => Self::write_multiple_sample_to::<S, u8 >(writer, frames),
                    U16 => Self::write_multiple_sample_to::<S, u16>(writer, frames),
                    U24 => Self::write_multiple_sample_to::<S, u24>(writer, frames),
                    U32 => Self::write_multiple_sample_to::<S, u32>(writer, frames),
                    U64 => Self::write_multiple_sample_to::<S, u64>(writer, frames),
                    F32 => Self::write_multiple_sample_to::<S, f32>(writer, frames),
                    F64 => Self::write_multiple_sample_to::<S, f64>(writer, frames),
                    other => Err(AudioWriteError::WrongSampleFormat(format!("{}", other)).into()),
                }
            })?;
            self.num_frames += frames.len() as u64;
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
    pub fn add_junk_chunk(&mut self, chunk: &JunkChunk) {
        self.junk_chunks.push(chunk.clone());
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // 结束对 data 块的写入
        self.data_chunk = None;
        
        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.smpl_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.inst_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.cue__chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.list_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.acid_chunk { chunk.write(self.writer.clone())?; }

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
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                write_str(writer, &chunk)?;
                Ok(())
            })?;
            cw.end()?;
        }

        // 写入所有的 JUNK 块
        for junk in self.junk_chunks.iter() {
            junk.write(self.writer.clone())?;
        }

        // 接下来是重点：判断文件大小是不是超过了 4GB，是的话，把文件头改为 RF64，然后在之前留坑的地方填入 RF64 的信息表
        self.riff_chunk = None;

        use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            let file_end_pos = writer.stream_position()?;
            let mut change_to_4gb_hreader = || -> Result<(), Box<dyn Error>> {
                writer.seek(SeekFrom::Start(0))?;
                writer.write_all(b"RF64")?;
                0xFFFFFFFFu32.write_le(writer)?;
                writer.write_all(b"WAVE")?;
                writer.write_all(b"ds64")?;
                28u32.write_le(writer)?; // ds64 段的长度
                let riff_size = file_end_pos - 8;
                let data_size = self.num_frames * self.frame_size as u64;
                let sample_count = self.num_frames / self.spec.channels as u64;
                riff_size.write_le(writer)?;
                data_size.write_le(writer)?;
                sample_count.write_le(writer)?;
                0u32.write_le(writer)?; // table length
                Ok(())
            };
            match self.file_size_option {
                FileSizeOption::NeverLargerThan4GB => {
                    if file_end_pos > 0xFFFFFFFFu64 {
                        Err(AudioWriteError::NotPreparedFor4GBFile.into())
                    } else {
                        Ok(())
                    }
                },
                FileSizeOption::AllowLargerThan4GB => {
                    if file_end_pos > 0xFFFFFFFFu64 {
                        change_to_4gb_hreader()
                    } else {
                        Ok(())
                    }
                },
                FileSizeOption::ForceUse4GBFormat => {
                    change_to_4gb_hreader()
                },
            }
        })?;
        Ok(())
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        self.finalize().unwrap();
    }
}
