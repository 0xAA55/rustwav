#![allow(dead_code)]

use std::{
    collections::{BTreeMap, HashMap},
    convert::From,
    fmt::{self, Debug, Display, Formatter},
    io::{self, Read, SeekFrom, Write},
};

use crate::SampleType;
use crate::adpcm::ms::AdpcmCoeffSet;
use crate::readwrite::{self, string_io::*};
use crate::savagestr::{SavageStringCodecs, StringCodecMaps};
use crate::{AudioError, AudioReadError, AudioWriteError};
use crate::{Mp3EncoderOptions, OpusEncoderOptions};
use crate::{Reader, Writer};
use crate::downmixer;

#[allow(unused_imports)]
pub use flac::{FlacCompression, FlacEncoderParams};

#[allow(unused_imports)]
pub use crate::{OggVorbisBitrateStrategy, OggVorbisEncoderParams};

/// ## Specify the audio codecs of the WAV file.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum DataFormat {
    /// * This is used for creating a new `DataFormat` to specify an `unknown` format.
    Unspecified,

    /// * PCM format, supports `u8`, `i16`, `i24`, `i32`, `f32`, `f64` for WAV, supports channel number >= 2, no compresion, lossless.
    Pcm,

    /// * ADPCM format, every sample stores as nibbles (One 4-bit nibble for a 16-bit sample), max channels is 2, lossy. Good for voice chatting, and very small memory usage.
    Adpcm(AdpcmSubFormat),

    /// * PCM-aLaw, every sample stores as a byte (One byte for a 16-bit sample), max channels is 2, lossy. Encoding/decoding is by table lookup. These tables are not that small.
    /// * Kind of useless. I prefer just to use the plain `u8` PCM format to replace it. My supreme audio card can handle my `u8` PCM and the playback is just as perfect as `i16` does.
    PcmALaw,

    /// * PCM-MuLaw. Not much different than the PCM-aLaw. Uses a different algorithm to encode.
    PcmMuLaw,

    /// * MP3. Just a pure MP3 file encapsulated in the `data` chunk. It needs some extra extension data in the `fmt ` chunk.
    /// * With the help of the WAV `fmt ` chunk, you can get the spec of the audio file without decoding it first.
    /// * The WAV file which encapsulates the MP3 file as its content, the size of the WAV file looks like an MP3 file size.
    Mp3(Mp3EncoderOptions),

    /// * Naked opus stream, without the Ogg container. The encoded data is stored as blocks in the `data` chunk, the block size is stored in the `fmt ` chunk.
    /// * Just take a look at the blocks, there are lots of zero bytes at the end, indicating that the Opus format is excellent at compressing the audio data.
    /// * But WAV can't get rid of these zero bytes, resulting in the compression ratio just like encoding each sample into a byte.
    /// * Opus was originally designed for low-lag digital audio transmission with good quality. Encapsulating this thing into a WAV file is very weird.
    Opus(OpusEncoderOptions),

    /// * FLAC. Just a pure FLAC file encapsulated in the `data` chunk.
    /// * With the help of the WAV `fmt ` chunk, you can get the spec of the audio file without decoding it first.
    /// * The WAV file which encapsulates the FLAC file as its content, the size of the WAV file looks like an FLAC file size.
    Flac(FlacEncoderParams),

    /// * OggVorbis. Just a pure OggVorbis file encapsulated in the `data` chunk.
    /// * The WAV file which encapsulates the OggVorbis file as its content, the size of the WAV file looks like an OggVorbis file size.
    OggVorbis(OggVorbisEncoderParams),
}

/// * When to encode audio to ADPCM format, choose one of the subformats.
/// * The value of the subformat is the `format_tag` field of the `fmt ` chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u16)]
pub enum AdpcmSubFormat {
    /// * This is for ADPCM-MS
    Ms = 0x0002,

    /// * This is for ADPCM-IMA
    Ima = 0x0011,

    /// * This is for ADPCM-YAMAHA. The Yamaha ADPCM algorithm is the easiest one to implement.
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
            Self::OggVorbis(options) => write!(f, "OggVorbis({:?})", options),
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

pub mod format_tags {
    pub const FORMAT_TAG_PCM          : u16 = 0x0001;
    pub const FORMAT_TAG_ADPCM_MS     : u16 = 0x0002;
    pub const FORMAT_TAG_PCM_IEEE     : u16 = 0x0003;
    pub const FORMAT_TAG_ALAW         : u16 = 0x0006;
    pub const FORMAT_TAG_MULAW        : u16 = 0x0007;
    pub const FORMAT_TAG_ADPCM_IMA    : u16 = 0x0011;
    pub const FORMAT_TAG_ADPCM_IMA_   : u16 = 0x0067;
    pub const FORMAT_TAG_ADPCM_YAMAHA : u16 = 0x0020;
    pub const FORMAT_TAG_MP3          : u16 = 0x0055;
    pub const FORMAT_TAG_OPUS         : u16 = 0x704F;
    pub const FORMAT_TAG_OGG_VORBIS1  : u16 = ('O' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_OGG_VORBIS2  : u16 = ('P' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_OGG_VORBIS3  : u16 = ('Q' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_OGG_VORBIS1P : u16 = ('o' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_OGG_VORBIS2P : u16 = ('p' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_OGG_VORBIS3P : u16 = ('q' as u16) | (('g' as u16) << 8);
    pub const FORMAT_TAG_FLAC         : u16 = 0xF1AC;
    pub const FORMAT_TAG_EXTENSIBLE   : u16 = 0xFFFE;
}

#[allow(unused_imports)]
pub use format_tags::*;

/// ## The rough type of the sample format.
#[derive(Debug, Clone, Copy)]
pub enum SampleFormat {
    Unknown,

    /// * IEEE 754 floaing number including `f32` and `f64`
    Float,

    /// * Unsigned integer
    UInt,

    /// * Signed integer
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

/// ## The concrete type of the sample format.
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
        use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, Unknown};
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
        use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, Unknown};
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::upper_case_acronyms)]
pub struct GUID(pub u32, pub u16, pub u16, pub [u8; 8]);

impl Debug for GUID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("GUID")
            .field(&format_args!(
                "{:08x}-{:04x}-{:04x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                self.0,
                self.1,
                self.2,
                self.3[0],
                self.3[1],
                self.3[2],
                self.3[3],
                self.3[4],
                self.3[5],
                self.3[6],
                self.3[7]
            ))
            .finish()
    }
}

impl Display for GUID {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <GUID as Debug>::fmt(self, f)
    }
}

