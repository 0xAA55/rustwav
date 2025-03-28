#![allow(dead_code)]

pub use std::{io::{self, Read, Write, Seek, SeekFrom}, sync::{Arc, Mutex}, fmt::Debug, error::Error};

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

pub fn use_writer(writer_shared: Arc<Mutex<Writer>>, action: fn(&mut Writer) -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
    let writer_shared = writer_shared.clone();
    let mut guard = writer_shared.lock().unwrap();
    let mut writer = guard.deref_mut();
    (action)(&writer)
}

pub fn expect_flag<T: Read>(r: &mut T, flag: &[u8; 4], err: Box<dyn std::error::Error>) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    if &buf != flag {
        Err(err)
    } else {
        Ok(())
    }
}

pub fn read_str<T: Read>(r: &mut T, size: usize, savage_decoder: &SavageStringDecoder) -> Result<String, Box<dyn std::error::Error>> {
    let mut buf = Vec::<u8>::new();
    buf.resize(size, 0);
    r.read_exact(&mut buf)?;
    Ok(savage_decoder.decode(&buf).trim_matches(char::from(0)).to_string())
}

pub fn read_sz<T: Read>(r: &mut T, savage_decoder: &SavageStringDecoder) -> Result<String, Box<dyn std::error::Error>> {
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
    Ok(savage_decoder.decode(&buf).trim_matches(char::from(0)).to_string())
}

pub fn write_str_sized<T: Write + ?Sized>(w: &mut T, data: &String, size: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = data.as_bytes().to_vec();
    buf.resize(size, 0);
    w.write_all(&buf)?;
    Ok(())
}

pub fn write_str<T: Write + ?Sized>(w: &mut T, data: &String) -> Result<(), Box<dyn std::error::Error>> {
    let buf = data.as_bytes();
    w.write_all(&buf)?;
    Ok(())
}

