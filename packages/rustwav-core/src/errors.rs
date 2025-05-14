#![allow(dead_code)]

use std::{
    error,
    fmt::{Display, Formatter},
    io::{self, ErrorKind},
};

/// * The error info from `std::io::Error` but this must contains the message
#[derive(Debug, Clone)]
pub struct IOErrorInfo {
    pub kind: ErrorKind,
    pub message: String,
}

impl IOErrorInfo {
    pub fn new(kind: ErrorKind, message: String) -> Self {
        Self { kind, message }
    }
}

/// The error info for reading an audio file
#[derive(Debug, Clone)]
pub enum AudioReadError {
    IncompleteFile(u64),
    IncompleteData(String),
    BufferTooSmall(String),
    InvalidArguments(String),
    IOError(IOErrorInfo),
    MissingData(String),
    FormatError(String),
    InvalidData(String),
    Unimplemented(String),
    Unsupported(String),
    UnexpectedFlag(String, String),
    StringDecodeError(Vec<u8>),
    OtherReason(String),
}

impl AudioReadError {
    #[allow(non_snake_case)]
    pub fn UnexpectedEof(message: String) -> Self {
        Self::IOError(IOErrorInfo::new(ErrorKind::UnexpectedEof, message))
    }
}

impl error::Error for AudioReadError {}

impl Display for AudioReadError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::IncompleteFile(offset) => write!(
                f,
                "The file is incomplete, the content from 0x{:x} is empty",
                offset
            ),
            Self::IncompleteData(info) => write!(f, "Incomplete data: {info}"),
            Self::BufferTooSmall(info) => write!(f, "The buffer is too small: {info}"),
            Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
            Self::IOError(ioerror) => write!(f, "IO error: {:?}", ioerror),
            Self::MissingData(data) => write!(f, "Missing data: \"{data}\""),
            Self::FormatError(info) => write!(f, "Invalid format: {info}"),
            Self::InvalidData(info) => write!(f, "Data corrupted: {info}"),
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
        AudioReadError::IOError(IOErrorInfo {
            kind: ioerr.kind(),
            message: ioerr.to_string(),
        })
    }
}

impl From<AudioReadError> for io::Error {
    fn from(err: AudioReadError) -> Self {
        match err {
            AudioReadError::IOError(ioerr) => io::Error::from(ioerr.kind),
            other => panic!(
                "When converting `AudioReadError` to `io::Error`, the given error is unrelated: {:?}",
                other
            ),
        }
    }
}

/// The error info for writing an audio file
#[derive(Debug, Clone)]
pub enum AudioWriteError {
    InvalidArguments(String),
    InvalidInput(String),
    InvalidData(String),
    IOError(IOErrorInfo),
    Unsupported(String),
    Unimplemented(String),
    AlreadyFinished(String),
    NotPreparedFor4GBFile,
    ChunkSizeTooBig(String),
    StringDecodeError(Vec<u8>),
    BufferIsFull(String),
    ChannelsNotInSameSize,
    FrameChannelsNotSame,
    WrongChannels(String),
    TruncatedSamples,
    MissingData(String),
    OtherReason(String),
}

impl error::Error for AudioWriteError {}

impl Display for AudioWriteError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
            Self::InvalidInput(info) => write!(f, "Invalid input: {info}"),
            Self::InvalidData(info) => write!(f, "Invalid data: {info}"),
            Self::IOError(errkind) => write!(f, "IO error: {:?}", errkind),
            Self::Unsupported(info) => write!(f, "Unsupported format: {info}"),
            Self::Unimplemented(info) => write!(f, "Unimplemented format: {info}"),
            Self::AlreadyFinished(info) => write!(f, "Already finished writing {info}"),
            Self::NotPreparedFor4GBFile => write!(f, "The WAV file wasn't prepared for being larger than 4GB, please check `file_size_option` when creating the `WaveWriter`."),
            Self::ChunkSizeTooBig(info) => write!(f, "Chunk size is too big: {info}"),
            Self::StringDecodeError(bytes) => write!(f, "String decode error: {}", String::from_utf8_lossy(bytes)),
            Self::BufferIsFull(info) => write!(f, "The buffer is full: {info}"),
            Self::ChannelsNotInSameSize => write!(f, "The lengths of the channels are not equal."),
            Self::FrameChannelsNotSame => write!(f, "The channels of each frames are not equal."),
            Self::WrongChannels(prompt) => write!(f, "Wrong channels: {prompt}"),
            Self::TruncatedSamples => write!(f, "The samples seem truncated because they can not form an audio frame"),
            Self::MissingData(data) => write!(f, "Missing data: \"{data}\""),
            Self::OtherReason(info) => write!(f, "Unknown error: {info}"),
        }
    }
}

