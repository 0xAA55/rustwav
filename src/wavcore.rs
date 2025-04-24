#![allow(dead_code)]

use std::{io::{self, Read, Write, SeekFrom}, fmt::{self, Debug, Display, Formatter}, convert::From, collections::{HashMap, BTreeMap}};

use crate::SampleType;
use crate::{Reader, Writer};
use crate::{AudioError, AudioReadError, AudioWriteError};
use crate::adpcm::ms::AdpcmCoeffSet;
use crate::readwrite::string_io::*;
use crate::savagestr::{StringCodecMaps, SavageStringCodecs};

#[allow(unused_imports)]
pub use crate::encoders::mp3::{Mp3EncoderOptions, Mp3Channels, Mp3Quality, Mp3Bitrate, Mp3VbrMode};

#[allow(unused_imports)]
pub use crate::encoders::opus::{OpusEncoderOptions, OpusBitrate, OpusEncoderSampleDuration};

#[cfg(feature = "flac")]
#[allow(unused_imports)]
pub use crate::flac::{
    FlacEncoderParams,
};

// Did you assume WAV is solely for storing PCM data?
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum DataFormat{
    Unspecified,
    Pcm,
    Adpcm(AdpcmSubFormat),
    PcmALaw,
    PcmMuLaw,
    Mp3(Mp3EncoderOptions),
    Opus(OpusEncoderOptions),
    Flac(FlacEncoderParams),
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum AdpcmSubFormat {
    Ms = 0x0002,
    Ima = 0x0011,
    Yamaha = 0x0020,
}

impl From<AdpcmSubFormat> for u16 {
    fn from(val: AdpcmSubFormat) -> Self {
        val as u16
    }
}

impl Display for DataFormat {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Unspecified => write!(f, "Unspecified"),
            Self::Pcm => write!(f, "PCM"),
            Self::Adpcm(subformat) => write!(f, "{:?}", subformat),
            Self::PcmALaw => write!(f, "PCM-ALaw"),
            Self::PcmMuLaw => write!(f, "PCM-MuLaw"),
            Self::Mp3(options) => write!(f, "MP3({:?})", options),
            Self::Opus(options) => write!(f, "Opus({:?})", options),
            Self::Flac(options) => write!(f, "Flac({:?})", options),
       }
    }
}

impl Display for AdpcmSubFormat {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Ms => write!(f, "ADPCM-MS"),
            Self::Ima => write!(f, "ADPCM-IMA"),
            Self::Yamaha => write!(f, "ADPCM-YAMAHA"),
       }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SampleFormat {
    Unknown,
    Float,
    UInt,
    Int,
}

impl Display for SampleFormat {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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

impl Display for WaveSampleType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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


#[allow(unused_imports)]
pub fn get_sample_type(bits_per_sample: u16, sample_format: SampleFormat) -> WaveSampleType {
    use SampleFormat::{UInt, Int, Float};
    use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
    match (bits_per_sample, sample_format) {
        (8, UInt) => U8,
        (16, Int) => S16,
        (24, Int) => S24,
        (32, Int) => S32,
        (64, Int) => S64,
        (32, Float) => F32,
        (64, Float) => F64,
        // PCM supports only the formats listed above.
        (_, _) => Unknown,
    }
}

#[derive(Clone, Copy, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub struct GUID (pub u32, pub u16, pub u16, pub [u8; 8]);

pub const GUID_PCM_FORMAT: GUID = GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
pub const GUID_IEEE_FLOAT_FORMAT: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);

impl Debug for GUID {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_tuple("GUID")
            .field(&format_args!("{:08x}-{:04x}-{:04x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0, self.1, self.2, self.3[0], self.3[1], self.3[2], self.3[3], self.3[4], self.3[5], self.3[6], self.3[7]))
            .finish()
    }
}

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
#[repr(u32)]
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

impl From<SpeakerPosition> for u32 {
    fn from(val: SpeakerPosition) -> Self {
        val as u32
    }
}

impl Display for SpeakerPosition {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
// Algorithm Design:
// 1. Spatial Mapping: 
//    - Assign a 3D direction vector to each audio source, representing its position relative to the listener's head.
//    - Vectors are normalized (magnitude = 1.0) to abstract distance, focusing on angular positioning.
//
// 2. Directional Influence Calculation:
//    - Compute dot products between each source vector and the listener's facing direction (head orientation vector).
//    - Sources behind the listener (dot product < 0.0) are attenuated by a decay factor (e.g., 0.2x gain).
//
// 3. Energy-Preserving Mixdown:
//    - Apply weighted summation: mixed_sample = Σ (source_sample * dot_product * decay_factor)
//    - Normalize weights dynamically to ensure Σ (effective_gain) ≤ 1.0, preventing clipping.
//
// This achieves lossless channel layout conversion (e.g., 5.1 → stereo) with spatial accuracy.

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
    if ret.len() == channels as usize {
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
    match channels {
        0 => Err(AudioError::GuessChannelMaskFailed(channels)),
        1 => Ok(SpeakerPosition::FrontCenter.into()),
        2 => Ok(SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32),
        o => {
            let mut mask = 0;
            for i in 0..o {
                let bit = 1 << i;
                if bit > 0x20000 {
                    return Err(AudioError::GuessChannelMaskFailed(channels));
                }
                mask |= bit;
            }
            Ok(mask)
        }
    }
}

#[derive(Debug, Clone, Copy)]
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

    pub fn verify_for_pcm(&self) -> Result<(), AudioError> {
        self.guess_channel_mask()?;
        if self.get_sample_type() == WaveSampleType::Unknown {
            Err(AudioError::InvalidArguments(format!("PCM doesn't support {} bits per sample {:?}", self.bits_per_sample, self.sample_format)))
        } else {
            Ok(())
        }
    }

    pub fn is_channel_mask_valid(&self) -> bool {
        let mut counter: u16 = 0;
        for i in 0..32 {
            if ((1 << i) & self.channel_mask) != 0 {
                counter += 1;
            }
        }
        counter == self.channels
    }
}

pub struct ChunkWriter<'a> {
    pub writer: &'a mut dyn Writer,
    pub flag: [u8; 4],
    pub pos_of_chunk_len: u64, // Byte position where the chunk size field is written (to be backfilled later)
    pub chunk_start: u64, // File offset marking the start of this chunk's payload data
    ended: bool,
}

impl Debug for ChunkWriter<'_> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("ChunkWriter")
            .field("writer", &self.writer)
            .field("flag", &format_args!("{}", String::from_utf8_lossy(&self.flag)))
            .field("pos_of_chunk_len", &self.pos_of_chunk_len)
            .field("chunk_start", &self.chunk_start)
            .finish()
    }
}

impl<'a> ChunkWriter<'a> {
    // Starts writing a chunk by first writing the chunk Flag, then recording the positions
    // for both the chunk size and the chunk data.
    pub fn begin(writer: &'a mut dyn Writer, flag: &[u8; 4]) -> Result<Self, AudioWriteError> {
        writer.write_all(flag)?;
        let pos_of_chunk_len = writer.stream_position()?;
        0u32.write_le(writer)?;
        let chunk_start = writer.stream_position()?;
        Ok(Self{
            writer,
            flag: *flag,
            pos_of_chunk_len,
            chunk_start,
            ended: false,
        })
    }

