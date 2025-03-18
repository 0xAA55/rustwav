use std::error::Error;

#[derive(Debug)]
pub enum AudioReadError {
    IOError(String), // 读写错误，应停止处理
    FormatError, // 格式错误，说明可以尝试使用别的格式的读取器来读取
    Unimplemented, // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
}

impl Error for AudioReadError {}

impl std::fmt::Display for AudioReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioReadError::IOError(error) => write!(f, "IOError {error}"),
           AudioReadError::FormatError => write!(f, "Invalid audio file format"),
           AudioReadError::Unimplemented => write!(f, "Unimplemented for the file format"),
       }
    }
}

#[derive(Clone)]
pub enum SampleFormat {
    Unknown,
    Float,
    Int,
}

#[derive(Clone)]
pub struct Spec {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

impl Spec {
    pub fn new() -> Self {
        Self {
            channels: 0,
            sample_rate: 0,
            bits_per_sample: 0,
            sample_format: SampleFormat::Unknown,
        }
    }
}

pub struct Frame(f32, f32);

pub trait AudioReader: Iterator<Item = Frame> {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        panic!("Abstract function called with param input_file = {input_file}.");
    }
    fn spec(&self) -> Spec;

    fn iter(&mut self) -> AudioReaderIter;
}

pub trait AudioReaderIter: Iterator {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item>;
}
