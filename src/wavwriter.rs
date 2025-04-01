#![allow(non_snake_case)]
#![allow(dead_code)]

use std::{fs::File, io::{self, BufWriter, SeekFrom}, path::Path};

use crate::errors::AudioWriteError;
use crate::wavcore::{DataFormat, Spec, SampleFormat, WaveSampleType};
use crate::wavcore::{GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
use crate::wavcore::{ChunkWriter};
use crate::wavcore::{FmtChunk, FmtChunkExtension, BextChunk, SmplChunk, InstChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::encoders::{Encoder, PcmEncoder};
use crate::savagestr::{StringCodecMaps, SavageStringCodecs};
use crate::sampleutils::{SampleType};
use crate::readwrite::{SharedWriter, string_io::*};
use crate::wavreader::WaveReader;

// 你以为 WAV 文件只能在 4GB 以内吗？
#[derive(Debug)]
pub enum FileSizeOption{
    NeverLargerThan4GB,
    AllowLargerThan4GB,
    ForceUse4GBFormat,
}

#[derive(Debug)]
pub struct WaveWriter {
    writer: SharedWriter,
    spec: Spec,
    data_format: DataFormat,
    file_size_option: FileSizeOption,
    num_frames: u64,
    frame_size: u16,
    data_offset: u64,
    sample_type: WaveSampleType,
    sample_packer: Encoder,
    text_encoding: Box<dyn SavageStringCodecs>,
    riff_chunk: Option<ChunkWriter>,
    data_chunk: Option<ChunkWriter>,
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
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter, AudioWriteError> {
        let file_reader = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(SharedWriter::new(file_reader), spec, data_format, file_size_option)?;
        Ok(wave_writer)
    }

    pub fn from(writer: SharedWriter, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter, AudioWriteError> {
        use DataFormat::{Pcm, Mp3, OggVorbis, Flac};
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let sample_type = spec.get_sample_type();
        let sample_packer = match data_format {
            Pcm => {
                spec.verify_for_pcm()?;
                Encoder::new(Box::new(PcmEncoder::new(sample_type)?))
            },
            other => return Err(AudioWriteError::Unsupported(format!("{:?}", other))),
        };
        let mut ret = Self{
            writer: writer.clone(),
            spec: *spec,
            data_format,
            file_size_option,
            num_frames: 0,
            frame_size,
            data_offset: 0,
            sample_type,
            sample_packer,
            text_encoding: Box::new(StringCodecMaps::new()),
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
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), AudioWriteError> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"RIFF")?);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        self.writer.escorted_write(|writer| -> Result<(), io::Error> {
            writer.write_all(b"WAVE")?;
            Ok(())
        })?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => (),
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
                self.writer.escorted_write(|writer| -> Result<(), io::Error> {
                    writer.write_all(&[0u8; 28])?;
                    Ok(())
                })?;
                cw.end()?;
            },
        }

        // 准备写入 fmt 块
        if self.spec.channel_mask == 0 {
            self.spec.guess_channel_mask()?;
        }

        // 如果声道掩码不等于猜测的声道掩码，则说明需要 0xFFFE 的扩展格式
        let mut ext = self.spec.channel_mask != self.spec.guess_channel_mask()?;
        ext |= self.spec.channels > 2;

        let fmt__chunk = FmtChunk {
            format_tag: match &self.data_format {
                DataFormat::Pcm => {
                    use SampleFormat::{Unknown, Float, UInt, Int};
                    match self.spec.sample_format {
                        Unknown => return Err(AudioWriteError::InvalidArguments("Please check `spec.sample_format` is not set to `Unknown`".to_owned())),
                        Int | UInt => {
                            match ext {
                                false => 1,
                                true => 0xFFFE,
                            }
                        },
                        Float => {
                            match ext {
                                false => 3,
                                true => {
                                    match self.spec.bits_per_sample {
                                        32 | 64 => 0xFFFE,
                                        other => return Err(AudioWriteError::InvalidArguments(format!("Could not save {} bits IEEE float numbers.", other))),
                                    }
                                },
                            }
                        },
                    }
                },
                DataFormat::Mp3 => {
                    ext = false;
                    0x00FF
                },
                DataFormat::OggVorbis => {
                    ext = false;
                    0x674f
                },
                DataFormat::Flac => {
                    ext = false;
                    0xF1AC
                }
            },
            channels: self.spec.channels,
            sample_rate: self.spec.sample_rate,
            byte_rate: self.spec.sample_rate * self.frame_size as u32,
            block_align: self.frame_size,
            bits_per_sample: self.spec.bits_per_sample,
            extension: match ext {
                false => None,
                true => Some(FmtChunkExtension {
                    ext_len: 22,
                    valid_bits_per_sample: self.spec.bits_per_sample,
                    channel_mask: self.spec.channel_mask,
                    sub_format: match self.spec.sample_format {
                        Int | UInt => GUID_PCM_FORMAT,
                        Float => GUID_IEEE_FLOAT_FORMAT,
                        other => return Err(AudioWriteError::InvalidArguments(format!("\"{:?}\" was given for specifying the sample format", other))),
                    },
                }),
            },
        };

        fmt__chunk.write(self.writer.clone())?;

        self.data_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"data")?);
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.writer.escorted_write(|writer| -> Result<(), io::Error> {
                Ok(self.sample_packer.write_frame::<S>(writer, frame)?)
            })?;
            self.num_frames += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
        }
    }

    // 保存多个样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_multiple_frames<S>(&mut self, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.writer.escorted_write(|writer| -> Result<(), io::Error> {
                Ok(self.sample_packer.write_multiple_frames::<S>(writer, frames)?)
            })?;
            self.num_frames += frames.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")))
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
        // 结束对 data 块的写入
        self.data_chunk = None;
        
        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk { chunk.write(self.writer.clone(), &*self.text_encoding)?; }
        if let Some(chunk) = &self.smpl_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.inst_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.cue__chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.list_chunk { chunk.write(self.writer.clone(), &*self.text_encoding)?; }
        if let Some(chunk) = &self.acid_chunk { chunk.write(self.writer.clone())?; }

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
            let mut cw = ChunkWriter::begin(self.writer.clone(), flag)?;
            self.writer.escorted_write(|writer| -> Result<(), io::Error> {
                write_str(writer, chunk, &*self.text_encoding)?;
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

        self.writer.escorted_write(|writer| -> Result<(), AudioWriteError> {
            let file_end_pos = writer.stream_position()?;
            let mut change_to_4gb_hreader = || -> Result<(), AudioWriteError> {
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
                        Err(AudioWriteError::NotPreparedFor4GBFile)
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
