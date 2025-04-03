use std::{hash::{Hasher, DefaultHasher}, io::{Read, Write, Seek, SeekFrom, Error}};

use crate::readwrite;

#[derive(Debug, Clone)]
pub struct FileHasher {
	hasher: DefaultHasher,
}

impl FileHasher{
	pub fn new() -> Self {
		Self {
			hasher: DefaultHasher::new(),
		}
	}

	pub fn hash<R>(&mut self, reader: &mut R, from_byte: u64, length: u64) -> Result<u64, Error>
	where R: Read + Seek {
		reader.seek(SeekFrom::Start(from_byte))?;
		readwrite::copy(reader, self, length)?;
		Ok(self.hasher.finish())
	}
}

impl Write for FileHasher {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>
    {
    	self.hasher.write(buf);
    	Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
    	Ok(())
    }
}
