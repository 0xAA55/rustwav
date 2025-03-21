#![allow(dead_code)]

use std::{io::{Read, Seek, SeekFrom, Error}};

use crate::sampleutils::i24;

#[derive(Debug)]
pub enum MatchError {
    NotMatch(String),
}

impl std::error::Error for MatchError {}

impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           MatchError::NotMatch(flag) => write!(f, "File flag {flag} not match"),
       }
    }
}

pub trait Reader: Read + Seek {}
impl<T> Reader for T where T: Read + Seek {}

pub struct StructRead {
	pub reader: Box<dyn Reader>,
}

impl StructRead {
	pub fn new(reader: Box<dyn Reader>) -> Self {
		Self {
			reader
		}
	}

	pub fn stream_position(&mut self) -> Result<u64, Error> {
		self.reader.stream_position()
	}
	
	pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
		self.reader.seek(pos)
	}

	pub fn seek_to(&mut self, pos: u64) -> Result<u64, Error> {
		self.reader.seek(SeekFrom::Start(pos))
	}

	pub fn skip(&mut self, bytes: i64) -> Result<u64, Error> {
		self.reader.seek(SeekFrom::Current(bytes))
	}

	pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
		self.reader.read_exact(buf)
	}

	pub fn read_string(&mut self, size: usize) -> Result<String, Box<dyn std::error::Error>> {
		let mut buf = Vec::<u8>::new();
		buf.resize(size, 0);
		self.read_exact(&mut buf)?;
		Ok(std::str::from_utf8(&buf)?.to_string())
	}

	pub fn read_zstring(&mut self) -> Result<String, Box<dyn std::error::Error>> {
		let mut buf = Vec::<u8>::new();
		loop {
			let b = self.read_le_u8()?;
			if b != 0 {
				buf.push(b);
			} else {
				break;
			}
		}
		Ok(std::str::from_utf8(&buf)?.to_string())
	}

	pub fn read_le_i8(&mut self) -> Result<i8, Error> {
		let mut buf = [0u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(i8::from_le_bytes(buf))
	}

	pub fn read_le_i16(&mut self) -> Result<i16, Error> {
		let mut buf = [0u8; 2];
		self.reader.read_exact(&mut buf)?;
		Ok(i16::from_le_bytes(buf))
	}

	pub fn read_le_i24(&mut self) -> Result<i24, Error> {
		let mut ret = [0u8; 3];
		self.reader.read_exact(&mut ret)?;
		Ok(i24(ret[0] as i8, ret[1] as i8, ret[2] as i8))
	}

	pub fn read_le_i32(&mut self) -> Result<i32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(i32::from_le_bytes(buf))
	}

	pub fn read_le_i64(&mut self) -> Result<i64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(i64::from_le_bytes(buf))
	}

	pub fn read_le_u8(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(u8::from_le_bytes(buf))
	}

	pub fn read_le_u16(&mut self) -> Result<u16, Error> {
		let mut buf = [0u8; 2];
		self.reader.read_exact(&mut buf)?;
		Ok(u16::from_le_bytes(buf))
	}

	pub fn read_le_u32(&mut self) -> Result<u32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(u32::from_le_bytes(buf))
	}

	pub fn read_le_u64(&mut self) -> Result<u64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(u64::from_le_bytes(buf))
	}

	pub fn read_le_f32(&mut self) -> Result<f32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(f32::from_le_bytes(buf))
	}

	pub fn read_le_f64(&mut self) -> Result<f64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(f64::from_le_bytes(buf))
	}

	pub fn read_be_i8(&mut self) -> Result<i8, Error> {
		let mut buf = [0u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(i8::from_be_bytes(buf))
	}

	pub fn read_be_i16(&mut self) -> Result<i16, Error> {
		let mut buf = [0u8; 2];
		self.reader.read_exact(&mut buf)?;
		Ok(i16::from_be_bytes(buf))
	}

	pub fn read_be_i32(&mut self) -> Result<i32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(i32::from_be_bytes(buf))
	}

	pub fn read_be_i64(&mut self) -> Result<i64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(i64::from_be_bytes(buf))
	}

	pub fn read_be_u8(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8; 1];
		self.reader.read_exact(&mut buf)?;
		Ok(u8::from_be_bytes(buf))
	}

	pub fn read_be_u16(&mut self) -> Result<u16, Error> {
		let mut buf = [0u8; 2];
		self.reader.read_exact(&mut buf)?;
		Ok(u16::from_be_bytes(buf))
	}

	pub fn read_be_u32(&mut self) -> Result<u32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(u32::from_be_bytes(buf))
	}

	pub fn read_be_u64(&mut self) -> Result<u64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(u64::from_be_bytes(buf))
	}

	pub fn read_be_f32(&mut self) -> Result<f32, Error> {
		let mut buf = [0u8; 4];
		self.reader.read_exact(&mut buf)?;
		Ok(f32::from_be_bytes(buf))
	}

	pub fn read_be_f64(&mut self) -> Result<f64, Error> {
		let mut buf = [0u8; 8];
		self.reader.read_exact(&mut buf)?;
		Ok(f64::from_be_bytes(buf))
	}

	fn bytes_to_string(u8s: &[u8]) -> String {
		let mut ret = String::new();
		for byte in u8s.iter() {
			if *byte >= 0x20 && *byte < 0x80 {
				ret.push(*byte as char);
			} else {
				ret.push('?');
			}
		}
		ret
	}

	pub fn expect_flag(&mut self, compare_to: &[u8], errtype: Box<dyn std::error::Error>) -> Result<(), Box<dyn std::error::Error>> {
		let mut buf = vec![0u8; compare_to.len()];
		self.reader.read_exact(&mut buf)?;
		if buf == compare_to {
			Ok(())
		} else {
			Err(errtype)
		}
	}

	pub fn peek_flag(&mut self, flag_len: usize) -> Result<Vec<u8>, Error> {
		let mut ret = vec![0u8; flag_len];
		self.reader.read_exact(&mut ret)?;
		self.reader.seek(SeekFrom::Current(-(flag_len as i64)))?;
		Ok(ret)
	}

	pub fn read_flag(&mut self, flag_len: usize) -> Result<Vec<u8>, Error> {
		let mut ret = vec![0u8; flag_len];
		self.reader.read_exact(&mut ret)?;
		Ok(ret)
	}
}
