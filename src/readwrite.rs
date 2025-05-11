#![allow(dead_code)]

use std::{
    any::type_name,
    cmp::min,
    mem,
    fmt::{self, Debug, Display, Formatter},
    io::{self, Read, Seek, Write, Cursor, SeekFrom},
    rc::Rc,
    cell::RefCell,
    ops::{Deref, DerefMut}
};

/// * The `Reader` trait, `Read + Seek + Debug`
pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

/// * The `Writer` trait, `Write + Seek + Debug`
pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

/// * The `ReadWrite` trait, `Read + Write + Seek + Debug`
pub trait ReadWrite: Read + Write + Seek + Debug {}
impl<T> ReadWrite for T where T: Read + Write + Seek + Debug {}

/// * Encapsulated shared `Read + Seek + Debug`
#[derive(Debug)]
pub struct SharedReader<T> (Rc<RefCell<T>>) where T: Read + Seek + Debug;

impl<T> SharedReader<T>
where
    T: Read + Seek + Debug {
    pub fn new(reader: T) -> Self {
        Self(Rc::new(RefCell::new(reader)))
    }
}

impl<T> Read for SharedReader<T>
where
    T: Read + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

impl<T> Seek for SharedReader<T>
where
    T: Read + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl<T> Clone for SharedReader<T>
where
    T: Read + Seek + Debug {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// * Encapsulated shared `Write + Seek + Debug`
#[derive(Debug)]
pub struct SharedWriter<T> (Rc<RefCell<T>>) where T: Write + Seek + Debug;

impl<T> SharedWriter<T>
where
    T: Write + Seek + Debug {
    pub fn new(reader: T) -> Self {
        Self(Rc::new(RefCell::new(reader)))
    }
}

impl<T> Write for SharedWriter<T>
where
    T: Write + Seek + Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

impl<T> Seek for SharedWriter<T>
where
    T: Write + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl<T> Clone for SharedWriter<T>
where
    T: Write + Seek + Debug {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// * Encapsulated shared `Read + Write + Seek + Debug`
#[derive(Debug)]
pub struct SharedReadWrite<T> (Rc<RefCell<T>>) where T: Read + Write + Seek + Debug;

impl<T> SharedReadWrite<T>
where
    T: Read + Write + Seek + Debug {
    pub fn new(readwrite: T) -> Self {
        Self(Rc::new(RefCell::new(readwrite)))
    }
}

impl<T> Read for SharedReadWrite<T>
where
    T: Read + Write + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

impl<T> Write for SharedReadWrite<T>
where
    T: Read + Write + Seek + Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

impl<T> Seek for SharedReadWrite<T>
where
    T: Read + Write + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl<T> Clone for SharedReadWrite<T>
where
    T: Read + Write + Seek + Debug {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// * Dishonest reader, a reader that reads data but modifies it.
pub struct DishonestReader<T>
where
    T: Read + Seek + Debug {
    reader: T,
    on_read: Box<dyn FnMut(&mut T, usize) -> Result<Vec<u8>, io::Error>>,
    on_seek: Box<dyn FnMut(&mut T, SeekFrom) -> io::Result<u64>>,
    cache: Vec<u8>,
}

impl<T> DishonestReader<T>
where
    T: Read + Seek + Debug {
    pub fn new(
        reader: T,
        on_read: Box<dyn FnMut(&mut T, usize) -> Result<Vec<u8>, io::Error>>,
        on_seek: Box<dyn FnMut(&mut T, SeekFrom) -> io::Result<u64>>,
    ) -> Self {
        Self {
            reader,
            on_read,
            on_seek,
            cache: Vec::new(),
        }
    }
}

impl<T> Read for DishonestReader<T>
where
    T: Read + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let write_buf_and_cache = |data: &[u8], buf: &mut [u8], cache: &mut Vec<u8>| -> usize {
            let len = min(data.len(), buf.len());
            buf[..len].copy_from_slice(&data[..len]);
            if len < data.len() {
                *cache = data[len..].to_vec();
            } else {
                *cache = Vec::new();
            }
            len
        };
        if self.cache.is_empty() {
            match (self.on_read)(&mut self.reader, buf.len()) {
                Ok(data) => Ok(write_buf_and_cache(&data, buf, &mut self.cache)),
                Err(e) => Err(e),
            }
        } else {
            let to_write = self.cache.clone();
            Ok(write_buf_and_cache(&to_write, buf, &mut self.cache))
        }
    }
}

impl<T> Seek for DishonestReader<T>
where
    T: Read + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        (self.on_seek)(&mut self.reader, pos)
    }
}

