use std::error::Error;

use crate::audiocore::{Spec, Frame};

#[derive(Debug)]
pub enum AudioWriteError {
    IOError(String), // 读写错误，应停止处理
    UnsupportedFormat, // 不支持的写入格式
}

impl Error for AudioWriteError {}

impl std::fmt::Display for AudioWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioWriteError::IOError(error) => write!(f, "IOError {error}"),
           AudioWriteError::UnsupportedFormat => write!(f, "Unsupported PCM format"),
       }
    }
}

pub trait AudioWriter {
    fn spec(&self) -> Spec;
    fn write(&mut self, frame: &Frame) -> Result<(), Box<dyn Error>>;
    fn finalize(&mut self) -> Result<(), Box<dyn Error>>;
}
