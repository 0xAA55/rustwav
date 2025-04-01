use std::{hash::{Hasher, DefaultHasher}, io::{Read, Seek, SeekFrom, Error}};

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
		const BUFFER_SIZE: u64 = 81920;
		let mut buf = vec![0u8; BUFFER_SIZE as usize];
		let mut to_hash = length;
		while to_hash >= BUFFER_SIZE {
			reader.read_exact(&mut buf)?;
			self.hasher.write(&buf);
			to_hash -= BUFFER_SIZE;
		}
		if to_hash > 0 {
			buf.resize(to_hash as usize, 0);
			reader.read_exact(&mut buf)?;
			self.hasher.write(&buf);
		}
		Ok(self.hasher.finish())
	}
}