impl<T> Debug for DishonestReader<T>
where
    T: Read + Seek + Debug {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let typename = type_name::<T>();
        f.debug_struct(&format!("DishonestReader<{typename}>"))
        .field("reader", &self.reader)
        .field("on_read", &format_args!("Box<dyn FnMut(&mut T, usize) -> Result<Vec<u8>, io::Error>>"))
        .field("on_seek", &format_args!("Box<dyn FnMut(&mut T, SeekFrom) -> io::Result<u64>>"))
        .field("cache", &format_args!("[u8; {}]", self.cache.len()))
        .finish()
    }
}

/// * A Reader that combines two readers into one with the ability to `Read` and `Seek` and `Debug`
#[derive(Debug)]
pub struct CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    first: R1,
    first_data_offset: u64,
    first_data_length: u64,
    second: R2,
    second_data_offset: u64,
    second_data_length: u64,
    stream_pos: u64,
}

impl<R1, R2> CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    pub fn new(
        mut first: R1,
        first_data_offset: u64,
        first_data_length: u64,
        mut second: R2,
        second_data_offset: u64,
        second_data_length: u64,
    ) -> Result<Self, io::Error> {
        first.seek(SeekFrom::Start(first_data_offset))?;
        second.seek(SeekFrom::Start(second_data_offset))?;
        Ok(Self {
            first,
            first_data_offset,
            first_data_length,
            second,
            second_data_offset,
            second_data_length,
            stream_pos: 0,
        })
    }
}

impl<R1, R2> Read for CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = (self.first_data_length + self.second_data_length) - self.stream_pos;
        if remaining == 0 {
            return Ok(0);
        }

        // Choose the reader to use
        let bytes_read = if self.stream_pos < self.first_data_length {
            let bytes_to_read = min((self.first_data_length - self.stream_pos) as usize, buf.len());
            let first_pos = self.stream_pos;
            self.first.seek(SeekFrom::Start(first_pos + self.first_data_offset))?;
            let n = self.first.read(&mut buf[..bytes_to_read])?;
            self.stream_pos += n as u64;
            n
        } else {
            let bytes_to_read = min((self.second_data_length - self.stream_pos) as usize, buf.len());
            let second_pos = self.stream_pos - self.first_data_length;
            self.second.seek(SeekFrom::Start(second_pos + self.second_data_offset))?;
            let n = self.second.read(&mut buf[..bytes_to_read])?;
            self.stream_pos += n as u64;
            n
        };

        Ok(bytes_read)
    }
}

impl<R1, R2> Seek for CombinedReader<R1, R2>
where
    R1: Reader,
    R2: Reader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                let total_len = self.first_data_length.checked_add(self.second_data_length).ok_or(io::ErrorKind::InvalidInput)?;
                if offset > 0 {
                    total_len.checked_add(offset as u64)
                } else {
                    total_len.checked_sub((-offset) as u64)
                }.ok_or(io::ErrorKind::InvalidInput)?
            }
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.stream_pos.checked_add(offset as u64)
                } else {
                    self.stream_pos.checked_sub((-offset) as u64)
                }.ok_or(io::ErrorKind::InvalidInput)?
            }
        };

        let total_len = self.first_data_length + self.second_data_length;
        if new_pos > total_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek position out of bounds"
            ));
        }

        self.stream_pos = new_pos;
        Ok(new_pos)
    }
}

/// * A better `Cursor<Vec<u8>>` which has a friendlier `Debug` trait implementation
#[derive(Clone)]
pub struct CursorVecU8(Cursor<Vec<u8>>);

impl CursorVecU8 {
    pub fn new(data: Vec<u8>) -> Self {
        Self(Cursor::new(data))
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0.into_inner()
    }
}

impl Default for CursorVecU8 {
    fn default() -> Self {
        Self(Cursor::new(Vec::new()))
    }
}

