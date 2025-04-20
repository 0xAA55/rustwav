#![allow(dead_code)]

use std::{io::{self, Read, Write, Seek}, sync::{Arc, Mutex}, ops::{DerefMut}, fmt::Debug};

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> Result<(), io::Error>
where R: Read, W: Write {
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

    pub fn read_sz<T: Read>(r: &mut T, text_encoding: &StringCodecMaps) -> Result<String, io::Error> {
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
        Ok(text_encoding.decode(&buf).trim_matches(char::from(0)).to_string())
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
}

