use std::io::{self, Read, Write, Seek, SeekFrom};

pub trait Reader: Read + Seek {}
impl<T> Reader for T where T: Read + Seek {}

pub trait Writer: Write + Seek{}
impl<T> Writer for T where T: Write + Seek{}

#[allow(non_camel_case_types)]
pub struct ReaderS_Over_D {
	reader: Box<dyn Reader>,
}

impl ReaderS_Over_D {
	fn from(reader: Box<dyn Reader>) -> Self {
		Self {
			reader,
		}
	}
}

impl Read for ReaderS_Over_D {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
    	self.reader.read(buf)
    }
}

impl Seek for ReaderS_Over_D {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
		self.reader.seek(pos)
	}
}

#[allow(non_camel_case_types)]
pub struct WriterS_Over_D {
	writer: Box<dyn Writer>,
}

impl WriterS_Over_D {
	fn from(writer: Box<dyn Writer>) -> Self {
		Self {
			writer,
		}
	}
}

impl Seek for WriterS_Over_D {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
		self.writer.seek(pos)
	}
}

impl Write for WriterS_Over_D {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
    	self.writer.write(buf)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
    	self.writer.flush()
    }
}