impl GUID {
    pub fn read<T>(r: &mut T) -> Result<Self, io::Error>
    where
        T: Read,
    {
        Ok(Self(
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
            ],
        ))
    }

    pub fn write<T>(&self, w: &mut T) -> Result<(), io::Error>
    where
        T: Write + ?Sized,
    {
        self.0.write_le(w)?;
        self.1.write_le(w)?;
        self.2.write_le(w)?;
        w.write_all(&self.3)?;
        Ok(())
    }
}

pub mod guids {
    pub use super::GUID;

    pub const GUID_PCM_FORMAT: GUID =        GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
    pub const GUID_IEEE_FLOAT_FORMAT: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
}

pub use guids::*;

/// ## The spec info for a generic audio file.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    /// * Num channels
    pub channels: u16,

    /// * The channel mask indicates the position of the speakers.
    pub channel_mask: u32,

    /// * The sample rate. How many audio frames are to be played in a second.
    pub sample_rate: u32,

    /// * For PCM, this indicates how many bits is for a sample.
    pub bits_per_sample: u16,

    /// * The roughly described sample format
    pub sample_format: SampleFormat,
}

impl Default for Spec {
    fn default() -> Self {
        Self::new()
    }
}

/// * Infer the concrete type of the sample format from some rough data
#[allow(unused_imports)]
pub fn get_sample_type(bits_per_sample: u16, sample_format: SampleFormat) -> WaveSampleType {
    use SampleFormat::{Float, Int, UInt};
    use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, Unknown};
    match (bits_per_sample, sample_format) {
        (8, UInt) => U8,
        (16, Int) => S16,
        (24, Int) => S24,
        (32, Int) => S32,
        (64, Int) => S64,
        (32, Float) => F32,
        (64, Float) => F64,
        // WAV PCM supports only the formats listed above.
        (_, _) => Unknown,
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

    /// * Get the concrete sample type
    pub fn get_sample_type(&self) -> WaveSampleType {
        get_sample_type(self.bits_per_sample, self.sample_format)
    }

    /// * Guess the channel mask
    pub fn guess_channel_mask(&self) -> Result<u32, AudioError> {
        downmixer::speaker_positions::guess_channel_mask(self.channels)
    }

    /// * Break down a channel mask to the speaker positions.
    pub fn channel_mask_to_speaker_positions(&self) -> Vec<u32> {
        downmixer::speaker_positions::channel_mask_to_speaker_positions(self.channel_mask)
    }

    /// * Break down a channel mask to the speaker position description strings.
    pub fn channel_mask_to_speaker_positions_descs(&self) -> Vec<&'static str> {
        downmixer::speaker_positions::channel_mask_to_speaker_positions_descs(self.channel_mask)
    }

    /// * Check if this spec is good for encoding PCM format.
    pub fn verify_for_pcm(&self) -> Result<(), AudioError> {
        self.guess_channel_mask()?;
        if self.get_sample_type() == WaveSampleType::Unknown {
            Err(AudioError::InvalidArguments(format!(
                "PCM doesn't support {} bits per sample {:?}",
                self.bits_per_sample, self.sample_format
            )))
        } else {
            Ok(())
        }
    }

    /// * Check if the channel mask matches the channel number.
    pub fn is_channel_mask_valid(&self) -> bool {
        downmixer::speaker_positions::is_channel_mask_valid(self.channels, self.channel_mask)
    }
}

/// ## The WAV chunk writer used by `WaveWriter`
/// * It remembers the chunk header field positions.
/// * When it gets out of the scope or to be dropped, it updates the field of the chunk header.
pub struct ChunkWriter<'a> {
    pub writer: &'a mut dyn Writer,
    pub flag: [u8; 4],
    pub pos_of_chunk_len: u64, // Byte position where the chunk size field is written (to be backfilled later)
    pub chunk_start: u64,      // File offset marking the start of this chunk's payload data
    ended: bool,
}

impl Debug for ChunkWriter<'_> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("ChunkWriter")
            .field("writer", &self.writer)
            .field(
                "flag",
                &format_args!("{}", String::from_utf8_lossy(&self.flag)),
            )
            .field("pos_of_chunk_len", &format_args!("0x{:x}", self.pos_of_chunk_len))
            .field("chunk_start", &format_args!("0x{:x}", self.chunk_start))
            .field("ended", &self.ended)
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
        Ok(Self {
            writer,
            flag: *flag,
            pos_of_chunk_len,
            chunk_start,
            ended: false,
        })
    }

    /// * At the end of the chunk, the chunk size will be updated since the ownership of `self` moved there, and `drop()` will be called.
    pub fn end(self) {}

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
                }
                other => {
                    let chunk_flag = String::from_utf8_lossy(other);
                    return Err(AudioWriteError::ChunkSizeTooBig(format!(
                        "{} is 0x{:x} bytes long.",
                        chunk_flag, chunk_size
                    )));
                }
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
        } // Alignment
        self.ended = true;
        Ok(())
    }

    /// * For you to write data from the start of the chunk, here's a method for you to get the offset.
    pub fn get_chunk_start_pos(&self) -> u64 {
        self.chunk_start
    }

    /// * A method to calculate currently how big the chunk is.
    pub fn get_chunk_data_size(&mut self) -> Result<u64, AudioWriteError> {
        Ok(self.writer.stream_position()? - self.get_chunk_start_pos())
    }
}

impl Drop for ChunkWriter<'_> {
    fn drop(&mut self) {
        self.on_drop().unwrap()
    }
}

/// ## This thing is for reading a chunk
#[derive(Clone, Copy)]
pub struct ChunkHeader {
    /// * The 4-byte identifier stored in the file (e.g., "RIFF", "fmt ")
    pub flag: [u8; 4],

    /// * The chunk size stored in the file header (may be 0xFFFFFFFF if actual size is in ds64 chunk)
    pub size: u32,

