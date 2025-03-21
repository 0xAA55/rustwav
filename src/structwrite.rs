#![allow(dead_code)]

use std::{io::{Write, Seek, SeekFrom, Error}};

pub trait Writer: Write + Seek {}
impl<T> Writer for T where T: Write + Seek {}

pub struct StructWrite {
	pub writer: Box<dyn Writer>,
}

impl StructWrite{
	pub fn new(writer: Box<dyn Writer>) -> Self {
		Self {
			writer
		}
	}

	pub fn stream_position(&mut self) -> Result<u64, Error> {
		self.writer.stream_position()
	}

	pub fn seek(&mut self, pos: SeekFrom) -> Result<u64, Error> {
		self.writer.seek(pos)
	}

	pub fn seek_to(&mut self, pos: u64) -> Result<u64, Error> {
		self.writer.seek(SeekFrom::Start(pos))
	}

	pub fn skip(&mut self, bytes: i64) -> Result<u64, Error> {
		self.writer.seek(SeekFrom::Current(bytes))
	}

	pub fn write_bytes(&mut self, v: &[u8]) -> Result<(), Error> {
		self.writer.write_all(v)
	}

	pub fn write_le_i8(&mut self, v: i8) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_i16(&mut self, v: i16) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_i32(&mut self, v: i32) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_i64(&mut self, v: i64) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_u8(&mut self, v: u8) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_u16(&mut self, v: u16) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_u32(&mut self, v: u32) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_u64(&mut self, v: u64) -> Result<(), Error> {
		self.writer.write_all(&v.to_le_bytes())
	}

	pub fn write_le_f32(&mut self, v: f32) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_le_f64(&mut self, v: f64) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_i8(&mut self, v: i8) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_i16(&mut self, v: i16) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_i32(&mut self, v: i32) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_i64(&mut self, v: i64) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_u8(&mut self, v: u8) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_u16(&mut self, v: u16) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_u32(&mut self, v: u32) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_u64(&mut self, v: u64) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_f32(&mut self, v: f32) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}

	pub fn write_be_f64(&mut self, v: f64) -> Result<(), Error> {
		self.writer.write_all(&v.to_be_bytes())
	}
}
