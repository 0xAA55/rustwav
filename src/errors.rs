#![allow(dead_code)]

use std::{io::{self, ErrorKind}, fmt::{Formatter, Display}, error};

#[derive(Debug, Clone)]
pub struct IOErrorInfo {
    pub kind: ErrorKind,
    pub message: String,
}

impl IOErrorInfo {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self {
            kind,
            message,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioReadError {
    IncompleteFile(u64),
    IncompleteData(String),
    BufferTooSmall(String),
    InvalidArguments(String),
    IOError(IOErrorInfo),
    FormatError(String),
    DataCorrupted(String),
    Unimplemented(String),
    Unsupported(String),
    UnexpectedFlag(String, String),
    StringDecodeError(Vec<u8>),
    OtherReason(String),
}

impl error::Error for AudioReadError {}

impl Display for AudioReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IncompleteFile(offset) => write!(f, "The file is incomplete, the content from 0x{:x} is empty", offset),
            Self::IncompleteData(info) => write!(f, "Incomplete data: {info}"),
            Self::BufferTooSmall(info) => write!(f, "The buffer is too small: {info}"),
            Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
            Self::IOError(ioerror) => write!(f, "IO error: {:?}", ioerror),
            Self::FormatError(info) => write!(f, "Invalid format: {info}"),
            Self::DataCorrupted(info) => write!(f, "Data corrupted: {info}"),
            Self::Unimplemented(info) => write!(f, "Unimplemented for the file format: {info}"),
            Self::Unsupported(feature) => write!(f, "Unsupported feature: {feature}"),
            Self::UnexpectedFlag(expected, got) => write!(f, "Expect \"{expected}\", got \"{got}\"."),
            Self::StringDecodeError(bytes) => write!(f, "String decode error: {}", String::from_utf8_lossy(bytes)),
            Self::OtherReason(info) => write!(f, "Unknown error: {info}"),
        }
    }
}

impl From<io::Error> for AudioReadError {
    fn from(ioerr: io::Error) -> Self {
        AudioReadError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()})
    }
}

impl From<crate::adpcm::ima::ImaAdpcmError> for AudioReadError {
    fn from(imaerr: crate::adpcm::ima::ImaAdpcmError) -> Self {
        match imaerr{
            crate::adpcm::ima::ImaAdpcmError::InvalidArgument(info) => Self::InvalidArguments(info),
        }
    }
}

impl From<AudioReadError> for io::Error {
    fn from(err: AudioReadError) -> Self {
        match err {
            AudioReadError::IOError(ioerr) => {
                io::Error::from(ioerr.kind)
            },
            other => panic!("When converting `AudioReadError` to `io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioWriteError {
    InvalidArguments(String),
    IOError(IOErrorInfo),
    Unsupported(String),
    Unimplemented(String),
    AlreadyFinished(String),
    NotPreparedFor4GBFile,
    ChunkSizeTooBig(String),
    StringDecodeError(Vec<u8>),
    BufferIsFull(String),
    MultipleMonosAreNotSameSize,
    FrameChannelsNotSame,
    WrongChannels(String),
    NotStereo,
    OtherReason(String),
}

impl error::Error for AudioWriteError {}

impl Display for AudioWriteError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
            Self::IOError(errkind) => write!(f, "IO error: {:?}", errkind),
            Self::Unsupported(info) => write!(f, "Unsupported format: {info}"),
            Self::Unimplemented(info) => write!(f, "Unimplemented format: {info}"),
            Self::AlreadyFinished(info) => write!(f, "Already finished writing {info}"),
            Self::NotPreparedFor4GBFile => write!(f, "The WAV file wasn't prepared for being larger than 4GB, please check `file_size_option` when creating the `WaveWriter`."),
            Self::ChunkSizeTooBig(info) => write!(f, "Chunk size is too big: {info}"),
            Self::StringDecodeError(bytes) => write!(f, "String decode error: {}", String::from_utf8_lossy(bytes)),
            Self::BufferIsFull(info) => write!(f, "The buffer is full: {info}"),
            Self::MultipleMonosAreNotSameSize => write!(f, "The lengths of the channels are not equal."),
            Self::FrameChannelsNotSame => write!(f, "The channels of each frames are not equal."),
            Self::WrongChannels(prompt) => write!(f, "Wrong channels: {prompt}"),
            Self::NotStereo => write!(f, "The samples are not stereo audio samples"),
            Self::OtherReason(info) => write!(f, "Unknown error: {info}"),
       }
    }
}

impl From<io::Error> for AudioWriteError {
    fn from(ioerr: io::Error) -> Self {
        AudioWriteError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()})
    }
}

