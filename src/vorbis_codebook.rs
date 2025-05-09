use std::io::{Seek, Write, Cursor, SeekFrom};
use crate::{AudioReadError, AudioWriteError};
use crate::format_array;

const SHOW_DEBUG: bool = true;
macro_rules! debugln {
    () => {
        if SHOW_DEBUG {
            println!("");
        }
    };
    ($($arg:tt)*) => {
        if SHOW_DEBUG {
            println!($($arg)*);
        }
    };
}

const MASK: [u32; 33] = [
    0x00000000,
    0x00000001, 0x00000003, 0x00000007, 0x0000000f,
    0x0000001f, 0x0000003f, 0x0000007f, 0x000000ff,
    0x000001ff, 0x000003ff, 0x000007ff, 0x00000fff,
    0x00001fff, 0x00003fff, 0x00007fff, 0x0000ffff,
    0x0001ffff, 0x0003ffff, 0x0007ffff, 0x000fffff,
    0x001fffff, 0x003fffff, 0x007fffff, 0x00ffffff,
    0x01ffffff, 0x03ffffff, 0x07ffffff, 0x0fffffff,
    0x1fffffff, 0x3fffffff, 0x7fffffff, 0xffffffff
];

fn ilog(mut v: u32) -> i32 {
    let mut ret = 0;
    while v != 0 {
        v >>= 1;
        ret += 1;
    }
    ret
}

/// * BitReader: read vorbis data bit by bit
pub struct BitReader<'a> {
    /// * Currently ends at which bit in the last byte
    pub endbit: i32,

    /// * How many bits did we read in total
    pub total_bits: usize,

    /// * Borrowed a slice of data
    pub data: &'a [u8],

    /// * Current byte index
    pub cursor: usize,
}

impl<'a> BitReader<'a> {
    /// * `data` is decapsulated from the Ogg stream
    /// * `cursor` is the read position of the `BitReader`
    /// * Pass `data` as a slice that begins from the part you want to read,
    ///   Then you'll get the `cursor` to indicate how many bytes this part of data takes.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            endbit: 0,
            total_bits: 0,
            cursor: 0,
            data,
        }
    }

    /// * Read data bit by bit
    /// * bits <= 32
    pub fn read(&mut self, mut bits: i32) -> Result<i32, AudioReadError> {
        if bits < 0 || bits > 32 {
            return Err(AudioReadError::InvalidArguments(format!("Invalid bit number: {bits}")));
        }
        let mut ret: i32;
        let m = MASK[bits as usize];
        let origbits = bits;
        let cursor = self.cursor;

        // Don't want it panic, and don't want an Option.
        let ptr_index = |mut index: usize| -> Result<u8, AudioReadError> {
            index += cursor;
            let eof_err = || -> AudioReadError {
                AudioReadError::UnexpectedEof(format!("UnexpectedEof when trying to read {origbits} bits from the input position 0x{:x}", index))
            };
            self.data.get(index).ok_or(eof_err()).copied()
        };

        bits += self.endbit;
        if bits == 0 {
            return Ok(0);
        }

        ret = (ptr_index(0)? as i32) >> self.endbit;
        if bits > 8 {
            ret |= (ptr_index(1)? as i32) << (8 - self.endbit);
            if bits > 16 {
                ret |= (ptr_index(2)? as i32) << (16 - self.endbit);
                if bits > 24 {
                    ret |= (ptr_index(3)? as i32) << (24 - self.endbit);
                    if bits > 32 && self.endbit != 0 {
                        ret |= (ptr_index(4)? as i32) << (32 - self.endbit);
                    }
                }
            }
        }
        ret &= m as i32;
        self.cursor += (bits / 8) as usize;
        self.endbit = bits & 7;
        self.total_bits += origbits as usize;
        Ok(ret)
    }
}

/// * BitWriter: write vorbis data bit by bit
pub struct BitWriter {
    /// * Currently ends at which bit in the last byte
    pub endbit: i32,

    /// * How many bits did we wrote in total
    pub total_bits: usize,

    /// * We owns the written data
    pub cursor: Cursor<Vec<u8>>,
}