    /// * File offset of the chunk's payload (excludes the 8-byte header)
    pub chunk_start_pos: u64,
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
        let mut ret = Self::new();
        reader.read_exact(&mut ret.flag)?;
        ret.size = u32::read_le(reader)?;
        ret.chunk_start_pos = reader.stream_position()?;
        Ok(ret)
    }

    pub fn read_unseekable(
        reader: &mut impl Reader,
        cur_pos: &mut u64,
    ) -> Result<Self, AudioReadError> {
        let mut ret = Self::new();
        reader.read_exact(&mut ret.flag)?;
        ret.size = u32::read_le(reader)?;
        *cur_pos += 8;
        ret.chunk_start_pos = *cur_pos;
        Ok(ret)
    }

    /// * Calculate alignment
    pub fn align(addr: u64) -> u64 {
        addr + (addr & 1)
    }

    /// * Calculate the position of the next chunk
    pub fn next_chunk_pos(&self) -> u64 {
        Self::align(self.chunk_start_pos + self.size as u64)
    }

    /// * Seek to the next chunk (with alignment)
    pub fn seek_to_next_chunk(&self, reader: &mut impl Reader) -> Result<u64, AudioReadError> {
        Ok(reader.seek(SeekFrom::Start(self.next_chunk_pos()))?)
    }

    /// * If can't seek, do some dummy reads to the next chunk (with alignment)
    pub fn goto_next_chunk_unseekable(
        &self,
        reader: &mut impl Reader,
        cur_pos: &mut u64,
    ) -> Result<u64, AudioReadError> {
        Ok(readwrite::goto_offset_without_seek(
            reader,
            cur_pos,
            self.next_chunk_pos(),
        )?)
    }
}

impl Debug for ChunkHeader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ChunkHeader")
        .field("flag", &String::from_utf8_lossy(&self.flag).to_owned())
        .field("size", &format_args!("0x{:x}", self.size))
        .field("chunk_start_pos", &format_args!("0x{:x}", self.chunk_start_pos))
        .finish()
    }
}

impl Default for ChunkHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// ## The `fmt ` chunk for the WAV file.
#[derive(Debug, Clone)]
pub struct FmtChunk {
    /// * See <https://github.com/tpn/winsdk-10/blob/master/Include/10.0.14393.0/shared/mmreg.h>
    pub format_tag: u16,

    /// * Num channels
    pub channels: u16,

    /// * Sample rate. It's actually `frame_rate` because it's the rate of how frequently the audio frames are played.
    /// * Each audio frame contains samples for each channel. For example, your audio file has two samples, which means an audio frame is two samples for each channel.
    /// * For another example, if the sample rate is 44100, but the channels are 2, it plays 88200 samples per second.
    pub sample_rate: u32,

    /// * How many bytes are to be played in a second. This field is important because lots of audio players use this field to calculate the playback progress.
    pub byte_rate: u32,

    /// * Block size. For PCM, it's sample size in bytes times to channel number. For non-PCM, it's the block size for the audio blocks.
    /// * The block size field is for quickly `seek()` the audio file.
    pub block_align: u16,

    /// * For PCM, this indicates how many bits are in a sample. For non-PCM, this field is either zero or some other meaningful value for the encoded format.
    pub bits_per_sample: u16,

    /// * The extension block for the `fmt ` chunk, its type depends on the `format_tag` value.
    pub extension: Option<FmtExtension>,
}

/// ## The `fmt ` chunk extension block
#[derive(Debug, Clone)]
pub struct FmtExtension {
    /// * Extension block size
    pub ext_len: u16,

    /// * Extension block data
    pub data: ExtensionData,
}

/// ## Extension block data
#[derive(Debug, Clone)]
pub enum ExtensionData {
    /// * If the extension block size is zero, here we have `Nodata` for it.
    Nodata,

    /// * ADPCM-MS specified extension data. Anyway, the decoder can generate the data if the format is ADPCM-MS and there's no data for it.
    AdpcmMs(AdpcmMsData),

    /// * ADPCM-IMA specified extension data. Kind of useless.
    AdpcmIma(AdpcmImaData),

    /// * MP3 specified extension data.
    Mp3(Mp3Data),

    /// * OggVorbis specified extension data.
    OggVorbis(OggVorbisData),

    /// * Another OggVorbis specified extension data.
    OggVorbisWithHeader(OggVorbisWithHeaderData),

    /// * Extensible data, it has channel mask, GUID for formats, etc, dedicated for multi-channel PCM format.
    Extensible(ExtensibleData),
}

/// ## The extension data for ADPCM-MS
#[derive(Debug, Clone, Copy)]
pub struct AdpcmMsData {
    pub samples_per_block: u16,
    pub num_coeff: u16,
    pub coeffs: [AdpcmCoeffSet; 7],
}

/// ## The extension data for ADPCM-IMA
#[derive(Debug, Clone, Copy)]
pub struct AdpcmImaData {
    pub samples_per_block: u16,
}

/// ## The extension data for MP3
#[derive(Debug, Clone, Copy)]
pub struct Mp3Data {
    pub id: u16,
    pub flags: u32,
    pub block_size: u16,
    pub frames_per_block: u16,
    pub codec_delay: u16,
}

/// ## The extension data for OggVorbis
#[derive(Clone, Copy)]
pub struct OggVorbisData {
    /// * The codec version. I'm coding this thing at 2025/5/6, so this filed for our encoded WAV file should be 0x20250506
    pub codec_version: u32,

    /// * The `libvorbis` version, our `rustwav` depends on `vorbis_rs 0.5.5`, which uses `vorbis-sys`, which uses `libvorbis 1.3.7 20200704`
    /// * So this field must be 0x20200704 for our encoded WAV file.
    pub vorbis_version: u32,
}

impl Debug for OggVorbisData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("OggVorbisWithHeaderData")
        .field("codec_version", &format_args!("{:x}/{:x}/{:x}", self.codec_version >> 16, (self.codec_version >> 8) & 0xFF, self.codec_version & 0xFF))
        .field("vorbis_version", &format_args!("{:x}/{:x}/{:x}", self.vorbis_version >> 16, (self.vorbis_version >> 8) & 0xFF, self.vorbis_version & 0xFF))
        .finish()
    }
}

