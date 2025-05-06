#![allow(dead_code)]

use std::{
    cmp::min,
    fmt::Debug,
    io::{self, Read, Seek, SeekFrom, Write},
    rc::Rc,
    cell::RefCell,
};

/// ## The `Reader` trait, `Read + Seek + Debug`
pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

/// ## The `Writer` trait, `Write + Seek + Debug`
pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

/// ## The `ReadBridge` hides a `dyn Reader` and acts like a struct that implements `Read + Seek + Debug`.
#[derive(Debug)]
pub struct ReadBridge<'a> {
    reader: &'a mut dyn Reader,
}

impl<'a> ReadBridge<'a> {
    pub fn new(reader: &'a mut dyn Reader) -> Self {
        Self { reader }
    }
}

impl Read for ReadBridge<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.reader.read(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        self.reader.read_exact(buf)
    }
}

impl Seek for ReadBridge<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.reader.seek(pos)
    }
    fn rewind(&mut self) -> Result<(), io::Error> {
        self.reader.rewind()
    }
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.reader.stream_position()
    }
}

/// ## The `WriteBridge` hides a `dyn Writer` and acts like a struct that implements `Write + Seek + Debug`.
#[derive(Debug)]
pub struct WriteBridge<'a> {
    writer: &'a mut dyn Writer,
}

impl<'a> WriteBridge<'a> {
    pub fn new(writer: &'a mut dyn Writer) -> Self {
        Self { writer }
    }
}

impl Write for WriteBridge<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        self.writer.flush()
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        self.writer.write_all(buf)
    }
}

impl Seek for WriteBridge<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.writer.seek(pos)
    }
    fn rewind(&mut self) -> Result<(), io::Error> {
        self.writer.rewind()
    }
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.writer.stream_position()
    }
}

/// ## Encapsulated shared `&mut dyn Reader`, implemented `Read + Seek + Debug + Clone`
#[derive(Debug, Clone)]
pub struct SharedReader<'a>(Rc<RefCell<&'a mut dyn Reader>>);

impl<'a> SharedReader<'a> {
    pub fn new(reader: &'a mut dyn Reader) -> Self {
        Self(Rc::new(RefCell::new(reader)))
    }

    /// * Let the reader work in your closure with a mutex lock guard.
    pub fn escorted_read<T, F, E>(&self, mut action: F) -> Result<T, E>
    where
        F: FnMut(&mut dyn Reader) -> Result<T, E>,
    {
        let mut reader = &mut *self.0.borrow_mut();
        (action)(&mut reader)
    }
}

impl Read for SharedReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.0.borrow_mut().read(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        self.0.borrow_mut().read_exact(buf)
    }
}

impl Seek for SharedReader<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.0.borrow_mut().seek(pos)
    }
    fn rewind(&mut self) -> Result<(), io::Error> {
        self.0.borrow_mut().rewind()
    }
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.0.borrow_mut().stream_position()
    }
}

/// ## Encapsulated shared `&mut dyn Writer`, implemented `Write + Seek + Debug + Clone`
#[derive(Debug, Clone)]
pub struct SharedWriter<'a>(Rc<RefCell<&'a mut dyn Writer>>);

impl<'a> SharedWriter<'a> {
    pub fn new(writer: &'a mut dyn Writer) -> Self {
        Self(Rc::new(RefCell::new(writer)))
    }

    /// * Let the writer work in your closure with a mutex lock guard.
    pub fn escorted_write<T, F, E>(&self, mut action: F) -> Result<T, E>
    where
        F: FnMut(&mut dyn Writer) -> Result<T, E>,
    {
        let mut writer = &mut *self.0.borrow_mut();
        (action)(&mut writer)
    }
}

impl Write for SharedWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        self.0.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        self.0.borrow_mut().flush()
    }
    fn write_all(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        self.0.borrow_mut().write_all(buf)
    }
}

impl Seek for SharedWriter<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.0.borrow_mut().seek(pos)
    }
    fn rewind(&mut self) -> Result<(), io::Error> {
        self.0.borrow_mut().rewind()
    }
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.0.borrow_mut().stream_position()
    }
}

/// ## Encapsulated shared `&mut dyn Reader`, implemented `Read + Seek + Debug + Clone`
#[derive(Debug, Clone)]
pub struct SharedReaderOwned(Rc<RefCell<Box<dyn Reader>>>);

impl SharedReaderOwned {
    pub fn new(reader: Box<dyn Reader>) -> Self {
        Self(Rc::new(RefCell::new(reader)))
    }
}

impl Read for SharedReaderOwned {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.0.borrow_mut().read(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        self.0.borrow_mut().read_exact(buf)
    }
}

impl Seek for SharedReaderOwned {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.0.borrow_mut().seek(pos)
    }
    fn rewind(&mut self) -> Result<(), io::Error> {
        self.0.borrow_mut().rewind()
    }
    fn stream_position(&mut self) -> Result<u64, io::Error> {
        self.0.borrow_mut().stream_position()
    }
}

/// ## A Reader that combines two readers into one with the ability to `Read` and `Seek` and `Debug`
#[derive(Debug)]
pub struct CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    first: R1,
    first_data_offset: u64,
    first_data_length: u64,
    second: R2,
    second_data_offset: u64,
    second_data_length: u64,
    stream_pos: u64,
}

impl<R1, R2> CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    pub fn new(
        mut first: R1,
        first_data_offset: u64,
        first_data_length: u64,
        mut second: R2,
        second_data_offset: u64,
        second_data_length: u64,
    ) -> Result<Self, io::Error> {
        first.seek(SeekFrom::Start(first_data_offset))?;
        second.seek(SeekFrom::Start(second_data_offset))?;
        Ok(Self {
            first,
            first_data_offset,
            first_data_length,
            second,
            second_data_offset,
            second_data_length,
            stream_pos: 0,
        })
    }
}

