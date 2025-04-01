#![allow(dead_code)]

use std::{io::{self, Read, Write, SeekFrom}, collections::HashMap};

use crate::errors::{AudioError, AudioReadError, AudioWriteError};
use crate::readwrite::{Reader, Writer, SharedWriter, string_io::*};
use crate::sampleutils::SampleType;
use crate::savagestr::SavageStringCodecs;

// 你以为 WAV 就是用来存 PCM 的吗？
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum DataFormat{
    PCM_Int,
    PCM_Float,
    Mp3,
    OggVorbis,
    Flac,
}

#[derive(Clone, Copy, Debug)]
pub enum SampleFormat {
    Unknown,
    Float,
    UInt,
    Int,
}

impl std::fmt::Display for SampleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SampleFormat::Unknown => write!(f, "Unknown"),
            SampleFormat::Float => write!(f, "Floating Point Number"),
            SampleFormat::UInt => write!(f, "Unsigned Integer"),
            SampleFormat::Int => write!(f, "Integer"),
       }
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum WaveSampleType {
    Unknown,
    S8,
    S16,
    S24,
    S32,
    S64,
    U8,
    U16,
    U24,
    U32,
    U64,
    F32,
    F64,
}

impl std::fmt::Display for WaveSampleType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match self {
            Unknown => write!(f, "Unknown"),
            S8  => write!(f, "i8"),
            S16 => write!(f, "i16"),
            S24 => write!(f, "i24"),
            S32 => write!(f, "i32"),
            S64 => write!(f, "i64"),
            U8  => write!(f, "u8"),
            U16 => write!(f, "u16"),
            U24 => write!(f, "u24"),
            U32 => write!(f, "u32"),
            U64 => write!(f, "u64"),
            F32 => write!(f, "f32"),
            F64 => write!(f, "f64"),
       }
    }
}

impl WaveSampleType {
    pub fn sizeof(&self) -> u16 {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match self {
            S8 =>  1,
            S16 => 2,
            S24 => 3,
            S32 => 4,
            S64 => 8,
            U8 =>  1,
            U16 => 2,
            U24 => 3,
            U32 => 4,
            U64 => 8,
            F32 => 4,
            F64 => 8,
            Unknown => 0,
        }
    }
}


pub fn get_sample_type(bits_per_sample: u16, sample_format: SampleFormat) -> WaveSampleType {
    use SampleFormat::{UInt, Int, Float};
    use WaveSampleType::{Unknown, U8,S16,S24,S32,F32,F64};
    match (bits_per_sample, sample_format) {
        (8, UInt) => U8,
        (16, Int) => S16,
        (24, Int) => S24,
        (32, Int) => S32,
        (32, Float) => F32,
        (64, Float) => F64,
        // 上述的就是 PCM 能支持的所有格式。
        (_, _) => Unknown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub struct GUID (pub u32, pub u16, pub u16, pub [u8; 8]);

pub const GUID_PCM_FORMAT: GUID = GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
pub const GUID_IEEE_FLOAT_FORMAT: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
// TODO
// 其实还有：GUID_DRM、GUID_LAW、GUID_MULAW、GUID_ADPCM
// 视情况实现

impl GUID {
    pub fn read<T>(r: &mut T) -> Result<Self, io::Error>
    where T: Read {
        Ok( Self (
            u32::read_le(r)?,
            u16::read_le(r)?,
            u16::read_le(r)?,
            [
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
                u8::read_le(r)?,
            ]
        ))
    }

    pub fn write<T>(&self, w: &mut T) -> Result<(), io::Error>
    where T: Write + ?Sized {
        self.0.write_le(w)?;
        self.1.write_le(w)?;
        self.2.write_le(w)?;
        w.write_all(&self.3)?;
        Ok(())
    }
}


#[derive(Clone, Copy, Debug)]
pub enum SpeakerPosition {
    FrontLeft = 0x1,
    FrontRight = 0x2,
    FrontCenter = 0x4,
    LowFreq = 0x8,
    BackLeft = 0x10,
    BackRight = 0x20,
    FrontLeftOfCenter = 0x40,
    FrontRightOfCenter = 0x80,
    BackCenter = 0x100,
    SideLeft = 0x200,
    SideRight = 0x400,
    TopCenter = 0x800,
    TopFrontLeft = 0x1000,
    TopFrontCenter = 0x2000,
    TopFrontRight = 0x4000,
    TopBackLeft = 0x8000,
    TopBackCenter = 0x10000,
    TopBackRight = 0x20000,
}

impl std::fmt::Display for SpeakerPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SpeakerPosition::FrontLeft           => write!(f, "front_left"),
            SpeakerPosition::FrontRight          => write!(f, "front_right"),
            SpeakerPosition::FrontCenter         => write!(f, "front_center"),
            SpeakerPosition::LowFreq             => write!(f, "low_freq"),
            SpeakerPosition::BackLeft            => write!(f, "back_left"),
            SpeakerPosition::BackRight           => write!(f, "back_right"),
            SpeakerPosition::FrontLeftOfCenter   => write!(f, "front_left_of_center"),
            SpeakerPosition::FrontRightOfCenter  => write!(f, "front_right_of_center"),
            SpeakerPosition::BackCenter          => write!(f, "back_center"),
            SpeakerPosition::SideLeft            => write!(f, "side_left"),
            SpeakerPosition::SideRight           => write!(f, "side_right"),
            SpeakerPosition::TopCenter           => write!(f, "top_center"),
            SpeakerPosition::TopFrontLeft        => write!(f, "top_front_left"),
            SpeakerPosition::TopFrontCenter      => write!(f, "top_front_center"),
            SpeakerPosition::TopFrontRight       => write!(f, "top_front_right"),
            SpeakerPosition::TopBackLeft         => write!(f, "top_back_left"),
            SpeakerPosition::TopBackCenter       => write!(f, "top_back_center"),
            SpeakerPosition::TopBackRight        => write!(f, "top_back_right"),
        }
    }
}

