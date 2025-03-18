use std::{error::Error};

use crate::audiocore::{Spec, Frame};

#[derive(Debug)]
pub enum AudioReadError {
    IOError(String), // 读写错误，应停止处理
    FormatError, // 格式错误，说明可以尝试使用别的格式的读取器来读取
    DataCorrupted, // 格式也许是正确的，但是数据是错误的
    Unimplemented, // 格式正确，但是这种格式的文件的读写方式没有被开发出来，应停止处理
    EndOfFile, // 超出文件结尾
}

impl Error for AudioReadError {}

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

pub trait AudioReader {
    fn spec(&self) -> Spec;

    fn get_sample(&mut self, position: u64) -> Result<Frame, Box<dyn Error>>;

    fn iter(&mut self) -> AudioReaderIter where Self: Sized {
        AudioReaderIter{
            reader: self,
            position: 0,
        }
    }
}

pub struct AudioReaderIter<'a> {
    reader: &'a mut dyn AudioReader,
    position: u64,
}

impl Iterator for AudioReaderIter<'_> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        let query = self.position;
        self.position += 1;
        match self.reader.get_sample(query) {
            Ok(frame) => Some(frame),
            Err(_) => None,
        }
    }
}
