#![allow(non_snake_case)]
#![allow(unused_imports)]

use std::{fs::File, io::{BufWriter, SeekFrom}, path::Path};

use crate::AudioWriteError;
use crate::{DataFormat, AdpcmSubFormat, Spec, SampleFormat};
use crate::{ChunkWriter};
use crate::FmtChunk;
use crate::{BextChunk, SmplChunk, InstChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::{Encoder, PcmEncoder, AdpcmEncoderWrap};
use crate::{EncIMA, EncMS};
use crate::{StringCodecMaps, SavageStringCodecs};
use crate::{SampleType};
use crate::{Writer, string_io::*};
use crate::WaveReader;
use crate::hacks;

#[cfg(feature = "mp3enc")]
use crate::encoders::MP3::Mp3Encoder;

// 你以为 WAV 文件只能在 4GB 以内吗？
#[derive(Debug)]
pub enum FileSizeOption{
    NeverLargerThan4GB,
    AllowLargerThan4GB,
    ForceUse4GBFormat,
}

#[derive(Debug)]
pub struct WaveWriter<'a> {
    writer: Box<dyn Writer + 'a>,
    spec: Spec,
    data_format: DataFormat,
    file_size_option: FileSizeOption,
    fmt_chunk_offset: u64,
    fact_chunk_offset: u64,
    num_frames_written: u64,
    data_offset: u64,
    encoder: Encoder,
    text_encoding: StringCodecMaps,
    riff_chunk: Option<ChunkWriter<'a>>,
    data_chunk: Option<ChunkWriter<'a>>,
    pub fmt__chunk: FmtChunk,
    pub bext_chunk: Option<BextChunk>,
    pub smpl_chunk: Option<SmplChunk>,
    pub inst_chunk: Option<InstChunk>,
    pub cue__chunk: Option<CueChunk>,
    pub axml_chunk: Option<String>,
    pub ixml_chunk: Option<String>,
    pub list_chunk: Option<ListChunk>,
    pub acid_chunk: Option<AcidChunk>,
    pub trkn_chunk: Option<String>,
    pub id3__chunk: Option<Id3::Tag>,
    pub junk_chunks: Vec<JunkChunk>,
    finalized: bool,
}