// TODO
// 设计算法
// 对每一个声道来源设计一个向量值，相当于从人耳到声道来源的方向
// 每个声道值对每个声道值互相之间做点乘计算。
// 背对人耳的音源乘以一个衰减值（或者干脆听不见？）
// 全部声道加权混音，确保不会改变总音量。
// 这样就能实现任意的声道组合互相转换。

pub fn channel_mask_to_speaker_positions(channels: u16, channel_mask: u32) -> Result<Vec<SpeakerPosition>, AudioError> {
    let enums = [
        SpeakerPosition::FrontLeft,
        SpeakerPosition::FrontRight,
        SpeakerPosition::FrontCenter,
        SpeakerPosition::LowFreq,
        SpeakerPosition::BackLeft,
        SpeakerPosition::BackRight,
        SpeakerPosition::FrontLeftOfCenter,
        SpeakerPosition::FrontRightOfCenter,
        SpeakerPosition::BackCenter,
        SpeakerPosition::SideLeft,
        SpeakerPosition::SideRight,
        SpeakerPosition::TopCenter,
        SpeakerPosition::TopFrontLeft,
        SpeakerPosition::TopFrontCenter,
        SpeakerPosition::TopFrontRight,
        SpeakerPosition::TopBackLeft,
        SpeakerPosition::TopBackCenter,
        SpeakerPosition::TopBackRight,
    ];
    let mut ret = Vec::<SpeakerPosition>::new();
    for (i, m) in enums.iter().enumerate() {
        let m = *m as u32;
        if channel_mask & m == m {ret.push(enums[i]);}
    }
    if ret.len() == channels.into() {
        Ok(ret)
    } else {
        Err(AudioError::ChannelNotMatchMask)
    }
}

pub fn channel_mask_to_speaker_positions_desc(channels: u16, channel_mask: u32) -> Result<Vec<String>, AudioError> {
    match channel_mask_to_speaker_positions(channels, channel_mask) {
        Ok(v) => {
            let mut ret = Vec::with_capacity(v.len());
            for e in v.iter() {
                ret.push(format!("{:?}", e));
            }
            Ok(ret)
        },
        Err(e) => Err(e),
    }
}

