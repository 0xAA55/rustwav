#![allow(dead_code)]

use std::{io::{Write, Error}};

pub trait StructWrite: Write {
	fn write_le_i8 (&mut self, v: i8 ) -> Result<(), Error>;
	fn write_le_i16(&mut self, v: i16) -> Result<(), Error>;
	fn write_le_i32(&mut self, v: i32) -> Result<(), Error>;
	fn write_le_i64(&mut self, v: i64) -> Result<(), Error>;
	fn write_le_u8 (&mut self, v: u8 ) -> Result<(), Error>;
	fn write_le_u16(&mut self, v: u16) -> Result<(), Error>;
	fn write_le_u32(&mut self, v: u32) -> Result<(), Error>;
	fn write_le_u64(&mut self, v: u64) -> Result<(), Error>;
	fn write_be_i8 (&mut self, v: i8 ) -> Result<(), Error>;
	fn write_be_i16(&mut self, v: i16) -> Result<(), Error>;
	fn write_be_i32(&mut self, v: i32) -> Result<(), Error>;
	fn write_be_i64(&mut self, v: i64) -> Result<(), Error>;
	fn write_be_u8 (&mut self, v: u8 ) -> Result<(), Error>;
	fn write_be_u16(&mut self, v: u16) -> Result<(), Error>;
	fn write_be_u32(&mut self, v: u32) -> Result<(), Error>;
	fn write_be_u64(&mut self, v: u64) -> Result<(), Error>;
}

impl<W: Write> StructWrite for W {
	fn write_le_i8(&mut self, v: i8) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_i16(&mut self, v: i16) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_i32(&mut self, v: i32) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_i64(&mut self, v: i64) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_u8(&mut self, v: u8) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_u16(&mut self, v: u16) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_u32(&mut self, v: u32) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_le_u64(&mut self, v: u64) -> Result<(), Error> {
		self.write_all(&v.to_le_bytes())
	}

	fn write_be_i8(&mut self, v: i8) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_i16(&mut self, v: i16) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_i32(&mut self, v: i32) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_i64(&mut self, v: i64) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_u8(&mut self, v: u8) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_u16(&mut self, v: u16) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_u32(&mut self, v: u32) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}

	fn write_be_u64(&mut self, v: u64) -> Result<(), Error> {
		self.write_all(&v.to_be_bytes())
	}
}
