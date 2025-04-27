#![allow(dead_code)]

use std::{any::type_name, io::{Read, Write, Error}, mem::size_of, fmt::Debug, clone::Clone};
use std::ops::{Add, Sub, Mul, Div, AddAssign, SubAssign, MulAssign, DivAssign};
use std::ops::{BitAnd, BitOr, BitXor, Shl, Shr, BitAndAssign, BitOrAssign, BitXorAssign, ShlAssign, ShrAssign};
use std::ops::{Rem, RemAssign};
use std::ops::{Neg};

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct i24(pub u8, pub u8, pub u8); // 低中高

impl i24{
    #[inline(always)]
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0], bytes[1], bytes[2])
    }
    #[inline(always)]
    fn from_be_bytes(bytes: &[u8]) -> Self {
        Self(bytes[2], bytes[1], bytes[0])
    }
    #[inline(always)]
    fn to_le_bytes(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
    #[inline(always)]
    fn to_be_bytes(self) -> [u8; 3] {
        [self.2, self.1, self.0]
    }
    #[inline(always)]
    fn get_highest_i8(&self) -> i8 {
        self.2 as i8
    }
    #[inline(always)]
    fn as_i64(&self) -> i64 {
        i64::from_le_bytes([self.0, self.1, self.2, 0, 0, 0, 0, 0])
    }
    #[inline(always)]
    fn as_i32(&self) -> i32 {
        i32::from_le_bytes([self.0, self.1, self.2, 0])
    }
    #[inline(always)]
    fn as_i16(&self) -> i16 {
        i16::from_le_bytes([self.0, self.1])
    }
    #[inline(always)]
    fn as_i8(&self) -> i8 {
        self.0 as i8
    }
    #[inline(always)]
    fn as_u64(&self) -> u64 {
        u64::from_le_bytes([self.0, self.1, self.2, 0, 0, 0, 0, 0])
    }
    #[inline(always)]
    fn as_u32(&self) -> u32 {
        u32::from_le_bytes([self.0, self.1, self.2, 0])
    }
    #[inline(always)]
    fn as_u16(&self) -> u16 {
        u16::from_le_bytes([self.0, self.1])
    }
    #[inline(always)]
    fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Add for i24 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        (self.as_i32() + rhs.as_i32()).as_i24()
    }
}
impl Sub for i24 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        (self.as_i32() - rhs.as_i32()).as_i24()
    }
}
impl Mul for i24 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        (self.as_i32() * rhs.as_i32()).as_i24()
    }
}
impl Div for i24 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        (self.as_i32() / rhs.as_i32()).as_i24()
    }
}
impl AddAssign for i24 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs);
    }
}
impl SubAssign for i24 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.sub(rhs);
    }
}
impl MulAssign for i24 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}
impl DivAssign for i24 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(rhs);
    }
}
impl BitAnd for i24 {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0, self.1 & rhs.1, self.2 & rhs.2)
    }
}
impl BitOr for i24 {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0, self.1 | rhs.1, self.2 | rhs.2)
    }
}
impl BitXor for i24 {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0, self.1 ^ rhs.1, self.2 ^ rhs.2)
    }
}
impl Shl for i24 {
    type Output = Self;
    #[inline(always)]
    fn shl(self, rhs: Self) -> Self::Output {
        (self.as_i32() << rhs.as_i32()).as_i24()
    }
}
impl Shr for i24 {
    type Output = Self;
    #[inline(always)]
    fn shr(self, rhs: Self) -> Self::Output {
        (self.as_i32() >> rhs.as_i32()).as_i24()
    }
}
impl BitAndAssign for i24 {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.bitand(rhs);
    }
}
impl BitOrAssign for i24 {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.bitor(rhs);
    }
}
impl BitXorAssign for i24 {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = self.bitxor(rhs);
    }
}
impl ShlAssign for i24 {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: Self) {
        *self = self.shl(rhs);
    }
}
impl ShrAssign for i24 {
    #[inline(always)]
    fn shr_assign(&mut self, rhs: Self) {
        *self = self.shr(rhs);
    }
}
impl Rem for i24 {
    type Output = Self;
    #[inline(always)]
    fn rem(self, rhs: Self) -> Self::Output {
        (self.as_i32() % rhs.as_i32()).as_i24()
    }
}
impl RemAssign for i24 {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: Self) {
        *self = self.rem(rhs);
    }
}
impl Neg for i24 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output{
        (-self.as_i32()).as_i24()
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct u24(pub u8, pub u8, pub u8); // 低中高

impl u24{
    #[inline(always)]
    fn from_le_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0], bytes[1], bytes[2])
    }
    #[inline(always)]
    fn from_be_bytes(bytes: &[u8]) -> Self {
        Self(bytes[2], bytes[1], bytes[0])
    }
    #[inline(always)]
    fn to_le_bytes(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
    #[inline(always)]
    fn to_be_bytes(self) -> [u8; 3] {
        [self.2, self.1, self.0]
    }
    #[inline(always)]
    fn get_highest_u8(&self) -> u8 {
        self.2
    }
    #[inline(always)]
    fn as_i64(&self) -> i64 {
        i64::from_le_bytes([self.0, self.1, self.2, 0, 0, 0, 0, 0])
    }
    #[inline(always)]
    fn as_i32(&self) -> i32 {
        i32::from_le_bytes([self.0, self.1, self.2, 0])
    }
    #[inline(always)]
    fn as_i16(&self) -> i16 {
        i16::from_le_bytes([self.0, self.1])
    }
    #[inline(always)]
    fn as_i8(&self) -> i8 {
        self.0 as i8
    }
    #[inline(always)]
    fn as_u64(&self) -> u64 {
        u64::from_le_bytes([self.0, self.1, self.2, 0, 0, 0, 0, 0])
    }
    #[inline(always)]
    fn as_u32(&self) -> u32 {
        u32::from_le_bytes([self.0, self.1, self.2, 0])
    }
    #[inline(always)]
    fn as_u16(&self) -> u16 {
        u16::from_le_bytes([self.0, self.1])
    }
    #[inline(always)]
    fn as_u8(&self) -> u8 {
        self.0
    }
}

