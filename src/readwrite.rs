#![allow(dead_code)]

pub use std::{io::{self, Read, Write, Seek, SeekFrom}, sync::{Arc, Mutex}, ops::{DerefMut}, fmt::Debug, error::Error};

pub use crate::savagestr::SavageStringCodec;

pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

pub fn escorted_write<F>(writer_shared: Arc<Mutex<dyn Writer>>, mut action: F) -> Result<(), Box<dyn Error>>
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

pub fn read_str<T: Read>(r: &mut T, size: usize, text_encoding: &dyn SavageStringCodec) -> Result<String, Box<dyn Error>> {
    let mut buf = vec![0; size];
    r.read_exact(&mut buf)?;
    Ok(text_encoding.decode(&buf).trim_matches(char::from(0)).to_string())
}

pub fn read_sz<T: Read>(r: &mut T, text_encoding: &dyn SavageStringCodec) -> Result<String, Box<dyn Error>> {
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

pub fn write_str_sized<T: Write + ?Sized>(w: &mut T, data: &String, size: usize, text_encoding: &dyn SavageStringCodec) -> Result<(), Box<dyn Error>> {
    let mut data = text_encoding.encode(&data);
    data.resize(size, 0);
    w.write_all(&data)?;
    Ok(())
}

pub fn write_str<T: Write + ?Sized>(w: &mut T, data: &String, text_encoding: &dyn SavageStringCodec) -> Result<(), Box<dyn Error>> {
    let data = text_encoding.encode(&data);
    w.write_all(&data)?;
    Ok(())
}