impl Default for BitWriter {
    fn default() -> Self {
        // We must have at least one byte written here because we have to add bits to the last byte.
        let mut cursor = Cursor::new(vec![0]);
        cursor.seek(SeekFrom::End(0)).unwrap();
        Self {
            endbit: 0,
            total_bits: 0,
            cursor,
        }
    }
}

impl BitWriter {
    /// * Create a `Cursor<Vec<u8>>` to write
    pub fn new() -> Self {
        Self::default()
    }

    /// * Get the last byte for modifying it
    pub fn last_byte(&mut self) -> &mut u8 {
        let v = self.cursor.get_mut();
        let len = v.len();
        &mut v[len - 1]
    }

    /// * Write data by bytes one by one
    fn write_byte(&mut self, byte: u8) -> Result<(), AudioWriteError> {
        self.cursor.write_all(&[byte])?;
        Ok(())
    }

    /// * Write data in bits, max is 32 bit.
    pub fn write(&mut self, mut value: u32, mut bits: i32) -> Result<(), AudioWriteError> {
        if bits < 0 || bits > 32 {
            return Err(AudioWriteError::InvalidArguments(format!("Invalid bits {bits}")));
        }
        value &= MASK[bits as usize];
        let origbits = bits;
        bits += self.endbit;

        *self.last_byte() |= (value << self.endbit) as u8;

        if bits >= 8 {
            self.write_byte((value >> (8 - self.endbit)) as u8)?;
            if bits >= 16 {
                self.write_byte((value >> (16 - self.endbit)) as u8)?;
                if bits >= 24 {
                    self.write_byte((value >> (24 - self.endbit)) as u8)?;
                    if bits >= 32 {
                        if self.endbit != 0 {
                            self.write_byte((value >> (32 - self.endbit)) as u8)?;
                        } else {
                            self.write_byte(0)?;
                        }
                    }
                }
            }
        }

        self.endbit = bits & 7;
        self.total_bits += origbits as usize;
        Ok(())
    }

    /// * Get the inner byte array and consumes the writer.
    pub fn to_bytes(self) -> Vec<u8> {
        self.cursor.into_inner()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CodeBook {
    pub dim: u16,
    pub entries: u32,
    pub lengthlist: Vec<i8>,
    pub maptype: u32,
    pub q_min: isize,
    pub q_delta: isize,
    pub q_quant: i32,
    pub q_sequencep: i32,
    pub quantlist: Vec<i16>,
}

impl Default for CodeBook {
    fn default() -> Self {
        Self {
            dim: 0,
            entries: 0,
            lengthlist: Vec::new(),
            maptype: 0,
            q_min: 0,
            q_delta: 0,
            q_quant: 0,
            q_sequencep: 0,
            quantlist: Vec::new(),
        }
    }
}

impl Debug for CodeBook {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("CodeBook")
        .field("dim", &self.dim)
        .field("entries", &self.entries)
        .field("lengthlist", &format_args!("[{}]", format_array!(self.lengthlist, " ", "0x{:02x}")))
        .field("maptype", &self.maptype)
        .field("q_min", &self.q_min)
        .field("q_delta", &self.q_delta)
        .field("q_quant", &self.q_quant)
        .field("q_sequencep", &self.q_sequencep)
        .field("quantlist", &format_args!("[{}]", format_array!(self.quantlist, " ", "0x{:04x}")))
        .finish()
    }
}

impl CodeBook {
    /// unpacks a codebook from the packet buffer into the codebook struct,
    /// readies the codebook auxiliary structures for decode
    pub fn read(bitreader: &mut BitReader) -> Result<Self, AudioReadError> {
        let mut ret = Self::default();
        ret.parse_book(bitreader)?;
        Ok(ret)
    }

