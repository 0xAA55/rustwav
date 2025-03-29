#![allow(dead_code)]

#[derive(Debug)]
pub enum AudioReadError {
    InvalidArguments(String), // 错误的参数
    IOError(String), // 读写错误，应停止处理
    FormatError(String), // 格式错误，说明可以尝试使用别的格式的读取器来读取
    DataCorrupted(String), // 格式也许是正确的，但是数据是错误的
    Unimplemented(String), // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
    EndOfFile(String), // 超出文件结尾
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
       }
    }
}

#[derive(Debug)]
pub enum AudioWriteError {
    InvalidArguments(String), // 输入了错误的参数
    IOError(String), // 读写错误，应停止处理
    UnsupportedFormat(String), // 不支持的写入格式
    ChannelCountNotMatch(String), // 声道数不匹配
    WrongSampleFormat(String), // 不支持的样本类型
    AlreadyFinished(String), // 早就停止写入了
    NotPreparedFor4GBFile, // 之前没准备好要写入超过 4GB 的 WAV 文件
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
       }
    }
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum MatchError {
    NotMatch(String),
}

impl std::error::Error for MatchError {}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           MatchError::NotMatch(flag) => write!(f, "File flag {flag} not match."),
       }
    }
}
