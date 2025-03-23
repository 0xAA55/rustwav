
#[derive(Debug)]
pub enum AudioReadError {
    IOError(String), // 读写错误，应停止处理
    FormatError, // 格式错误，说明可以尝试使用别的格式的读取器来读取
    DataCorrupted, // 格式也许是正确的，但是数据是错误的
    Unimplemented, // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
    EndOfFile, // 超出文件结尾
}

impl std::error::Error for AudioReadError {}

impl std::fmt::Display for AudioReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioReadError::IOError(error) => write!(f, "IOError {error}"),
           AudioReadError::FormatError => write!(f, "Invalid audio file format"),
           AudioReadError::DataCorrupted => write!(f, "Audio file data corrupted"),
           AudioReadError::Unimplemented => write!(f, "Unimplemented for the file format"),
           AudioReadError::EndOfFile => write!(f, "Read to the end of the file"),
       }
    }
}

#[derive(Debug)]
pub enum AudioWriteError {
    InvalidArguments, // 输入了错误的参数
    IOError(String), // 读写错误，应停止处理
    UnsupportedFormat, // 不支持的写入格式
    ChannelCountNotMatch, // 声道数不匹配
}

impl std::error::Error for AudioWriteError {}

impl std::fmt::Display for AudioWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioWriteError::InvalidArguments => write!(f, "Invalid arguments or fields to write"),
           AudioWriteError::IOError(error) => write!(f, "IOError {error}"),
           AudioWriteError::UnsupportedFormat => write!(f, "Unsupported PCM format to be saved"),
           AudioWriteError::ChannelCountNotMatch => write!(f, "Channel count not match"),
       }
    }
}

#[derive(Debug)]
pub enum AudioError {
    CantGuessChannelMask(u16), // 无法猜出声道掩码
    ChannelNotMatchMask, // 声道数不和声道掩码匹配
    Unimplemented, // 没有实现的解析格式
    UnknownSampleType, // 不知道的样本的格式
}

impl std::error::Error for AudioError {}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioError::CantGuessChannelMask(channels) => write!(f, "Can't guess channel mask for channels = {}.", channels),
           AudioError::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
           AudioError::Unimplemented => write!(f, "Unimplemented for the file format"),
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
           MatchError::NotMatch(flag) => write!(f, "File flag {flag} not match"),
       }
    }
}