    // At the end of the chunk, the chunk size will be updated since the ownership of `self` moved there, and `drop()` will be called.
    pub fn end(self){}

    fn on_drop(&mut self) -> Result<(), AudioWriteError> {
        if self.ended {
            return Ok(());
        }
        // Chunk size handling constraints:
        // ---------------------------------------------------------------
        // 1. u32 Overflow Handling:
        //    - RIFF/RF64/data chunks: If size exceeds u32::MAX (0xFFFFFFFF), 
        //      write 0xFFFFFFFF and store the actual u64 size in the ds64 chunk.
        //    - Other chunks: Size must not exceed u32::MAX. If violated, returns an error.
        //
        // 2. ds64 Table Limitations:
        //    - The ds64 chunk's table can store u64 sizes for non-RIFF/RF64/data chunks,
        //      but adding entries increases the ds64 chunk's own size.
        //    - Current implementation does NOT pre-allocate space in the ds64 chunk.
        //
        // 3. Enforcement:
        //    - Non-RIFF/RF64/data chunks exceeding 0xFFFFFFFF bytes will fail encoding.
        //    - Callers must ensure chunks (except RIFF/RF64/data) stay within u32 limits.
        let mut chunk_size = self.get_chunk_data_size()?;
        if chunk_size >= 0xFFFFFFFFu64 {
            match &self.flag {
                b"RIFF" | b"data" => {
                    chunk_size = 0xFFFFFFFF;
                },
                other => {
                    let chunk_flag = String::from_utf8_lossy(other);
                    return Err(AudioWriteError::ChunkSizeTooBig(format!("{} is 0x{:x} bytes long.", chunk_flag, chunk_size)));
                },
            }
        }
        self.end_and_write_size(chunk_size as u32)
    }

    fn end_and_write_size(&mut self, chunk_size_to_write: u32) -> Result<(), AudioWriteError> {
        let end_of_chunk = self.writer.stream_position()?;
        self.writer.seek(SeekFrom::Start(self.pos_of_chunk_len))?;
        chunk_size_to_write.write_le(self.writer)?;
        self.writer.seek(SeekFrom::Start(end_of_chunk))?;
        if end_of_chunk & 1 > 0 {
            0u8.write_le(self.writer)?;
        }
        self.ended = true;
        Ok(())
    }

    pub fn get_chunk_start_pos(&self) -> u64 {
        self.chunk_start
    }

    pub fn get_chunk_data_size(&mut self) -> Result<u64, AudioWriteError> {
        Ok(self.writer.stream_position()? - self.get_chunk_start_pos())
    }
}

