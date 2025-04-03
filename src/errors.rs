#![allow(dead_code)]

#[derive(Debug, Clone)]
pub struct IOErrorInfo {
    pub kind: std::io::ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum AudioReadError {
    IncompleteFile(u64), // 不完整的文件
    InvalidArguments(String), // 错误的参数
    IOError(IOErrorInfo), // 读写错误，应停止处理
    FormatError(String), // 格式错误，说明可以尝试使用别的格式的读取器来读取
    DataCorrupted(String), // 格式也许是正确的，但是数据是错误的
    Unimplemented(String), // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
    Unsupported(String), // 不支持的写入格式
    UnexpectedFlag(String, String), // 没有预料到的符号
    StringDecodeError(Vec<u8>), // 字符串解码错误
    OtherReason(String), // 不知道的问题
}

impl std::error::Error for AudioReadError {}

impl std::fmt::Display for AudioReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AudioReadError::IncompleteFile(offset) => write!(f, "The file is incomplete, the content from 0x{:x} is empty", offset),
            AudioReadError::InvalidArguments(reason) => write!(f, "Invalid arguments: {}", reason),
            AudioReadError::IOError(ioerror) => write!(f, "IO error: {:?}", ioerror),
            AudioReadError::FormatError(reason) => write!(f, "Invalid format: {}", reason),
            AudioReadError::DataCorrupted(reason) => write!(f, "Data corrupted: {}", reason),
            AudioReadError::Unimplemented(reason) => write!(f, "Unimplemented for the file format: {}", reason),
            AudioReadError::Unsupported(feature) => write!(f, "Unsupported feature: {}", feature),
            AudioReadError::UnexpectedFlag(expected, got) => write!(f, "Expect \"{}\", got \"{}\".", expected, got),
            AudioReadError::StringDecodeError(bytes) => write!(f, "String decode error: {}", String::from_utf8_lossy(bytes)),
            AudioReadError::OtherReason(reason) => write!(f, "Unknown error: {}", reason),
        }
    }
}

impl From<std::io::Error> for AudioReadError {
    fn from(ioerr: std::io::Error) -> Self {
        AudioReadError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()})
    }
}

impl From<AudioReadError> for std::io::Error {
    fn from(err: AudioReadError) -> Self {
        match err {
            AudioReadError::IOError(ioerr) => {
                std::io::Error::from(ioerr.kind)
            },
            other => panic!("When converting `AudioReadError` to `std::io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioWriteError {
    InvalidArguments(String), // 输入了错误的参数
    IOError(IOErrorInfo), // 读写错误，应停止处理
    Unsupported(String), // 不支持的写入格式
    Unimplemented(String), // 没实现的写入格式
    AlreadyFinished(String), // 早就停止写入了
    NotPreparedFor4GBFile, // 之前没准备好要写入超过 4GB 的 WAV 文件
    ChunkSizeTooBig(String), // 块大小太大
    StringDecodeError(Vec<u8>), // 字符串解码错误
    OtherReason(String), // 不知道的问题
}

impl std::error::Error for AudioWriteError {}

impl std::fmt::Display for AudioWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AudioWriteError::InvalidArguments(reason) => write!(f, "Invalid arguments: {}", reason),
            AudioWriteError::IOError(errkind) => write!(f, "IO error: {:?}", errkind),
            AudioWriteError::Unsupported(reason) => write!(f, "Unsupported format: {}", reason),
            AudioWriteError::Unimplemented(reason) => write!(f, "Unimplemented format: {}", reason),
            AudioWriteError::AlreadyFinished(reason) => write!(f, "Already finished writing {}", reason),
            AudioWriteError::NotPreparedFor4GBFile => write!(f, "The WAV file wasn't prepared for being larger than 4GB, please check `file_size_option` when creating the `WaveWriter`."),
            AudioWriteError::ChunkSizeTooBig(reason) => write!(f, "Chunk size is too big: {}", reason),
            AudioWriteError::StringDecodeError(bytes) => write!(f, "String decode error: {}", String::from_utf8_lossy(bytes)),
            AudioWriteError::OtherReason(reason) => write!(f, "Unknown error: {}", reason),
       }
    }
}

impl From<std::io::Error> for AudioWriteError {
    fn from(ioerr: std::io::Error) -> Self {
        AudioWriteError::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()})
    }
}

impl From<AudioWriteError> for std::io::Error {
    fn from(err: AudioWriteError) -> Self {
        match err {
            AudioWriteError::IOError(ioerr) => {
                std::io::Error::from(ioerr.kind)
            },
            other => panic!("When converting `AudioWriteError` to `std::io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioError {
    CantGuessChannelMask(u16), // 无法猜出声道掩码
    ChannelNotMatchMask, // 声道数不和声道掩码匹配
    Unimplemented(String), // 没有实现的解析格式
    InvalidArguments(String), // 不知道的样本的格式
}

impl std::error::Error for AudioError {}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           Self::CantGuessChannelMask(channels) => write!(f, "Can't guess channel mask for channels = {}", channels),
           Self::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
           Self::Unimplemented(reason) => write!(f, "Unimplemented behavior: {}", reason),
           Self::InvalidArguments(reason) => write!(f, "Invalid arguments: {}", reason),
       }
    }
}

impl From<AudioError> for AudioReadError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::CantGuessChannelMask(channels) => Self::InvalidArguments(format!("can't guess channel mask by channel number {}", channels)),
            AudioError::ChannelNotMatchMask => Self::DataCorrupted("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(reason) => Self::Unimplemented(reason),
            AudioError::InvalidArguments(reason) => Self::InvalidArguments(reason),
        }
    }
}

impl From<AudioError> for AudioWriteError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::CantGuessChannelMask(channels) => Self::InvalidArguments(format!("can't guess channel mask by channel number {}", channels)),
            AudioError::ChannelNotMatchMask => Self::InvalidArguments("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(reason) => Self::Unimplemented(reason),
            AudioError::InvalidArguments(reason) => Self::InvalidArguments(reason),
        }
    }
}

#[cfg(feature = "mp3dec")]
impl From<puremp3::Error> for AudioReadError {
    fn from(err: puremp3::Error) -> Self {
        match err {
            puremp3::Error::Mp3Error(mp3err) => {
                match mp3err {
                    puremp3::Mp3Error::InvalidData(s) => Self::FormatError(s.to_owned()),
                    puremp3::Mp3Error::Unsupported(s) => Self::Unsupported(s.to_owned()),
                }
            },
            puremp3::Error::IoError(ioerr) => Self::IOError(IOErrorInfo{kind: ioerr.kind(), message: ioerr.to_string()}),
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
            mp3lame_encoder::BuildError::Other(c_int) => Self::OtherReason(format!("Other lame error code: {}", c_int)),
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