    fn parse_book(&mut self, bitreader: &mut BitReader) -> Result<(), AudioReadError> {
        /* make sure alignment is correct */
        if bitreader.read(24)? != 0x564342 {
            return Err(AudioReadError::FormatError("Check the `BCV` flag failed.".to_string()));
        }
        /* first the basic parameters */
        let dim = bitreader.read(16)? as i32;
        let entries = bitreader.read(24)? as i32;
        if ilog(dim as u32) + ilog(entries as u32) > 24 {
            return Err(AudioReadError::FormatError(format!("{} + {} > 24", ilog(dim as u32), ilog(entries as u32))));
        }
        self.dim = dim as u16;
        self.entries = entries as u32;
        /* codeword ordering.... length ordered or unordered? */
        match bitreader.read(1)? {
            0 => {
                debugln!("  unordered");

                /* allocated but unused entries? */
                let unused = if bitreader.read(1)? != 0 {true} else {false};

                /* unordered */
                self.lengthlist.resize(self.entries as usize, 0);
                /* allocated but unused entries? */
                if unused {
                    /* yes, unused entries */
                    debugln!("  with unused entries");

                    for i in 0..self.entries as usize {
                        if bitreader.read(1)? != 0 {
                            let num = bitreader.read(5)? as i8;
                            self.lengthlist[i] = num + 1;
                        } else {
                            self.lengthlist[i] = 0;
                        }
                    }
                } else { /* all entries used; no tagging */
                    for i in 0..self.entries as usize {
                        let num = bitreader.read(5)? as i8;
                        self.lengthlist[i] = num + 1;
                    }
                }
            }
            1 => { /* ordered */
                debugln!("  ordered");

                let mut length = (bitreader.read(5)? + 1) as i8;
                self.lengthlist.resize(self.entries as usize, 0);
                let mut i = 0;
                while i < self.entries {
                    let num = bitreader.read(ilog((self.entries - i) as u32))? as u32;
                    if length > 32 || num > self.entries - i || (num > 0 && (num - 1) >> (length - 1) > 1) {
                        return Err(AudioReadError::FormatError(format!("length({length}) > 32 || num({num}) > entries({}) - i({i}) || (num({num}) > 0 && (num({num}) - 1) >> (length({length}) - 1) > 1)", self.entries)));
                    }
                    for _ in 0..num {
                        self.lengthlist[i as usize] = length;
                        i += 1;
                    }
                    length += 1;
                }
            }
            o => return Err(AudioReadError::FormatError(format!("Unexpected codeword ordering {o}"))),
        }

        debugln!("  lengthlist: [{}]", format_array!(&self.lengthlist, ", ", "{:02}"));

        /* Do we have a mapping to unpack? */
        self.maptype = bitreader.read(4)? as u32;
        debugln!("  maptype: {}", self.maptype);
        match self.maptype {
            0 => (),
            1 | 2 => {
                /* implicitly populated value mapping */
                /* explicitly populated value mapping */
                self.q_min = bitreader.read(32)? as isize;
                self.q_delta = bitreader.read(32)? as isize;
                self.q_quant = bitreader.read(4)? + 1;
                self.q_sequencep = bitreader.read(1)?;

                debugln!("    q_min: {}", self.q_min);
                debugln!("    q_delta: {}", self.q_delta);
                debugln!("    q_quant: {}", self.q_quant);
                debugln!("    q_sequencep: {}", self.q_sequencep);

                let quantvals = match self.maptype {
                    1 => if self.dim == 0 {0} else {self.book_maptype1_quantvals() as usize},
                    2 => self.entries as usize * self.dim as usize,
                    _ => unreachable!(),
                };

                debugln!("    quantvals: {quantvals}");

                /* quantized values */
                self.quantlist.resize(quantvals, 0);
                for i in 0..quantvals {
                    self.quantlist[i] = bitreader.read(self.q_quant)? as i16;
                }

                debugln!("    quantlist: [{}]", format_array!(&self.quantlist, ", ", "0x{:04}"));
            }
            o => return Err(AudioReadError::FormatError(format!("Unexpected maptype {o}"))),
        }
        Ok(())
    }

    /// there might be a straightforward one-line way to do the below
    /// that's portable and totally safe against roundoff, but I haven't
    /// thought of it.  Therefore, we opt on the side of caution
    fn book_maptype1_quantvals(&self) -> i32 {
        if self.entries < 1 {
            return 0;
        }
        let entries = self.entries as i32;
        let dim = self.dim as i32;
        let mut vals: i32 = (entries as f32).powf(1.0 / (dim as f32)).floor() as i32;
        /* the above *should* be reliable, but we'll not assume that FP is
           ever reliable when bitstream sync is at stake; verify via integer
           means that vals really is the greatest value of dim for which
           vals^b->bim <= b->entries */
        /* treat the above as an initial guess */
        use std::cmp::max;
        vals = max(vals, 1);
        loop {
            let mut acc = 1i32;
            let mut acc1 = 1i32;
            let mut i = 0i32;
            while i < dim {
                if entries / vals < acc {
                    break;
                }
                acc *= vals;
                if i32::MAX / (vals + 1) < acc1 {
                    acc1 = i32::MAX;
                } else {
                    acc1 *= vals + 1;
                }
                i += 1;
            }
            if i >= dim && acc <= entries && acc1 > entries {
                return vals;
            } else {
                if i < dim || acc > entries {
                    vals -= 1;
                } else {
                    vals += 1;
                }
            }
        }
    }