impl From<Cursor<Vec<u8>>> for CursorVecU8 {
    fn from(cursor: Cursor<Vec<u8>>) -> Self {
        Self(cursor)
    }
}

impl Into<Cursor<Vec<u8>>> for CursorVecU8 {
    fn into(self) -> Cursor<Vec<u8>> {
        self.0
    }
}

impl Deref for CursorVecU8 {
    type Target = Cursor<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CursorVecU8 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Read for CursorVecU8 {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for CursorVecU8 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Seek for CursorVecU8 {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.seek(pos)
    }
}

impl Debug for CursorVecU8 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Cursor")
        .field("inner", &format_args!("[u8; {}]", self.0.get_ref().len()))
        .field("pos", &self.0.position())
        .finish()
    }
}

impl Display for CursorVecU8 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}

/// * The shared `Cursor`.
/// * Because it's shared, when the 3rd library owned it, we still can access to it..
#[derive(Debug)]
pub struct SharedCursor (Rc<RefCell<CursorVecU8>>);

impl SharedCursor {
    pub fn new() -> Self {
        Self::default()
    }

    /// * Get the inner data size
    pub fn len(&self) -> usize {
        self.0.borrow().get_ref().len()
    }

    /// * Get the inner data as `Vec<u8>`
    pub fn get_vec(&self) -> Vec<u8> {
        self.0.borrow().get_ref().to_vec()
    }

    /// * Discard current inner data, replace it with new data, and set the read/write position to the end of the data
    pub fn set_vec(&mut self, data: &[u8], rw_pos: u64) {
        let mut new_cursor = CursorVecU8::new(data.to_vec());
        new_cursor.set_position(rw_pos);
        *self.0.borrow_mut() = new_cursor;
    }

    /// * Discard the inner data, set the read/write position to 0
    pub fn clear(&mut self) {
        *self.0.borrow_mut() = CursorVecU8::default();
    }
}

impl Default for SharedCursor {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(CursorVecU8::default())))
    }
}

impl Read for SharedCursor {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

impl Write for SharedCursor {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

impl Seek for SharedCursor {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl Clone for SharedCursor {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub enum StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    Reader(R),
    Writer(W),
    ReadWrite(RW),
    CursorU8(CursorVecU8)
}

impl<R, W, RW> StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {

    pub fn as_reader(&mut self) -> &mut R {
        let name_r = type_name::<R>();
        let name_w = type_name::<W>();
        let name_rw = type_name::<RW>();
        match self {
            Self::Reader(reader) => reader,
            o => panic!("The `StreamType<{name_r}, {name_w}, {name_rw}>` is {:?}", o),
        }
    }

    pub fn as_writer(&mut self) -> &mut W {
        let name_r = type_name::<R>();
        let name_w = type_name::<W>();
        let name_rw = type_name::<RW>();
        match self {
            Self::Writer(writer) => writer,
            o => panic!("The `StreamType<{name_r}, {name_w}, {name_rw}>` is {:?}", o),
        }
    }

    pub fn as_readwrite(&mut self) -> &mut RW {
        let name_r = type_name::<R>();
        let name_w = type_name::<W>();
        let name_rw = type_name::<RW>();
        match self {
            Self::ReadWrite(readwrite) => readwrite,
            o => panic!("The `StreamType<{name_r}, {name_w}, {name_rw}>` is {:?}", o),
        }
    }

    pub fn as_cursor(&mut self) -> &mut CursorVecU8 {
        let name_r = type_name::<R>();
        let name_w = type_name::<W>();
        let name_rw = type_name::<RW>();
        match self {
            Self::CursorU8(cursor) => cursor,
            o => panic!("The `StreamType<{name_r}, {name_w}, {name_rw}>` is {:?}", o),
        }
    }

    pub fn take_cursor_data(&mut self) -> Vec<u8> {
        let name_r = type_name::<R>();
        let name_w = type_name::<W>();
        let name_rw = type_name::<RW>();
        match self {
            Self::CursorU8(cursor) => {
                mem::take(cursor).into_inner()
            }
            o => panic!("The `StreamType<{name_r}, {name_w}, {name_rw}>` is {:?}", o),
        }
    }
}

impl<R, W, RW> Read for StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Reader(reader) => {
                reader.read(buf)
            }
            Self::ReadWrite(readwrite) => {
                readwrite.read(buf)
            }
            Self::CursorU8(cursor) => {
                cursor.read(buf)
            }
            Self::Writer(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "`StreamType::Writer()` can't read.")),
        }
    }
}

