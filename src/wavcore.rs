#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{error::Error, collections::HashMap};

pub use crate::errors::*;
pub use crate::readwrite::*;
pub use crate::sampleutils::*;
pub use crate::savagestr::*;

pub enum WaveSampleType {
    U8,
    S16,
    S24,
    S32,
    F32,
    F64,
}

pub fn get_sample_type(bits_per_sample: u16, sample_format: SampleFormat) -> Result<WaveSampleType, AudioError> {
    use SampleFormat::{UInt, Int, Float};
    use WaveSampleType::{U8,S16,S24,S32,F32,F64};
    match (bits_per_sample, sample_format) {
        (8, UInt) => Ok(U8),
        (16, Int) => Ok(S16),
        (24, Int) => Ok(S24),
        (32, Int) => Ok(S32),
        (32, Float) => Ok(F32),
        (64, Float) => Ok(F64),
        _ => Err(AudioError::UnknownSampleType),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GUID (pub u32, pub u16, pub u16, pub [u8; 8]);

pub const GUID_PCM_FORMAT: GUID = GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
pub const GUID_IEEE_FLOAT_FORMAT: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);

impl GUID {
    pub fn read<T: Read>(r: &mut T) -> Result<Self, std::io::Error> {
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

    pub fn write<T: Write>(&self, w: &mut T) -> Result<(), std::io::Error> {
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

#[derive(Clone, Copy, Debug)]
pub enum SampleFormat {
    Unknown,
    Float,
    UInt,
    Int,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub channels: u16,
    pub channel_mask: u32,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
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

    pub fn guess_channel_mask(channels: u16) -> Result<u32, AudioError> {
        match channels {
            1 => Ok(SpeakerPosition::FrontCenter as u32),
            2 => Ok((SpeakerPosition::FrontLeft as u32) | (SpeakerPosition::FrontRight as u32)),
            other => Err(AudioError::CantGuessChannelMask(other)),
        }
    }

    pub fn which_channel_which_speaker(&self) -> Result<Vec<SpeakerPosition>, AudioError> {
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
            if self.channel_mask & m == m {ret.push(enums[i]);}
        }
        return if ret.len() == self.channels.into() {
            Ok(ret)
        } else {
            Err(AudioError::ChannelNotMatchMask)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Chunk {
    pub flag: [u8; 4], // 实际存储在文件里的
    pub size: u32, // 实际存储在文件里的
    pub chunk_start_pos: u64, // Chunk 内容在文件中的位置，不包含 Chunk 头
}

impl Chunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        // 读取 WAV 中的每个块
        // 注意 WAV 中会有 JUNK 块，目前的做法就是跳过所有的 JUNK 块。
        // 在 AVI 里面，JUNK 块里面会包含重要信息，但是 WAV 我就管它丫的了。
        let mut flag = [0u8; 4];
        let mut size : u32;
        loop {
            reader.read_exact(&mut flag)?;
            size = u32::read_le(reader)?;
            if &flag == b"JUNK" {
                reader.seek(SeekFrom::Current(size.into()))?;
            } else {
                break;
            }
        }
        Ok(Self {
            flag,
            size,
            chunk_start_pos: reader.stream_position()?
        })
    }

    pub fn align(addr: u64) -> u64 {
        addr + (addr & 1)
    }

    pub fn next_chunk_pos(&self) -> u64 {
        Self::align(self.chunk_start_pos + self.size as u64)
    }

    pub fn seek_to_next_chunk<R>(&self, reader: &mut R) -> Result<u64, io::Error>
    where R: Reader {
        reader.seek(SeekFrom::Start(self.next_chunk_pos()))
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct fmt_Chunk {
    pub format_tag: u16,
    pub channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extension: Option<fmt_Chunk_Extension>,
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct fmt_Chunk_Extension {
    pub ext_len: u16,
    pub bits_per_sample: u16,
    pub channel_mask: u32,
    pub sub_format: GUID,
}

impl fmt_Chunk {
    pub fn read<R>(reader: &mut R, chunk_size: u32) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let mut ret = fmt_Chunk{
            format_tag: u16::read_le(reader)?,
            channels: u16::read_le(reader)?,
            sample_rate: u32::read_le(reader)?,
            byte_rate: u32::read_le(reader)?,
            block_align: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            extension: None,
        };
        match ret.format_tag {
            1 | 3 => (),
            0xFFFE => {
                if chunk_size >= 40 {
                    ret.extension = Some(fmt_Chunk_Extension::read(reader)?);
                }
            },
            0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => {
                // Ogg Vorbis 数据
                return Err(AudioError::Unimplemented.into());
            },
            _ => return Err(AudioError::Unimplemented.into()),
        }
        Ok(ret)
    }

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }

    pub fn get_sample_format(&self) -> Result<SampleFormat, AudioError> {
        use SampleFormat::{Int, UInt, Float};
        match (self.format_tag, self.bits_per_sample) {
            (1, 8) => Ok(UInt),
            (1, 16) => Ok(Int),
            (0xFFFE, 24) => Ok(Int),
            (0xFFFE, 32) => {
                match self.extension {
                    Some(extension) => {
                        match extension.sub_format {
                            GUID_PCM_FORMAT => Ok(Int),
                            GUID_IEEE_FLOAT_FORMAT => Ok(Float),
                            _ => Err(AudioError::Unimplemented),
                        }
                    },
                    None => Ok(Int),
                }
            },
            (3, 32) => Ok(Float),
            (3, 46) => Ok(Float),
            _ => Err(AudioError::Unimplemented),
        }
    }
}

impl fmt_Chunk_Extension {
    pub fn read<R>(reader: &mut R) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        Ok(Self{
            ext_len: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            channel_mask: u32::read_le(reader)?,
            sub_format: GUID::read(reader)?,
        })
    }

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

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
    pub fn read<R>(reader: &mut R, savage_decoder: &SavageStringDecoder) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let description = read_str(reader, 256, savage_decoder)?;
        let originator = read_str(reader, 32, savage_decoder)?;
        let originator_ref = read_str(reader, 32, savage_decoder)?;
        let origination_date = read_str(reader, 10, savage_decoder)?;
        let origination_time = read_str(reader, 8, savage_decoder)?;
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

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
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

impl SmplSampleLoop {
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

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
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct Cue_Chunk {
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

impl Cue_Chunk {
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        let mut ret = Cue_Chunk {
            num_cues: u32::read_le(reader)?,
            cues: Vec::<Cue>::new(),
        };
        for _ in 0..ret.num_cues {
            ret.cues.push(Cue::read(reader)?);
        }
        Ok(ret)
    }

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

impl Cue {
    pub fn read<R>(reader: &mut R) -> Result<Self, io::Error>
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

#[derive(Debug, Clone)]
pub enum ListChunk {
    Info(HashMap<String, String>),
    Adtl(Vec<AdtlChunk>),
}

impl ListChunk {
    pub fn read<R>(reader: &mut R, chunk_size: u64, savage_decoder: &SavageStringDecoder) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let end_of_chunk = Chunk::align(reader.stream_position()? + chunk_size);
        let mut flag = [0u8; 4];
        reader.read_exact(&mut flag)?;
        match &flag {
            b"info" | b"INFO" => {
                // INFO 节其实是很多键值对，用来标注歌曲信息。在它的字节范围的限制下，读取所有的键值对。
                let mut dict = HashMap::<String, String>::new();
                while reader.stream_position()? < end_of_chunk {
                    let key_chunk = Chunk::read(reader)?; // 每个键其实就是一个 Chunk，它的大小值就是字符串大小值。
                    let value_str = read_str(reader, key_chunk.size as usize, savage_decoder)?;
                    dict.insert(savage_decoder.decode(&key_chunk.flag), value_str);
                    key_chunk.seek_to_next_chunk(reader)?;
                }
                Ok(Self::Info(dict))
            },
            b"adtl" => {
                let mut adtl = Vec::<AdtlChunk>::new();
                while reader.stream_position()? < end_of_chunk {
                    let sub_chunk = Chunk::read(reader)?;
                    match &sub_chunk.flag {
                        b"labl" => {
                            adtl.push(AdtlChunk::Labl(LablChunk{
                                identifier: u32::read_le(reader)?,
                                data: read_str(reader, (sub_chunk.size - 4) as usize, savage_decoder)?,
                            }));
                        },
                        b"note" => {
                            adtl.push(AdtlChunk::Note(NoteChunk{
                                identifier: u32::read_le(reader)?,
                                data: read_str(reader, (sub_chunk.size - 4) as usize, savage_decoder)?,
                            }));
                        },
                        b"ltxt" => {
                            adtl.push(AdtlChunk::Ltxt(LtxtChunk{
                                identifier: u32::read_le(reader)?,
                                sample_length: u32::read_le(reader)?,
                                purpose_id: read_str(reader, 4, savage_decoder)?,
                                country: u16::read_le(reader)?,
                                language: u16::read_le(reader)?,
                                dialect: u16::read_le(reader)?,
                                code_page: u16::read_le(reader)?,
                                data: read_str(reader, (sub_chunk.size - 20) as usize, savage_decoder)?,
                            }));
                        },
                        other => {
                            println!("Unknown sub chunk in adtl chunk: {}", savage_decoder.decode_flags(&other));
                        },
                    }
                    sub_chunk.seek_to_next_chunk(reader)?;
                }
                Ok(Self::Adtl(adtl))
            },
            other => {
                println!("Unknown indentifier in LIST chunk: {}", savage_decoder.decode_flags(&other));
                Err(AudioReadError::Unimplemented.into())
            },
        }
    }

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

#[derive(Debug, Clone)]
pub enum AdtlChunk {
    Labl(LablChunk),
    Note(NoteChunk),
    Ltxt(LtxtChunk),
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
    pub fn read<R>(reader: &mut R) -> Result<Self, Box<dyn Error>>
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

    pub fn write<W>(&self, writer: &mut W) -> Result<Self, Box<dyn Error>>
    where W: Writer {

    }
}