    /// * Pack the book into the bitstream
    pub fn write(&self, bitwriter: &mut BitWriter) -> Result<(), AudioWriteError> {
        /* first the basic parameters */
        bitwriter.write(0x564342, 24)?;
        bitwriter.write(self.dim as u32, 16)?;
        bitwriter.write(self.entries, 24)?;

        /* pack the codewords.  There are two packings; length ordered and
           length random.  Decide between the two now. */

        let mut ordered = false;
        for i in 1..self.entries as usize {
            if self.lengthlist[i - 1] == 0 || self.lengthlist[i] < self.lengthlist[i - 1] {
                if i == self.entries as usize {
                    ordered = true;
                }
                break;
            }
        }

        if ordered {
            /* length ordered.  We only need to say how many codewords of
               each length.  The actual codewords are generated
               deterministically */
            let mut count = 0u32;
            bitwriter.write(1, 1)?; /* ordered */
            bitwriter.write(self.lengthlist[0] as u32 - 1, 5)?;

            for i in 1..self.entries as usize {
                let this = self.lengthlist[i];
                let last = self.lengthlist[i - 1];
                if this > last {
                    for _ in last..this {
                        bitwriter.write(i as u32 - count, ilog(self.entries - count))?;
                        count = i as u32;
                    }
                }
            }
            bitwriter.write(self.entries - count, ilog(self.entries - count))?;
        } else {
            /* length random.  Again, we don't code the codeword itself, just
               the length.  This time, though, we have to encode each length */
            bitwriter.write(0, 1)?; /* unordered */

            /* algortihmic mapping has use for 'unused entries', which we tag
               here.  The algorithmic mapping happens as usual, but the unused
               entry has no codeword. */
            let mut i = 0u32;
            while i < self.entries {
                if self.lengthlist[i as usize] == 0 {
                    break;
                }
                i += 1;
            }

            if i == self.entries {
                bitwriter.write(0, 1)?; /* no unused entries */
                for i in 0..self.entries as usize {
                    bitwriter.write(self.lengthlist[i] as u32 - 1, 5)?;
                }
            } else {
                bitwriter.write(1, 1)?; /* we have unused entries; thus we tag */
                for i in 0..self.entries as usize {
                    if self.lengthlist[i] == 0 {
                        bitwriter.write(0, 1)?;
                    } else {
                        bitwriter.write(1, 1)?;
                        bitwriter.write(self.lengthlist[i] as u32 - 1, 5)?;
                    }
                }
            }
        }

        /* is the entry number the desired return value, or do we have a
           mapping? If we have a mapping, what type? */
        bitwriter.write(self.maptype, 4)?;
        match self.maptype {
            0 => (),
            1 | 2 => {
                if self.quantlist.is_empty() {
                    return Err(AudioWriteError::MissingData("Missing quantlist data".to_string()));
                }

                bitwriter.write(self.q_min as u32, 32)?;
                bitwriter.write(self.q_delta as u32, 32)?;
                bitwriter.write((self.q_quant - 1) as u32, 4)?;
                bitwriter.write(self.q_sequencep as u32, 1)?;

                let quantvals = match self.maptype {
                    1 => self.book_maptype1_quantvals() as usize,
                    2 => self.entries as usize * self.dim as usize,
                    _ => unreachable!(),
                };

                for i in 0..quantvals {
                    bitwriter.write(self.quantlist[i].abs() as u32, self.q_quant)?;
                }
            }
            o => return Err(AudioWriteError::InvalidData(format!("Unexpected maptype {o}"))),
        }

        Ok(())
    }
}

    }
}

    }
}