impl Drop for ChunkWriter<'_> {
    fn drop(&mut self) {
        self.on_drop().unwrap()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkHeader {
    pub flag: [u8; 4],        // The 4-byte identifier stored in the file (e.g., "RIFF", "fmt ")
    pub size: u32,            // The chunk size stored in the file header (may be 0xFFFFFFFF if actual size is in ds64 chunk)
    pub chunk_start_pos: u64, // File offset of the chunk's payload (excludes the 8-byte header)
}

impl ChunkHeader {
    pub fn new() -> Self {
        Self {
            flag: [0u8; 4],
            size: 0,
            chunk_start_pos: 0,
        }
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
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

    pub fn seek_to_next_chunk(&self, reader: &mut dyn Reader) -> Result<u64, AudioReadError> {
        Ok(reader.seek(SeekFrom::Start(self.next_chunk_pos()))?)
    }
}

impl Default for ChunkHeader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FmtChunk {
    pub format_tag: u16, // https://github.com/tpn/winsdk-10/blob/master/Include/10.0.14393.0/shared/mmreg.h
    pub channels: u16,
    pub sample_rate: u32,
    pub byte_rate: u32,
    pub block_align: u16,
    pub bits_per_sample: u16,
    pub extension: Option<FmtExtension>,
}

#[derive(Debug, Clone, Copy)]
pub struct FmtExtension {
    pub ext_len: u16,
    pub data: ExtensionData,
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionData{
    Nodata,
    AdpcmMs(AdpcmMsData),
    AdpcmIma(AdpcmImaData),
    Mp3(Mp3Data),
    Extensible(ExtensibleData),
}

#[derive(Debug, Clone, Copy)]
pub struct AdpcmMsData{
    pub samples_per_block: u16,
    pub num_coeff: u16,
    pub coeffs: [AdpcmCoeffSet; 7],
}

#[derive(Debug, Clone, Copy)]
pub struct AdpcmImaData{
    pub samples_per_block: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Mp3Data{
    pub id: u16,
    pub flags: u32,
    pub block_size: u16,
    pub frames_per_block: u16,
    pub codec_delay: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct ExtensibleData {
    pub valid_bits_per_sample: u16,
    pub channel_mask: u32,
    pub sub_format: GUID,
}

impl FmtChunk {
    pub fn new() -> Self {
        Self {
            format_tag: 0,
            channels: 0,
            sample_rate: 0,
            byte_rate: 0,
            block_align: 0,
            bits_per_sample: 0,
            extension: None,
        }
    }

    pub fn read(reader: &mut impl Reader, chunk_size: u32) -> Result<Self, AudioReadError> {
        let mut ret = FmtChunk{
            format_tag: u16::read_le(reader)?,
            channels: u16::read_le(reader)?,
            sample_rate: u32::read_le(reader)?,
            byte_rate: u32::read_le(reader)?,
            block_align: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            extension: None,
        };
        if chunk_size > 16 {
            ret.extension = Some(FmtExtension::read(reader, ret.format_tag)?);
        }
        Ok(ret)
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.format_tag.write_le(writer)?;
        self.channels.write_le(writer)?;
        self.sample_rate.write_le(writer)?;
        self.byte_rate.write_le(writer)?;
        self.block_align.write_le(writer)?;
        self.bits_per_sample.write_le(writer)?;
        if let Some(extension) = self.extension {
            extension.write(writer)?;
        }
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
                if let Some(extension) = self.extension {
                    match extension.data{
                        ExtensionData::Extensible(extensible) => {
                            match extensible.sub_format {
                                GUID_PCM_FORMAT => Int,
                                GUID_IEEE_FLOAT_FORMAT => Float,
                                _ => Unknown, // Let the decoders to decide
                            }
                        },
                        other => panic!("Unexpected extension data in the `fmt ` chunk: {:?}", other),
                    }
                } else {
                    Int // 我们还是宽松的，0xFFFE 也允许没有扩展数据。
                }
            },
            (3, 32) => Float,
            (3, 64) => Float,
            (_, _) => Unknown, // Let the decoders to decide
        }
    }

    pub fn get_sample_type(&self) -> WaveSampleType {
        get_sample_type(self.bits_per_sample, self.get_sample_format())
    }
}

impl Default for FmtChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl FmtExtension {
    pub fn new_adpcm_ms(adpcm_ms: AdpcmMsData) -> Self {
        Self {
            ext_len: AdpcmMsData::sizeof() as u16,
            data: ExtensionData::AdpcmMs(adpcm_ms),
        }
    }

    pub fn new_adpcm_ima(adpcm_ima: AdpcmImaData) -> Self {
        Self {
            ext_len: AdpcmImaData::sizeof() as u16,
            data: ExtensionData::AdpcmIma(adpcm_ima),
        }
    }

    pub fn new_mp3(mp3: Mp3Data) -> Self {
        Self {
            ext_len: Mp3Data::sizeof() as u16,
            data: ExtensionData::Mp3(mp3),
        }
    }

    pub fn new_extensible(extensible: ExtensibleData) -> Self {
        Self {
            ext_len: ExtensibleData::sizeof() as u16,
            data: ExtensionData::Extensible(extensible),
        }
    }

    pub fn get_length(&self) -> u16 {
        self.ext_len
    }

    pub fn read(reader: &mut impl Reader, format_tag: u16) -> Result<Self, AudioReadError> {
        const TAG_ADPCM_IMA: u16 = AdpcmSubFormat::Ima as u16;
        const TAG_ADPCM_MS: u16 = AdpcmSubFormat::Ms as u16;
        let ext_len = u16::read_le(reader)?;
        Ok(Self{
            ext_len,
            data: match format_tag {
                TAG_ADPCM_MS => {
                    if ext_len as usize >= AdpcmMsData::sizeof() {
                        Ok(ExtensionData::AdpcmMs(AdpcmMsData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!("The extension data for ADPCM-MS should be bigger than {}, got {ext_len}", AdpcmMsData::sizeof())))
                    }
                },
                TAG_ADPCM_IMA => {
                    if ext_len as usize >= AdpcmImaData::sizeof() {
                        Ok(ExtensionData::AdpcmIma(AdpcmImaData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!("The extension data for ADPCM-IMA should be bigger than {}, got {ext_len}", AdpcmImaData::sizeof())))
                    }
                },
                0x0055 => {
                    if ext_len as usize >= Mp3Data::sizeof() {
                        Ok(ExtensionData::Mp3(Mp3Data::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!("The extension data for Mpeg Layer III should be bigger than {}, got {ext_len}", Mp3Data::sizeof())))
                    }
                },
                0xFFFE => {
                    if ext_len as usize >= ExtensibleData::sizeof() {
                        Ok(ExtensionData::Extensible(ExtensibleData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!("The extension data for EXTENSIBLE should be bigger than {}, got {ext_len}", ExtensibleData::sizeof())))
                    }
                },
                _ => Ok(ExtensionData::Nodata),
            }?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        const TAG_ADPCM_IMA: u16 = AdpcmSubFormat::Ima as u16;
        const TAG_ADPCM_MS: u16 = AdpcmSubFormat::Ms as u16;
        self.ext_len.write_le(writer)?;
        if self.ext_len != 0 { // 这里我们也是宽松的，允许在这里存个表示长度为 0 的数值。
            match self.data {
                ExtensionData::Nodata => Err(AudioWriteError::InvalidArguments(format!("There should be data in {} bytes to be written, but the data is `Nodata`.", self.ext_len))),
                ExtensionData::AdpcmMs(data) => Ok(data.write(writer)?),
                ExtensionData::AdpcmIma(data) => Ok(data.write(writer)?),
                ExtensionData::Mp3(data) => Ok(data.write(writer)?),
                ExtensionData::Extensible(data) => Ok(data.write(writer)?),
            }
        } else {
            Ok(())
        }
    }
}

impl AdpcmMsData {
    pub fn new() -> Self {
        Self {
            samples_per_block: 0,
            num_coeff: 7,
            coeffs: [
                AdpcmCoeffSet{coeff1: 256, coeff2: 0   },
                AdpcmCoeffSet{coeff1: 512, coeff2: -256},
                AdpcmCoeffSet{coeff1: 0  , coeff2: 0   },
                AdpcmCoeffSet{coeff1: 192, coeff2: 64  },
                AdpcmCoeffSet{coeff1: 240, coeff2: 0   },
                AdpcmCoeffSet{coeff1: 460, coeff2: -208},
                AdpcmCoeffSet{coeff1: 392, coeff2: -232},
            ],
        }
    }

    pub fn sizeof() -> usize {
        32
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            samples_per_block: u16::read_le(reader)?,
            num_coeff: u16::read_le(reader)?,
            coeffs: [
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
                AdpcmCoeffSet{coeff1: i16::read_le(reader)?, coeff2: i16::read_le(reader)?},
            ],
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.samples_per_block.write_le(writer)?;
        self.num_coeff.write_le(writer)?;
        for coeff in self.coeffs {
            coeff.coeff1.write_le(writer)?;
            coeff.coeff2.write_le(writer)?;
        }
        Ok(())
    }
}

impl Default for AdpcmMsData {
    fn default() -> Self {
        Self::new()
    }
}

impl AdpcmImaData {
    pub fn new(samples_per_block: u16) -> Self {
        Self {
            samples_per_block,
        }
    }

    pub fn sizeof() -> usize {
        2
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            samples_per_block: u16::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.samples_per_block.write_le(writer)?;
        Ok(())
    }
}

impl Mp3Data {
    pub const MPEGLAYER3_FLAG_PADDING_ISO: u32 = 0x00000000;
    pub const MPEGLAYER3_FLAG_PADDING_ON : u32 = 0x00000001;
    pub const MPEGLAYER3_FLAG_PADDING_OFF: u32 = 0x00000002;

    pub fn new(bitrate: u32, sample_rate: u32) -> Self {
        Self {
            id: 1,
            flags: Self::MPEGLAYER3_FLAG_PADDING_OFF,
            block_size: (144 * bitrate / sample_rate) as u16,
            frames_per_block: 1,
            codec_delay: 0,
        }
    }

    pub fn sizeof() -> usize {
        12
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            id: u16::read_le(reader)?,
            flags: u32::read_le(reader)?,
            block_size: u16::read_le(reader)?,
            frames_per_block: u16::read_le(reader)?,
            codec_delay: u16::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.id.write_le(writer)?;
        self.flags.write_le(writer)?;
        self.block_size.write_le(writer)?;
        self.frames_per_block.write_le(writer)?;
        self.codec_delay.write_le(writer)?;
        Ok(())
    }
}

impl ExtensibleData {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            valid_bits_per_sample: u16::read_le(reader)?,
            channel_mask: u32::read_le(reader)?,
            sub_format: GUID::read(reader)?,
        })
    }

    pub fn sizeof() -> usize {
        22
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.valid_bits_per_sample.write_le(writer)?;
        self.channel_mask.write_le(writer)?;
        self.sub_format.write(writer)?;
        Ok(())
    }
}

// https://www.recordingblogs.com/wiki/silent-chunk-of-a-wave-file
#[derive(Debug, Clone, Copy)]
pub struct SlntChunk {
    data: u32,
}

impl SlntChunk {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            data: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"bext")?;
        self.data.write_le(cw.writer)?;
        Ok(())
    }
}

#[derive(Clone)]
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
    pub fn read(reader: &mut impl Reader, text_encoding: &StringCodecMaps) -> Result<Self, AudioReadError> {
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

    pub fn write(&self, writer: &mut dyn Writer, text_encoding: &StringCodecMaps) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"bext")?;
        write_str_sized(cw.writer, &self.description, 256, text_encoding)?;
        write_str_sized(cw.writer, &self.originator, 32, text_encoding)?;
        write_str_sized(cw.writer, &self.originator_ref, 32, text_encoding)?;
        write_str_sized(cw.writer, &self.origination_date, 10, text_encoding)?;
        write_str_sized(cw.writer, &self.origination_time, 8, text_encoding)?;
        self.time_ref.write_le(cw.writer)?;
        self.version.write_le(cw.writer)?;
        cw.writer.write_all(&self.umid)?;
        cw.writer.write_all(&self.reserved)?;
        cw.writer.write_all(&self.coding_history)?;
        Ok(())
    }
}