impl Add for u24 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        (self.as_u32() + rhs.as_u32()).as_u24()
    }
}
impl Sub for u24 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        (self.as_u32() - rhs.as_u32()).as_u24()
    }
}
impl Mul for u24 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self::Output {
        (self.as_u32() * rhs.as_u32()).as_u24()
    }
}
impl Div for u24 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self::Output {
        (self.as_u32() / rhs.as_u32()).as_u24()
    }
}
impl AddAssign for u24 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        *self = self.add(rhs);
    }
}
impl SubAssign for u24 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        *self = self.sub(rhs);
    }
}
impl MulAssign for u24 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}
impl DivAssign for u24 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: Self) {
        *self = self.div(rhs);
    }
}
impl BitAnd for u24 {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0, self.1 & rhs.1, self.2 & rhs.2)
    }
}
impl BitOr for u24 {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0, self.1 | rhs.1, self.2 | rhs.2)
    }
}
impl BitXor for u24 {
    type Output = Self;
    #[inline(always)]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0, self.1 ^ rhs.1, self.2 ^ rhs.2)
    }
}
impl Shl for u24 {
    type Output = Self;
    #[inline(always)]
    fn shl(self, rhs: Self) -> Self::Output {
        (self.as_u32() << rhs.as_u32()).as_u24()
    }
}
impl Shr for u24 {
    type Output = Self;
    #[inline(always)]
    fn shr(self, rhs: Self) -> Self::Output {
        (self.as_u32() >> rhs.as_u32()).as_u24()
    }
}
impl BitAndAssign for u24 {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.bitand(rhs);
    }
}
impl BitOrAssign for u24 {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.bitor(rhs);
    }
}
impl BitXorAssign for u24 {
    #[inline(always)]
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = self.bitxor(rhs);
    }
}
impl ShlAssign for u24 {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: Self) {
        *self = self.shl(rhs);
    }
}
impl ShrAssign for u24 {
    #[inline(always)]
    fn shr_assign(&mut self, rhs: Self) {
        *self = self.shr(rhs);
    }
}
impl Rem for u24 {
    type Output = Self;
    #[inline(always)]
    fn rem(self, rhs: Self) -> Self::Output {
        (self.as_u32() % rhs.as_u32()).as_u24()
    }
}
impl RemAssign for u24 {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: Self) {
        *self = self.rem(rhs);
    }
}