impl From<AudioWriteError> for io::Error {
    fn from(err: AudioWriteError) -> Self {
        match err {
            AudioWriteError::IOError(ioerr) => {
                io::Error::from(ioerr.kind)
            },
            other => panic!("When converting `AudioWriteError` to `io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioError {
    GuessChannelMaskFailed(u16),
    ChannelNotMatchMask,
    Unimplemented(String),
    InvalidArguments(String),
}

impl error::Error for AudioError {}

impl Display for AudioError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
       match self {
           Self::GuessChannelMaskFailed(channels) => write!(f, "Can't guess channel mask for channels = {channels}"),
           Self::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
           Self::Unimplemented(info) => write!(f, "Unimplemented behavior: {info}"),
           Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
       }
    }
}

impl From<AudioError> for AudioReadError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::GuessChannelMaskFailed(channels) => Self::InvalidArguments(format!("can't guess channel mask by channel number {channels}")),
            AudioError::ChannelNotMatchMask => Self::DataCorrupted("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(info) => Self::Unimplemented(info),
            AudioError::InvalidArguments(info) => Self::InvalidArguments(info),
        }
    }
}

impl From<AudioError> for AudioWriteError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::GuessChannelMaskFailed(channels) => Self::InvalidArguments(format!("can't guess channel mask by channel number {channels}")),
            AudioError::ChannelNotMatchMask => Self::InvalidArguments("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(info) => Self::Unimplemented(info),
            AudioError::InvalidArguments(info) => Self::InvalidArguments(info),
        }
    }
}

#[cfg(feature = "mp3enc")]
impl From<mp3lame_encoder::BuildError> for AudioWriteError {
    fn from(err: mp3lame_encoder::BuildError) -> Self {
        match err {
            mp3lame_encoder::BuildError::Generic => Self::InvalidArguments("Generic error".to_owned()),
            mp3lame_encoder::BuildError::NoMem => Self::OtherReason("No enough memory".to_owned()),
            mp3lame_encoder::BuildError::BadBRate => Self::InvalidArguments("Bad bit rate".to_owned()),
            mp3lame_encoder::BuildError::BadSampleFreq => Self::InvalidArguments("Bad sample rate".to_owned()),
            mp3lame_encoder::BuildError::InternalError => Self::OtherReason("Internal error".to_owned()),
            mp3lame_encoder::BuildError::Other(c_int) => Self::OtherReason(format!("Other lame error code: {c_int}")),
        }
    }
}

#[cfg(feature = "mp3enc")]
impl From<mp3lame_encoder::Id3TagError> for AudioWriteError {
    fn from(err: mp3lame_encoder::Id3TagError) -> Self {
        match err {
            mp3lame_encoder::Id3TagError::AlbumArtOverflow => Self::InvalidArguments("Specified Id3 tag buffer exceed limit of 128kb".to_owned()),
        }
    }
}

#[cfg(feature = "mp3enc")]
impl From<mp3lame_encoder::EncodeError> for AudioWriteError {
    fn from(err: mp3lame_encoder::EncodeError) -> Self {
        match err {
            mp3lame_encoder::EncodeError::BufferTooSmall => Self::InvalidArguments("Buffer is too small".to_owned()),
            mp3lame_encoder::EncodeError::NoMem => Self::OtherReason("No enough memory".to_owned()),
            mp3lame_encoder::EncodeError::InvalidState => Self::InvalidArguments("Invalid state".to_owned()),
            mp3lame_encoder::EncodeError::PsychoAcoustic => Self::InvalidArguments("Psycho acoustic problems".to_owned()),
            mp3lame_encoder::EncodeError::Other(c_int) => Self::OtherReason(format!("Other lame error code: {c_int}")),
        }
    }
}

#[cfg(feature = "opus")]
impl From<opus::Error> for AudioReadError {
    fn from(err: opus::Error) -> Self {
        match err.code() {
            opus::ErrorCode::BadArg => Self::InvalidArguments(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::BufferTooSmall => Self::BufferTooSmall(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InternalError => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InvalidPacket => Self::DataCorrupted(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::Unimplemented => Self::Unimplemented(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InvalidState => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::AllocFail => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::Unknown => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
        }
    }
}

#[cfg(feature = "opus")]
impl From<opus::Error> for AudioWriteError {
    fn from(err: opus::Error) -> Self {
        match err.code() {
            opus::ErrorCode::BadArg => Self::InvalidArguments(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::BufferTooSmall => Self::BufferIsFull(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InternalError => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InvalidPacket => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::Unimplemented => Self::Unimplemented(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::InvalidState => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::AllocFail => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
            opus::ErrorCode::Unknown => Self::OtherReason(format!("On calling `{}`: {}", err.function(), err.description())),
        }
    }
}
