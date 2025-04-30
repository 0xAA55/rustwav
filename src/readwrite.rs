#![allow(dead_code)]

use std::{io::{self, Read, Write, Seek, SeekFrom}, sync::{Arc, Mutex}, ops::{DerefMut}, fmt::Debug};

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

/// ## The `ReadBridge` hides a `dyn Reader` and acts like a struct that implements `Read + Seek + Debug`.
#[derive(Debug)]
pub struct ReadBridge<'a> {
    reader: &'a mut dyn Reader,
}

impl<'a> ReadBridge<'a> {
    pub fn new(reader: &'a mut dyn Reader) -> Self {
        Self{
            reader,
        }
    }
}

impl<'a> Read for ReadBridge<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        self.reader.read(buf)
    }
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), io::Error> {
        self.reader.read_exact(buf)
    }
}

impl<'a> Seek for ReadBridge<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.reader.seek(pos)
    }
}

/// ## The `WriteBridge` hides a `dyn Writer` and acts like a struct that implements `Write + Seek + Debug`.
#[derive(Debug)]
pub struct WriteBridge<'a> {
    writer: &'a mut dyn Writer,
}

impl<'a> WriteBridge<'a> {
    pub fn new(writer: &'a mut dyn Writer) -> Self {
        Self{
            writer,
        }
    }
}

impl<'a> Write for WriteBridge<'_> {
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

impl<'a> Seek for WriteBridge<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, io::Error> {
        self.writer.seek(pos)
    }
}

#[derive(Debug, Clone)]
pub struct SharedReader(Arc<Mutex<dyn Reader>>);

impl SharedReader{
    pub fn new<T>(reader: T) -> Self
    where T: Reader + 'static {
        Self(Arc::new(Mutex::new(reader)))
    }

    pub fn escorted_work<T, F, E>(&self, mut action: F) -> Result<T, E>
    where F: FnMut(&mut dyn Reader) -> Result<T, E> {
        let mut guard = self.0.lock().unwrap();
        let mut reader = guard.deref_mut();
        (action)(&mut reader)
    }

    pub fn escorted_read<F, E>(&self, mut action: F) -> Result<(), E>
    where F: FnMut(&mut dyn Reader) -> Result<(), E> {
        let mut guard = self.0.lock().unwrap();
        let mut reader = guard.deref_mut();
        (action)(&mut reader)
    }
}

#[derive(Debug, Clone)]
pub struct SharedWriter(Arc<Mutex<dyn Writer>>);

impl SharedWriter{
    pub fn new<T>(writer: T) -> Self
    where T: Writer + 'static {
        Self(Arc::new(Mutex::new(writer)))
    }

    pub fn escorted_work<T, F, E>(&self, mut action: F) -> Result<T, E>
    where F: FnMut(&mut dyn Writer) -> Result<T, E> {
        let mut guard = self.0.lock().unwrap();
        let mut writer = guard.deref_mut();
        (action)(&mut writer)
    }

    pub fn escorted_write<F, E>(&self, mut action: F) -> Result<(), E>
    where F: FnMut(&mut dyn Writer) -> Result<(), E> {
        let mut guard = self.0.lock().unwrap();
        let mut writer = guard.deref_mut();
        (action)(&mut writer)
    }
}

pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> Result<(), io::Error>
where
    R: Read, W: Write {
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

pub mod string_io {
    use std::{io::{self, Read, Write}};
    use crate::savagestr::{StringCodecMaps, SavageStringCodecs};

    pub fn read_bytes<T: Read>(r: &mut T, size: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_str<T: Read>(r: &mut T, size: usize, text_encoding: &StringCodecMaps) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding.decode(&buf).trim_matches(char::from(0)).to_string())
    }

    pub fn read_str_by_code_page<T: Read>(r: &mut T, size: usize, text_encoding: &StringCodecMaps, code_page: u32) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding.decode_bytes_by_code_page(&buf, code_page).trim_matches(char::from(0)).to_string())
    }

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

    pub fn read_sz<T: Read>(r: &mut T, text_encoding: &StringCodecMaps) -> Result<String, io::Error> {
        Ok(text_encoding.decode(&read_sz_raw(r)?).trim_matches(char::from(0)).to_string())
    }

    pub fn read_sz_by_code_page<T: Read>(r: &mut T, text_encoding: &StringCodecMaps, code_page: u32) -> Result<String, io::Error> {
        Ok(text_encoding.decode_bytes_by_code_page(&read_sz_raw(r)?, code_page).trim_matches(char::from(0)).to_string())
    }

    pub fn write_str_sized<T: Write + ?Sized>(w: &mut T, data: &str, size: usize, text_encoding: &StringCodecMaps) -> Result<(), io::Error> {
        let mut data = text_encoding.encode(data);
        data.resize(size, 0);
        w.write_all(&data)?;
        Ok(())
    }

    pub fn write_str<T: Write + ?Sized>(w: &mut T, data: &str, text_encoding: &StringCodecMaps) -> Result<(), io::Error> {
        let data = text_encoding.encode(data);
        w.write_all(&data)?;
        Ok(())
    }

    pub fn write_str_by_code_page<T: Write + ?Sized>(w: &mut T, data: &str, text_encoding: &StringCodecMaps, code_page: u32) -> Result<(), io::Error> {
        let data = text_encoding.encode_strings_by_code_page(data, code_page);
        w.write_all(&data)?;
        Ok(())
    }
}

