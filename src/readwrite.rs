#![allow(dead_code)]

pub use std::{io::{self, Read, Write, Seek, SeekFrom}, sync::{Arc, Mutex}, ops::{DerefMut}, fmt::Debug, error::Error};

pub use crate::savagestr::SavageStringDecoder;

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

pub fn use_writer<F>(writer_shared: Arc<Mutex<dyn Writer>>, mut action: F) -> Result<(), Box<dyn Error>>
where F: FnMut(&mut dyn Writer) -> Result<(), Box<dyn Error>> {
    let mut guard = writer_shared.lock().unwrap();
    let mut writer = guard.deref_mut();
    (action)(&mut writer)
}

pub fn read_bytes<T: Read>(r: &mut T, size: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_str<T: Read>(r: &mut T, size: usize, savage_decoder: &SavageStringDecoder) -> Result<String, Box<dyn Error>> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;
    Ok(savage_decoder.decode(&buf).trim_matches(char::from(0)).to_string())
}

pub fn read_sz<T: Read>(r: &mut T, savage_decoder: &SavageStringDecoder) -> Result<String, Box<dyn Error>> {
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

pub fn write_str_sized<T: Write + ?Sized>(w: &mut T, data: &String, size: usize) -> Result<(), Box<dyn Error>> {
    let mut buf = data.as_bytes().to_vec();
    buf.resize(size, 0);
    w.write_all(&buf)?;
    Ok(())
}

pub fn write_str<T: Write + ?Sized>(w: &mut T, data: &String) -> Result<(), Box<dyn Error>> {
    let buf = data.as_bytes();
    w.write_all(buf)?;
    Ok(())
}

