#![allow(dead_code)]

use std::io::{self, Read, Write, Seek, SeekFrom};

pub trait Reader: Read + Seek {}
impl<T> Reader for T where T: Read + Seek {}

pub trait Writer: Write + Seek{}
impl<T> Writer for T where T: Write + Seek {}

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