impl<'a> WaveWriter<'a> {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter<'a>, AudioWriteError> {
        let file_writer = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(Box::new(file_writer), spec, data_format, file_size_option)?;
        Ok(wave_writer)
    }

    pub fn from(writer: Box<dyn Writer + 'a>, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter<'a>, AudioWriteError> {
        use DataFormat::{Pcm, Adpcm, Mp3, OggVorbis, Flac};
        let encoder = match data_format {
            Pcm => {
                spec.verify_for_pcm()?;
                Encoder::new(Box::new(PcmEncoder::new(spec.sample_rate, spec.get_sample_type())?))
            },
            Adpcm(sub_format) => {
                use AdpcmSubFormat::{Ima, Ms};
                match sub_format {
                    Ima => Encoder::new(Box::new(AdpcmEncoderWrap::<EncIMA>::new(spec.channels, spec.sample_rate)?)),
                    Ms => Encoder::new(Box::new(AdpcmEncoderWrap::<EncMS>::new(spec.channels, spec.sample_rate)?)),
                }
            },
            Mp3 => {
                Encoder::new(Box::new(Mp3Encoder::<f32>::new(spec.channels as u8, spec.sample_rate, None, None, None, None)?))
            },
            other => return Err(AudioWriteError::Unsupported(format!("{:?}", other))),
        };
        let mut ret = Self{
            writer,
            spec: *spec,
            data_format,
            file_size_option,
            fmt_chunk_offset: 0,
            fact_chunk_offset: 0,
            num_frames_written: 0,
            data_offset: 0,
            encoder,
            text_encoding: StringCodecMaps::new(),
            fmt__chunk: FmtChunk::new(),
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
            id3__chunk: None,
            junk_chunks: Vec::<JunkChunk>::new(),
            finalized: false,
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), AudioWriteError> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(hacks::force_borrow!(*self.writer, dyn Writer), b"RIFF")?);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        self.writer.write_all(b"WAVE")?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => (),
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                let cw = ChunkWriter::begin(&mut self.writer, b"JUNK")?;
                cw.writer.write_all(&[0u8; 28])?;
                cw.end()?;
            },
        }

        // 使用编码器的 `new_fmt_chunk()` 生成 fmt 块的内容
        self.fmt__chunk = self.encoder.new_fmt_chunk(self.spec.channels, self.spec.sample_rate, self.spec.bits_per_sample, match self.spec.is_channel_mask_valid() {
            true => Some(self.spec.channel_mask),
            false => None
        })?;

        // 此处存储 fmt 块
        let mut cw = ChunkWriter::begin(&mut self.writer, b"fmt ")?;
        // 此处获取 fmt 块的位置，以便于其中有数据变动的时候可以更新。
        self.fmt_chunk_offset = cw.writer.stream_position()?;
        self.fmt__chunk.write(&mut cw.writer)?;
        cw.end()?;

        // 此处为 fact 块留出空间，之后要来这里修改的。
        let mut cw = ChunkWriter::begin(&mut self.writer, b"fact")?;
        // 此处获取 fact 块的位置
        self.fact_chunk_offset = cw.writer.stream_position()?;
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                0u32.write_le(&mut cw.writer)?;
            },
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                0u64.write_le(&mut cw.writer)?;
            },
        }
        cw.end()?;

        self.data_chunk = Some(ChunkWriter::begin(hacks::force_borrow!(*self.writer, dyn Writer), b"data")?);
        self.data_offset = self.data_chunk.as_ref().unwrap().get_chunk_start_pos();

        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_samples<S>(&mut self, samples: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_samples(&mut self.writer, samples)?;
            self.num_frames_written += (samples.len() / self.spec.channels as usize) as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 单声道保存一个样本。频繁调用会非常慢。
    pub fn write_mono<S>(&mut self, mono: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 1 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write mono audio to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_mono(&mut self.writer, mono)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 单声道批量保存样本。
    pub fn write_monos<S>(&mut self, monos: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 1 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write mono audio to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_multiple_mono(&mut self.writer, monos)?;
            self.num_frames_written += monos.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 保存一个立体声样本，频繁调用会非常慢。
    pub fn write_stereo<S>(&mut self, stereo: (S, S)) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 2 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write stereo audio to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_stereo(&mut self.writer, stereo)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 批量保存立体声样本。
    pub fn write_stereos<S>(&mut self, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 2 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write stereo audio to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_multiple_stereos(&mut self.writer, stereos)?;
            self.num_frames_written += stereos.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 保存两个单声道（也就是一个立体声）的样本，频繁调用会非常慢。
    pub fn write_dual_mono<S>(&mut self, mono1: S, mono2: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 2 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write dual mono to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_dual_mono(&mut self.writer, mono1, mono2)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 保存两个单声道（也就是一个立体声）的批量样本。
    pub fn write_dual_monos<S>(&mut self, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if mono1.len() != mono2.len() {
                return Err(AudioWriteError::MultipleMonosAreNotSameSize);
            }
            if self.spec.channels != 2 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write dual mono to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_multiple_dual_mono(&mut self.writer, mono1, mono2)?;
            self.num_frames_written += mono1.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 保存一个音频帧，频繁调用会非常慢。支持多种声道格式。
    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != frame.len() as u16 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write {} channel audio to {} channels audio file.", frame.len(), self.spec.channels)));
            }
            self.encoder.write_frame(&mut self.writer, frame)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 批量保存音频帧。支持多种声道格式。
    pub fn write_frames<S>(&mut self, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_multiple_frames(&mut self.writer, frames, self.spec.channels)?;
            self.num_frames_written += frames.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    pub fn spec(&self) -> &Spec{
        &self.spec
    }
    pub fn get_data_format(&self) -> DataFormat {
        self.data_format
    }
    pub fn get_num_frames_written(&self) -> u64 {
        self.num_frames_written
    }
    pub fn set_bext_chunk(&mut self, chunk: &BextChunk) {
        self.bext_chunk = Some(chunk.clone());
    }
    pub fn set_smpl_chunk(&mut self, chunk: &SmplChunk) {
        self.smpl_chunk = Some(chunk.clone());
    }
    pub fn set_inst_chunk(&mut self, chunk: &InstChunk) {
        self.inst_chunk = Some(*chunk);
    }
    pub fn set_cue__chunk(&mut self, chunk: &CueChunk) {
        self.cue__chunk = Some(chunk.clone());
    }
    pub fn set_axml_chunk(&mut self, chunk: &String) {
        self.axml_chunk = Some(chunk.to_owned());
    }
    pub fn set_ixml_chunk(&mut self, chunk: &String) {
        self.ixml_chunk = Some(chunk.to_owned());
    }
    pub fn set_list_chunk(&mut self, chunk: &ListChunk) {
        self.list_chunk = Some(chunk.clone());
    }
    pub fn set_acid_chunk(&mut self, chunk: &AcidChunk) {
        self.acid_chunk = Some(chunk.clone());
    }
    pub fn set_trkn_chunk(&mut self, chunk: &String) {
        self.trkn_chunk = Some(chunk.to_owned());
    }
    pub fn add_junk_chunk(&mut self, chunk: &JunkChunk) {
        self.junk_chunks.push(chunk.clone());
    }

    // 从读取器那里迁移乐曲信息的元数据。但是不迁移 JUNK 块。
    pub fn migrate_metadata_from_reader(&mut self, reader: &WaveReader) {
        if reader.get_bext_chunk().is_some() {self.bext_chunk = reader.get_bext_chunk().clone();}
        if reader.get_smpl_chunk().is_some() {self.smpl_chunk = reader.get_smpl_chunk().clone();}
        if reader.get_inst_chunk().is_some() {self.inst_chunk = reader.get_inst_chunk().clone();}
        if reader.get_cue__chunk().is_some() {self.cue__chunk = reader.get_cue__chunk().clone();}
        if reader.get_axml_chunk().is_some() {self.axml_chunk = reader.get_axml_chunk().clone();}
        if reader.get_ixml_chunk().is_some() {self.ixml_chunk = reader.get_ixml_chunk().clone();}
        if reader.get_list_chunk().is_some() {self.list_chunk = reader.get_list_chunk().clone();}
        if reader.get_acid_chunk().is_some() {self.acid_chunk = reader.get_acid_chunk().clone();}
        if reader.get_trkn_chunk().is_some() {self.trkn_chunk = reader.get_trkn_chunk().clone();}
        if reader.get_id3__chunk().is_some() {self.id3__chunk = reader.get_id3__chunk().clone();}
    }

    pub fn finalize(&mut self) -> Result<(), AudioWriteError> {
        if self.finalized {
            return Ok(());
        }

        // 完成对 data 最后内容的写入，同时更新 fmt 块的一些数据
        self.encoder.finalize(&mut self.writer)?;

        // 结束写入 data
        if self.data_chunk.is_some() {
            self.data_chunk = None;
        }

        // 记录 data 末尾的位置
        let end_of_data = self.writer.stream_position()?;

        // 找到 fmt 块
        self.writer.seek(SeekFrom::Start(self.fmt_chunk_offset))?;

        // 更新 fmt 头部信息，重新写入 fmt 头部
        self.encoder.update_fmt_chunk(&mut self.fmt__chunk)?;
        self.fmt__chunk.write(&mut self.writer)?;

        // 写完 fmt 后，还要更新 fact 的数据
        self.writer.seek(SeekFrom::Start(self.fact_chunk_offset))?;
        let mut fact_data = self.num_frames_written * self.spec.channels as u64;
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                if fact_data >= 0xFFFFFFFF {
                    fact_data = 0xFFFFFFFF;
                } 
                (fact_data as u32).write_le(&mut self.writer)?;
            },
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                fact_data.write_le(&mut self.writer)?;
            },
        }

        // 回到 data 末尾的位置
        self.writer.seek(SeekFrom::Start(end_of_data))?;

        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk { chunk.write(&mut self.writer, &self.text_encoding)?; }
        if let Some(chunk) = &self.smpl_chunk { chunk.write(&mut self.writer)?; }
        if let Some(chunk) = &self.inst_chunk { chunk.write(&mut self.writer)?; }
        if let Some(chunk) = &self.cue__chunk { chunk.write(&mut self.writer)?; }
        if let Some(chunk) = &self.list_chunk { chunk.write(&mut self.writer, &self.text_encoding)?; }
        if let Some(chunk) = &self.acid_chunk { chunk.write(&mut self.writer)?; }
        if let Some(chunk) = &self.id3__chunk {
            let mut cw = ChunkWriter::begin(&mut self.writer, b"id3 ")?;
            Id3::id3_write(&chunk, &mut cw.writer)?;
            cw.end()?;
        }

        // 写入其它全部的字符串块
        let mut string_chunks_to_write = Vec::<([u8; 4], &String)>::new();
        if let Some(chunk) = &self.axml_chunk {
            string_chunks_to_write.push((*b"axml", chunk));
        }
        if let Some(chunk) = &self.ixml_chunk {
            string_chunks_to_write.push((*b"ixml", chunk));
        }
        if let Some(chunk) = &self.trkn_chunk {
            string_chunks_to_write.push((*b"Trkn", chunk));
        }
        for (flag, chunk) in string_chunks_to_write.iter() {
            let mut cw = ChunkWriter::begin(&mut self.writer, flag)?;
            write_str(&mut cw.writer, chunk, &self.text_encoding)?;
            cw.end()?;
        }

        // 写入所有的 JUNK 块
        for junk in self.junk_chunks.iter() {
            junk.write(&mut self.writer)?;
        }

        // 接下来是重点：判断文件大小是不是超过了 4GB，是的话，把文件头改为 RF64，然后在之前留坑的地方填入 RF64 的信息表
        self.riff_chunk = None;

        let file_end_pos = self.writer.stream_position()?;
        let mut change_to_4gb_hreader = || -> Result<(), AudioWriteError> {
            self.writer.seek(SeekFrom::Start(0))?;
            self.writer.write_all(b"RF64")?;
            0xFFFFFFFFu32.write_le(&mut self.writer)?;
            self.writer.write_all(b"WAVE")?;
            self.writer.write_all(b"ds64")?;
            28u32.write_le(&mut self.writer)?; // ds64 段的长度
            let riff_size = file_end_pos - 8;
            let data_size = self.num_frames_written * self.block_size as u64;
            let sample_count = self.num_frames_written / self.spec.channels as u64;
            riff_size.write_le(&mut self.writer)?;
            data_size.write_le(&mut self.writer)?;
            sample_count.write_le(&mut self.writer)?;
            0u32.write_le(&mut self.writer)?; // table length
            Ok(())
        };
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                if file_end_pos > 0xFFFFFFFFu64 {
                    Err(AudioWriteError::NotPreparedFor4GBFile)?;
                }
            },
            FileSizeOption::AllowLargerThan4GB => {
                if file_end_pos > 0xFFFFFFFFu64 {
                    change_to_4gb_hreader()?;
                }
            },
            FileSizeOption::ForceUse4GBFormat => {
                change_to_4gb_hreader()?;
            },
        }
        self.writer.flush()?;
        self.finalized = true;
        Ok(())
    }
}

impl Drop for WaveWriter<'_> {
    fn drop(&mut self) {
        match self.finalize() {
            Ok(_) => (),
            Err(e) => println!("On `WaveWriter::drop()`: {:?}", e),
        }
    }
}