impl From<io::Error> for AudioWriteError {
    fn from(ioerr: io::Error) -> Self {
        AudioWriteError::IOError(IOErrorInfo {
            kind: ioerr.kind(),
            message: ioerr.to_string(),
        })
    }
}

impl From<AudioWriteError> for io::Error {
    fn from(err: AudioWriteError) -> Self {
        match err {
            AudioWriteError::IOError(ioerr) => io::Error::from(ioerr.kind),
            other => panic!(
                "When converting `AudioWriteError` to `io::Error`, the given error is unrelated: {:?}",
                other
            ),
        }
    }
}

/// The error info for processing an audio file
#[derive(Debug, Clone)]
pub enum AudioError {
    GuessChannelMaskFailed(u16),
    ChannelNotMatchMask,
    ChannekMaskNotMatch(String),
    Unparseable(String),
    InvalidData(String),
    NoSuchData(String),
    Unimplemented(String),
    InvalidArguments(String),
    WrongExtensionData(String),
}

impl error::Error for AudioError {}

impl Display for AudioError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
       match self {
           Self::GuessChannelMaskFailed(channels) => write!(f, "Can't guess channel mask for channels = {channels}"),
           Self::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
           Self::ChannekMaskNotMatch(info) => write!(f, "The channel mask does not match: {info}"),
           Self::Unparseable(info) => write!(f, "Could not parse {info}"),
           Self::InvalidData(info) => write!(f, "Invalid data {info}"),
           Self::NoSuchData(info) => write!(f, "Could not find data \"{info}\""),
           Self::Unimplemented(info) => write!(f, "Unimplemented behavior: {info}"),
           Self::InvalidArguments(info) => write!(f, "Invalid arguments: {info}"),
           Self::WrongExtensionData(info) => write!(f, "Wrong extension data: {info}"),
       }
    }
}

impl From<AudioError> for AudioReadError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::GuessChannelMaskFailed(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::ChannelNotMatchMask => Self::InvalidData(format!("{:?}", err)),
            AudioError::ChannekMaskNotMatch(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::Unparseable(_) => Self::InvalidData(format!("{:?}", err)),
            AudioError::InvalidData(_) => Self::InvalidData(format!("{:?}", err)),
            AudioError::NoSuchData(_) => Self::MissingData(format!("{:?}", err)),
            AudioError::Unimplemented(_) => Self::Unimplemented(format!("{:?}", err)),
            AudioError::InvalidArguments(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::WrongExtensionData(_) => Self::InvalidData(format!("{:?}", err)),
        }
    }
}

impl From<AudioError> for AudioWriteError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::GuessChannelMaskFailed(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::ChannelNotMatchMask => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::ChannekMaskNotMatch(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::Unparseable(_) => Self::InvalidInput(format!("{:?}", err)),
            AudioError::InvalidData(_) => Self::InvalidData(format!("{:?}", err)),
            AudioError::NoSuchData(_) => Self::MissingData(format!("{:?}", err)),
            AudioError::Unimplemented(_) => Self::Unimplemented(format!("{:?}", err)),
            AudioError::InvalidArguments(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioError::WrongExtensionData(_) => Self::InvalidData(format!("{:?}", err)),
        }
    }
}

use audioutils::AudioConvError;
impl From<AudioConvError> for AudioWriteError {
    fn from(err: AudioConvError) -> Self {
        match err {
            AudioConvError::InvalidArguments(_) => Self::InvalidArguments(format!("{:?}", err)),
            AudioConvError::FrameChannelsNotSame => Self::FrameChannelsNotSame,
            AudioConvError::ChannelsNotInSameSize => Self::ChannelsNotInSameSize,
            AudioConvError::TruncatedSamples => Self::TruncatedSamples,
        }
    }
}
