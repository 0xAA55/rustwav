use std::{hash::DefaultHasher, io::{Read, Seek, SeekFrom, Error}, cmp::min};

trait Reader: Read + Seek {}
impl<T> Reader for T
where T: Read + Seek{}

pub struct FileHasher {
	hasher: DefaultHasher,
}

impl FileHasher{
	fn new() -> Self {
		Self {
			hasher: DefaultHasher::new(),
		}
	}

	fn hash<R>(&mut self, reader: &mut R, from_byte: u64, length: u64) -> Result<u64, Error>
	where R: Reader {
		reader.seek(SeekFrom::Start(from_byte))?;
		const BUFFER_SIZE = 8192usize;
		let mut buf = vec![0u8; BUFFER_SIZE];
		let mut to_hash = length;
		while to_hash >= BUFFER_SIZE {
			reader.read_exact(&buf)?;
			self.hasher.write(&buf);
			to_hash -= BUFFER_SIZE;
		}
		if to_hash != 0 {
			buf.resize(to_hash, 0);
			reader.read_exact(&buf)?;
			self.hasher.write(&buf);
		}
		Ok(self.hasher.finish())
	}
}
