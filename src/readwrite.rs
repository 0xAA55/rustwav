#![allow(dead_code)]

pub use std::{io::{self, Read, Write, Seek, SeekFrom}, fmt::Debug};

pub trait Reader: Read + Seek + Debug{}
impl<T> Reader for T where T: Read + Seek + Debug{}

pub trait Writer: Write + Seek+ Debug{}
impl<T> Writer for T where T: Write + Seek + Debug{}

impl Writer {
    // 使文件的长度以 2 字节对齐。
    fn pad(&self) -> Result<(), io::Error> {
        let pad_size = self.stream_position()? & 1;
        if pad_size {
            self.write_all(&[0u8; pad_size])?
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct DynWriter {
    writer: Box<dyn Writer>,
}

impl DynWriter {
    pub fn new(writer: Box<dyn Writer>) -> Self {
        Self {
            writer,
        }
    }

    pub fn borrow_inside(&self) -> &Box<dyn Writer> {
    	&self.writer
    }
}

impl Seek for DynWriter {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.writer.seek(pos)
    }
}

impl Write for DynWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.writer.flush()
    }
}