pub fn guess_channel_mask(channels: u16) -> Result<u32, AudioError> {
    let mut mask = 0;
    for i in 0..channels {
        let bit = 1 << i;
        if bit > 0x20000 {
            return Err(AudioError::CantGuessChannelMask(channels));
        }
        mask |= bit;
    }
    Ok(mask)
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub channels: u16,
    pub channel_mask: u32,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

impl Default for Spec {
    fn default() -> Self {
        Self::new()
    }
}

impl Spec {
    pub fn new() -> Self {
        Self {
            channels: 0,
            channel_mask: 0,
            sample_rate: 0,
            bits_per_sample: 0,
            sample_format: SampleFormat::Unknown,
        }
    }

    pub fn get_sample_type(&self) -> WaveSampleType {
        get_sample_type(self.bits_per_sample, self.sample_format)
    }

    pub fn guess_channel_mask(&self) -> Result<u32, AudioError> {
        guess_channel_mask(self.channels)
    }

    pub fn channel_mask_to_speaker_positions(&self) -> Result<Vec<SpeakerPosition>, AudioError> {
        channel_mask_to_speaker_positions(self.channels, self.channel_mask)
    }

    pub fn channel_mask_to_speaker_positions_desc(&self) -> Result<Vec<String>, AudioError> {
        channel_mask_to_speaker_positions_desc(self.channels, self.channel_mask)
    }
}

#[derive(Clone)]
pub struct ChunkWriter {
    writer_shared: SharedWriter,
    flag: [u8; 4],
    pos_of_chunk_len: u64, // 写入 chunk 大小的地方
    chunk_start: u64, // chunk 数据开始的地方
    ended: bool, // 是否早已完成 Chunk 的写入
}

impl std::fmt::Debug for ChunkWriter {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.debug_struct("ChunkWriter")
            .field("writer_shared", &self.writer_shared)
            .field("flag", &format!("{}", String::from_utf8_lossy(&self.flag)))
            .field("pos_of_chunk_len", &self.pos_of_chunk_len)
            .field("chunk_start", &self.chunk_start)
            .field("ended", &self.ended)
            .finish()
    }
}

impl ChunkWriter {
    // 开始写入 Chunk，此时写入 Chunk Flag，然后记录 Chunk Size 的位置，以及 Chunk 数据的位置。
    pub fn begin(writer_shared: SharedWriter, flag: &[u8; 4]) -> Result<Self, AudioWriteError> {
        let mut pos_of_chunk_len = 0u64;
        let mut chunk_start = 0u64;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            writer.write_all(flag)?;
            pos_of_chunk_len = writer.stream_position()?;
            0u32.write_le(writer)?;
            chunk_start = writer.stream_position()?;
            Ok(())
        })?;
        Ok(Self{
            flag: *flag,
            writer_shared: writer_shared.clone(),
            pos_of_chunk_len,
            chunk_start,
            ended: false,
        })
    }

    // 结束写入 Chunk，更新 Chunk Size
    pub fn end(&mut self) -> Result<(), AudioWriteError> {
        if self.ended {
            Ok(())
        } else {
            // 此处存在一个情况：块的大小是 u32 类型，但是如果实际写入的块的大小超过这个大小就会存不下。
            // 对于 RIFF 和 data 段，因为 ds64 段里专门留了字段用于存储 u64 的块大小，所以这里写入 0xFFFFFFFF。
            // 对于其它段，其实 ds64 的 table 字段就是专门为其它段提供 u64 的块大小的存储的，但是你要增加 table 的数量，就会增加 ds64 段的长度
            // 目前不考虑预留长度给 ds64 用于存储长度过长的段。非 RIFF、RF64、data 的段不允许超过 0xFFFFFFFF 大小，否则报错。
            let mut chunk_size = self.get_chunk_data_size()?;
            if chunk_size >= 0xFFFFFFFFu64 {
                match &self.flag {
                    b"RIFF" | b"data" => {
                        chunk_size = 0xFFFFFFFF;
                    },
                    other => {
                        let chunk_flag = String::from_utf8_lossy(other);
                        return Err(AudioWriteError::ChunkSizeTooBig(format!("{} is 0x{:x} bytes long.", chunk_flag, chunk_size)).into());
                    },
                }
            }
            Ok(self.end_and_write_size(chunk_size as u32)?)
        }
    }

    // 放弃写入 Chunk，写一个假的 Chunk 大小
    pub fn end_and_write_size(&mut self, chunk_size_to_write: u32) -> Result<(), AudioWriteError> {
        if self.ended {
            Ok(())
        } else {
            Ok(self.writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                let end_of_chunk = writer.stream_position()?;
                writer.seek(SeekFrom::Start(self.pos_of_chunk_len))?;
                chunk_size_to_write.write_le(writer)?;
                writer.seek(SeekFrom::Start(end_of_chunk))?;
                if end_of_chunk & 1 > 0 {
                    0u8.write_le(writer)?;
                }
                self.ended = true;
                Ok(())
            })
        }
    }

    // 取得 Chunk 的数据部分的开始位置
    pub fn get_chunk_start_pos(&self) -> u64 {
        self.chunk_start
    }

    // 取得 Chunk 数据当前写入的大小
    pub fn get_chunk_data_size(&mut self) -> Result<u64, AudioWriteError> {
        Ok(self.writer_shared.escorted_work::<u64, _, io::Error>(|writer| -> Result<u64, io::Error> {
            self.get_chunk_data_size_priv(writer)
        })?)
    }

    fn get_chunk_data_size_priv(&self, writer: &mut dyn Writer) -> Result<u64, io::Error> {
        Ok(writer.stream_position()? - self.get_chunk_start_pos())
    }
}

