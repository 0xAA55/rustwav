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

}

