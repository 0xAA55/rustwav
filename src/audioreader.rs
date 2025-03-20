use std::{error::Error};

use crate::audiocore::{Spec};

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

    fn iter<T>(&mut self) -> Iter<T> where Self: Sized;
}

pub trait Iter<T> : Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