impl<R, W, RW> Write for StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Writer(writer) => {
                writer.write(buf)
            }
            Self::ReadWrite(readwrite) => {
                readwrite.write(buf)
            }
            Self::CursorU8(cursor) => {
                cursor.write(buf)
            }
            Self::Reader(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "`StreamType::Reader()` can't write.")),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Writer(writer) => {
                writer.flush()
            }
            Self::ReadWrite(readwrite) => {
                readwrite.flush()
            }
            Self::CursorU8(cursor) => {
                cursor.flush()
            }
            Self::Reader(_) => Err(io::Error::new(io::ErrorKind::Unsupported, "`StreamType::Reader()` can't flush.")),
        }
    }
}

impl<R, W, RW> Seek for StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Reader(reader) => {
                reader.seek(pos)
            }
            Self::Writer(writer) => {
                writer.seek(pos)
            }
            Self::ReadWrite(readwrite) => {
                readwrite.seek(pos)
            }
            Self::CursorU8(cursor) => {
                cursor.seek(pos)
            }
        }
    }
}

#[derive(Debug)]
pub struct MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    pub streams: Vec<StreamType<R, W, RW>>,
    pub cur_stream: usize,
}

impl<R, W, RW> Default for MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn default() -> Self {
        Self {
            streams: Vec::new(),
            cur_stream: 0,
        }
    }
}

impl<R, W, RW> MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_cur_stream(&self) -> &StreamType<R, W, RW> {
        &self.streams[self.cur_stream]
    }

    pub fn get_cur_stream_mut(&mut self) -> &mut StreamType<R, W, RW> {
        &mut self.streams[self.cur_stream]
    }

    pub fn get_stream(&self, index: usize) -> &StreamType<R, W, RW> {
        &self.streams[index]
    }

    pub fn get_stream_mut(&mut self, index: usize) -> &mut StreamType<R, W, RW> {
        &mut self.streams[index]
    }

    pub fn push_stream(&mut self, stream: StreamType<R, W, RW>) {
        self.streams.push(stream);
    }

    pub fn pop_stream(&mut self) -> Option<StreamType<R, W, RW>> {
        self.streams.pop()
    }

    pub fn set_stream(&mut self, index: usize) {
        self.cur_stream = index;
    }
}

impl<R, W, RW> Read for MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.get_cur_stream_mut().read(buf)
    }
}

impl<R, W, RW> Write for MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_cur_stream_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.get_cur_stream_mut().flush()
    }
}

impl<R, W, RW> Seek for MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.get_cur_stream_mut().seek(pos)
    }
}

/// * The shared version of the `MultistreamIO`.
/// * Because it's shared, when the 3rd library owned it, we still can access to it, e.g. switch it to a cursor stream to capture some data.
#[derive(Debug)]
pub struct SharedMultistreamIO<R, W, RW> (Rc<RefCell<MultistreamIO<R, W, RW>>>)
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug;

impl<R, W, RW> SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    pub fn new(writer_with_cursor: MultistreamIO<R, W, RW>) -> Self {
        Self(Rc::new(RefCell::new(writer_with_cursor)))
    }
}

impl<R, W, RW> Deref for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    type Target = MultistreamIO<R, W, RW>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.0.as_ptr() as *const MultistreamIO<R, W, RW>) }
    }
}

impl<R, W, RW> DerefMut for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

impl<R, W, RW> Read for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

impl<R, W, RW> Write for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().flush()
    }
}

impl<R, W, RW> Seek for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.0.borrow_mut().seek(pos)
    }
}

impl<R, W, RW> Clone for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R, W, RW> Default for SharedMultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(MultistreamIO::new())))
    }
}