pub trait SampleType:
Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self> +
AddAssign + SubAssign + MulAssign + DivAssign +
Debug + Sized + Clone + Copy + 'static {
    type Longer;
    type Shorter;
    type Signed;
    type Unsigned;
    fn new() -> Self;
    fn from(v: impl SampleType) -> Self;
    fn average(s1: Self, s2: Self) -> Self;
    fn average_arr(arr: &[Self]) -> Self;
    fn to_i8 (&self) -> i8;
    fn to_i16(&self) -> i16;
    fn to_i32(&self) -> i32;
    fn to_i64(&self) -> i64;
    fn to_u8 (&self) -> u8;
    fn to_u16(&self) -> u16;
    fn to_u32(&self) -> u32;
    fn to_u64(&self) -> u64;
    fn to_i24(&self) -> i24;
    fn to_u24(&self) -> u24;
    fn to_f32(&self) -> f32;
    fn to_f64(&self) -> f64;
    fn as_i24(&self) -> i24;
    fn as_u24(&self) -> u24;
    fn to_longer(&self) -> Self::Longer;
    fn to_shorter(&self) -> Self::Shorter;
    fn is_signed(&self) -> bool;
    fn is_unsigned(&self) -> bool;
    fn is_integer(&self) -> bool;
    fn is_float(&self) -> bool;
    fn to_signed(&self) -> Self::Signed;
    fn to_unsigned(&self) -> Self::Unsigned {
        panic!("The type `{}` can't be turned to an unsigned type.", type_name::<Self>());
    }
    fn read_le<T>(r: &mut T) -> Result<Self, Error> where T: Read + ?Sized;
    fn read_be<T>(r: &mut T) -> Result<Self, Error> where T: Read + ?Sized;
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error> where T: Write + ?Sized;
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error> where T: Write + ?Sized;
}

