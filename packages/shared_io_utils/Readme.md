# Shared IO utilities

A utility to provide more convenient `Read` `Write` `Seek` `Debug` `Cursor` that could be shared, e.g. `SharedReader`, `SharedWriter`, `SharedReadWrite`, `DishonestReader` for modifying data using closures when being called `read()`, `CombinedReader` combines two readers but you can 'slice' the readers to make it only able to read parts of them, `CursorVecU8` have a better formatting behavior, `SharedCursor` shares a `CursorVecU8`, `MultistreamIO` allows you to switch its streams to read or write, `SharedMultistreamIO` shares the `MultistreamIO`. All of these 'shared' versions are used for the 3rd party library to read/write and you can capture the data or modify the data.

## Overview

Each of the traits/structs implemented `Read` or `Write`, and `Seek + Debug`.

The `Shared` things can be cloned everywhere, and everyone who owns it can share one specified reader or writer to manipulate the same file/stream.

The `CombinedReader` that combines two readers into one with the ability to `Read` and `Seek` and `Debug`, with `data offset` and `data length` for both readers, `seek()` is available. Convenient for reading across two readers, and not only the whole reader but also the specific part of the reader.

The `CursorVecU8` allows you to index/slice its inner `Vec<u8>` data, and its `Debug` implementation is friendly for your terminal size.

The `DishonestReader` allows you to modify data when the caller calls its `Read` trait function `read()` by specifying a closure `on_read()`.

The `MultistreamIO` allows you to add multiple readers/writers into it, and you can switch its current reader/writer. When its `Write` trait function `write()` is called, the data is written into your specified reader/writer. Use this to capture the 3rd-party libraries' outputs, and manipulate the data.

The `SharedMultistreamIO` makes `MultistreamIO` shared, so you can let your code and the 3rd-party code own it together.

```rust
/// * The `Reader` trait, `Read + Seek + Debug`
pub trait Reader: Read + Seek + Debug {}
impl<T> Reader for T where T: Read + Seek + Debug {}

/// * The `Writer` trait, `Write + Seek + Debug`
pub trait Writer: Write + Seek + Debug {}
impl<T> Writer for T where T: Write + Seek + Debug {}

/// * The `ReadWrite` trait, `Read + Write + Seek + Debug`
pub trait ReadWrite: Read + Write + Seek + Debug {}
impl<T> ReadWrite for T where T: Read + Write + Seek + Debug {}

pub struct SharedReader<T> (Rc<RefCell<T>>) where T: Read + Seek + Debug;
pub struct SharedWriter<T> (Rc<RefCell<T>>) where T: Write + Seek + Debug;
pub struct SharedReadWrite<T> (Rc<RefCell<T>>) where T: Read + Write + Seek + Debug;
pub struct SharedCursor(Rc<RefCell<CursorVecU8>>);

/// * Dishonest reader, a reader that reads data but modifies it.
pub struct DishonestReader<T>
where
    T: Read + Seek + Debug {
    reader: T,
    on_read: Box<dyn FnMut(&mut T, usize) -> Result<Vec<u8>, io::Error>>,
    on_seek: Box<dyn FnMut(&mut T, SeekFrom) -> io::Result<u64>>,
    cache: Vec<u8>,
}

/// * A Reader that combines two readers into one with the ability to `Read` and `Seek` and `Debug`
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
    total_length: u64,
}

pub struct CursorVecU8(Cursor<Vec<u8>>);

pub enum StreamType<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    /// * The `Read + Seek + Debug`
    Reader(R),

    /// * The `Write + Seek + Debug`
    Writer(W),

    /// * The `Read + Write + Seek + Debug`, better use it with the `tempfile()`
    ReadWrite(RW),

    /// * The `Read + Write + Seek + Debug` cursor.
    CursorU8(CursorVecU8)
}

/// * The `MultistreamIO<R, W, RW>` is for managing multiple IO objects.
/// * This thing itself implements `Read + Write + Seek + Debug`, when these traits methods are called, the selected stream is manipulated.
/// * by using this, you can control the 3rd party library to read or write data from/into different stream objects, and you can manipulate these data or streams.
pub struct MultistreamIO<R, W, RW>
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug {
    pub streams: Vec<StreamType<R, W, RW>>,
    pub cur_stream: usize,
}

pub struct SharedMultistreamIO<R, W, RW> (Rc<RefCell<MultistreamIO<R, W, RW>>>)
where
    R: Read + Seek + Debug,
    W: Write + Seek + Debug,
    RW: Read + Write + Seek + Debug;

/// * Go to an offset without using seek. It's achieved by using dummy reads.
pub fn goto_offset_without_seek<T>(
    mut reader: T,
    cur_pos: &mut u64,
    position: u64,
) -> io::Result<u64>
where
    T: Read;

/// * Copy data from a reader to a writer from the current position.
pub fn copy<R, W>(reader: &mut R, writer: &mut W, bytes_to_copy: u64) -> io::Result<()>
where
    R: Read,
    W: Write;
```