/// * Go to an offset without using seek. It's achieved by using dummy reads.
pub fn goto_offset_without_seek<T>(
    mut reader: T,
    cur_pos: &mut u64,
    position: u64,
) -> io::Result<u64>
where
    T: Read,
{
    const SKIP_SIZE: u64 = 1024;
    let mut skip_buf = [0u8; SKIP_SIZE as usize];
    while *cur_pos + SKIP_SIZE <= position {
        reader.read_exact(&mut skip_buf)?;
        *cur_pos += SKIP_SIZE;
    }
    if *cur_pos < position {
        let mut skip_buf = vec![0u8; (position - *cur_pos) as usize];
        reader.read_exact(&mut skip_buf)?;
        *cur_pos = position;
    }
    if *cur_pos > position {
        Err(io::Error::new(
            io::ErrorKind::NotSeekable,
            format!(
                "The current position {cur_pos} has already exceeded the target position {position}"
            ),
        ))
    } else {
        Ok(*cur_pos)
    }
}

/// * Copy data from a reader to a writer from the current position.
pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> io::Result<()>
where
    R: Read,
    W: Write,
{
    const BUFFER_SIZE: u64 = 1024;
    let mut buf = vec![0u8; BUFFER_SIZE as usize];
    let mut to_copy = bytes_to_copy;
    while to_copy >= BUFFER_SIZE {
        reader.read_exact(&mut buf)?;
        writer.write_all(&buf)?;
        to_copy -= BUFFER_SIZE;
    }
    if to_copy > 0 {
        buf.resize(to_copy as usize, 0);
        reader.read_exact(&mut buf)?;
        writer.write_all(&buf)?;
    }
    Ok(())
}

/// * This is for read/write strings from/to file with specific encoding and size, or read/write as NUL-terminated strings.
pub mod string_io {
    use crate::savagestr::{SavageStringCodecs, StringCodecMaps};
    use std::io::{self, Read, Write};

    /// * Read some bytes, and return the bytes, without you to create a local `vec![0u8; size]` and scratch your head with the messy codes
    pub fn read_bytes<T: Read>(r: &mut T, size: usize) -> Result<Vec<u8>, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// * Read a fixed-size string and decode it using the `StringCodecMaps`
    pub fn read_str<T: Read>(
        r: &mut T,
        size: usize,
        text_encoding: &StringCodecMaps,
    ) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding
            .decode(&buf)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a fixed-size string and decode it using the `StringCodecMaps` while you can specify the code page.
    pub fn read_str_by_code_page<T: Read>(
        r: &mut T,
        size: usize,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> Result<String, io::Error> {
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        Ok(text_encoding
            .decode_bytes_by_code_page(&buf, code_page)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a NUL terminated string by raw, not decode it.
    pub fn read_sz_raw<T: Read>(r: &mut T) -> Result<Vec<u8>, io::Error> {
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
        Ok(buf)
    }

    /// * Read a NUL terminated string and decode it.
    pub fn read_sz<T: Read>(
        r: &mut T,
        text_encoding: &StringCodecMaps,
    ) -> Result<String, io::Error> {
        Ok(text_encoding
            .decode(&read_sz_raw(r)?)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Read a NUL terminated string and decode it with the specified code page.
    pub fn read_sz_by_code_page<T: Read>(
        r: &mut T,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> Result<String, io::Error> {
        Ok(text_encoding
            .decode_bytes_by_code_page(&read_sz_raw(r)?, code_page)
            .trim_matches(char::from(0))
            .to_string())
    }

    /// * Write a fixed-size encoded string.
    pub fn write_str_sized<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        size: usize,
        text_encoding: &StringCodecMaps,
    ) -> io::Result<()> {
        let mut data = text_encoding.encode(data);
        data.resize(size, 0);
        w.write_all(&data)?;
        Ok(())
    }

    /// * Write an encoded string.
    pub fn write_str<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        text_encoding: &StringCodecMaps,
    ) -> io::Result<()> {
        let data = text_encoding.encode(data);
        w.write_all(&data)?;
        Ok(())
    }

    /// * Write an encoded string encoded with the specified code page.
    pub fn write_str_by_code_page<T: Write + ?Sized>(
        w: &mut T,
        data: &str,
        text_encoding: &StringCodecMaps,
        code_page: u32,
    ) -> io::Result<()> {
        let data = text_encoding.encode_strings_by_code_page(data, code_page);
        w.write_all(&data)?;
        Ok(())
    }
}