impl Drop for ChunkWriter {
    fn drop(&mut self) {
        self.end().unwrap();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ChunkHeader {
    pub flag: [u8; 4], // 实际存储在文件里的
    pub size: u32, // 实际存储在文件里的
    pub chunk_start_pos: u64, // Chunk 内容在文件中的位置，不包含 Chunk 头
}

impl ChunkHeader {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        // 读取 WAV 中的每个块
        let mut flag = [0u8; 4];
        reader.read_exact(&mut flag)?;
        Ok(Self {
            flag,
            size: u32::read_le(reader)?,
            chunk_start_pos: reader.stream_position()?
        })
    }

    pub fn align(addr: u64) -> u64 {
        addr + (addr & 1)
    }

    pub fn next_chunk_pos(&self) -> u64 {
        Self::align(self.chunk_start_pos + self.size as u64)
    }

    pub fn seek_to_next_chunk<R>(&self, reader: &mut R) -> Result<u64, AudioReadError>
    where R: Reader {
        Ok(reader.seek(SeekFrom::Start(self.next_chunk_pos()))?)
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct FmtChunk {
    pub format_tag: u16, // https://github.com/tpn/winsdk-10/blob/master/Include/10.0.14393.0/shared/mmreg.h
    pub channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extension: Option<FmtChunkExtension>,
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct FmtChunkExtension {
    pub ext_len: u16,
    pub valid_bits_per_sample: u16,
    pub channel_mask: u32,
    pub sub_format: GUID,
}

impl FmtChunk {
    pub fn read<R>(reader: &mut R, chunk_size: u32) -> Result<Self, AudioReadError>
    where R: Reader {
        let mut ret = FmtChunk{
            format_tag: u16::read_le(reader)?,
            channels: u16::read_le(reader)?,
            sample_rate: u32::read_le(reader)?,
            byte_rate: u32::read_le(reader)?,
            block_align: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            extension: None,
        };
        match ret.format_tag {
            0xFFFE => {
                if chunk_size >= 40 {
                    ret.extension = Some(FmtChunkExtension::read(reader)?);
                }
            },
            _ => (),
        }
        Ok(ret)
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"fmt ")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            self.format_tag.write_le(writer)?;
            self.channels.write_le(writer)?;
            self.sample_rate.write_le(writer)?;
            self.byte_rate.write_le(writer)?;
            self.block_align.write_le(writer)?;
            self.bits_per_sample.write_le(writer)?;
            if let Some(ext) = self.extension {
                ext.write(writer)?;
            }
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }

    pub fn get_sample_format(&self) -> SampleFormat {
        use SampleFormat::{Int, UInt, Float, Unknown};
        match (self.format_tag, self.bits_per_sample) {
            (1, 8) => UInt,
            (1, 16) => Int,
            (1, 24) => Int,
            (1, 32) => Int,
            (1, 64) => Int,
            (0xFFFE, 8) => UInt,
            (0xFFFE, 16) => Int,
            (0xFFFE, 24) => Int,
            (0xFFFE, 32) | (0xFFFE, 64) => {
                match self.extension {
                    Some(extension) => {
                        match extension.sub_format {
                            GUID_PCM_FORMAT => Int,
                            GUID_IEEE_FLOAT_FORMAT => Float,
                            _ => Unknown, // 由插件系统判断
                        }
                    },
                    None => Int,
                }
            },
            (3, 32) => Float,
            (3, 64) => Float,
            (_, _) => Unknown, // 由插件系统判断
        }
    }

    pub fn get_sample_type(&self) -> WaveSampleType {
        get_sample_type(self.bits_per_sample, self.get_sample_format())
    }
}

impl FmtChunkExtension {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        Ok(Self{
            ext_len: u16::read_le(reader)?,
            valid_bits_per_sample: u16::read_le(reader)?,
            channel_mask: u32::read_le(reader)?,
            sub_format: GUID::read(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.ext_len.write_le(writer)?;
        self.valid_bits_per_sample.write_le(writer)?;
        self.channel_mask.write_le(writer)?;
        self.sub_format.write(writer)?;
        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct BextChunk {
    pub description: String,
    pub originator: String,
    pub originator_ref: String,
    pub origination_date: String,
    pub origination_time: String,
    pub time_ref: u64,
    pub version: u16,
    pub umid: [u8; 64],
    pub reserved: [u8; 190],
    pub coding_history: [u8; 1],
}

impl BextChunk {
    pub fn read<R>(reader: &mut R, text_encoding: &dyn SavageStringCodecs) -> Result<Self, AudioReadError>
    where R: Reader {
        let description = read_str(reader, 256, text_encoding)?;
        let originator = read_str(reader, 32, text_encoding)?;
        let originator_ref = read_str(reader, 32, text_encoding)?;
        let origination_date = read_str(reader, 10, text_encoding)?;
        let origination_time = read_str(reader, 8, text_encoding)?;
        let time_ref = u64::read_le(reader)?;
        let version = u16::read_le(reader)?;
        let mut umid = [0u8; 64];
        let mut reserved = [0u8; 190];
        let mut coding_history = [0u8; 1];
        reader.read_exact(&mut umid)?;
        reader.read_exact(&mut reserved)?;
        reader.read_exact(&mut coding_history)?;
        Ok(Self {
            description,
            originator,
            originator_ref,
            origination_date,
            origination_time,
            time_ref,
            version,
            umid,
            reserved,
            coding_history,
        })
    }

    pub fn write(&self, writer_shared: SharedWriter, text_encoding: &dyn SavageStringCodecs) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"bext")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            write_str_sized(writer, &self.description, 256, text_encoding)?;
            write_str_sized(writer, &self.originator, 32, text_encoding)?;
            write_str_sized(writer, &self.originator_ref, 32, text_encoding)?;
            write_str_sized(writer, &self.origination_date, 10, text_encoding)?;
            write_str_sized(writer, &self.origination_time, 8, text_encoding)?;
            self.time_ref.write_le(writer)?;
            self.version.write_le(writer)?;
            writer.write_all(&self.umid)?;
            writer.write_all(&self.reserved)?;
            writer.write_all(&self.coding_history)?;
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SmplChunk {
    pub manufacturer: u32,
    pub product: u32,
    pub sample_period: u32,
    pub midi_unity_note: u32,
    pub midi_pitch_fraction: u32,
    pub smpte_format: u32,
    pub smpte_offset: u32,
    pub num_sample_loops: u32,
    pub sampler_data: u32,
    pub loops: Vec<SmplSampleLoop>,
}

#[derive(Debug, Clone, Copy)]
pub struct SmplSampleLoop {
    pub identifier: u32,
    pub type_: u32,
    pub start: u32,
    pub end: u32,
    pub fraction: u32,
    pub play_count: u32,
}

impl SmplChunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        let mut ret = Self{
            manufacturer: u32::read_le(reader)?,
            product: u32::read_le(reader)?,
            sample_period: u32::read_le(reader)?,
            midi_unity_note: u32::read_le(reader)?,
            midi_pitch_fraction: u32::read_le(reader)?,
            smpte_format: u32::read_le(reader)?,
            smpte_offset: u32::read_le(reader)?,
            num_sample_loops: u32::read_le(reader)?,
            sampler_data: u32::read_le(reader)?,
            loops: Vec::<SmplSampleLoop>::new(),
        };
        for _ in 0..ret.num_sample_loops {
            ret.loops.push(SmplSampleLoop::read(reader)?);
        }
        Ok(ret)
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"smpl")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            self.manufacturer.write_le(writer)?;
            self.product.write_le(writer)?;
            self.sample_period.write_le(writer)?;
            self.midi_unity_note.write_le(writer)?;
            self.midi_pitch_fraction.write_le(writer)?;
            self.smpte_format.write_le(writer)?;
            self.smpte_offset.write_le(writer)?;
            self.num_sample_loops.write_le(writer)?;
            self.sampler_data.write_le(writer)?;
            for l in self.loops.iter() {
                l.write(writer)?;
            }
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

impl SmplSampleLoop {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        Ok(Self{
            identifier: u32::read_le(reader)?,
            type_: u32::read_le(reader)?,
            start: u32::read_le(reader)?,
            end: u32::read_le(reader)?,
            fraction: u32::read_le(reader)?,
            play_count: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.identifier.write_le(writer)?;
        self.type_.write_le(writer)?;
        self.start.write_le(writer)?;
        self.end.write_le(writer)?;
        self.fraction.write_le(writer)?;
        self.play_count.write_le(writer)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InstChunk {
    pub base_note: u8,
    pub detune: u8,
    pub gain: u8,
    pub low_note: u8,
    pub high_note: u8,
    pub low_velocity: u8,
    pub high_velocity: u8,
}

impl InstChunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        Ok(Self{
            base_note: u8::read_le(reader)?,
            detune: u8::read_le(reader)?,
            gain: u8::read_le(reader)?,
            low_note: u8::read_le(reader)?,
            high_note: u8::read_le(reader)?,
            low_velocity: u8::read_le(reader)?,
            high_velocity: u8::read_le(reader)?,
        })
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"INST")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            self.base_note.write_le(writer)?;
            self.detune.write_le(writer)?;
            self.gain.write_le(writer)?;
            self.low_note.write_le(writer)?;
            self.high_note.write_le(writer)?;
            self.low_velocity.write_le(writer)?;
            self.high_velocity.write_le(writer)?;
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct CueChunk {
    pub num_cues: u32,
    pub cues: Vec<Cue>,
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct Cue {
    pub identifier: u32,
    pub order: u32,
    pub chunk_id: u32,
    pub chunk_start: u32,
    pub block_start: u32,
    pub offset: u32,
}

impl CueChunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        let mut ret = CueChunk {
            num_cues: u32::read_le(reader)?,
            cues: Vec::<Cue>::new(),
        };
        for _ in 0..ret.num_cues {
            ret.cues.push(Cue::read(reader)?);
        }
        Ok(ret)
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"cue ")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            self.num_cues.write_le(writer)?;
            for cue in self.cues.iter() {
                cue.write(writer)?;
            }
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

impl Cue {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        Ok(Self{
            identifier: u32::read_le(reader)?,
            order: u32::read_le(reader)?,
            chunk_id: u32::read_le(reader)?,
            chunk_start: u32::read_le(reader)?,
            block_start: u32::read_le(reader)?,
            offset: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.identifier.write_le(writer)?;
        self.order.write_le(writer)?;
        self.chunk_id.write_le(writer)?;
        self.chunk_start.write_le(writer)?;
        self.block_start.write_le(writer)?;
        self.offset.write_le(writer)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ListChunk {
    Info(HashMap<String, String>),
    Adtl(Vec<AdtlChunk>),
}

impl ListChunk {
    pub fn read<R>(reader: &mut R, chunk_size: u64, text_encoding: &dyn SavageStringCodecs) -> Result<Self, AudioReadError>
    where R: Reader {
        let end_of_chunk = ChunkHeader::align(reader.stream_position()? + chunk_size);
        let mut flag = [0u8; 4];
        reader.read_exact(&mut flag)?;
        match &flag {
            b"info" | b"INFO" => {
                let dict = Self::read_dict(reader, end_of_chunk, text_encoding)?;
                Ok(Self::Info(dict))
            },
            b"adtl" => {
                let mut adtl = Vec::<AdtlChunk>::new();
                while reader.stream_position()? < end_of_chunk {
                    adtl.push(AdtlChunk::read(reader, text_encoding)?);
                }
                Ok(Self::Adtl(adtl))
            },
            other => {
                Err(AudioReadError::Unimplemented(format!("Unknown indentifier in LIST chunk: {}", text_encoding.decode_flags(other))))
            },
        }
    }

    pub fn write(&self, writer_shared: SharedWriter, text_encoding: &dyn SavageStringCodecs) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"LIST")?;
        match self {
            Self::Info(dict) => {
                writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                    writer.write_all(b"INFO")?;
                    Ok(())
                })?;
                Self::write_dict(writer_shared.clone(), dict, text_encoding)?;
            },
            Self::Adtl(adtls) => {
                writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                    writer.write_all(b"adtl")?;
                    Ok(())
                })?;
                for adtl in adtls.iter() {
                    adtl.write(writer_shared.clone(), text_encoding)?;
                }
            },
        };
        cw.end()?;
        Ok(())
    }

    pub fn read_dict<R>(reader: &mut R, end_of_chunk: u64, text_encoding: &dyn SavageStringCodecs) -> Result<HashMap<String, String>, AudioReadError>
    where R: Reader {
        // INFO 节其实是很多键值对，用来标注歌曲信息。在它的字节范围的限制下，读取所有的键值对。
        let mut dict = HashMap::<String, String>::new();
        while reader.stream_position()? < end_of_chunk {
            let key_chunk = ChunkHeader::read(reader)?; // 每个键其实就是一个 Chunk，它的大小值就是字符串大小值。
            let value_str = read_str(reader, key_chunk.size as usize, text_encoding)?;
            dict.insert(text_encoding.decode(&key_chunk.flag), value_str);
            key_chunk.seek_to_next_chunk(reader)?;
        }
        Ok(dict)
    }

    pub fn write_dict(writer_shared: SharedWriter, dict: &HashMap<String, String>, text_encoding: &dyn SavageStringCodecs) -> Result<(), AudioWriteError> {
        Ok(writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            for (key, val) in dict.iter() {
                if key.len() != 4 {
                    return Err(AudioWriteError::InvalidArguments(String::from("flag must be 4 bytes")))
                }
                let mut val = val.clone();
                val.push('\0');
                write_str(writer, key, text_encoding)?;
                (val.len() as u32).write_le(writer)?;
                write_str(writer, &val, text_encoding)?;
                if writer.stream_position()? & 1 > 0 {
                    0u8.write_le(writer)?;
                }
            }
            Ok(())
        })
    }
}

#[derive(Debug, Clone)]
pub enum AdtlChunk {
    Labl(LablChunk),
    Note(NoteChunk),
    Ltxt(LtxtChunk),
}

impl AdtlChunk {
    pub fn read<R>(reader: &mut R, text_encoding: &dyn SavageStringCodecs) -> Result<Self, AudioReadError>
    where R: Reader {
        let sub_chunk = ChunkHeader::read(reader)?;
        let ret = match &sub_chunk.flag {
            b"labl" => {
                Self::Labl(LablChunk{
                    identifier: u32::read_le(reader)?,
                    data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
                })
            },
            b"note" => {
                Self::Note(NoteChunk{
                    identifier: u32::read_le(reader)?,
                    data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
                })
            },
            b"ltxt" => {
                Self::Ltxt(LtxtChunk{
                    identifier: u32::read_le(reader)?,
                    sample_length: u32::read_le(reader)?,
                    purpose_id: read_str(reader, 4, text_encoding)?,
                    country: u16::read_le(reader)?,
                    language: u16::read_le(reader)?,
                    dialect: u16::read_le(reader)?,
                    code_page: u16::read_le(reader)?,
                    data: read_str(reader, (sub_chunk.size - 20) as usize, text_encoding)?,
                })
            },
            other => {
                return Err(AudioReadError::UnexpectedFlag("labl/note/ltxt".to_owned(), String::from_utf8_lossy(other).to_string()));
            },
        };
        sub_chunk.seek_to_next_chunk(reader)?;
        Ok(ret)
    }

    pub fn write(&self, writer_shared: SharedWriter, text_encoding: &dyn SavageStringCodecs) -> Result<(), AudioWriteError> {
        match self {
            Self::Labl(labl) => {
                let mut cw = ChunkWriter::begin(writer_shared.clone(), b"labl")?;
                writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                    labl.identifier.write_le(writer)?;
                    write_str(writer, &labl.data, text_encoding)?;
                    Ok(())
                })?;
                cw.end()?;
            },
            Self::Note(note) => {
                let mut cw = ChunkWriter::begin(writer_shared.clone(), b"note")?;
                writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                    note.identifier.write_le(writer)?;
                    write_str(writer, &note.data, text_encoding)?;
                    Ok(())
                })?;
                cw.end()?;
            },
            Self::Ltxt(ltxt) => {
                let mut cw = ChunkWriter::begin(writer_shared.clone(), b"ltxt")?;
                writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
                    ltxt.identifier.write_le(writer)?;
                    ltxt.sample_length.write_le(writer)?;
                    write_str_sized(writer, &ltxt.purpose_id, 4, text_encoding)?;
                    ltxt.country.write_le(writer)?;
                    ltxt.language.write_le(writer)?;
                    ltxt.dialect.write_le(writer)?;
                    ltxt.code_page.write_le(writer)?;
                    write_str(writer, &ltxt.data, text_encoding)?;
                    Ok(())
                })?;
                cw.end()?;
            },
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LablChunk {
    pub identifier: u32,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct NoteChunk {
    pub identifier: u32,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct LtxtChunk {
    pub identifier: u32,
    pub sample_length: u32,
    pub purpose_id: String,
    pub country: u16,
    pub language: u16,
    pub dialect: u16,
    pub code_page: u16,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct AcidChunk {
    pub flags: u32,
    pub root_node: u16,
    pub reserved1: u16,
    pub reserved2: f32,
    pub num_beats: u32,
    pub meter_denominator: u16,
    pub meter_numerator: u16,
    pub tempo: f32,
}

impl AcidChunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, AudioReadError>
    where R: Reader {
        Ok(Self {
            flags: u32::read_le(reader)?,
            root_node: u16::read_le(reader)?,
            reserved1: u16::read_le(reader)?,
            reserved2: f32::read_le(reader)?,
            num_beats: u32::read_le(reader)?,
            meter_denominator: u16::read_le(reader)?,
            meter_numerator: u16::read_le(reader)?,
            tempo: f32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"acid")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            self.flags.write_le(writer)?;
            self.root_node.write_le(writer)?;
            self.reserved1.write_le(writer)?;
            self.reserved2.write_le(writer)?;
            self.num_beats.write_le(writer)?;
            self.meter_denominator.write_le(writer)?;
            self.meter_numerator.write_le(writer)?;
            self.tempo.write_le(writer)?;
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum JunkChunk{
    FullZero(u64), // 全零
    SomeData(Vec<u8>), // 有些数据
}

impl JunkChunk {
    pub fn from(data: Vec<u8>) -> Self {
        let mut is_all_zero = true;
        for i in data.iter() {
            if *i != 0 {
                is_all_zero = false;
                break;
            }
        }
        if is_all_zero {
            Self::FullZero(data.len() as u64)
        } else {
            Self::SomeData(data)
        }
    }

    pub fn write(&self, writer_shared: SharedWriter) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer_shared.clone(), b"JUNK")?;
        writer_shared.escorted_write(|writer| -> Result<(), io::Error> {
            match self {
                Self::FullZero(size) => writer.write_all(&vec![0u8; *size as usize])?,
                Self::SomeData(data) => writer.write_all(data)?,
            }
            Ok(())
        })?;
        cw.end()?;
        Ok(())
    }
}

// 如果有 id3 则使用它来读取 id3 数据
#[cfg(feature = "id3")]
#[allow(non_snake_case)]
pub mod Id3{
    use std::io::{Read, Write, Seek};
    use crate::errors::{AudioReadError, AudioWriteError};
    pub type Tag = id3::Tag;

    pub fn id3_read<R>(reader: &mut R, _size: usize) -> Result<Tag, AudioReadError>
    where R: Read + Seek + ?Sized {
        Ok(Tag::read_from2(reader)?)
    }
}

// 如果没有 id3 则读取原始字节数组
#[cfg(not(feature = "id3"))]
#[allow(non_snake_case)]
pub mod Id3{
    use std::io::Read;
    use std::vec::Vec;
    use std::error::Error;
    #[derive(Clone)]
    pub struct Tag {
        data: Vec<u8>,
    }
    impl Tag {
        fn new(data: Vec<u8>) -> Self {
            Self {
                data,
            }
        }
    }

    pub fn id3_read<R>(reader: &mut R, size: usize) -> Result<Tag, AudioReadError>
    where R: Read + Seek + ?Sized {
        #[cfg(debug_assertions)]
        println!("Feature \"id3\" was not enabled, consider compile with \"cargo build --features id3\"");
        Ok(Tag::new(super::read_bytes(reader, size)?))
    }

    impl std::fmt::Debug for Tag {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Tag")
                .finish_non_exhaustive()
        }
    }
}