pub trait SampleFrom: Debug + Sized + Clone + Copy + 'static {
    fn to(s: impl SampleType) -> Self;
}
impl SampleFrom for i8  {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_i8()  }}
impl SampleFrom for i16 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_i16() }}
impl SampleFrom for i24 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_i24() }}
impl SampleFrom for i32 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_i32() }}
impl SampleFrom for i64 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_i64() }}
impl SampleFrom for u8  {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_u8()  }}
impl SampleFrom for u16 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_u16() }}
impl SampleFrom for u24 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_u24() }}
impl SampleFrom for u32 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_u32() }}
impl SampleFrom for u64 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_u64() }}
impl SampleFrom for f32 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_f32() }}
impl SampleFrom for f64 {#[inline(always)] fn to(s: impl SampleType) -> Self { s.to_f64() }}

impl SampleType for i8{
    type Longer = i16;
    type Shorter = i8;
    type Signed = i8;
    type Unsigned = u8;
    #[inline(always)]
    fn new() -> Self {
        0i8
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> i8{
        v.to_i8()
    }
    #[inline(always)]
    fn average(s1: i8, s2: i8) -> i8 {
        ((s1 as i16 + s2 as i16) / 2) as i8
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: i16 = arr.iter().map(|s|{*s as i16}).sum();
        (sum / arr.len() as i16) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        *self
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.to_u8().to_u16().to_i16()
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.to_u8().to_u32().to_i32()
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_u8().to_u64().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        (*self as u8).wrapping_add(0x80)
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        self.to_u8().to_u16()
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_u16().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_u16().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        let lo = self.to_u8();
        i24::from_le_bytes(&[lo, lo, (*self) as u8])
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        let lo = self.to_u8();
        u24::from_le_bytes(&[lo, lo, lo])
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24(*self as u8, 0, 0)
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24(*self as u8, 0, 0)
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_i16()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_i8()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i8()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u8()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for i16{
    type Longer = i24;
    type Shorter = i8;
    type Signed = i16;
    type Unsigned = u16;
    #[inline(always)]
    fn new() -> Self {
        0i16
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> i16{
        v.to_i16()
    }
    #[inline(always)]
    fn average(s1: i16, s2: i16) -> i16 {
        ((s1 as i32 + s2 as i32) / 2) as i16
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: i32 = arr.iter().map(|s|{*s as i32}).sum();
        (sum / arr.len() as i32) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        (*self >> 8) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        *self
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.to_u16().to_u32().to_i32()
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_u16().to_u64().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.to_u16().to_u8()
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        (*self as u16).wrapping_add(0x8000)
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_u16().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_u16().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_u32().to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..2])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..2])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_i24()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_i8()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i16()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u16()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for i24 {
    type Shorter = i16;
    type Longer = i32;
    type Signed = i24;
    type Unsigned = u24;
    #[inline(always)]
    fn new() -> Self {
        Self::from_le_bytes(&[0, 0, 0])
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> i24{
        v.to_i24()
    }
    #[inline(always)]
    fn average(s1: i24, s2: i24) -> i24 {
        ((s1.as_i32() + s2.as_i32()) / 2).as_i24()
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: i32 = arr.iter().map(|s|{s.as_i32()}).sum();
        let bytes = (sum / arr.len() as i32).to_le_bytes();
        Self(bytes[0], bytes[1], bytes[2])
    }
    #[inline(always)]
    fn to_i8(&self) -> i8 {
        self.get_highest_i8()
    }
    #[inline(always)]
    fn to_i16(&self) -> i16{
        i16::from_le_bytes([self.1, self.2])
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        let hi = self.get_highest_i8().to_u8();
        i32::from_le_bytes([hi, self.0, self.1, self.2])
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        let hi = self.get_highest_i8().to_u8();
        i64::from_le_bytes([self.1, hi, self.0, self.1, hi, self.0, self.1, self.2])
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.get_highest_i8().to_u8()
    }
    #[inline(always)]
    fn to_u16(&self) -> u16{
        let hi = self.get_highest_i8().to_u8();
        u16::from_le_bytes([self.1, hi])
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        let hi = self.get_highest_i8().to_u8();
        u32::from_le_bytes([hi, self.0, self.1, hi])
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        let hi = self.get_highest_i8().to_u8();
        u64::from_le_bytes([self.1, hi, self.0, self.1, hi, self.0, self.1, hi])
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        *self
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        let hi = self.get_highest_i8().to_u8();
        u24::from_le_bytes(&[self.0, self.1, hi])
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        *self
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes())
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        self.to_i32().to_f32()
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        self.to_i64().to_f64()
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_i32()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_i16()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i24()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u24()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(&buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(&buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for i32{
    type Shorter = i16;
    type Longer = i64;
    type Signed = i32;
    type Unsigned = u32;
    #[inline(always)]
    fn new() -> Self {
        0i32
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> i32{
        v.to_i32()
    }
    #[inline(always)]
    fn average(s1: i32, s2: i32) -> i32 {
        ((s1 as i64 + s2 as i64) / 2) as i32
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: i64 = arr.iter().map(|s|{*s as i64}).sum();
        (sum / arr.len() as i64) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        (*self >> 24) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        (*self >> 16) as i16
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        *self
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_u64().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.to_u32().to_u8()
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        self.to_u32().to_u16()
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        (*self as u32).wrapping_add(0x80000000)
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_u32().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        let b = self.to_le_bytes();
        i24::from_le_bytes(&[b[1], b[2], b[3]])
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_i64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_i16()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i32()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u32()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for i64{
    type Shorter = i32;
    type Longer = i64;
    type Signed = i64;
    type Unsigned = u64;
    #[inline(always)]
    fn new() -> Self {
        0i64
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> i64{
        v.to_i64()
    }
    #[inline(always)]
    fn average(s1: i64, s2: i64) -> i64 {
        ((s1 as i128 + s2 as i128) / 2) as i64
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: i128 = arr.iter().map(|s|{*s as i128}).sum();
        (sum / arr.len() as i128) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        (*self >> 56) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        (*self >> 48) as i16
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        (*self >> 32) as i32
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        *self
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.to_u64().to_u8()
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        self.to_u64().to_u16()
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_u64().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        (*self as u64).wrapping_add(0x80000000_00000000)
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        let b = self.to_le_bytes();
        i24::from_le_bytes(&[b[5], b[6], b[7]])
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_i64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_i32()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i64()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u64()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for u8{
    type Shorter = u8;
    type Longer = u16;
    type Signed = i8;
    type Unsigned = u8;
    #[inline(always)]
    fn new() -> Self {
        0x80u8
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> u8{
        v.to_u8()
    }
    #[inline(always)]
    fn average(s1: u8, s2: u8) -> u8 {
        ((s1 as u16 + s2 as u16) / 2) as u8
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: u16 = arr.iter().map(|s|{*s as u16}).sum();
        (sum / arr.len() as u16) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        self.wrapping_sub(0x80) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.to_i8().to_i16()
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.to_i8().to_i32()
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_i8().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        *self
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        let v = self.to_u8() as u16;
        (v << 8) | v
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_u16().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_u32().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        let hi = self.to_i8() as u8;
        i24::from_le_bytes(&[*self, *self, hi])
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        u24::from_le_bytes(&[*self, *self, *self])
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24(*self, 0, 0)
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24(*self, 0, 0)
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_u16()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_u8()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i8()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u8()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for u16{
    type Shorter = u8;
    type Longer = u24;
    type Signed = i16;
    type Unsigned = u16;
    #[inline(always)]
    fn new() -> Self {
        0x8000u16
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> u16{
        v.to_u16()
    }
    #[inline(always)]
    fn average(s1: u16, s2: u16) -> u16 {
        ((s1 as u32 + s2 as u32) / 2) as u16
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: u32 = arr.iter().map(|s|{*s as u32}).sum();
        (sum / arr.len() as u32) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        self.to_i16().to_i8()
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.wrapping_sub(0x8000) as i16
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.to_i16().to_i32()
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_i16().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        (*self >> 8) as u8
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        *self
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        let v = *self as u32;
        (v << 16) | v
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_u32().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..2])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..2])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_u24()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_u8()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i16()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u16()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for u24 {
    type Shorter = u16;
    type Longer = u32;
    type Signed = i24;
    type Unsigned = u24;
    #[inline(always)]
    fn new() -> Self {
        Self::from_le_bytes(&[0x00, 0x00, 0x80])
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> u24{
        v.to_u24()
    }
    #[inline(always)]
    fn average(s1: u24, s2: u24) -> u24 {
        ((s1.as_u32() + s2.as_u32()) / 2).as_u24()
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: u32 = arr.iter().map(|s|{s.as_u32()}).sum();
        let bytes = (sum / arr.len() as u32).to_le_bytes();
        Self(bytes[0], bytes[1], bytes[2])
    }
    #[inline(always)]
    fn to_i8(&self) -> i8 {
        self.0.to_i8()
    }
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.to_i32().to_i16()
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        i32::from_le_bytes([self.2, self.0, self.1, self.2.to_i8() as u8])
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_i32().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.0
    }
    #[inline(always)]
    fn to_u16(&self) -> u16{
        u16::from_le_bytes([self.1, self.2])
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        u32::from_le_bytes([self.2, self.0, self.1, self.2])
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        u64::from_le_bytes([self.1, self.2, self.0, self.1, self.2, self.0, self.1, self.2])
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        i24::from_le_bytes(&[self.0, self.1, self.2.to_i8() as u8])
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        *self
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes())
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        *self
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        self.to_i32().to_f32()
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        self.to_i64().to_f64()
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_u32()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_u16()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i24()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u24()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(&buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(&buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for u32{
    type Shorter = u24;
    type Longer = u64;
    type Signed = i32;
    type Unsigned = u32;
    #[inline(always)]
    fn new() -> Self {
        0x80000000u32
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> u32{
        v.to_u32()
    }
    #[inline(always)]
    fn average(s1: u32, s2: u32) -> u32 {
        ((s1 as u64 + s2 as u64) / 2) as u32
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: u64 = arr.iter().map(|s|{*s as u64}).sum();
        (sum / arr.len() as u64) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        self.to_i32().to_i8()
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.to_i32().to_i16()
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.wrapping_sub(0x80000000) as i32
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.to_i32().to_i64()
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        (*self >> 24) as u8
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        (*self >> 16) as u16
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        *self
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        let v = *self as u64;
        (v << 32) | v
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_u64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_u24()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i32()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u32()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for u64{
    type Shorter = u32;
    type Longer = u64;
    type Signed = i64;
    type Unsigned = u64;
    #[inline(always)]
    fn new() -> Self {
        0x80000000_00000000u64
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> u64{
        v.to_u64()
    }
    #[inline(always)]
    fn average(s1: u64, s2: u64) -> u64 {
        ((s1 as u128 + s2 as u128) / 2) as u64
    }
    #[inline(always)]
    fn average_arr(arr: &[Self]) -> Self {
        let sum: u128 = arr.iter().map(|s|{*s as u128}).sum();
        (sum / arr.len() as u128) as Self
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        self.to_i64().to_i8()
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        self.to_i64().to_i16()
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        self.to_i64().to_i32()
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        self.wrapping_sub(0x80000000_00000000) as i64
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        (*self >> 56) as u8
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        (*self >> 48) as u16
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        (*self >> 32) as u32
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        *self
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        i24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        u24::from_le_bytes(&self.to_le_bytes()[..3])
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        (*self as f32) / (Self::MAX as f32)
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        (*self as f64) / (Self::MAX as f64)
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_u64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_u32()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        false
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_i64()
    }
    #[inline(always)]
    fn to_unsigned(&self) -> Self::Unsigned {
        self.to_u64()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for f32{
    type Shorter = f32;
    type Longer = f64;
    type Signed = f32;
    type Unsigned = f32;
    #[inline(always)]
    fn new() -> Self {
        0.0
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> f32{
        v.to_f32()
    }
    #[inline(always)]
    fn average(s1: f32, s2: f32) -> f32 {
        (s1 + s2) * 0.5
    }
    fn average_arr(arr: &[Self]) -> Self {
        let sum: f32 = arr.iter().map(|s|{*s as f32}).sum();
        sum / arr.len() as f32
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        (self.clamp(-1.0, 1.0) * (i8::MAX as f32)) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        (self.clamp(-1.0, 1.0) * (i16::MAX as f32)) as i16
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        (self.clamp(-1.0, 1.0) * (i32::MAX as f32)) as i32
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        (self.clamp(-1.0, 1.0) * (i64::MAX as f32)) as i64
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.to_i8().to_u8()
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        self.to_i16().to_u16()
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_i32().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_i64().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        (*self as i32).as_i24()
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        (*self as u32).as_u24()
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        *self
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        *self as f64
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_f64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_f32()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        true
    }
    #[inline(always)]
    fn to_signed(&self) -> Self::Signed {
        self.to_f32()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}

impl SampleType for f64{
    type Shorter = f32;
    type Longer = f64;
    type Signed = f64;
    type Unsigned = f64;
    #[inline(always)]
    fn new() -> Self {
        0.0
    }
    #[inline(always)]
    fn from(v: impl SampleType) -> f64{
        v.to_f64()
    }
    #[inline(always)]
    fn average(s1: f64, s2: f64) -> f64 {
        (s1 + s2) * 0.5
    }
    fn average_arr(arr: &[Self]) -> Self {
        let sum: f64 = arr.iter().map(|s|{*s as f64}).sum();
        sum / arr.len() as f64
    }
    #[inline(always)]
    fn to_i8(&self) -> i8{
        (self.clamp(-1.0, 1.0) * (i8::MAX as f64)) as i8
    } 
    #[inline(always)]
    fn to_i16(&self) -> i16{
        (self.clamp(-1.0, 1.0) * (i16::MAX) as f64) as i16
    }
    #[inline(always)]
    fn to_i32(&self) -> i32{
        (self.clamp(-1.0, 1.0) * (i32::MAX) as f64) as i32
    }
    #[inline(always)]
    fn to_i64(&self) -> i64{
        (self.clamp(-1.0, 1.0) * (i64::MAX) as f64) as i64
    }
    #[inline(always)]
    fn to_u8(&self) -> u8{
        self.to_i8().to_u8()
    } 
    #[inline(always)]
    fn to_u16(&self) -> u16{
        self.to_i16().to_u16()
    }
    #[inline(always)]
    fn to_u32(&self) -> u32{
        self.to_i32().to_u32()
    }
    #[inline(always)]
    fn to_u64(&self) -> u64{
        self.to_i64().to_u64()
    }
    #[inline(always)]
    fn to_i24(&self) -> i24{
        self.to_i32().to_i24()
    }
    #[inline(always)]
    fn to_u24(&self) -> u24{
        self.to_i24().to_u24()
    }
    #[inline(always)]
    fn as_i24(&self) -> i24{
        (*self as i32).as_i24()
    }
    #[inline(always)]
    fn as_u24(&self) -> u24{
        (*self as u32).as_u24()
    }
    #[inline(always)]
    fn to_f32(&self) -> f32{
        *self as f32
    }
    #[inline(always)]
    fn to_f64(&self) -> f64{
        *self
    }
    #[inline(always)]
    fn to_longer(&self) -> Self::Longer {
        self.to_f64()
    }
    #[inline(always)]
    fn to_shorter(&self) -> Self::Shorter {
        self.to_f32()
    }
    #[inline(always)]
    fn is_signed(&self) -> bool {
        true
    }
    #[inline(always)]
    fn is_unsigned(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_integer(&self) -> bool {
        false
    }
    #[inline(always)]
    fn is_float(&self) -> bool {
        true
    }
    fn to_signed(&self) -> Self::Signed {
        self.to_f64()
    }
    #[inline(always)]
    fn read_le<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_le_bytes(buf))
    }
    #[inline(always)]
    fn read_be<T>(r: &mut T) -> Result<Self, Error>
    where T: Read + ?Sized {
        let mut buf = [0u8; size_of::<Self>()];
        r.read_exact(&mut buf)?;
        Ok(Self::from_be_bytes(buf))
    }
    #[inline(always)]
    fn write_le<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_le_bytes())
    }
    #[inline(always)]
    fn write_be<T>(&self, w: &mut T) -> Result<(), Error>
    where T: Write + ?Sized {
        w.write_all(&self.to_be_bytes())
    }
}
