#![allow(dead_code)]

use std::{io::{Read, Error}};

pub trait StructRead: Read {
	fn read_le_i8 (&mut self) -> Result<i8 , Error>;
	fn read_le_i16(&mut self) -> Result<i16, Error>;
	fn read_le_i32(&mut self) -> Result<i32, Error>;
	fn read_le_i64(&mut self) -> Result<i64, Error>;
	fn read_le_u8 (&mut self) -> Result<u8 , Error>;
	fn read_le_u16(&mut self) -> Result<u16, Error>;
	fn read_le_u32(&mut self) -> Result<u32, Error>;
	fn read_le_u64(&mut self) -> Result<u64, Error>;
	fn read_be_i8 (&mut self) -> Result<i8 , Error>;
	fn read_be_i16(&mut self) -> Result<i16, Error>;
	fn read_be_i32(&mut self) -> Result<i32, Error>;
	fn read_be_i64(&mut self) -> Result<i64, Error>;
	fn read_be_u8 (&mut self) -> Result<u8 , Error>;
	fn read_be_u16(&mut self) -> Result<u16, Error>;
	fn read_be_u32(&mut self) -> Result<u32, Error>;
	fn read_be_u64(&mut self) -> Result<u64, Error>;
	fn read_compare(&mut self, compare_to: &[u8]) -> Result<bool, Error>;
}

impl<R: Read> StructRead for R {
	fn read_le_i8(&mut self) -> Result<i8, Error> {
		let mut buf = [0u8; 1];
		self.read_exact(&mut buf)?;
		Ok(i8::from_le_bytes(buf))
	}

	fn read_le_i16(&mut self) -> Result<i16, Error> {
		let mut buf = [0u8; 2];
		self.read_exact(&mut buf)?;
		Ok(i16::from_le_bytes(buf))
	}

	fn read_le_i32(&mut self) -> Result<i32, Error> {
		let mut buf = [0u8; 4];
		self.read_exact(&mut buf)?;
		Ok(i32::from_le_bytes(buf))
	}

	fn read_le_i64(&mut self) -> Result<i64, Error> {
		let mut buf = [0u8; 8];
		self.read_exact(&mut buf)?;
		Ok(i64::from_le_bytes(buf))
	}

	fn read_le_u8(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8; 1];
		self.read_exact(&mut buf)?;
		Ok(u8::from_le_bytes(buf))
	}

	fn read_le_u16(&mut self) -> Result<u16, Error> {
		let mut buf = [0u8; 2];
		self.read_exact(&mut buf)?;
		Ok(u16::from_le_bytes(buf))
	}

	fn read_le_u32(&mut self) -> Result<u32, Error> {
		let mut buf = [0u8; 4];
		self.read_exact(&mut buf)?;
		Ok(u32::from_le_bytes(buf))
	}

	fn read_le_u64(&mut self) -> Result<u64, Error> {
		let mut buf = [0u8; 8];
		self.read_exact(&mut buf)?;
		Ok(u64::from_le_bytes(buf))
	}

	fn read_be_i8(&mut self) -> Result<i8, Error> {
		let mut buf = [0u8; 1];
		self.read_exact(&mut buf)?;
		Ok(i8::from_be_bytes(buf))
	}

	fn read_be_i16(&mut self) -> Result<i16, Error> {
		let mut buf = [0u8; 2];
		self.read_exact(&mut buf)?;
		Ok(i16::from_be_bytes(buf))
	}

	fn read_be_i32(&mut self) -> Result<i32, Error> {
		let mut buf = [0u8; 4];
		self.read_exact(&mut buf)?;
		Ok(i32::from_be_bytes(buf))
	}

	fn read_be_i64(&mut self) -> Result<i64, Error> {
		let mut buf = [0u8; 8];
		self.read_exact(&mut buf)?;
		Ok(i64::from_be_bytes(buf))
	}

	fn read_be_u8(&mut self) -> Result<u8, Error> {
		let mut buf = [0u8; 1];
		self.read_exact(&mut buf)?;
		Ok(u8::from_be_bytes(buf))
	}

	fn read_be_u16(&mut self) -> Result<u16, Error> {
		let mut buf = [0u8; 2];
		self.read_exact(&mut buf)?;
		Ok(u16::from_be_bytes(buf))
	}

	fn read_be_u32(&mut self) -> Result<u32, Error> {
		let mut buf = [0u8; 4];
		self.read_exact(&mut buf)?;
		Ok(u32::from_be_bytes(buf))
	}

	fn read_be_u64(&mut self) -> Result<u64, Error> {
		let mut buf = [0u8; 8];
		self.read_exact(&mut buf)?;
		Ok(u64::from_be_bytes(buf))
	}

	fn read_compare(&mut self, compare_to: &[u8]) -> Result<bool, Error> {
		let mut buf = vec![0u8; compare_to.len()];
		self.read_exact(&mut buf)?;
		Ok(buf == compare_to)
	}
}