/// ## The another extension data for OggVorbis
#[derive(Clone)]
pub struct OggVorbisWithHeaderData {
    /// * The codec version. I'm coding this thing at 2025/5/6, so this filed for our encoded WAV file should be 0x20250506
    pub codec_version: u32,

    /// * The `libvorbis` version, our `rustwav` depends on `vorbis_rs 0.5.5`, which uses `vorbis-sys`, which uses `libvorbis 1.3.7 20200704`
    /// * So this field must be 0x20200704 for our encoded WAV file.
    pub vorbis_version: u32,

    /// * The OggVorbis header data
    pub header: Vec<u8>,
}

impl Debug for OggVorbisWithHeaderData {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("OggVorbisWithHeaderData")
        .field("codec_version", &format_args!("{:x}/{:x}/{:x}", self.codec_version >> 16, (self.codec_version >> 8) & 0xFF, self.codec_version & 0xFF))
        .field("vorbis_version", &format_args!("{:x}/{:x}/{:x}", self.vorbis_version >> 16, (self.vorbis_version >> 8) & 0xFF, self.vorbis_version & 0xFF))
        .field("header", &format_args!("[u8; {}]", self.header.len()))
        .finish()
    }
}

/// ## The extension data for extensible.
#[derive(Debug, Clone, Copy)]
pub struct ExtensibleData {
    /// * Valid bits per sample
    pub valid_bits_per_sample: u16,

    /// * This is for multi-channel speaker position masks, see `struct Spec`
    pub channel_mask: u32,

    /// * This field indicates the exact format for the PCM samples.
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
        let mut ret = FmtChunk {
            format_tag: u16::read_le(reader)?,
            channels: u16::read_le(reader)?,
            sample_rate: u32::read_le(reader)?,
            byte_rate: u32::read_le(reader)?,
            block_align: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            extension: None,
        };
        if chunk_size > 16 {
            ret.extension = Some(FmtExtension::read(reader, &ret)?);
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
        if let Some(extension) = &self.extension {
            extension.write(writer)?;
        }
        Ok(())
    }

    pub fn get_sample_format(&self) -> SampleFormat {
        use SampleFormat::{Float, Int, UInt, Unknown};
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
                if let Some(extension) = &self.extension {
                    match &extension.data {
                        ExtensionData::Extensible(extensible) => {
                            match extensible.sub_format {
                                GUID_PCM_FORMAT => Int,
                                GUID_IEEE_FLOAT_FORMAT => Float,
                                _ => Unknown, // Let the decoders to decide
                            }
                        }
                        other => {
                            panic!("Unexpected extension data in the `fmt ` chunk: {:?}", other)
                        }
                    }
                } else {
                    Int
                }
            }
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

    pub fn new_oggvorbis(oggvorbis: OggVorbisData) -> Self {
        Self {
            ext_len: OggVorbisData::sizeof() as u16,
            data: ExtensionData::OggVorbis(oggvorbis),
        }
    }