impl Debug for BextChunk{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("BextChunk")
            .field("description", &self.description)
            .field("originator", &self.originator)
            .field("originator_ref", &self.originator_ref)
            .field("origination_date", &self.origination_date)
            .field("origination_time", &self.origination_time)
            .field("time_ref", &self.time_ref)
            .field("version", &self.version)
            .field("umid", &format_args!("[{}]", self.umid.iter().map(|byte|{format!("0x{:02x}", byte)}).collect::<Vec<String>>().join(",")))
            .field("reserved", &format_args!("[u8; {}]", self.reserved.len()))
            .field("coding_history", &self.coding_history)
            .finish()
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
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
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

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"smpl")?;
        self.manufacturer.write_le(cw.writer)?;
        self.product.write_le(cw.writer)?;
        self.sample_period.write_le(cw.writer)?;
        self.midi_unity_note.write_le(cw.writer)?;
        self.midi_pitch_fraction.write_le(cw.writer)?;
        self.smpte_format.write_le(cw.writer)?;
        self.smpte_offset.write_le(cw.writer)?;
        self.num_sample_loops.write_le(cw.writer)?;
        self.sampler_data.write_le(cw.writer)?;
        for l in self.loops.iter() {
            l.write(cw.writer)?;
        }
        Ok(())
    }
}

impl SmplSampleLoop {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
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
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
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

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"INST")?;
        self.base_note.write_le(cw.writer)?;
        self.detune.write_le(cw.writer)?;
        self.gain.write_le(cw.writer)?;
        self.low_note.write_le(cw.writer)?;
        self.high_note.write_le(cw.writer)?;
        self.low_velocity.write_le(cw.writer)?;
        self.high_velocity.write_le(cw.writer)?;
        Ok(())
    }
}

// https://www.recordingblogs.com/wiki/playlist-chunk-of-a-wave-file
#[derive(Debug, Clone)]
pub struct PlstChunk {
    pub playlist_len: u32,
    pub data: Vec<Plst>,
}

#[derive(Debug, Clone, Copy)]
pub struct Plst {
    pub cue_point_id: u32,
    pub num_samples: u32,
    pub repeats: u32,
}

impl PlstChunk {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        let playlist_len = u32::read_le(reader)?;
        Ok(Self {
            playlist_len,
            data: (0..playlist_len).map(|_| -> Result<Plst, AudioReadError> {
                Plst::read(reader)
            }).collect::<Result<Vec<Plst>, AudioReadError>>()?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"plst")?;
        self.playlist_len.write_le(cw.writer)?;
        for data in self.data.iter() {
            data.write(cw.writer)?;
        }
        Ok(())
    }

    pub fn build_map(&self) -> BTreeMap<u32, Plst> {
        self.data.iter().map(|plst|{(plst.cue_point_id, *plst)}).collect()
    }
}

impl Plst {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self{
            cue_point_id: u32::read_le(reader)?,
            num_samples: u32::read_le(reader)?,
            repeats: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.cue_point_id.write_le(writer)?;
        self.num_samples.write_le(writer)?;
        self.repeats.write_le(writer)?;
        Ok(())
    }
}

// https://www.recordingblogs.com/wiki/cue-chunk-of-a-wave-file
// https://wavref.til.cafe/chunk/cue/
#[derive(Debug, Clone)]
pub struct CueChunk {
    pub num_cues: u32,
    pub cue_points: Vec<CuePoint>,
}

#[derive(Debug, Clone, Copy)]
pub struct CuePoint {
    pub cue_point_id: u32,
    pub position: u32,
    pub data_chunk_id: [u8; 4],
    pub chunk_start: u32,
    pub block_start: u32,
    pub offset: u32,
}

impl CueChunk {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        let num_cues = u32::read_le(reader)?;
        Ok(Self {
            num_cues,
            cue_points: (0.. num_cues).map(|_| -> Result<CuePoint, AudioReadError> {
                CuePoint::read(reader)
            }).collect::<Result<Vec<CuePoint>, AudioReadError>>()?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"cue ")?;
        self.num_cues.write_le(cw.writer)?;
        for cue_point in self.cue_points.iter() {
            cue_point.write(cw.writer)?;
        }
        Ok(())
    }

    pub fn build_map(&self) -> BTreeMap<u32, &CuePoint> {
        self.cue_points.iter().map(|cue|{(cue.cue_point_id, cue)}).collect()
    }
}

