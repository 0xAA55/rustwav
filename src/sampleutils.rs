use std::{io::{Read, Write, Error}};

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct i24(pub u8, pub u8, pub u8); // 低中高

impl i24 {
    pub fn from_le_bytes(data: [u8; 3]) -> Self {
        Self(data[0], data[1], data[2])
    }
    pub fn from_be_bytes(data: [u8; 3]) -> Self {
        Self(data[2], data[1], data[0])
    }
    pub fn to_i32(&self) -> i32 {
        i32::from_le_bytes([self.2, self.0, self.1, self.2])
    }
    pub fn to_i64(&self) -> i64 {
        i64::from_le_bytes([self.1, self.2, self.0, self.1, self.2, self.0, self.1, self.2])
    }
    pub fn to_le_bytes(&self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
    pub fn to_be_bytes(&self) -> [u8; 3] {
        [self.2, self.1, self.0]
    }
}

pub trait SampleConv {
    fn clampf(&self) -> f32 {
        panic!("There shouldn't a `clampf()` call on integers");
    }
    fn clampd(&self) -> f64 {
        panic!("There shouldn't a `clampf()` call on integers");
    }
    fn from(v: impl SampleConv) -> Self;
    fn to_i8(&self) -> i8;
    fn to_i16(&self) -> i16;
    fn to_i32(&self) -> i32;
    fn to_i64(&self) -> i64;
    fn to_u8(&self) -> u8;
    fn to_u16(&self) -> u16;
    fn to_u32(&self) -> u32;
    fn to_u64(&self) -> u64;
    fn to_i24(&self) -> i24;
    fn to_f32(&self) -> f32;
    fn to_f64(&self) -> f64;
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized;
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized;
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized;
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized;
}

impl SampleConv for i8{
    fn from(v: impl SampleConv) -> i8{
        v.to_i8()
    }
    fn to_i8(&self) -> i8{
        *self
    } 
    fn to_i16(&self) -> i16{
        let v = *self as i16;
        (v << 8) | (v & 0xFF)
    }
    fn to_i32(&self) -> i32{
        let v = self.to_i16() as i32;
        (v << 16) | (v & 0xFFFF)
    }
    fn to_i64(&self) -> i64{
        let v = self.to_i32() as i64;
        (v << 32) | (v & 0xFFFFFFFF)
    }
    fn to_u8(&self) -> u8{
        (*self as u8).wrapping_add(0x80)
    } 
    fn to_u16(&self) -> u16{
        self.to_u8().to_u16()
    }
    fn to_u32(&self) -> u32{
        self.to_u16().to_u32()
    }
    fn to_u64(&self) -> u64{
        self.to_u16().to_u64()
    }
    fn to_i24(&self) -> i24{
        let v = *self as u8;
        i24(v, v, v)
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for i16{
    fn from(v: impl SampleConv) -> i16{
        v.to_i16()
    }
    fn to_i8(&self) -> i8{
        (*self >> 8) as i8
    } 
    fn to_i16(&self) -> i16{
        *self
    }
    fn to_i32(&self) -> i32{
        let v = *self as i32;
        (v << 16) | (v & 0xFFFF)
    }
    fn to_i64(&self) -> i64{
        let v = self.to_i32() as i64;
        (v << 32) | (v & 0xFFFFFFFF)
    }
    fn to_u8(&self) -> u8{
        self.to_u16().to_u8()
    } 
    fn to_u16(&self) -> u16{
        (*self as u16).wrapping_add(0x8000)
    }
    fn to_u32(&self) -> u32{
        self.to_u16().to_u32()
    }
    fn to_u64(&self) -> u64{
        self.to_u16().to_u64()
    }
    fn to_i24(&self) -> i24{
        let b = self.to_le_bytes();
        i24(b[1] as u8, b[0] as u8, b[1] as u8)
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for i24 {
    fn from(v: impl SampleConv) -> i24{
        v.to_i24()
    }
    fn to_i8(&self) -> i8 {
        self.0 as i8
    }
    fn to_i16(&self) -> i16{
        i16::from_le_bytes([self.1, self.2])
    }
    fn to_i32(&self) -> i32{
        self.to_i32()
    }
    fn to_i64(&self) -> i64{
        self.to_i32().to_i64()
    }
    fn to_u8(&self) -> u8{
        self.0.to_u8()
    }
    fn to_u16(&self) -> u16{
        self.to_i16().to_u16()
    }
    fn to_u32(&self) -> u32{
        self.to_i32().to_u32()
    }
    fn to_u64(&self) -> u64{
        self.to_i64().to_u64()
    }
    fn to_i24(&self) -> i24{
        *self
    }
    fn to_f32(&self) -> f32{
        self.to_i32().to_f32()
    }
    fn to_f64(&self) -> f64{
        self.to_i64().to_f64()
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 3];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 3];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for i32{
    fn from(v: impl SampleConv) -> i32{
        v.to_i32()
    }
    fn to_i8(&self) -> i8{
        (*self >> 24) as i8
    } 
    fn to_i16(&self) -> i16{
        (*self >> 16) as i16
    }
    fn to_i32(&self) -> i32{
        *self
    }
    fn to_i64(&self) -> i64{
        let v = self.to_i32() as i64;
        (v << 32) | (v & 0xFFFFFFFF)
    }
    fn to_u8(&self) -> u8{
        self.to_u32().to_u8()
    } 
    fn to_u16(&self) -> u16{
        self.to_u32().to_u16()
    }
    fn to_u32(&self) -> u32{
        (*self as u32).wrapping_add(0x80000000)
    }
    fn to_u64(&self) -> u64{
        self.to_u32().to_u64()
    }
    fn to_i24(&self) -> i24{
        let b = self.to_le_bytes();
        i24(b[1] as u8, b[2] as u8, b[3] as u8)
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for i64{
    fn from(v: impl SampleConv) -> i64{
        v.to_i64()
    }
    fn to_i8(&self) -> i8{
        (*self >> 56) as i8
    } 
    fn to_i16(&self) -> i16{
        (*self >> 48) as i16
    }
    fn to_i32(&self) -> i32{
        (*self >> 32) as i32
    }
    fn to_i64(&self) -> i64{
        *self
    }
    fn to_u8(&self) -> u8{
        self.to_u64().to_u8()
    } 
    fn to_u16(&self) -> u16{
        self.to_u64().to_u16()
    }
    fn to_u32(&self) -> u32{
        self.to_u64().to_u32()
    }
    fn to_u64(&self) -> u64{
        (*self as u64).wrapping_add(0x80000000_00000000)
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for u8{
    fn from(v: impl SampleConv) -> u8{
        v.to_u8()
    }
    fn to_i8(&self) -> i8{
        self.wrapping_sub(0x80) as i8
    } 
    fn to_i16(&self) -> i16{
        self.to_i8().to_i16()
    }
    fn to_i32(&self) -> i32{
        self.to_i8().to_i32()
    }
    fn to_i64(&self) -> i64{
        self.to_i8().to_i64()
    }
    fn to_u8(&self) -> u8{
        *self
    } 
    fn to_u16(&self) -> u16{
        let v = self.to_u8() as u16;
        (v << 8) | v
    }
    fn to_u32(&self) -> u32{
        let v = self.to_u16() as u32;
        (v << 16) | v
    }
    fn to_u64(&self) -> u64{
        let v = self.to_u32() as u64;
        (v << 32) | v
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for u16{
    fn from(v: impl SampleConv) -> u16{
        v.to_u16()
    }
    fn to_i8(&self) -> i8{
        self.to_i16().to_i8()
    } 
    fn to_i16(&self) -> i16{
        self.wrapping_sub(0x8000) as i16
    }
    fn to_i32(&self) -> i32{
        self.to_i16().to_i32()
    }
    fn to_i64(&self) -> i64{
        self.to_i16().to_i64()
    }
    fn to_u8(&self) -> u8{
        (*self >> 8) as u8
    } 
    fn to_u16(&self) -> u16{
        *self
    }
    fn to_u32(&self) -> u32{
        let v = self.to_u16() as u32;
        (v << 16) | v
    }
    fn to_u64(&self) -> u64{
        let v = self.to_u32() as u64;
        (v << 32) | v
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 2];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for u32{
    fn from(v: impl SampleConv) -> u32{
        v.to_u32()
    }
    fn to_i8(&self) -> i8{
        self.to_i32().to_i8()
    } 
    fn to_i16(&self) -> i16{
        self.to_i32().to_i16()
    }
    fn to_i32(&self) -> i32{
        self.wrapping_sub(0x80000000) as i32
    }
    fn to_i64(&self) -> i64{
        self.to_i32().to_i64()
    }
    fn to_u8(&self) -> u8{
        (*self >> 24) as u8
    } 
    fn to_u16(&self) -> u16{
        (*self >> 16) as u16
    }
    fn to_u32(&self) -> u32{
        *self
    }
    fn to_u64(&self) -> u64{
        let v = self.to_u32() as u64;
        (v << 32) | v
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for u64{
    fn from(v: impl SampleConv) -> u64{
        v.to_u64()
    }
    fn to_i8(&self) -> i8{
        self.to_i64().to_i8()
    } 
    fn to_i16(&self) -> i16{
        self.to_i64().to_i16()
    }
    fn to_i32(&self) -> i32{
        self.to_i64().to_i32()
    }
    fn to_i64(&self) -> i64{
        self.wrapping_sub(0x80000000_00000000) as i64
    }
    fn to_u8(&self) -> u8{
        (*self >> 56) as u8
    } 
    fn to_u16(&self) -> u16{
        (*self >> 48) as u16
    }
    fn to_u32(&self) -> u32{
        (*self >> 32) as u32
    }
    fn to_u64(&self) -> u64{
        *self
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for f32{
    fn clampf(&self) -> f32 {
        let v = *self;
        if v > 1.0 {
            1.0
        } else if v < -1.0 {
            -1.0
        } else {
            v
        }
    }

    fn from(v: impl SampleConv) -> f32{
        v.to_f32()
    }
    fn to_i8(&self) -> i8{
        (self.clampf() * (i8::MAX as f32)) as i8
    } 
    fn to_i16(&self) -> i16{
        (self.clampf() * (i16::MAX as f32)) as i16
    }
    fn to_i32(&self) -> i32{
        (self.clampf() * (i32::MAX as f32)) as i32
    }
    fn to_i64(&self) -> i64{
        (self.clampf() * (i64::MAX as f32)) as i64
    }
    fn to_u8(&self) -> u8{
        self.to_i8().to_u8()
    } 
    fn to_u16(&self) -> u16{
        self.to_i16().to_u16()
    }
    fn to_u32(&self) -> u32{
        self.to_i32().to_u32()
    }
    fn to_u64(&self) -> u64{
        self.to_i64().to_u64()
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        *self
    }
    fn to_f64(&self) -> f64{
        *self as f64
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleConv for f64{
    fn clampd(&self) -> f64 {
        let v = *self;
        if v > 1.0 {
            1.0
        } else if v < -1.0 {
            -1.0
        } else {
            v
        }
    }

    fn from(v: impl SampleConv) -> f64{
        v.to_f64()
    }
    fn to_i8(&self) -> i8{
        (self.clampd() * (i8::MAX as f64)) as i8
    } 
    fn to_i16(&self) -> i16{
        (self.clampd() * (i16::MAX) as f64) as i16
    }
    fn to_i32(&self) -> i32{
        (self.clampd() * (i32::MAX) as f64) as i32
    }
    fn to_i64(&self) -> i64{
        (self.clampd() * (i64::MAX) as f64) as i64
    }
    fn to_u8(&self) -> u8{
        self.to_i8().to_u8()
    } 
    fn to_u16(&self) -> u16{
        self.to_i16().to_u16()
    }
    fn to_u32(&self) -> u32{
        self.to_i32().to_u32()
    }
    fn to_u64(&self) -> u64{
        self.to_i64().to_u64()
    }
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    fn to_f32(&self) -> f32{
        *self as f32
    }
    fn to_f64(&self) -> f64{
        *self
    }
    fn read_le<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    fn read_be<T: Read>(r: &mut T) -> Result<Self, Error> where Self: Sized {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    fn write_le<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_le_bytes())
    }
    fn write_be<T: Write>(&self, w: &mut T) -> Result<(), Error> where Self: Sized {
        w.write_all(&self.to_be_bytes())
    }
}