impl<R1, R2> Read for CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let remaining = (self.first_data_length + self.second_data_length) - self.stream_pos;
        if remaining == 0 {
            return Ok(0);
        }

        // Choose the reader to use
        let bytes_read = if self.stream_pos < self.first_data_length {
            let bytes_to_read = min((self.first_data_length - self.stream_pos) as usize, buf.len());
            let first_pos = self.stream_pos;
            self.first.seek(SeekFrom::Start(first_pos + self.first_data_offset))?;
            let n = self.first.read(&mut buf[..bytes_to_read])?;
            self.stream_pos += n as u64;
            n
        } else {
            let bytes_to_read = min((self.second_data_length - self.stream_pos) as usize, buf.len());
            let second_pos = self.stream_pos - self.first_data_length;
            self.second.seek(SeekFrom::Start(second_pos + self.second_data_offset))?;
            let n = self.second.read(&mut buf[..bytes_to_read])?;
            self.stream_pos += n as u64;
            n
        };

        Ok(bytes_read)
    }
}

impl<R1, R2> Seek for CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let total_len = self.first_data_length.checked_add(self.second_data_length).ok_or(io::ErrorKind::InvalidInput)?;
                if offset > 0 {
                    total_len.checked_add(offset as u64)
                } else {
                    total_len.checked_sub((-offset) as u64)
                }.ok_or(io::ErrorKind::InvalidInput)?
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.stream_pos.checked_add(offset as u64)
                } else {
                    self.stream_pos.checked_sub((-offset) as u64)
                }.ok_or(io::ErrorKind::InvalidInput)?
            }
        };

        let total_len = self.first_data_length + self.second_data_length;
        if new_pos > total_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek position out of bounds"
            ));
        }

        self.stream_pos = new_pos;
        Ok(new_pos)
    }
}

/// * Go to an offset without using seek. It's achieved by using dummy reads.
pub fn goto_offset_without_seek<T>(
    mut reader: T,
    cur_pos: &mut u64,
    position: u64,
) -> Result<u64, io::Error>
where
    T: Read,
{
    const SKIP_SIZE: u64 = 1024;
    let mut skip_buf = [0u8; SKIP_SIZE as usize];
    while *cur_pos + SKIP_SIZE <= position {
        reader.read_exact(&mut skip_buf)?;
        *cur_pos += SKIP_SIZE;
    }
    if *cur_pos < position {
        let mut skip_buf = vec![0u8; (position - *cur_pos) as usize];
        reader.read_exact(&mut skip_buf)?;
        *cur_pos = position;
    }
    if *cur_pos > position {
        Err(io::Error::new(
            io::ErrorKind::NotSeekable,
            format!(
                "The current position {cur_pos} has already exceeded the target position {position}"
            ),
        ))
    } else {
        Ok(*cur_pos)
    }
}

/// * Copy data from a reader to a writer from the current position.
pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> Result<(), io::Error>
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

/// ## This is for read/write strings from/to file with specific encoding and size, or read/write as NUL-terminated strings.
pub mod string_io {
    use crate::savagestr::{SavageStringCodecs, StringCodecMaps};
    use std::io::{self, Read, Write};

    /// * Read some bytes, and return the bytes, without you to create a local `vec![0u8; size]` and scratch your head with the messy codes
    pub fn read_bytes<T: Read>(r: &mut T, size: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// * Read a fixed-size string and decode it using the `StringCodecMaps`
    pub fn read_str<T: Read>(
        r: &mut T,
        size: usize,
        text_encoding: &StringCodecMaps,
    ) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding
            .decode(&buf)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a fixed-size string and decode it using the `StringCodecMaps` while you can specify the code page.
    pub fn read_str_by_code_page<T: Read>(
        r: &mut T,
        size: usize,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding
            .decode_bytes_by_code_page(&buf, code_page)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a NUL terminated string by raw, not decode it.
    pub fn read_sz_raw<T: Read>(r: &mut T) -> Result<Vec<u8>, io::Error> {
        let mut buf = Vec::<u8>::new();
        loop {
            let b = [0u8; 1];
            r.read_exact(&mut buf)?;
            let b = b[0];
            if b != 0 {
                buf.push(b);
            } else {
                break;
            }
        }
        Ok(buf)
    }

    /// * Read a NUL terminated string and decode it.
    pub fn read_sz<T: Read>(
        r: &mut T,
        text_encoding: &StringCodecMaps,
    ) -> Result<String, io::Error> {
        Ok(text_encoding
            .decode(&read_sz_raw(r)?)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a NUL terminated string and decode it with the specified code page.
    pub fn read_sz_by_code_page<T: Read>(
        r: &mut T,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> Result<String, io::Error> {
        Ok(text_encoding
            .decode_bytes_by_code_page(&read_sz_raw(r)?, code_page)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Write a fixed-size encoded string.
    pub fn write_str_sized<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        size: usize,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), io::Error> {
        let mut data = text_encoding.encode(data);
        data.resize(size, 0);
        w.write_all(&data)?;
        Ok(())
    }

    /// * Write an encoded string.
    pub fn write_str<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        text_encoding: &StringCodecMaps,
    ) -> Result<(), io::Error> {
        let data = text_encoding.encode(data);
        w.write_all(&data)?;
        Ok(())
    }

    /// * Write an encoded string encoded with the specified code page.
    pub fn write_str_by_code_page<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> Result<(), io::Error> {
        let data = text_encoding.encode_strings_by_code_page(data, code_page);
        w.write_all(&data)?;
        Ok(())
    }
}