impl CuePoint {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        let mut data_chunk_id = [0u8; 4];
        reader.read_exact(&mut data_chunk_id)?;
        Ok(Self{
            cue_point_id: u32::read_le(reader)?,
            position: u32::read_le(reader)?,
            data_chunk_id,
            chunk_start: u32::read_le(reader)?,
            block_start: u32::read_le(reader)?,
            offset: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.cue_point_id.write_le(writer)?;
        self.position.write_le(writer)?;
        writer.write_all(&self.data_chunk_id)?;
        self.chunk_start.write_le(writer)?;
        self.block_start.write_le(writer)?;
        self.offset.write_le(writer)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ListChunk {
    Info(BTreeMap<String, String>),
    Adtl(BTreeMap<u32, AdtlChunk>),
}

#[derive(Debug, Clone)]
pub enum AdtlChunk { // https://wavref.til.cafe/chunk/adtl/
    Labl(LablChunk),
    Note(NoteChunk),
    Ltxt(LtxtChunk),
    File(FileChunk),
}

#[derive(Debug, Clone)]
pub struct LablChunk {
    pub cue_point_id: u32,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct NoteChunk {
    pub cue_point_id: u32,
    pub data: String,
}

#[derive(Debug, Clone)]
pub struct LtxtChunk {
    pub cue_point_id: u32,
    pub sample_length: u32,
    pub purpose_id: String,
    pub country: u16,
    pub language: u16,
    pub dialect: u16,
    pub code_page: u16,
    pub data: String,
}

#[derive(Clone)]
pub struct FileChunk {
    pub cue_point_id: u32,
    pub media_type: u32,
    pub file_data: Vec<u8>,
}

impl Debug for FileChunk{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FileChunk")
            .field("cue_point_id", &self.cue_point_id)
            .field("media_type", &self.media_type)
            .field("file_data", &format_args!("[u8; {}]", self.file_data.len()))
            .finish()
    }
}

impl AdtlChunk {
    pub fn read(reader: &mut impl Reader, text_encoding: &StringCodecMaps) -> Result<Self, AudioReadError> {
        let sub_chunk = ChunkHeader::read(reader)?;
        let ret = match &sub_chunk.flag {
            b"labl" => {
                Self::Labl(LablChunk{
                    cue_point_id: u32::read_le(reader)?,
                    data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
                })
            },
            b"note" => {
                Self::Note(NoteChunk{
                    cue_point_id: u32::read_le(reader)?,
                    data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
                })
            },
            b"ltxt" => {
                let mut ltxt = LtxtChunk{
                    cue_point_id: u32::read_le(reader)?,
                    sample_length: u32::read_le(reader)?,
                    purpose_id: read_str(reader, 4, text_encoding)?,
                    country: u16::read_le(reader)?,
                    language: u16::read_le(reader)?,
                    dialect: u16::read_le(reader)?,
                    code_page: u16::read_le(reader)?,
                    data: String::new(),
                };
                ltxt.data = read_str_by_code_page(reader, (sub_chunk.size - 20) as usize, text_encoding, ltxt.code_page as u32)?;
                Self::Ltxt(ltxt)
            },
            b"file" => {
                Self::File(FileChunk{
                    cue_point_id: u32::read_le(reader)?,
                    media_type: u32::read_le(reader)?,
                    file_data: read_bytes(reader, (sub_chunk.size - 8) as usize)?,
                })
            }
            other => {
                return Err(AudioReadError::UnexpectedFlag("labl/note/ltxt".to_owned(), String::from_utf8_lossy(other).to_string()));
            },
        };
        sub_chunk.seek_to_next_chunk(reader)?;
        Ok(ret)
    }

    pub fn write(&self, writer: &mut dyn Writer, text_encoding: &StringCodecMaps) -> Result<(), AudioWriteError> {
        fn to_sz(s: &str) -> String {
            if !s.is_empty() {
                let mut s = s.to_owned();
                if !s.ends_with('\0') {s.push('\0');}
                s
            } else {
                "\0".to_owned()
            }
        }
        match self {
            Self::Labl(labl) => {
                let cw = ChunkWriter::begin(writer, b"labl")?;
                labl.cue_point_id.write_le(cw.writer)?;
                write_str(cw.writer, &to_sz(&labl.data), text_encoding)?;
            },
            Self::Note(note) => {
                let cw = ChunkWriter::begin(writer, b"note")?;
                note.cue_point_id.write_le(cw.writer)?;
                write_str(cw.writer, &to_sz(&note.data), text_encoding)?;
            },
            Self::Ltxt(ltxt) => {
                let cw = ChunkWriter::begin(writer, b"ltxt")?;
                ltxt.cue_point_id.write_le(cw.writer)?;
                ltxt.sample_length.write_le(cw.writer)?;
                write_str_sized(cw.writer, &ltxt.purpose_id, 4, text_encoding)?;
                ltxt.country.write_le(cw.writer)?;
                ltxt.language.write_le(cw.writer)?;
                ltxt.dialect.write_le(cw.writer)?;
                ltxt.code_page.write_le(cw.writer)?;
                write_str(cw.writer, &to_sz(&ltxt.data), text_encoding)?;
            },
            Self::File(file) => {
                let cw = ChunkWriter::begin(writer, b"file")?;
                file.cue_point_id.write_le(cw.writer)?;
                file.media_type.write_le(cw.writer)?;
                cw.writer.write_all(&file.file_data)?;
            },
        }
        Ok(())
    }

    pub fn get_cue_point_id(&self) -> u32 {
        match self {
            Self::Labl(labl) => labl.cue_point_id,
            Self::Note(lote) => lote.cue_point_id,
            Self::Ltxt(ltxt) => ltxt.cue_point_id,
            Self::File(lile) => lile.cue_point_id,
        }
    }
}

impl ListChunk {
    pub fn read(reader: &mut impl Reader, chunk_size: u64, text_encoding: &StringCodecMaps) -> Result<Self, AudioReadError> {
        let end_of_chunk = ChunkHeader::align(reader.stream_position()? + chunk_size);
        let mut flag = [0u8; 4];
        reader.read_exact(&mut flag)?;
        match &flag {
            b"info" | b"INFO" => {
                let dict = Self::read_dict(reader, end_of_chunk, text_encoding)?;
                Ok(Self::Info(dict))
            },
            b"adtl" => {
                let mut adtl_map = BTreeMap::<u32, AdtlChunk>::new();
                while reader.stream_position()? < end_of_chunk {
                    let adtl = AdtlChunk::read(reader, text_encoding)?;
                    let cue_point_id = adtl.get_cue_point_id();
                    if let Some(dup) = adtl_map.insert(cue_point_id, adtl.clone()) {
                        // If the chunk point ID duplicates,  the new one will be used to overwrite the old one.
                        eprintln!("Duplicated chunk point ID {cue_point_id} for the `Adtl` data: old is {:?}, and will be overwritten by the new one: {:?}", dup, adtl);
                    }
                }
                Ok(Self::Adtl(adtl_map))
            },
            other => {
                Err(AudioReadError::Unimplemented(format!("Unknown indentifier in LIST chunk: {}", text_encoding.decode_flags(other))))
            },
        }
    }

    pub fn write(&self, writer: &mut dyn Writer, text_encoding: &StringCodecMaps) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer, b"LIST")?;
        match self {
            Self::Info(dict) => {
                cw.writer.write_all(b"INFO")?;
                Self::write_dict(&mut cw.writer, dict, text_encoding)?;
            },
            Self::Adtl(adtls) => {
                cw.writer.write_all(b"adtl")?;
                for (_cue_point_id, adtl) in adtls.iter() {
                    adtl.write(&mut cw.writer, text_encoding)?;
                }
            },
        };
        Ok(())
    }

    fn read_dict(reader: &mut impl Reader, end_of_chunk: u64, text_encoding: &StringCodecMaps) -> Result<BTreeMap<String, String>, AudioReadError> {
        // The INFO chunk consists of multiple key-value pairs for song metadata. 
        // Within its byte size constraints, read all key-value entries.
        let mut dict = BTreeMap::<String, String>::new();
        while reader.stream_position()? < end_of_chunk {
            let key_chunk = ChunkHeader::read(reader)?; // Every chunk's name is a key, its content is the value.
            let value_str = read_str(reader, key_chunk.size as usize, text_encoding)?;
            let key_str = text_encoding.decode(&key_chunk.flag);
            dict.insert(key_str, value_str);
            key_chunk.seek_to_next_chunk(reader)?;
        }
        // Let's try to store the key in uppercase form, if the INFO chunk provides both uppercase or lowercase, we store both of them.
        let mut to_be_added = Vec::<(String, String)>::new();
        for (key, val) in dict.iter() {
            let key_uppercase = key.to_uppercase();
            if key_uppercase == *key {
                // It is uppercase originally.
                continue;
            }
            if dict.contains_key(&key_uppercase) {
                // The LIST INFO chunk provided the uppercase key, and its value may be different from the lowercase key value, better not to overwrite it.
                continue;
            }
            to_be_added.push((key_uppercase, val.clone()));
        }
        for (key_uppercase, val) in to_be_added.into_iter() {
            dict.insert(key_uppercase, val);
        }
        Ok(dict)
    }

    fn write_dict(writer: &mut dyn Writer, dict: &BTreeMap<String, String>, text_encoding: &StringCodecMaps) -> Result<(), AudioWriteError> {
        for (key, val) in dict.iter() {
            if key.len() != 4 {
                return Err(AudioWriteError::InvalidArguments("flag must be 4 bytes".to_owned()));
            }
            let bytes = key.as_bytes();
            let flag = [bytes[0], bytes[1], bytes[2], bytes[3]];
            let cw = ChunkWriter::begin(writer, &flag)?;
            let mut val = val.clone();
            val.push('\0');
            write_str(cw.writer, &val, text_encoding)?;
        }
        Ok(())
    }
}

pub fn get_list_info_map() -> BTreeMap<&'static str, &'static str> {
    [ // https://www.recordingblogs.com/wiki/list-chunk-of-a-wave-file
        ("IARL", "The location where the subject of the file is archived"),
        ("IART", "The artist of the original subject of the file"),
        ("ICMS", "The name of the person or organization that commissioned the original subject of the file"),
        ("ICMT", "General comments about the file or its subject"),
        ("ICOP", "Copyright information about the file (e.g., \"Copyright Some Company 2011\")"),
        ("ICRD", "The date the subject of the file was created (creation date) (e.g., \"2022-12-31\")"),
        ("ICRP", "Whether and how an image was cropped"),
        ("IDIM", "The dimensions of the original subject of the file"),
        ("IDPI", "Dots per inch settings used to digitize the file"),
        ("IENG", "The name of the engineer who worked on the file"),
        ("IGNR", "The genre of the subject"),
        ("IKEY", "A list of keywords for the file or its subject"),
        ("ILGT", "Lightness settings used to digitize the file"),
        ("IMED", "Medium for the original subject of the file"),
        ("INAM", "Title of the subject of the file (name)"),
        ("IPLT", "The number of colors in the color palette used to digitize the file"),
        ("IPRD", "Name of the title the subject was originally intended for"),
        ("ISBJ", "Description of the contents of the file (subject)"),
        ("ISFT", "Name of the software package used to create the file"),
        ("ISRC", "The name of the person or organization that supplied the original subject of the file"),
        ("ISRF", "The original form of the material that was digitized (source form)"),
        ("ITCH", "The name of the technician who digitized the subject file"),
        ("ITRK", "The track number of the file")
    ].iter().copied().collect()
}

pub trait ListInfo {
    fn get_is_list_info(&self) -> bool;
    fn get(&self, key: &str) -> Option<&String>;
    fn set(&mut self, key: &str, value: &str) -> Result<Option<String>, AudioError>;

