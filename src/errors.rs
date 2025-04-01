#![allow(dead_code)]

#[derive(Debug, Clone)]
pub enum AudioReadError {
    InvalidArguments(String), // 错误的参数
    IOError(String), // 读写错误，应停止处理
    FormatError(String), // 格式错误，说明可以尝试使用别的格式的读取器来读取
    DataCorrupted(String), // 格式也许是正确的，但是数据是错误的
    Unimplemented(String), // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
    EndOfFile(String), // 超出文件结尾
    IncompleteFile, // 文件内容不完整
    UnexpectedFlag(String, String), // 没有预料到的符号
}

impl std::error::Error for AudioReadError {}

impl std::fmt::Display for AudioReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioReadError::InvalidArguments(reason) => write!(f, "Invalid arguments: {}", reason),
           AudioReadError::IOError(reason) => write!(f, "IOError: {}", reason),
           AudioReadError::FormatError(reason) => write!(f, "Invalid audio file format: {}", reason),
           AudioReadError::DataCorrupted(reason) => write!(f, "Audio file data corrupted: {}", reason),
           AudioReadError::Unimplemented(reason) => write!(f, "Unimplemented for the file format: {}", reason),
           AudioReadError::EndOfFile(reason) => write!(f, "Read to the end of the file: {}", reason),
           AudioReadError::IncompleteFile => write!(f, "The wave file is not complete."),
           AudioReadError::UnexpectedFlag(expected, got) => write!(f, "Expect \"{}\", got \"{}\".", expected, got),
       }
}

impl From<std::io::Error> for AudioReadError {
    fn from(err: std::io::Error) -> Self {
        AudioReadError::IOError(err.kind())
    }
}

impl From<AudioReadError> for std::io::Error {
    fn from(err: AudioReadError) -> Self {
        match err {
            AudioReadError::IOError(errkind) => std::io::Error::from(errkind),
            other => panic!("When converting `AudioReadError` to `std::io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioWriteError {
    InvalidArguments(String), // 输入了错误的参数
    IOError(String), // 读写错误，应停止处理
    UnsupportedFormat(String), // 不支持的写入格式
    ChannelCountNotMatch(String), // 声道数不匹配
    WrongSampleFormat(String), // 不支持的样本类型
    AlreadyFinished(String), // 早就停止写入了
    NotPreparedFor4GBFile, // 之前没准备好要写入超过 4GB 的 WAV 文件
    ChunkSizeTooBig(String), // 块大小太大
}

impl std::error::Error for AudioWriteError {}

impl std::fmt::Display for AudioWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AudioWriteError::InvalidArguments(reason) => write!(f, "Invalid arguments: {}", reason),
            AudioWriteError::IOError(reason) => write!(f, "IOError {}", reason),
            AudioWriteError::UnsupportedFormat(reason) => write!(f, "Unsupported PCM format to be saved: {}", reason),
            AudioWriteError::ChannelCountNotMatch(reason) => write!(f, "Channel count not match: {}", reason),
            AudioWriteError::WrongSampleFormat(reason) => write!(f, "Sample format \"{}\" not supported", reason),
            AudioWriteError::AlreadyFinished(reason) => write!(f, "Already finished writing {}", reason),
            AudioWriteError::NotPreparedFor4GBFile => write!(f, "The WAV file wasn't prepared for being larger than 4GB, please check `file_size_option` when creating the `WaveWriter`."),
            AudioWriteError::ChunkSizeTooBig(reason) => write!(f, "Chunk size is too big: {}", reason),
       }
    }
}

impl From<std::io::Error> for AudioWriteError {
    fn from(err: std::io::Error) -> Self {
        AudioWriteError::IOError(err.kind())
    }
}

impl From<AudioWriteError> for std::io::Error {
    fn from(err: AudioWriteError) -> Self {
        match err {
            AudioWriteError::IOError(errkind) => std::io::Error::from(errkind),
            other => panic!("When converting `AudioWriteError` to `std::io::Error`, the given error is unrelated: {:?}", other),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioError {
    CantGuessChannelMask(u16), // 无法猜出声道掩码
    ChannelNotMatchMask, // 声道数不和声道掩码匹配
    Unimplemented(String), // 没有实现的解析格式
    UnknownSampleType, // 不知道的样本的格式
}

impl std::error::Error for AudioError {}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioError::CantGuessChannelMask(channels) => write!(f, "Can't guess channel mask for channels = {}", channels),
           AudioError::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
           AudioError::Unimplemented(reason) => write!(f, "Unimplemented behavior: {}", reason),
           AudioError::UnknownSampleType => write!(f, "Unknown sample type we got from the spec"),
       }
    }
}

impl From<AudioError> for AudioReadError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::CantGuessChannelMask(channels) => AudioReadError::InvalidArguments(format!("can't guess channel mask by channel number {}", channels)),
            AudioError::ChannelNotMatchMask => AudioReadError::DataCorrupted("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(reason) => AudioReadError::Unimplemented(reason),
            AudioError::InvalidArguments(reason) => AudioReadError::InvalidArguments(reason),
        }
    }
}

impl From<AudioError> for AudioWriteError {
    fn from(err: AudioError) -> Self {
        match err {
            AudioError::CantGuessChannelMask(channels) => AudioWriteError::InvalidArguments(format!("can't guess channel mask by channel number {}", channels)),
            AudioError::ChannelNotMatchMask => AudioWriteError::InvalidArguments("the channel number does not match the channel mask".to_owned()),
            AudioError::Unimplemented(reason) => AudioWriteError::Unimplemented(reason),
            AudioError::InvalidArguments(reason) => AudioWriteError::InvalidArguments(reason),
        }
    }
}

#[cfg(feature = "mp3")]
impl From<puremp3::Error> for AudioReadError {
    fn from(err: puremp3::Error) -> Self {
        match err {
            puremp3::Error::Mp3Error(mp3err) => {
                match mp3err {
                    puremp3::Mp3Error::InvalidData(s) => AudioReadError::FormatError(s.to_owned()),
                    puremp3::Mp3Error::Unsupported(s) => AudioReadError::Unsupported(s.to_owned()),
                }
            },
            puremp3::Error::IoError(ioerr) => AudioReadError::IOError(ioerr.kind()),
        }
    }
}

