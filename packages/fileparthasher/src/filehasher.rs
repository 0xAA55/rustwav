use std::{
    hash::{DefaultHasher, Hasher},
    io::{self, Error, Read, Seek, SeekFrom, Write},
};

/// * Copy data from a reader to a writer from the current position.
pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    const BUFFER_SIZE: u64 = 1024;
    let mut buf = vec![0u8; BUFFER_SIZE as usize];
    let mut to_copy = bytes_to_copy;
    while to_copy >= BUFFER_SIZE {
        reader.read_exact(&mut buf)?;
        writer.write_all(&buf)?;
        to_copy -= BUFFER_SIZE;
    }
    if to_copy > 0 {
        buf.resize(to_copy as usize, 0);
        reader.read_exact(&mut buf)?;
        writer.write_all(&buf)?;
    }
    Ok(())
}

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
        copy(reader, self, length)?;
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