    fn get_archive(&self) -> Option<&String> {self.get("IARL")}
    fn get_artist(&self) -> Option<&String> {self.get("IART")}
    fn get_comment(&self) -> Option<&String> {self.get("ICMT")}
    fn get_producer(&self) -> Option<&String> {self.get("ICMS")}
    fn get_copyright(&self) -> Option<&String> {self.get("ICOP")}
    fn get_create_date(&self) -> Option<&String> {self.get("ICRD")}
    fn get_engineer(&self) -> Option<&String> {self.get("IENG")}
    fn get_genre(&self) -> Option<&String> {self.get("IGNR")}
    fn get_keywords(&self) -> Option<&String> {self.get("IKEY")}
    fn get_lightness(&self) -> Option<&String> {self.get("ILGT")}
    fn get_medium(&self) -> Option<&String> {self.get("IMED")}
    fn get_name(&self) -> Option<&String> {self.get("INAM")}
    fn get_album(&self) -> Option<&String> {self.get("IPRD")}
    fn get_description(&self) -> Option<&String> {self.get("ISBJ")}
    fn get_software(&self) -> Option<&String> {self.get("ISFT")}
    fn get_source(&self) -> Option<&String> {self.get("ISRC")}
    fn get_orig_form(&self) -> Option<&String> {self.get("ISRF")}
    fn get_technician(&self) -> Option<&String> {self.get("ITCH")}
    fn get_track_no(&self) -> Option<&String> {self.get("ITRK")}

    fn get_track_no_as_number(&self) -> Result<u32, AudioError> {
        if let Some(track_no) = self.get_track_no() {
            match track_no.parse::<u32>() {
                Ok(track_no) => Ok(track_no),
                Err(_) => Err(AudioError::Unparseable(track_no.clone())),
            }
        } else {
            Err(AudioError::NoSuchData("ITRK".to_owned()))
        }
    }

