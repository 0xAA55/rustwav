use std::{
    hash::{DefaultHasher, Hasher},
    io::{Error, Read, Seek, SeekFrom, Write},
};

use crate::readwrite;

/// * File hasher to calculate the hash for a section of a file, the hash is `u64` size. The `Write` trait was implemented for it.
#[derive(Debug, Clone)]
pub struct FileHasher {
    hasher: DefaultHasher,
}

impl FileHasher {
    pub fn new() -> Self {
        Self {
            hasher: DefaultHasher::new(),
        }
    }

    /// * Calculate the hash of the data from the `reader` with offset `from_byte` and length `length`
    pub fn hash<R>(&mut self, reader: &mut R, from_byte: u64, length: u64) -> Result<u64, Error>
    where
        R: Read + Seek,
    {
        reader.seek(SeekFrom::Start(from_byte))?;
        readwrite::copy(reader, self, length)?;
        Ok(self.hasher.finish())
    }
}

impl Write for FileHasher {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.hasher.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

impl Default for FileHasher {
    fn default() -> Self {
        Self::new()
    }
}