    pub fn new_oggvorbis_with_header(oggvorbis_with_header: &OggVorbisWithHeaderData) -> Self {
        Self {
            ext_len: oggvorbis_with_header.sizeof() as u16,
            data: ExtensionData::OggVorbisWithHeader(oggvorbis_with_header.clone()),
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

    pub fn read(reader: &mut impl Reader, fmt_chunk: &FmtChunk) -> Result<Self, AudioReadError> {
        let ext_len = u16::read_le(reader)?;
        Ok(Self {
            ext_len,
            data: match fmt_chunk.format_tag {
                FORMAT_TAG_ADPCM_MS => {
                    if ext_len as usize >= AdpcmMsData::sizeof() {
                        Ok(ExtensionData::AdpcmMs(AdpcmMsData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for ADPCM-MS should be bigger than {}, got {ext_len}",
                            AdpcmMsData::sizeof()
                        )))
                    }
                }
                FORMAT_TAG_ADPCM_IMA => {
                    if ext_len as usize >= AdpcmImaData::sizeof() {
                        Ok(ExtensionData::AdpcmIma(AdpcmImaData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for ADPCM-IMA should be bigger than {}, got {ext_len}",
                            AdpcmImaData::sizeof()
                        )))
                    }
                }
                FORMAT_TAG_MP3 => {
                    if ext_len as usize >= Mp3Data::sizeof() {
                        Ok(ExtensionData::Mp3(Mp3Data::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for Mpeg Layer III should be bigger than {}, got {ext_len}",
                            Mp3Data::sizeof()
                        )))
                    }
                }
                FORMAT_TAG_OGG_VORBIS1 | FORMAT_TAG_OGG_VORBIS3 | FORMAT_TAG_OGG_VORBIS1P | FORMAT_TAG_OGG_VORBIS3P => {
                    if ext_len as usize >= OggVorbisData::sizeof() {
                        Ok(ExtensionData::OggVorbis(OggVorbisData::read(reader)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for OggVorbis should be bigger than {}, got {ext_len}",
                            OggVorbisData::sizeof()
                        )))
                    }
                }
                FORMAT_TAG_OGG_VORBIS2 | FORMAT_TAG_OGG_VORBIS2P => {
                    if ext_len as usize >= OggVorbisWithHeaderData::sizeof_min() {
                        Ok(ExtensionData::OggVorbisWithHeader(OggVorbisWithHeaderData::read(reader, ext_len)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for OggVorbis should be bigger than {}, got {ext_len}",
                            OggVorbisWithHeaderData::sizeof_min()
                        )))
                    }
                }
                FORMAT_TAG_EXTENSIBLE => {
                    if ext_len as usize >= ExtensibleData::sizeof() {
                        Ok(ExtensionData::Extensible(ExtensibleData::read(reader)?))
                    } else if ext_len == 0 {
                        Ok(ExtensionData::Extensible(ExtensibleData::new(fmt_chunk)?))
                    } else {
                        Err(AudioReadError::IncompleteData(format!(
                            "The extension data for EXTENSIBLE should be bigger than {}, got {ext_len}",
                            ExtensibleData::sizeof()
                        )))
                    }
                }
                _ => Ok(ExtensionData::Nodata),
            }?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.ext_len.write_le(writer)?;
        if self.ext_len != 0 {
            match &self.data {
                ExtensionData::Nodata => Err(AudioWriteError::InvalidArguments(format!(
                    "There should be data in {} bytes to be written, but the data is `Nodata`.",
                    self.ext_len
                ))),
                ExtensionData::AdpcmMs(data) => Ok(data.write(writer)?),
                ExtensionData::AdpcmIma(data) => Ok(data.write(writer)?),
                ExtensionData::Mp3(data) => Ok(data.write(writer)?),
                ExtensionData::OggVorbis(data) => Ok(data.write(writer)?),
                ExtensionData::OggVorbisWithHeader(data) => Ok(data.write(writer)?),
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
        Ok(Self {
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
        Self { samples_per_block }
    }

    pub fn sizeof() -> usize {
        2
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
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
        Ok(Self {
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

impl OggVorbisData {
    pub fn new() -> Self {
        Self {
            codec_version: 0x20250506,
            vorbis_version: 0x20200704,
        }
    }

    pub fn sizeof() -> usize {
        8
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
            codec_version: u32::read_le(reader)?,
            vorbis_version: u32::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.codec_version.write_le(writer)?;
        self.vorbis_version.write_le(writer)?;
        Ok(())
    }
}

impl OggVorbisWithHeaderData {
    pub fn new(header: &[u8]) -> Self {
        Self {
            codec_version: 0x20250506,
            vorbis_version: 0x20200704,
            header: header.to_vec(),
        }
    }

    pub fn sizeof_min() -> usize {
        8
    }

    pub fn sizeof(&self) -> usize {
        Self::sizeof_min() + self.header.len()
    }

    pub fn read(reader: &mut impl Reader, ext_len: u16) -> Result<Self, AudioReadError> {
        let mut ret = Self {
            codec_version: u32::read_le(reader)?,
            vorbis_version: u32::read_le(reader)?,
            header: vec![0u8; ext_len as usize - Self::sizeof_min()],
        };
        reader.read_exact(&mut ret.header)?;
        Ok(ret)
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        self.codec_version.write_le(writer)?;
        self.vorbis_version.write_le(writer)?;
        writer.write_all(&self.header)?;
        Ok(())
    }
}


impl ExtensibleData {
    pub fn new(fmt_chunk: &FmtChunk) -> Result<Self, AudioReadError> {
        Ok(Self {
            valid_bits_per_sample: fmt_chunk.bits_per_sample,
            channel_mask: {
                let spec = Spec {
                    channels: fmt_chunk.channels,
                    channel_mask: 0,
                    sample_rate: fmt_chunk.sample_rate,
                    bits_per_sample: fmt_chunk.bits_per_sample,
                    sample_format: SampleFormat::Unknown,
                };
                spec.guess_channel_mask()?
            },
            sub_format: GUID_PCM_FORMAT,
        })
    }

    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
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

/// See <https://www.recordingblogs.com/wiki/silent-chunk-of-a-wave-file>
#[derive(Debug, Clone, Copy, Default)]
pub struct SlntChunk {
    /// * The number of samples through which playback should be silent
    data: u32,
}

impl SlntChunk {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
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
    pub fn read(
        reader: &mut impl Reader,
        text_encoding: &StringCodecMaps,
    ) -> Result<Self, AudioReadError> {
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

    pub fn write(
        &self,
        writer: &mut dyn Writer,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), AudioWriteError> {
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

impl Debug for BextChunk {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("BextChunk")
            .field("description", &self.description)
            .field("originator", &self.originator)
            .field("originator_ref", &self.originator_ref)
            .field("origination_date", &self.origination_date)
            .field("origination_time", &self.origination_time)
            .field("time_ref", &self.time_ref)
            .field("version", &self.version)
            .field(
                "umid",
                &format_args!(
                    "[{}]",
                    self.umid
                        .iter()
                        .map(|byte| { format!("0x{:02x}", byte) })
                        .collect::<Vec<String>>()
                        .join(",")
                ),
            )
            .field("reserved", &format_args!("[u8; {}]", self.reserved.len()))
            .field("coding_history", &self.coding_history)
            .finish()
    }
}

impl Default for BextChunk {
    fn default() -> Self {
        Self {
            description: String::new(),
            originator: String::new(),
            originator_ref: String::new(),
            origination_date: String::new(),
            origination_time: String::new(),
            time_ref: 0,
            version: 0,
            umid: [0u8; 64],
            reserved: [0u8; 190],
            coding_history: [0u8; 1],
        }
    }
}

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Copy, Default)]
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
        let mut ret = Self {
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
        Ok(Self {
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

#[derive(Debug, Clone, Copy, Default)]
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
        Ok(Self {
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

/// * See <https://www.recordingblogs.com/wiki/playlist-chunk-of-a-wave-file>
#[derive(Debug, Clone, Default)]
pub struct PlstChunk {
    pub playlist_len: u32,
    pub data: Vec<Plst>,
}

#[derive(Debug, Clone, Copy, Default)]
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
            data: (0..playlist_len)
                .map(|_| -> Result<Plst, AudioReadError> { Plst::read(reader) })
                .collect::<Result<Vec<Plst>, AudioReadError>>()?,
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
        self.data
            .iter()
            .map(|plst| (plst.cue_point_id, *plst))
            .collect()
    }
}

impl Plst {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
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

/// See <https://www.recordingblogs.com/wiki/cue-chunk-of-a-wave-file>
/// See <https://wavref.til.cafe/chunk/cue/>
#[derive(Debug, Clone, Default)]
pub struct CueChunk {
    pub num_cues: u32,
    pub cue_points: Vec<CuePoint>,
}

#[derive(Debug, Clone, Copy, Default)]
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
            cue_points: (0..num_cues)
                .map(|_| -> Result<CuePoint, AudioReadError> { CuePoint::read(reader) })
                .collect::<Result<Vec<CuePoint>, AudioReadError>>()?,
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
        self.cue_points
            .iter()
            .map(|cue| (cue.cue_point_id, cue))
            .collect()
    }
}

impl CuePoint {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        let mut data_chunk_id = [0u8; 4];
        reader.read_exact(&mut data_chunk_id)?;
        Ok(Self {
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

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum ListChunk {
    Info(BTreeMap<String, String>),
    Adtl(BTreeMap<u32, AdtlChunk>),
}

impl Default for ListChunk {
    fn default() -> Self {
        Self::Info(BTreeMap::<String, String>::new())
    }
}

/// See <https://wavref.til.cafe/chunk/adtl/>
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum AdtlChunk {
    Labl(LablChunk),
    Note(NoteChunk),
    Ltxt(LtxtChunk),
    File(FileChunk),
}

impl Default for AdtlChunk {
    fn default() -> Self {
        Self::Labl(LablChunk::default())
    }
}

#[derive(Debug, Clone, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct LablChunk {
    pub cue_point_id: u32,
    pub data: String,
}

#[derive(Debug, Clone, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct NoteChunk {
    pub cue_point_id: u32,
    pub data: String,
}

#[derive(Debug, Clone, Default, PartialOrd, Ord, PartialEq, Eq)]
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

#[derive(Clone, Default, PartialOrd, Ord, PartialEq, Eq)]
pub struct FileChunk {
    pub cue_point_id: u32,
    pub media_type: u32,
    pub file_data: Vec<u8>,
}

impl Debug for FileChunk {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FileChunk")
            .field("cue_point_id", &self.cue_point_id)
            .field("media_type", &self.media_type)
            .field("file_data", &format_args!("[u8; {}]", self.file_data.len()))
            .finish()
    }
}

impl AdtlChunk {
    pub fn read(
        reader: &mut impl Reader,
        text_encoding: &StringCodecMaps,
    ) -> Result<Self, AudioReadError> {
        let sub_chunk = ChunkHeader::read(reader)?;
        let ret = match &sub_chunk.flag {
            b"labl" => Self::Labl(LablChunk {
                cue_point_id: u32::read_le(reader)?,
                data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
            }),
            b"note" => Self::Note(NoteChunk {
                cue_point_id: u32::read_le(reader)?,
                data: read_str(reader, (sub_chunk.size - 4) as usize, text_encoding)?,
            }),
            b"ltxt" => {
                let mut ltxt = LtxtChunk {
                    cue_point_id: u32::read_le(reader)?,
                    sample_length: u32::read_le(reader)?,
                    purpose_id: read_str(reader, 4, text_encoding)?,
                    country: u16::read_le(reader)?,
                    language: u16::read_le(reader)?,
                    dialect: u16::read_le(reader)?,
                    code_page: u16::read_le(reader)?,
                    data: String::new(),
                };
                ltxt.data = read_str_by_code_page(
                    reader,
                    (sub_chunk.size - 20) as usize,
                    text_encoding,
                    ltxt.code_page as u32,
                )?;
                Self::Ltxt(ltxt)
            }
            b"file" => Self::File(FileChunk {
                cue_point_id: u32::read_le(reader)?,
                media_type: u32::read_le(reader)?,
                file_data: read_bytes(reader, (sub_chunk.size - 8) as usize)?,
            }),
            other => {
                return Err(AudioReadError::UnexpectedFlag(
                    "labl/note/ltxt".to_owned(),
                    String::from_utf8_lossy(other).to_string(),
                ));
            }
        };
        sub_chunk.seek_to_next_chunk(reader)?;
        Ok(ret)
    }

    pub fn write(
        &self,
        writer: &mut dyn Writer,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), AudioWriteError> {
        fn to_sz(s: &str) -> String {
            if !s.is_empty() {
                let mut s = s.to_owned();
                if !s.ends_with('\0') {
                    s.push('\0');
                }
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
            }
            Self::Note(note) => {
                let cw = ChunkWriter::begin(writer, b"note")?;
                note.cue_point_id.write_le(cw.writer)?;
                write_str(cw.writer, &to_sz(&note.data), text_encoding)?;
            }
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
            }
            Self::File(file) => {
                let cw = ChunkWriter::begin(writer, b"file")?;
                file.cue_point_id.write_le(cw.writer)?;
                file.media_type.write_le(cw.writer)?;
                cw.writer.write_all(&file.file_data)?;
            }
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
    pub fn read(
        reader: &mut impl Reader,
        chunk_size: u64,
        text_encoding: &StringCodecMaps,
    ) -> Result<Self, AudioReadError> {
        let end_of_chunk = ChunkHeader::align(reader.stream_position()? + chunk_size);
        let mut flag = [0u8; 4];
        reader.read_exact(&mut flag)?;
        match &flag {
            b"info" | b"INFO" => {
                let dict = Self::read_dict(reader, end_of_chunk, text_encoding)?;
                Ok(Self::Info(dict))
            }
            b"adtl" => {
                let mut adtl_map = BTreeMap::<u32, AdtlChunk>::new();
                while reader.stream_position()? < end_of_chunk {
                    let adtl = AdtlChunk::read(reader, text_encoding)?;
                    let cue_point_id = adtl.get_cue_point_id();
                    if let Some(dup) = adtl_map.insert(cue_point_id, adtl.clone()) {
                        // If the chunk point ID duplicates,  the new one will be used to overwrite the old one.
                        eprintln!(
                            "Duplicated chunk point ID {cue_point_id} for the `Adtl` data: old is {:?}, and will be overwritten by the new one: {:?}",
                            dup, adtl
                        );
                    }
                }
                Ok(Self::Adtl(adtl_map))
            }
            other => Err(AudioReadError::Unimplemented(format!(
                "Unknown indentifier in LIST chunk: {}",
                text_encoding.decode_flags(other)
            ))),
        }
    }

    pub fn write(
        &self,
        writer: &mut dyn Writer,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), AudioWriteError> {
        let mut cw = ChunkWriter::begin(writer, b"LIST")?;
        match self {
            Self::Info(dict) => {
                cw.writer.write_all(b"INFO")?;
                Self::write_dict(&mut cw.writer, dict, text_encoding)?;
            }
            Self::Adtl(adtls) => {
                cw.writer.write_all(b"adtl")?;
                for (_cue_point_id, adtl) in adtls.iter() {
                    adtl.write(&mut cw.writer, text_encoding)?;
                }
            }
        };
        Ok(())
    }

    fn read_dict(
        reader: &mut impl Reader,
        end_of_chunk: u64,
        text_encoding: &StringCodecMaps,
    ) -> Result<BTreeMap<String, String>, AudioReadError> {
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

    fn write_dict(
        writer: &mut dyn Writer,
        dict: &BTreeMap<String, String>,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), AudioWriteError> {
        for (key, val) in dict.iter() {
            if key.len() != 4 {
                return Err(AudioWriteError::InvalidArguments(
                    "flag must be 4 bytes".to_owned(),
                ));
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

/// See <https://www.recordingblogs.com/wiki/list-chunk-of-a-wave-file>
pub fn get_list_info_map() -> BTreeMap<&'static str, &'static str> {
    [
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
            }
        }
    }
}

impl ListInfo for ListChunk {
    fn get_is_list_info(&self) -> bool {
        matches!(self, Self::Info(_))
    }

    fn get(&self, key: &str) -> Option<&String> {
        if let Self::Info(dict) = self {
            dict.get(key)
        } else {
            None
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<Option<String>, AudioError> {
        if let Self::Info(dict) = self {
            Ok(dict.insert(key.to_owned(), value.to_string()))
        } else {
            Err(AudioError::InvalidArguments("The type of the `LIST` chunk is `adtl`, not `INFO`, so can not set the metadata values.".to_owned()))
        }
    }
}

/// See <https://wavref.til.cafe/chunk/cset/>
#[allow(clippy::zero_prefixed_literal)]
pub fn get_country_code_map() -> HashMap<u16, &'static str> {
    [
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
    ]
    .iter()
    .copied()
    .collect()
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct LanguageDialect {
    lang: u16,
    dial: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct LanguageSpecification {
    lang: &'static str,
    spec: &'static str,
}

/// See <https://wavref.til.cafe/chunk/cset/>
pub fn get_language_dialect_code_map() -> HashMap<LanguageDialect, LanguageSpecification> {
    [
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

/// ## The fully assembled cue point data from various of chunks in the WAV file.
#[derive(Debug, Clone, Default)]
pub struct FullInfoCuePoint {
    pub data_chunk_id: [u8; 4],

    /// * The label of the cue point
    pub label: String,

    /// * The notes for the cue point
    pub note: String,

    /// * How many samples in the cue point
    pub sample_length: u32,

    /// * What is the purpose of the cue point
    pub purpose_id: String,

    /// * Country name
    pub country: String,

    /// * Language name
    pub language: String,

    /// * Some text data
    pub text_data: String,

    /// * The media type
    pub media_type: u32,

    /// * The file data
    pub file_data: Vec<u8>,

    /// * Start sample
    pub start_sample: u32,

    /// * Num samples
    pub num_samples: u32,

    /// * repeats for playback
    pub repeats: u32,
}

impl FullInfoCuePoint {
    pub fn new(
        cue_point_id: u32,
        cue_point: &CuePoint,
        adtl_chunks: &BTreeMap<u32, AdtlChunk>,
        plst: &Option<&Plst>,
        country_code_map: &HashMap<u16, &'static str>,
        dialect_code_map: &HashMap<LanguageDialect, LanguageSpecification>,
    ) -> Result<Self, AudioError> {
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
        if let Some(plst) = plst {
            ret.num_samples = plst.num_samples;
            ret.repeats = plst.repeats;
        } else {
            eprintln!(
                "Lack of `plst` chunk, `num_samples` should be calculated by yourself, and `repeats` remains zero."
            );
        }
        if let Some(adtl) = adtl_chunks.get(&cue_point_id) {
            match adtl {
                AdtlChunk::Labl(labl) => ret.label = labl.data.clone(),
                AdtlChunk::Note(note) => ret.note = note.data.clone(),
                AdtlChunk::Ltxt(ltxt) => {
                    let lang_dial = LanguageDialect {
                        lang: ltxt.language,
                        dial: ltxt.dialect,
                    };
                    let unknown_lang_spec = LanguageSpecification {
                        lang: "Unknown",
                        spec: "UKNW1994",
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
                    }
                    .lang
                    .to_owned();
                    ret.sample_length = ltxt.sample_length;
                    ret.purpose_id = ltxt.purpose_id.clone();
                    ret.country = country;
                    ret.language = language;
                    ret.text_data = ltxt.data.clone();
                }
                AdtlChunk::File(file) => {
                    ret.media_type = file.media_type;
                    ret.file_data = file.file_data.clone();
                }
            }
            Ok(ret)
        } else {
            Err(AudioError::NoSuchData(format!(
                "ADTL data for cue point ID: {cue_point_id}"
            )))
        }
    }
}

/// ## Create a fully assembled cue point data from various of chunks in the WAV file.
pub fn create_full_info_cue_data(
    cue_chunk: &CueChunk,
    adtl_chunks: &BTreeMap<u32, AdtlChunk>,
    plstchunk: &Option<PlstChunk>,
) -> Result<BTreeMap<u32, FullInfoCuePoint>, AudioError> {
    let country_code_map = get_country_code_map();
    let dialect_code_map = get_language_dialect_code_map();
    let plstmap = if let Some(plstchunk) = plstchunk {
        plstchunk.build_map()
    } else {
        BTreeMap::<u32, Plst>::new()
    };
    cue_chunk
        .cue_points
        .iter()
        .map(|cue| -> Result<(u32, FullInfoCuePoint), AudioError> {
            match FullInfoCuePoint::new(
                cue.cue_point_id,
                cue,
                adtl_chunks,
                &plstmap.get(&cue.cue_point_id),
                &country_code_map,
                &dialect_code_map,
            ) {
                Ok(full_info_cue_data) => Ok((cue.cue_point_id, full_info_cue_data)),
                Err(e) => Err(e),
            }
        })
        .collect::<Result<BTreeMap<u32, FullInfoCuePoint>, AudioError>>()
}

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Clone, Copy, Default)]
pub struct TrknChunk {
    pub track_no: u16,
    pub total_tracks: u16,
}

impl TrknChunk {
    pub fn read(reader: &mut impl Reader) -> Result<Self, AudioReadError> {
        Ok(Self {
            track_no: u16::read_le(reader)?,
            total_tracks: u16::read_le(reader)?,
        })
    }

    pub fn write(&self, writer: &mut dyn Writer) -> Result<(), AudioWriteError> {
        let cw = ChunkWriter::begin(writer, b"Trkn")?;
        self.track_no.write_le(cw.writer)?;
        self.total_tracks.write_le(cw.writer)?;
        Ok(())
    }
}

#[derive(Clone, PartialOrd, PartialEq, Ord, Eq)]
pub enum JunkChunk {
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
            Self::SomeData(data) => write!(
                f,
                "[{}]",
                data.iter()
                    .map(|byte| { format!("0x{:02}", byte) })
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        }
    }
}

impl Default for JunkChunk {
    fn default() -> Self {
        Self::FullZero(0)
    }
}

#[cfg(feature = "flac")]
pub fn get_listinfo_flacmeta() -> &'static BTreeMap<&'static str, &'static str> {
    use std::sync::OnceLock;
    static LISTINFO_FLACMETA: OnceLock<BTreeMap<&'static str, &'static str>> = OnceLock::new();
    LISTINFO_FLACMETA.get_or_init(|| {
        [
            ("ITRK", "TRACKNUMBER"),
            ("IART", "ARTIST"),
            ("INAM", "TITLE"),
            ("IPRD", "ALBUM"),
            ("ICMT", "COMMENT"),
            ("ICOP", "COPYRIGHT"),
            ("ICRD", "DATE"),
            ("IGNR", "GENRE"),
            ("ISRC", "ISRC"),
            ("ISFT", "ENCODER"),
            ("ISMP", "TIMECODE"),
            ("ILNG", "LANGUAGE"),
            ("ICMS", "PRODUCER"),
        ]
        .iter()
        .copied()
        .collect()
    })
}

#[cfg(not(feature = "flac"))]
pub mod flac {
    /// ## The compression level of the FLAC file
    /// A higher number means less file size. Default compression level is 5
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum FlacCompression {
        /// Almost no compression
        Level0 = 0,
        Level1 = 1,
        Level2 = 2,
        Level3 = 3,
        Level4 = 4,
        Level5 = 5,
        Level6 = 6,
        Level7 = 7,

        /// Maximum compression
        Level8 = 8,
    }

    /// ## Parameters for the encoder to encode the audio.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FlacEncoderParams {
        /// * If set to true, the FLAC encoder will send the encoded data to a decoder to verify if the encoding is successful, and the encoding process will be slower.
        pub verify_decoded: bool,

        /// * The compression level of the FLAC file, a higher number means less file size.
        pub compression: FlacCompression,

        /// * Num channels of the audio file, max channels is 8.
        pub channels: u16,

        /// * The sample rate of the audio file. Every FLAC frame contains this value.
        pub sample_rate: u32,

        /// * How many bits in an `i32` are valid for a sample, for example, if this value is 16, your `i32` sample should be between -32768 to +32767.
        ///   Because the FLAC encoder **only eats `[i32]`** , and you can't just pass `[i16]` to it.
        ///   It seems like 8, 12, 16, 20, 24, 32 are valid values for this field.
        pub bits_per_sample: u32,

        /// * How many samples you will put into the encoder, set to zero if you don't know.
        pub total_samples_estimate: u64,
    }
}

/// ## If the `id3` feature is enabled, use it to read ID3 data.
#[cfg(feature = "id3")]
#[allow(non_snake_case)]
pub mod Id3 {
    use crate::errors::{AudioReadError, AudioWriteError, IOErrorInfo};
    use std::io::{Read, Seek, Write};
    pub type Tag = id3::Tag;

    pub fn id3_read<R>(reader: &mut R, _size: usize) -> Result<Tag, AudioReadError>
    where
        R: Read + Seek + ?Sized,
    {
        Ok(Tag::read_from2(reader)?)
    }

    pub fn id3_write<W>(tag: &Tag, writer: &mut W) -> Result<(), AudioWriteError>
    where
        W: Write + ?Sized,
    {
        Ok(tag.write_to(writer, tag.version())?)
    }

    impl From<id3::Error> for AudioReadError {
        fn from(err: id3::Error) -> Self {
            match err.kind {
                id3::ErrorKind::Io(ioerr) => AudioReadError::IOError(IOErrorInfo {
                    kind: ioerr.kind(),
                    message: ioerr.to_string(),
                }),
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
                id3::ErrorKind::Io(ioerr) => AudioWriteError::IOError(IOErrorInfo {
                    kind: ioerr.kind(),
                    message: ioerr.to_string(),
                }),
                id3::ErrorKind::StringDecoding(bytes) => AudioWriteError::StringDecodeError(bytes),
                id3::ErrorKind::NoTag => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::Parsing => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::InvalidInput => AudioWriteError::OtherReason(err.description),
                id3::ErrorKind::UnsupportedFeature => AudioWriteError::Unsupported(err.description),
            }
        }
    }
}

/// ## If the `id3` feature is disabled, read the raw bytes.
#[cfg(not(feature = "id3"))]
#[allow(non_snake_case)]
pub mod Id3 {
    use std::error::Error;
    use std::io::Read;
    use std::vec::Vec;
    #[derive(Clone)]
    pub struct Tag {
        pub data: Vec<u8>,
    }
    impl Tag {
        fn new(data: Vec<u8>) -> Self {
            Self { data }
        }
    }

    pub fn id3_read<R>(reader: &mut R, size: usize) -> Result<Tag, AudioReadError>
    where
        R: Read + Seek + ?Sized,
    {
        #[cfg(debug_assertions)]
        println!(
            "Feature \"id3\" was not enabled, consider compile with \"cargo build --features id3\""
        );
        Ok(Tag::new(super::read_bytes(reader, size)?))
    }

    pub fn id3_write<W>(tag: &Tag, writer: &mut W) -> Result<(), AudioWriteError>
    where
        W: Write + ?Sized,
    {
        #[cfg(debug_assertions)]
        println!(
            "Feature \"id3\" was not enabled, the saved id3 binary bytes may not correct, consider compile with \"cargo build --features id3\""
        );
        Ok(writer.write_all(&tag.data))
    }

    impl std::fmt::Debug for Tag {
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            fmt.debug_struct("Tag").finish_non_exhaustive()
        }
    }
}