    fn set_archive(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IARL", value)}
    fn set_artist(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IART", value)}
    fn set_comment(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ICMT", value)}
    fn set_producer(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ICMS", value)}
    fn set_copyright(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ICOP", value)}
    fn set_create_date(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ICRD", value)}
    fn set_engineer(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IENG", value)}
    fn set_genre(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IGNR", value)}
    fn set_keywords(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IKEY", value)}
    fn set_lightness(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ILGT", value)}
    fn set_medium(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IMED", value)}
    fn set_name(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("INAM", value)}
    fn set_album(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("IPRD", value)}
    fn set_description(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ISBJ", value)}
    fn set_software(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ISFT", value)}
    fn set_source(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ISRC", value)}
    fn set_orig_form(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ISRF", value)}
    fn set_technician(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ITCH", value)}
    fn set_track_no(&mut self, value: &str) -> Result<Option<String>, AudioError> {self.set("ITRK", value)}
    fn set_track_no_as_number(&mut self, track_no: u32) -> Result<u32, AudioError> {
        match self.set_track_no(&format!("{track_no}")) {
            Err(e) => Err(e),
            Ok(track_no) => {
                if let Some(track_no) = track_no {
                    match track_no.parse::<u32>() {
                        Ok(track_no) => Ok(track_no),
                        Err(_) => Err(AudioError::Unparseable(track_no.clone())),
                    }
                } else {
                    Ok(0)
                }
            },
        }
    }
}

impl ListInfo for ListChunk {
    fn get_is_list_info(&self) -> bool {
        matches!(self, Self::Info(_))
    }

    fn get(&self, key: &str) -> Option<&String> {
        if let Self::Info(dict) = self {dict.get(key)} else {None}
    }

    fn set(&mut self, key: &str, value: &str) -> Result<Option<String>, AudioError> {
        if let Self::Info(ref mut dict) = self {
            Ok(dict.insert(key.to_owned(), value.to_string()))
        } else {
            Err(AudioError::InvalidArguments("The type of the `LIST` chunk is `adtl`, not `INFO`, so can not set the metadata values.".to_owned()))
        }
    }
}

#[allow(clippy::zero_prefixed_literal)]
pub fn get_country_code_map() -> HashMap<u16, &'static str> {
    [ // https://wavref.til.cafe/chunk/cset/
        (000, "None (assume 001 = USA)"),
        (001, "USA"),
        (002, "Canada"),
        (003, "Latin America"),
        (030, "Greece"),
        (031, "Netherlands"),
        (032, "Belgium"),
        (033, "France"),
        (034, "Spain"),
        (039, "Italy"),
        (041, "Switzerland"),
        (043, "Austria"),
        (044, "United Kingdom"),
        (045, "Denmark"),
        (046, "Sweden"),
        (047, "Norway"),
        (049, "West Germany"),
        (052, "Mexico"),
        (055, "Brazil"),
        (061, "Australia"),
        (064, "New Zealand"),
        (081, "Japan"),
        (082, "Korea"),
        (086, "People’s Republic of China"),
        (088, "Taiwan"),
        (090, "Turkey"),
        (351, "Portugal"),
        (352, "Luxembourg"),
        (354, "Iceland"),
        (358, "Finland"),
    ].iter().copied().collect()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq)]
pub struct LanguageDialect {
    lang: u16,
    dial: u16,
}

impl Eq for LanguageDialect{}

#[derive(Debug, Clone, Copy)]
pub struct LanguageSpecification {
    lang: &'static str,
    spec: &'static str,
}

pub fn get_language_dialect_code_map() -> HashMap<LanguageDialect, LanguageSpecification> {
    [ // https://wavref.til.cafe/chunk/cset/
        (LanguageDialect{lang: 0 ,  dial: 0}, LanguageSpecification{lang: "None (assume 9,1 = US English)", spec: "RIFF1991"}),
        (LanguageDialect{lang: 1 ,  dial: 1}, LanguageSpecification{lang: "Arabic", spec: "RIFF1991"}),
        (LanguageDialect{lang: 2 ,  dial: 1}, LanguageSpecification{lang: "Bulgarian", spec: "RIFF1991"}),
        (LanguageDialect{lang: 3 ,  dial: 1}, LanguageSpecification{lang: "Catalan", spec: "RIFF1991"}),
        (LanguageDialect{lang: 4 ,  dial: 1}, LanguageSpecification{lang: "Traditional Chinese", spec: "RIFF1991"}),
        (LanguageDialect{lang: 4 ,  dial: 2}, LanguageSpecification{lang: "Simplified Chinese", spec: "RIFF1991"}),
        (LanguageDialect{lang: 5 ,  dial: 1}, LanguageSpecification{lang: "Czech", spec: "RIFF1991"}),
        (LanguageDialect{lang: 6 ,  dial: 1}, LanguageSpecification{lang: "Danish", spec: "RIFF1991"}),
        (LanguageDialect{lang: 7 ,  dial: 1}, LanguageSpecification{lang: "German", spec: "RIFF1991"}),
        (LanguageDialect{lang: 7 ,  dial: 2}, LanguageSpecification{lang: "Swiss German", spec: "RIFF1991"}),
        (LanguageDialect{lang: 8 ,  dial: 1}, LanguageSpecification{lang: "Greek", spec: "RIFF1991"}),
        (LanguageDialect{lang: 9 ,  dial: 1}, LanguageSpecification{lang: "US English", spec: "RIFF1991"}),
        (LanguageDialect{lang: 9 ,  dial: 2}, LanguageSpecification{lang: "UK English", spec: "RIFF1991"}),
        (LanguageDialect{lang: 10,  dial: 1}, LanguageSpecification{lang: "Spanish", spec: "RIFF1991"}),
        (LanguageDialect{lang: 10,  dial: 2}, LanguageSpecification{lang: "Spanish", spec: "Mexican RIFF1991"}),
        (LanguageDialect{lang: 11,  dial: 1}, LanguageSpecification{lang: "Finnish", spec: "RIFF1991"}),
        (LanguageDialect{lang: 12,  dial: 1}, LanguageSpecification{lang: "French", spec: "RIFF1991"}),
        (LanguageDialect{lang: 12,  dial: 2}, LanguageSpecification{lang: "Belgian French", spec: "RIFF1991"}),
        (LanguageDialect{lang: 12,  dial: 3}, LanguageSpecification{lang: "Canadian French", spec: "RIFF1991"}),
        (LanguageDialect{lang: 12,  dial: 4}, LanguageSpecification{lang: "Swiss French", spec: "RIFF1991"}),
        (LanguageDialect{lang: 13,  dial: 1}, LanguageSpecification{lang: "Hebrew", spec: "RIFF1991"}),
        (LanguageDialect{lang: 14,  dial: 1}, LanguageSpecification{lang: "Hungarian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 15,  dial: 1}, LanguageSpecification{lang: "Icelandic", spec: "RIFF1994"}),
        (LanguageDialect{lang: 16,  dial: 1}, LanguageSpecification{lang: "Italian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 16,  dial: 2}, LanguageSpecification{lang: "Swiss Italian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 17,  dial: 1}, LanguageSpecification{lang: "Japanese", spec: "RIFF1994"}),
        (LanguageDialect{lang: 18,  dial: 1}, LanguageSpecification{lang: "Korean", spec: "RIFF1994"}),
        (LanguageDialect{lang: 19,  dial: 1}, LanguageSpecification{lang: "Dutch", spec: "RIFF1994"}),
        (LanguageDialect{lang: 19,  dial: 2}, LanguageSpecification{lang: "Belgian Dutch", spec: "RIFF1994"}),
        (LanguageDialect{lang: 20,  dial: 1}, LanguageSpecification{lang: "Norwegian - Bokmal", spec: "RIFF1994"}),
        (LanguageDialect{lang: 20,  dial: 2}, LanguageSpecification{lang: "Norwegian - Nynorsk", spec: "RIFF1994"}),
        (LanguageDialect{lang: 21,  dial: 1}, LanguageSpecification{lang: "Polish", spec: "RIFF1994"}),
        (LanguageDialect{lang: 22,  dial: 1}, LanguageSpecification{lang: "Brazilian Portuguese", spec: "RIFF1994"}),
        (LanguageDialect{lang: 22,  dial: 2}, LanguageSpecification{lang: "Portuguese", spec: "RIFF1994"}),
        (LanguageDialect{lang: 23,  dial: 1}, LanguageSpecification{lang: "Rhaeto-Romanic", spec: "RIFF1994"}),
        (LanguageDialect{lang: 24,  dial: 1}, LanguageSpecification{lang: "Romanian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 25,  dial: 1}, LanguageSpecification{lang: "Russian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 26,  dial: 1}, LanguageSpecification{lang: "Serbo-Croatian (Latin)", spec: "RIFF1994"}),
        (LanguageDialect{lang: 26,  dial: 2}, LanguageSpecification{lang: "Serbo-Croatian (Cyrillic)", spec: "RIFF1994"}),
        (LanguageDialect{lang: 27,  dial: 1}, LanguageSpecification{lang: "Slovak", spec: "RIFF1994"}),
        (LanguageDialect{lang: 28,  dial: 1}, LanguageSpecification{lang: "Albanian", spec: "RIFF1994"}),
        (LanguageDialect{lang: 29,  dial: 1}, LanguageSpecification{lang: "Swedish", spec: "RIFF1994"}),
        (LanguageDialect{lang: 30,  dial: 1}, LanguageSpecification{lang: "Thai", spec: "RIFF1994"}),
        (LanguageDialect{lang: 31,  dial: 1}, LanguageSpecification{lang: "Turkish", spec: "RIFF1994"}),
        (LanguageDialect{lang: 32,  dial: 1}, LanguageSpecification{lang: "Urdu", spec: "RIFF1994"}),
        (LanguageDialect{lang: 33,  dial: 1}, LanguageSpecification{lang: "Bahasa", spec: "RIFF1994"}),
    ].iter().copied().collect()
}

#[derive(Debug, Clone)]
pub struct FullInfoCuePoint {
    pub data_chunk_id: [u8; 4],
    pub label: String,
    pub note: String,
    pub sample_length: u32,
    pub purpose_id: String,
    pub country: String,
    pub language: String,
    pub text_data: String,
    pub media_type: u32,
    pub file_data: Vec<u8>,
    pub start_sample: u32,
    pub num_samples: u32,
    pub repeats: u32,
}

impl FullInfoCuePoint {
    pub fn new(cue_point_id: u32, cue_point: &CuePoint, adtl_chunks: &BTreeMap<u32, AdtlChunk>, plst: &Option<&Plst>, country_code_map: &HashMap<u16, &'static str>, dialect_code_map: &HashMap<LanguageDialect, LanguageSpecification>) -> Result<Self, AudioError> {
        let mut ret = Self {
            data_chunk_id: cue_point.data_chunk_id,
            label: String::new(),
            note: String::new(),
            sample_length: 0,
            purpose_id: String::new(),
            country: String::new(),
            language: String::new(),
            text_data: String::new(),
            media_type: 0,
            file_data: Vec::<u8>::new(),
            start_sample: cue_point.position,
            num_samples: 0,
            repeats: 0,
        };
        if let Some(plst) = plst{
            ret.num_samples = plst.num_samples;
            ret.repeats = plst.repeats;
        } else {
            eprintln!("Lack of `plst` chunk, `num_samples` should be calculated by yourself, and `repeats` remains zero.");
        }
        if let Some(adtl) = adtl_chunks.get(&cue_point_id) {
            match adtl {
                AdtlChunk::Labl(labl) => ret.label = labl.data.clone(),
                AdtlChunk::Note(note) => ret.note = note.data.clone(),
                AdtlChunk::Ltxt(ltxt) => {
                    let lang_dial = LanguageDialect{
                        lang: ltxt.language,
                        dial: ltxt.dialect,
                    };
                    let unknown_lang_spec = LanguageSpecification{
                        lang: "Unknown",
                        spec: "UKNW1994"
                    };
                    let country = if let Some(country) = country_code_map.get(&ltxt.country) {
                        country.to_string()
                    } else {
                        format!("Unknown country code {}", ltxt.country)
                    };
                    let language = if let Some(lang_dial) = dialect_code_map.get(&lang_dial) {
                        *lang_dial
                    } else {
                        unknown_lang_spec
                    }.lang.to_owned();
                    ret.sample_length = ltxt.sample_length;
                    ret.purpose_id = ltxt.purpose_id.clone();
                    ret.country = country;
                    ret.language = language;
                    ret.text_data = ltxt.data.clone();
                },
                AdtlChunk::File(file) => {
                    ret.media_type = file.media_type;
                    ret.file_data = file.file_data.clone();
                },
            }
            Ok(ret)
        } else {
            Err(AudioError::NoSuchData(format!("ADTL data for cue point ID: {cue_point_id}")))
        }
    }
}

pub fn create_full_info_cue_data(cue_chunk: &CueChunk, adtl_chunks: &BTreeMap<u32, AdtlChunk>, plstchunk: &Option<PlstChunk>) -> Result<BTreeMap<u32, FullInfoCuePoint>, AudioError> {
    let country_code_map = get_country_code_map();
    let dialect_code_map = get_language_dialect_code_map();
    let plstmap = if let Some(plstchunk) = plstchunk {
        plstchunk.build_map()
    } else {
        BTreeMap::<u32, Plst>::new()
    };
    cue_chunk.cue_points.iter().map(|cue| -> Result<(u32, FullInfoCuePoint), AudioError> {
        match FullInfoCuePoint::new(cue.cue_point_id, cue, adtl_chunks, &plstmap.get(&cue.cue_point_id), &country_code_map, &dialect_code_map) {
            Ok(full_info_cue_data) => Ok((cue.cue_point_id, full_info_cue_data)),
            Err(e) => Err(e),
        }
    }).collect::<Result<BTreeMap<u32, FullInfoCuePoint>, AudioError>>()
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
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
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

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"acid")?;
        self.flags.write_le(cw.writer)?;
        self.root_node.write_le(cw.writer)?;
        self.reserved1.write_le(cw.writer)?;
        self.reserved2.write_le(cw.writer)?;
        self.num_beats.write_le(cw.writer)?;
        self.meter_denominator.write_le(cw.writer)?;
        self.meter_numerator.write_le(cw.writer)?;
        self.tempo.write_le(cw.writer)?;
        Ok(())
    }
}

#[derive(Clone)]
pub enum JunkChunk{
    FullZero(u64),
    SomeData(Vec<u8>),
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

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"JUNK")?;
        match self {
            Self::FullZero(size) => cw.writer.write_all(&vec![0u8; *size as usize])?,
            Self::SomeData(data) => cw.writer.write_all(data)?,
        }
        Ok(())
    }
}

impl Debug for JunkChunk {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::FullZero(size) => write!(f, "[0u8; {size}]"),
            Self::SomeData(data) => write!(f, "[{}]", data.iter().map(|byte|{format!("0x{:02}", byte)}).collect::<Vec<String>>().join(", ")),
        }
    }
}

// If the `id3` feature is enabled, use it to read ID3 data.
#[cfg(feature = "id3")]
#[allow(non_snake_case)]
pub mod Id3{
    use std::{io::{Read, Write, Seek}};
    use crate::errors::{AudioReadError, AudioWriteError, IOErrorInfo};
    pub type Tag = id3::Tag;

    pub fn id3_read<R>(reader: &mut R, _size: usize) -> Result<Tag, AudioReadError>
    where R: Read + Seek + ?Sized {
        Ok(Tag::read_from2(reader)?)
    }

    pub fn id3_write<W>(tag: &Tag, writer: &mut W) -> Result<(), AudioWriteError>
    where W: Write + ?Sized {
        Ok(tag.write_to(writer, tag.version())?)
    }

    impl From<id3::Error> for AudioReadError {
        fn from(err: id3::Error) -> Self {
            match err.kind {
                id3::ErrorKind::Io(ioerr) => AudioReadError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()}),
                id3::ErrorKind::StringDecoding(bytes) => AudioReadError::StringDecodeError(bytes),
                id3::ErrorKind::NoTag => AudioReadError::FormatError(err.description),
                id3::ErrorKind::Parsing => AudioReadError::DataCorrupted(err.description),
                id3::ErrorKind::InvalidInput => AudioReadError::DataCorrupted(err.description),
                id3::ErrorKind::UnsupportedFeature => AudioReadError::Unsupported(err.description),
            }
        }
    }

    impl From<id3::Error> for AudioWriteError {
        fn from(err: id3::Error) -> Self {
            match err.kind {
                id3::ErrorKind::Io(ioerr) => AudioWriteError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()}),
                id3::ErrorKind::StringDecoding(bytes) => AudioWriteError::StringDecodeError(bytes),
                id3::ErrorKind::NoTag => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::Parsing => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::InvalidInput => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::UnsupportedFeature => AudioWriteError::Unsupported(err.description),
            }
        }
    }
}

// If the `id3` feature is disabled, read the raw bytes.
#[cfg(not(feature = "id3"))]
#[allow(non_snake_case)]
pub mod Id3{
    use std::io::Read;
    use std::vec::Vec;
    use std::error::Error;
    #[derive(Clone)]
    pub struct Tag {
        pub data: Vec<u8>,
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

    pub fn id3_write<W>(tag: &Tag, writer: &mut W) -> Result<(), AudioWriteError>
    where W: Write + ?Sized {
        #[cfg(debug_assertions)]
        println!("Feature \"id3\" was not enabled, the saved id3 binary bytes may not correct, consider compile with \"cargo build --features id3\"");
        Ok(writer.write_all(&tag.data))
    }

    impl std::fmt::Debug for Tag {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            fmt.debug_struct("Tag")
                .finish_non_exhaustive()
        }
    }
}
