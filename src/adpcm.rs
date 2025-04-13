#![allow(dead_code)]

use std::{io, fmt::Debug};
use crate::{FmtChunk};

macro_rules! define_copiable_buffer {
    ($name:ident, $name_intoiter:ident, $type:ty, $size:expr) => {
        #[derive(Debug, Clone, Copy)]
        struct $name {
            pub buffer: [$type; $size],
            buf_used: usize,
        }

        #[derive(Debug, Clone, Copy)]
        struct $name_intoiter {
            pub buffer: [$type; $size],
            buf_used: usize,
            iter_index: usize,
        }

        #[allow(dead_code)]
        impl $name {
            pub fn new() -> Self {
                Self {
                    buffer: [0 as $type; $size],
                    buf_used: 0,
                }
            }

            pub fn len(&self) -> usize {
                self.buf_used
            }

            pub fn is_full(&self) -> bool {
                self.buf_used == self.buffer.len()
            }

            #[track_caller]
            pub fn last(&mut self) -> &mut $type {
                if self.buf_used == 0 {
                    panic!("{} is empty.", stringify!($name));
                }
                &mut self.buffer[self.buf_used - 1]
            }

            #[track_caller]
            pub fn push(&mut self, data: $type) {
                if self.buf_used >= self.buffer.len() {
                    panic!("{} is full. Max size is {}.", stringify!($name), $size);
                } else {
                    self.buffer[self.buf_used] = data;
                    self.buf_used += 1;
                }
            }

            pub fn clear(&mut self) {
                self.buf_used = 0;
            }

            pub fn iter(&self) -> impl Iterator<Item = &$type> {
                self.buffer.iter()
            }

            pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut $type> {
                self.buffer.iter_mut()
            }
        }

        #[allow(dead_code)]
        impl Iterator for $name_intoiter {
            type Item = $type;

            fn next(&mut self) -> Option<Self::Item> {
                if self.iter_index < self.buf_used {
                    let ret = Some(self.buffer[self.iter_index]);
                    self.iter_index += 1;
                    ret
                } else {
                    None
                }
            }

            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.iter_index += n;
                self.next()
            }
        }

        #[allow(dead_code)]
        impl IntoIterator for $name {
            type Item = $type;
            type IntoIter = $name_intoiter;

            fn into_iter(self) -> Self::IntoIter {
                Self::IntoIter {
                    buffer: self.buffer,
                    buf_used: self.buf_used,
                    iter_index: 0,
                }
            }
        }

        #[allow(dead_code)]
        impl std::ops::Index<usize> for $name {
            type Output = $type;

            fn index(&self, index: usize) -> &Self::Output {
                if index >= self.buf_used {
                    panic!(
                        "Index out of bounds: {} >= {}",
                        index, self.buf_used
                    );
                }
                &self.buffer[index]
            }
        }

        #[allow(dead_code)]
        impl std::ops::IndexMut<usize> for $name {
            fn index_mut(&mut self, index: usize) -> &mut Self::Output {
                if index >= self.buf_used {
                    panic!(
                        "Index out of bounds: {} >= {}",
                        index, self.buf_used
                    );
                }
                &mut self.buffer[index]
            }
        }

        #[allow(dead_code)]
        impl std::iter::FromIterator<$type> for $name {
            fn from_iter<T: IntoIterator<Item = $type>>(iter: T) -> Self {
                let mut iter = iter.into_iter();
                let mut ret = Self::new();
                while let Some(data) = iter.next() {
                    ret.push(data);
                }
                ret
            }
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub enum CurrentChannel {
    Left,
    Right
}

pub trait AdpcmEncoder: Debug {
    fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized;
    fn encode(&mut self, input: impl FnMut() -> Option<i16>, output: impl FnMut(u8)) -> Result<(), io::Error>;
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16) -> FmtChunk {
        let block_align = (bits_per_sample as u32 * channels as u32 / 8) as u16;
        FmtChunk {
            format_tag: 1,
            channels,
            sample_rate,
            byte_rate: sample_rate * block_align as u32,
            block_align,
            bits_per_sample,
            extension: None,
        }
    }
    fn modify_fmt_chunk(&self, _fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
        Ok(())
    }
    fn flush(&mut self, _output: impl FnMut(u8)) -> Result<(), io::Error> {
        Ok(())
    }
}

pub trait AdpcmDecoder: Debug {
    fn new(fmt_chunk: &FmtChunk) -> Result<Self, io::Error> where Self: Sized;
    fn decode(&mut self, input: impl FnMut() -> Option<u8>, output: impl FnMut(i16)) -> Result<(), io::Error>;
    fn flush(&mut self, _output: impl FnMut(i16)) -> Result<(), io::Error> {
        Ok(())
    }
}

pub fn get_num_samples(fact_data: &Vec<u8>) -> Result<u64, io::Error> {
    match fact_data.len() {
        4 => Ok(u32::from_le_bytes([fact_data[0], fact_data[1], fact_data[2], fact_data[3]]) as u64),
        8 => Ok(u64::from_le_bytes([fact_data[0], fact_data[1], fact_data[2], fact_data[3], fact_data[4], fact_data[5], fact_data[6], fact_data[7]])),
        other => Err(io::Error::new(io::ErrorKind::InvalidData, format!("fact data size should be 4 or 8, not {other}."))),
    }
}

pub fn test(encoder: &mut impl AdpcmEncoder, decoder: &mut impl AdpcmDecoder, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
    encoder.encode(
        ||-> Option<i16> { input() },
        |code: u8| {
            let buf = vec![code];
            let mut iter = buf.into_iter();
            decoder.decode(
                || -> Option<u8> { iter.next() },
                |sample: i16|{ output(sample) }).unwrap()
        }
    )
}

pub type AdpcmEncoderIMA     = ima::Encoder;
// pub type AdpcmEncoderMS      = ms::Encoder;

pub type AdpcmDecoderIMA     = ima::Decoder;
// pub type AdpcmDecoderMS      = ms::Decoder;

pub type EncIMA     = AdpcmEncoderIMA;
// pub type EncMS      = AdpcmEncoderMS;

pub type DecIMA     = AdpcmDecoderIMA;
// pub type DecMS      = AdpcmDecoderMS;

pub mod ima {
    use std::{io, cmp::min, mem};

    use super::{AdpcmEncoder, AdpcmDecoder, CurrentChannel};
    use crate::{FmtChunk, ExtensionData, AdpcmImaData};

    #[derive(Debug)]
    pub enum ImaAdpcmError {
        InvalidArgument(String), // 参数错误
    }

    impl std::error::Error for ImaAdpcmError {}

    impl std::fmt::Display for ImaAdpcmError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Self::InvalidArgument(info) => write!(f, "Invalid arguments: {info}"),
            }
        }
    }

    const IMAADPCM_INDEX_TABLE: [i8; 16] = [
        -1, -1, -1, -1, 2, 4, 6, 8, 
        -1, -1, -1, -1, 2, 4, 6, 8 
    ];

    const IMAADPCM_STEPSIZE_TABLE: [u16; 89] = [
        7,     8,     9,     10,    11,    12,    13,    14, 
        16,    17,    19,    21,    23,    25,    28,    31, 
        34,    37,    41,    45,    50,    55,    60,    66,
        73,    80,    88,    97,    107,   118,   130,   143, 
        157,   173,   190,   209,   230,   253,   279,   307,
        337,   371,   408,   449,   494,   544,   598,   658,
        724,   796,   876,   963,   1060,  1166,  1282,  1411, 
        1552,  1707,  1878,  2066,  2272,  2499,  2749,  3024,
        3327,  3660,  4026,  4428,  4871,  5358,  5894,  6484,
        7132,  7845,  8630,  9493,  10442, 11487, 12635, 13899,
        15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
        32767
    ];

    const BLOCK_SIZE: usize = 512;
    const INTERLEAVE_BYTES: usize = 4;
    const SAMPLE_BUFFER_SIZE: usize = 512;
    const NIBBLE_BUFFER_SIZE: usize = 512;

    #[derive(Debug, Clone, Copy)]
    pub struct EncoderCore {
        prev_sample: i16,
        stepsize_index: i8,
        nibble: [u8; 2],
        nibble_index: u8,
        header_written: bool,
        num_outputs: usize,
    }

    #[derive(Debug, Clone)]
    pub struct StereoEncoder {
        current_channel: CurrentChannel,
        core_l: EncoderCore,
        core_r: EncoderCore,
        buffer_l: Vec<i16>,
        buffer_r: Vec<i16>,
        nibble_l: Vec<u8>,
        nibble_r: Vec<u8>,
    }

    #[derive(Debug, Clone)]
    pub enum Encoder {
        Mono(EncoderCore),
        Stereo(StereoEncoder),
    }

    impl EncoderCore{
        pub fn new() -> Self {
            Self {
                prev_sample: 0,
                stepsize_index: 0,
                nibble: [0u8; 2],
                nibble_index: 0,
                header_written: false,
                num_outputs: 0,
            }
        }

        // 编一个码
        pub fn encode_sample(&mut self, sample: i16) -> u8 {
            let mut prev = self.prev_sample as i32;
            let idx = self.stepsize_index;
            let stepsize = IMAADPCM_STEPSIZE_TABLE[idx as usize] as i32;
            let diff = sample as i32 - prev;
            let sign = diff < 0;
            let diffabs = diff.abs();
            let mut nibble = min((diffabs << 2) / stepsize, 7) as u8;
            if sign {
                nibble |= 8;
            }
            let delta = (nibble & 7) as i32;
            let qdiff = (stepsize * ((delta << 1) + 1)) >> 3;
            if sign {
                prev -= qdiff;
            } else {
                prev += qdiff;
            }
            prev = prev.clamp(-32768, 32767);
            let idx = (idx + IMAADPCM_INDEX_TABLE[nibble as usize]).clamp(0, 88);
            self.prev_sample = prev as i16;
            self.stepsize_index = idx;
            nibble
        }

        // 编码器逻辑
        // 一开始输出 4 字节的头部信息
        // 然后每两个样本转一个码
        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                if !self.header_written {
                    // 写出 4 字节头部
                    let buf = self.prev_sample.to_le_bytes();
                    output(buf[0]);
                    output(buf[1]);
                    output(self.stepsize_index as u8);
                    output(0);
                    self.num_outputs += 4;
                    self.header_written = true;
                }
                self.nibble[self.nibble_index as usize] = self.encode_sample(sample);
                self.nibble_index += 1;
                if self.nibble_index >= 2 {
                    self.nibble_index = 0;
                    output(self.nibble[0] | (self.nibble[1] << 4));
                    self.num_outputs += 1;
                    if self.num_outputs >= BLOCK_SIZE {
                        // 到达块大小上限，重置编码器
                        self.prev_sample = sample;
                        self.header_written = false;
                        self.num_outputs = 0;
                    }
                }
            }
            Ok(())
        }

        pub fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            let aligned_size = ((self.num_outputs - 1) / INTERLEAVE_BYTES + 1) * INTERLEAVE_BYTES;
            let pad_size = aligned_size - self.num_outputs;
            if pad_size != 0 {
                let mut iter = {let mut pad = Vec::<i16>::new(); pad.resize(pad_size, 0); pad.into_iter()};
                self.encode(|| -> Option<i16> {iter.next()}, |nibble: u8| {output(nibble)})?
            }
            Ok(())
        }
    }

    impl StereoEncoder {
        pub fn new() -> Self {
            Self {
                current_channel: CurrentChannel::Left,
                core_l: EncoderCore::new(),
                core_r: EncoderCore::new(),
                buffer_l: Vec::<i16>::new(),
                buffer_r: Vec::<i16>::new(),
                nibble_l: Vec::<u8>::new(),
                nibble_r: Vec::<u8>::new(),
            }
        }

        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                match self.current_channel{
                    CurrentChannel::Left => {
                        self.current_channel = CurrentChannel::Right;
                        self.buffer_l.push(sample);
                    },
                    CurrentChannel::Right => {
                        self.current_channel = CurrentChannel::Left;
                        self.buffer_r.push(sample);
                    },
                }
                if self.buffer_l.len() >= SAMPLE_BUFFER_SIZE &&
                   self.buffer_r.len() >= SAMPLE_BUFFER_SIZE {
                    let iter_l = mem::replace(&mut self.buffer_l, Vec::<i16>::new()).into_iter();
                    let iter_r = mem::replace(&mut self.buffer_r, Vec::<i16>::new()).into_iter();
                    let mut feeder_l = iter_l.clone().take(SAMPLE_BUFFER_SIZE);
                    let mut feeder_r = iter_r.clone().take(SAMPLE_BUFFER_SIZE);
                    self.core_l.encode(|| -> Option<i16> {feeder_l.next()}, |nibble:u8|{self.nibble_l.push(nibble)})?;
                    self.core_r.encode(|| -> Option<i16> {feeder_r.next()}, |nibble:u8|{self.nibble_r.push(nibble)})?;
                    self.buffer_l = iter_l.skip(SAMPLE_BUFFER_SIZE).collect();
                    self.buffer_r = iter_r.skip(SAMPLE_BUFFER_SIZE).collect();
                }
                while self.nibble_l.len() >= INTERLEAVE_BYTES &&
                      self.nibble_r.len() >= INTERLEAVE_BYTES {
                    let iter_l = mem::replace(&mut self.nibble_l, Vec::<u8>::new()).into_iter();
                    let iter_r = mem::replace(&mut self.nibble_r, Vec::<u8>::new()).into_iter();
                    let feeder_l = iter_l.clone().take(INTERLEAVE_BYTES);
                    let feeder_r = iter_r.clone().take(INTERLEAVE_BYTES);
                    for nibble in feeder_l {output(nibble);}
                    for nibble in feeder_r {output(nibble);}
                    self.nibble_l = iter_l.skip(INTERLEAVE_BYTES).collect();
                    self.nibble_r = iter_r.skip(INTERLEAVE_BYTES).collect();
                }
            }
            Ok(())
        }

        pub fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            // 全部写入
            let mut iter_l = mem::replace(&mut self.buffer_l, Vec::<i16>::new()).into_iter();
            let mut iter_r = mem::replace(&mut self.buffer_r, Vec::<i16>::new()).into_iter();
            self.core_l.encode(|| -> Option<i16> {iter_l.next()}, |nibble:u8|{self.nibble_l.push(nibble)})?;
            self.core_r.encode(|| -> Option<i16> {iter_r.next()}, |nibble:u8|{self.nibble_r.push(nibble)})?;
            self.core_l.flush(|nibble:u8|{self.nibble_l.push(nibble)})?;
            self.core_r.flush(|nibble:u8|{self.nibble_r.push(nibble)})?;
            while self.nibble_l.len() > 0 &&
                  self.nibble_r.len() > 0 {
                let iter_l = mem::replace(&mut self.nibble_l, Vec::<u8>::new()).into_iter();
                let iter_r = mem::replace(&mut self.nibble_r, Vec::<u8>::new()).into_iter();
                let feeder_l = iter_l.clone().take(INTERLEAVE_BYTES);
                let feeder_r = iter_r.clone().take(INTERLEAVE_BYTES);
                for nibble in feeder_l {output(nibble);}
                for nibble in feeder_r {output(nibble);}
                self.nibble_l = iter_l.skip(INTERLEAVE_BYTES).collect();
                self.nibble_r = iter_r.skip(INTERLEAVE_BYTES).collect();
            }
            Ok(())
        }
    }

    impl AdpcmEncoder for Encoder {
        fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized {
            match channels {
                1 => Ok(Encoder::Mono(EncoderCore::new())),
                2 => Ok(Encoder::Stereo(StereoEncoder::new())),
                other => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Wrong channel number \"{other}\" for ADPCM-IMA encoder."))),
            }
        }
        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            match self {
                Encoder::Mono(ref mut enc) => enc.encode(|| -> Option<i16> {input()}, |nibble:u8|{output(nibble)}),
                Encoder::Stereo(ref mut enc) => enc.encode(|| -> Option<i16> {input()}, |nibble:u8|{output(nibble)}),
            }
        }
        fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            match self {
                Encoder::Mono(ref mut enc) => enc.flush(|nibble:u8|{output(nibble)}),
                Encoder::Stereo(ref mut enc) => enc.flush(|nibble:u8|{output(nibble)}),
            }
        }
        fn get_required_fmt_chunk_size(&mut self) -> usize {
            16 + 2 + AdpcmImaData::sizeof()
        }
        fn modify_fmt_chunk(&self, fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
            fmt_chunk.block_align = BLOCK_SIZE as u16 * fmt_chunk.channels;
            fmt_chunk.bits_per_sample = 4;
            fmt_chunk.byte_rate = fmt_chunk.sample_rate * 8 / (fmt_chunk.channels as u32 * fmt_chunk.bits_per_sample as u32);
            if let Some(ref mut extension) = fmt_chunk.extension {
                if let ExtensionData::AdpcmIma(ref mut adpcm_ima) = extension.data {
                    adpcm_ima.samples_per_block = (BLOCK_SIZE as u16 - 4 * fmt_chunk.channels) * fmt_chunk.channels * 2;
                    Ok(())
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, format!("Wrong extension data stored in the `fmt ` chunk for ADPCM-IMA")))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("For ADPCM-IMA, must store the extension data in the `fmt ` chunk")))
            }
        }
    }

    // 解码器逻辑
    // data 里面是交错存储的 u32
    // 对于每个声道，第一个 u32 用于初始化解码器
    // 之后的每个 u32 相当于 4 个字节，能解出 8 个码
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderCore {
        sample_val: i16,
        stepsize_index: i8,
        ready: bool,
        buffer: [u8; 4],
        bufsize: usize,
        input_count: usize,
    }

    #[derive(Debug, Clone)]
    pub struct StereoDecoder {
        current_channel: CurrentChannel,
        counter: u8,
        core_l: DecoderCore,
        core_r: DecoderCore,
        nibble_l: Vec<u8>,
        nibble_r: Vec<u8>,
        sample_l: Vec<i16>,
        sample_r: Vec<i16>,
    }

    #[derive(Debug, Clone)]
    pub enum Decoder {
        Mono(DecoderCore),
        Stereo(StereoDecoder),
    }

    impl DecoderCore{
        pub fn new() -> Self {
            Self {
                sample_val: 0,
                stepsize_index: 0,
                ready: false,
                buffer: [0u8; INTERLEAVE_BYTES],
                bufsize: 0,
                input_count: 0,
            }
        }

        // 解一个码
        pub fn decode_sample(&mut self, nibble: u8) -> i16 {
            let mut predict = self.sample_val as i32;
            let idx = self.stepsize_index;
            let stepsize = IMAADPCM_STEPSIZE_TABLE[idx as usize] as i32;
            let idx = (idx + IMAADPCM_INDEX_TABLE[nibble as usize]).clamp(0, 88);
            let delta = (nibble & 7) as i32;
            let qdiff = (stepsize * ((delta << 1) + 1)) >> 3;
            if (nibble & 8) != 0 {
                predict -= qdiff;
            } else {
                predict += qdiff;
            }
            predict = predict.clamp(-32768, 32767);
            self.sample_val = predict as i16;
            self.stepsize_index = idx;
            self.sample_val
        }

        fn push_buf(&mut self, byte: u8) {
            self.buffer[self.bufsize as usize] = byte;
            self.bufsize += 1;
        }

        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            loop {
                if !self.ready {
                    // 先吃四个字节用来初始化，并输出第一个样本。
                    while self.bufsize < INTERLEAVE_BYTES {
                        match input() {
                            Some(byte) => {
                                self.push_buf(byte);
                                self.input_count += 1;
                            },
                            None => return Ok(()),
                        }
                    }
                    self.sample_val = i16::from_le_bytes([self.buffer[0], self.buffer[1]]);
                    self.stepsize_index = self.buffer[2] as i8;
                    if self.buffer[3] != 0 {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Reserved byte for ADPCM-IMA must be zero."));
                    }
                    self.bufsize = 0;
                    self.ready = true;
                    output(self.sample_val);
                }
                if self.ready {
                    // 完成初始化后，每吃一个字节输出两个样本。
                    while self.bufsize < INTERLEAVE_BYTES {
                        match input() {
                            Some(byte) => {
                                self.push_buf(byte);
                                self.input_count += 1;
                            },
                            None => return Ok(()),
                        }
                    }
                    // 每读取 4 个字节解 8 个码
                    let (b1, b2, b3, b4) = (self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3]);
                    output(self.decode_sample((b1 >> 0) & 0xF));
                    output(self.decode_sample((b1 >> 4) & 0xF));
                    output(self.decode_sample((b2 >> 0) & 0xF));
                    output(self.decode_sample((b2 >> 4) & 0xF));
                    output(self.decode_sample((b3 >> 0) & 0xF));
                    output(self.decode_sample((b3 >> 4) & 0xF));
                    output(self.decode_sample((b4 >> 0) & 0xF));
                    output(self.decode_sample((b4 >> 4) & 0xF));
                    self.bufsize = 0;
                    if self.input_count >= BLOCK_SIZE {
                        self.input_count = 0;
                        self.ready = false;
                    }
                }
            }
        }

        pub fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            if (self.ready, self.bufsize > 0, self.bufsize < INTERLEAVE_BYTES) == (true, true, true) {
                let pad_size = INTERLEAVE_BYTES - self.bufsize;
                let mut pad = Vec::<u8>::new();
                pad.resize(pad_size, 0);
                let mut iter = pad.into_iter();
                self.decode(|| -> Option<u8> {iter.next()}, |sample: i16| {output(sample)})?;
            }
            Ok(())
        }
    }

    impl StereoDecoder {
        pub fn new() -> Self {
            assert_eq!(NIBBLE_BUFFER_SIZE % INTERLEAVE_BYTES, 0);
            Self {
                current_channel: CurrentChannel::Left,
                counter: 0,
                core_l: DecoderCore::new(),
                core_r: DecoderCore::new(),
                nibble_l: Vec::<u8>::new(),
                nibble_r: Vec::<u8>::new(),
                sample_l: Vec::<i16>::new(),
                sample_r: Vec::<i16>::new(),
            }
        }

        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            // 每 INTERLEAVE_BYTES 字节数一个声道
            while let Some(nibble) = input() {
                match self.current_channel{
                    CurrentChannel::Left => {
                        self.nibble_l.push(nibble);
                        self.counter += 1;
                        if self.counter as usize == INTERLEAVE_BYTES {
                            self.counter = 0;
                            self.current_channel = CurrentChannel::Right;
                        }
                    },
                    CurrentChannel::Right => {
                        self.nibble_r.push(nibble);
                        self.counter += 1;
                        if self.counter as usize == INTERLEAVE_BYTES {
                            self.counter = 0;
                            self.current_channel = CurrentChannel::Left;
                        }
                    },
                }
                if self.nibble_l.len() >= NIBBLE_BUFFER_SIZE &&
                   self.nibble_r.len() >= NIBBLE_BUFFER_SIZE {
                    let iter_l = mem::replace(&mut self.nibble_l, Vec::<u8>::new()).into_iter();
                    let iter_r = mem::replace(&mut self.nibble_r, Vec::<u8>::new()).into_iter();
                    let mut feeder_l = iter_l.clone().take(NIBBLE_BUFFER_SIZE);
                    let mut feeder_r = iter_r.clone().take(NIBBLE_BUFFER_SIZE);
                    self.core_l.decode(|| -> Option<u8> {feeder_l.next()}, |sample:i16|{self.sample_l.push(sample)})?;
                    self.core_r.decode(|| -> Option<u8> {feeder_r.next()}, |sample:i16|{self.sample_r.push(sample)})?;
                    self.nibble_l = iter_l.skip(NIBBLE_BUFFER_SIZE).collect();
                    self.nibble_r = iter_r.skip(NIBBLE_BUFFER_SIZE).collect();
                }
                let iter_l = mem::replace(&mut self.sample_l, Vec::<i16>::new()).into_iter();
                let iter_r = mem::replace(&mut self.sample_r, Vec::<i16>::new()).into_iter();
                for stereo in iter_l.zip(iter_r) {
                    output(stereo.0);
                    output(stereo.1);
                }
            }
            Ok(())
        }

        pub fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            // 每 INTERLEAVE_BYTES 字节数一个声道，两边填零，确保两边都达到 INTERLEAVE_BYTES
            while match self.current_channel {
                CurrentChannel::Left => {
                    if self.counter == 0 {
                        false
                    } else {
                        self.nibble_l.push(0);
                        self.counter += 1;
                        if self.counter as usize == INTERLEAVE_BYTES {
                            self.counter = 0;
                            self.current_channel = CurrentChannel::Right;
                        }
                        true
                    }
                },
                CurrentChannel::Right => {
                    self.nibble_r.push(0);
                    self.counter += 1;
                    if self.counter as usize== INTERLEAVE_BYTES {
                        self.counter = 0;
                        self.current_channel = CurrentChannel::Left;
                        false
                    } else {
                        true
                    }
                },
            }{}
            if self.nibble_l.len() > 0 &&
               self.nibble_r.len() > 0 {
                let mut iter_l = mem::replace(&mut self.nibble_l, Vec::<u8>::new()).into_iter();
                let mut iter_r = mem::replace(&mut self.nibble_r, Vec::<u8>::new()).into_iter();
                self.core_l.decode(|| -> Option<u8> {iter_l.next()}, |sample:i16|{self.sample_l.push(sample)})?;
                self.core_r.decode(|| -> Option<u8> {iter_r.next()}, |sample:i16|{self.sample_r.push(sample)})?;
                self.core_l.flush(|sample:i16|{self.sample_l.push(sample)})?;
                self.core_r.flush(|sample:i16|{self.sample_r.push(sample)})?;
                self.nibble_l = Vec::<u8>::new();
                self.nibble_r = Vec::<u8>::new();
            }
            let iter_l = mem::replace(&mut self.sample_l, Vec::<i16>::new()).into_iter();
            let iter_r = mem::replace(&mut self.sample_r, Vec::<i16>::new()).into_iter();
            for stereo in iter_l.zip(iter_r) {
                output(stereo.0);
                output(stereo.1);
            }
            Ok(())
        }
    }


    impl AdpcmDecoder for Decoder {
        fn new(fmt_chunk: &FmtChunk) -> Result<Self, io::Error> where Self: Sized {
            match fmt_chunk.channels {
                1 => Ok(Decoder::Mono(DecoderCore::new())),
                2 => Ok(Decoder::Stereo(StereoDecoder::new())),
                other => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Wrong channel number \"{other}\" for ADPCM-IMA decoder."))),
            }
        }
        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error>{
            match self {
                Decoder::Mono(ref mut dec) => dec.decode(|| -> Option<u8> {input()}, |sample:i16|{output(sample)}),
                Decoder::Stereo(ref mut dec) => dec.decode(|| -> Option<u8> {input()}, |sample:i16|{output(sample)}),
            }
        }
        fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            match self {
                Decoder::Mono(ref mut dec) => dec.flush(|sample:i16|{output(sample)}),
                Decoder::Stereo(ref mut dec) => dec.flush(|sample:i16|{output(sample)}),
            }
        }
    }
}


pub mod ms {
    // 巨硬的 ADPCM
    // https://ffmpeg.org/doxygen/3.1/adpcmenc_8c_source.html
    // https://ffmpeg.org/doxygen/3.1/adpcm_8c_source.html
    use std::io;

    use super::{AdpcmEncoder, AdpcmDecoder, CurrentChannel};
    use crate::{FmtChunk, FmtExtension, ExtensionData, AdpcmMsData};

    const ADAPTATIONTABLE: [i16; 16] = [
        230, 230, 230, 230, 307, 409, 512, 614,
        768, 614, 512, 409, 307, 230, 230, 230
    ];

    #[derive(Debug, Clone, Copy)]
    pub struct AdpcmCoeffSet {
        pub coeff1: i16,
        pub coeff2: i16,
    }

    const DEF_COEFF_TABLE: [AdpcmCoeffSet; 7] = [
        AdpcmCoeffSet{coeff1: 256, coeff2: 0   },
        AdpcmCoeffSet{coeff1: 512, coeff2: -256},
        AdpcmCoeffSet{coeff1: 0  , coeff2: 0   },
        AdpcmCoeffSet{coeff1: 192, coeff2: 64  },
        AdpcmCoeffSet{coeff1: 240, coeff2: 0   },
        AdpcmCoeffSet{coeff1: 460, coeff2: -208},
        AdpcmCoeffSet{coeff1: 392, coeff2: -232},
    ];

    const BLOCK_SIZE: usize = 1024;
    const HEADER_SIZE: usize = 7;
    const NIBBLE_BUFFER_SIZE: usize = BLOCK_SIZE - HEADER_SIZE;
    const SAMPLES_PER_BLOCK: usize = NIBBLE_BUFFER_SIZE * 2;

    impl AdpcmCoeffSet{
        pub fn new() -> Self {
            Self {
                coeff1: 0,
                coeff2: 0,
            }
        }

        pub fn get(&self, index: usize) -> i16 {
            match index {
                1 => self.coeff1,
                2 => self.coeff2,
                o => panic!("Index must be 1 or 2, not {o}"),
            }
        }

        pub fn calculate_coefficient(data: &[i16]) -> Self {
            let mut alpha = 0.0f64;
            let mut beta = 0.0f64;
            let mut gamma = 0.0f64;
            let mut m = 0.0f64;
            let mut n = 0.0f64;
            for i in 2..data.len() {
                alpha += data[i - 1] as f64 * data[i - 1] as f64;
                beta += data[i - 1] as f64 * data[i - 2] as f64;
                gamma += data[i - 2] as f64 * data[i - 2] as f64;
                m += data[i] as f64 * data[i - 1] as f64;
                n += data[i] as f64 * data[i - 2] as f64;
            }
            Self {
                coeff1: ((m * gamma - n * beta) * 256.0 / (alpha * gamma - beta * beta)) as i16,
                coeff2: ((m * beta - n * alpha) * 256.0 / (beta * beta - alpha * gamma)) as i16,
            }
        }

        pub fn get_closest_coefficient_index(&self, coeff_table: &[AdpcmCoeffSet; 7]) -> u8 {
            let mut diff: u32 = 0xFFFFFFFF;
            let mut index = 0u8;
            for (i, coeff) in coeff_table.iter().enumerate() {
                let dx = (coeff.get(1) - self.coeff1) as i32;
                let dy = (coeff.get(2) - self.coeff2) as i32;
                let length_sq = (dx * dx + dy * dy) as u32;
                if length_sq < diff {
                    diff = length_sq;
                    index = i as u8;
                }
            }
            index
        }
    }

    pub fn trim_to_nibble(c: i8) -> u8 {
        (c.clamp(-8, 7) & 0x0F) as u8
    }

    #[derive(Debug, Clone, Copy)]
    pub struct EncoderCore {
        predictor: i32,
        sample1: i16,
        sample2: i16,
        coeff: AdpcmCoeffSet,
        delta: i32,
        ready: bool,
    }

    impl EncoderCore {
        pub fn new() -> Self {
            Self {
                predictor: 0,
                sample1: 0,
                sample2: 0,
                coeff: ADAPTATIONTABLE[0],
                delta: 16,
                ready: false,
            }
        }

        pub fn is_ready(&self) -> bool {
            self.ready
        }

        pub fn unready(&mut self) {
            if self.delta < 16 {
                self.delta = 16;
            }
            self.ready = false;
        }

        pub fn compress_sample(&mut self, sample: i16) -> u8 {
            let predictor = (
                self.sample1 as i32 * self.coeff.get(1) as i32 +
                self.sample2 as i32 * self.coeff.get(2) as i32) / 256;
            let nibble = sample as i32 - predictor;
            let bias = if nibble >= 0 {
                self.delta / 2
            } else {
                -self.delta / 2
            };
            let nibble = ((nibble + bias) / self.delta).clamp(-8, 7) & 0x0F;
            let predictor = predictor + if nibble & 0x08 != 0 {nibble.wrapping_sub(0x10)} else {nibble}) * self.delta;
            self.sample2 = self.sample1;
            self.sample1 = predictor.clamp(-32768, 32767) as i16;
            self.delta = (ADAPTATIONTABLE[nibble as usize] as i32 * self.delta) >> 8;
            if self.delta < 16 {
                self.delta = 16;
            }

            nibble as u8
        }

        pub fn get_ready(&mut self, samples: [i16; 2]) -> (u8, i16, i16, i16) {
            self.sample2 = samples[0];
            self.sample1 = samples[1];
            self.ready = true;
            (0u8, self.delta as i16, self.sample1, self.sample2)
        }
    }

    fn output_le_i16(val: i16, output: impl FnMut(u8)) {
        let bytes = val.to_le_bytes();
        output(bytes[0]);
        output(bytes[1]);
    }

    #[derive(Debug, Clone, Copy)]
    pub struct StereoEncoder {
        core_l: EncoderCore,
        core_r: EncoderCore,
        current_channel: CurrentChannel,
        ready: bool,
    }

    impl StereoEncoder {
        pub fn new () -> Self{
            Self {
                core_l: EncoderCore::new(),
                core_r: EncoderCore::new(),
                current_channel: CurrentChannel::Left,
                ready: false,
            }
        }

        pub fn is_ready(&self) -> bool {
            self.ready
        }

        pub fn unready(&mut self) {
            core_l.unready();
            core_r.unready();
        }

        pub fn compress_sample(&mut self, sample: i16) -> u8 {
            match self.current_channel {
                CurrentChannel::Left => {
                    self.current_channel = CurrentChannel::Right;
                    core_l.compress_sample(sample)
                },
                CurrentChannel::Right => {
                    self.current_channel = CurrentChannel::Left;
                    core_r.compress_sample(sample)
                },
            }
        }

        pub fn get_ready(&mut self, samples: [i16; 4]) -> (u8, u8, i16, i16, i16, i16, i16, i16) {
            let ready1 = self.core_l.get_ready([samples[0], samples[2]]);
            let ready2 = self.core_r.get_ready([samples[1], samples[3]]);
            self.ready = true;
            (ready1.0, ready2.0, ready1.1, ready2.1, ready1.2, ready2.2, ready1.3, ready2.3)
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder{
        channels: Channels,
        coeff_table: [AdpcmCoeffSet; 7],
        bytes_yield: usize,
        buffer: [i16; 4],
        bufsize: usize,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Channels{
        Mono(EncoderCore),
        Stereo(StereoEncoder),
    }

    impl Encoder {
        pub fn max_block_bytes(&self) -> usize {
            match self.channels {
                Mono(_): BLOCK_SIZE,
                Stereo(_): BLOCK_SIZE * 2,
            }
        }
    }

    impl AdpcmEncoder for Encoder {
        fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized;
            match channels {
                1 => {
                    Ok(Self {
                        channels: Mono(EncoderCore::new()),
                        coeff_table: DEF_COEFF_TABLE,
                        bytes_yield: 0,
                        buffer: [0i16; 4],
                        bufsize: 0,
                    })
                },
                2 => {
                    Ok(Self {
                        channels: Stereo(StereoEncoder::new()),
                        coeff_table: DEF_COEFF_TABLE,
                        bytes_yield: 0,
                        buffer: [0i16; 4],
                        bufsize: 0,
                    })
                },
                o => {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Channels must be 1 or 2, not {o}")))
                }
            }
        }

        fn encode(&mut self, input: impl FnMut() -> Option<i16>, output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                let ready = match self.channels {
                    Mono(ref mut enc) => enc.is_ready(),
                    Stereo(ref mut enc) => enc.is_ready(),
                };
                if !ready {
                    self.buffer[self.bufsize] = sample;
                    self.bufsize += 1;
                    match self.channels {
                        Mono(ref mut enc) => {
                            if self.bufsize >= 2 {
                                self.bufsize = 0;
                                let header = enc.get_ready([self.buffer[0], self.buffer[1]]);
                                output(header.0);
                                output_le_i16(header.1, |byte:u8|{output(byte)});
                                output_le_i16(header.2, |byte:u8|{output(byte)});
                                output_le_i16(header.3, |byte:u8|{output(byte)});
                                self.bytes_yield += 7;
                            }
                        },
                        Stereo(ref mut enc) => {
                            if self.bufsize >= 4 {
                                self.bufsize = 0;
                                let header = enc.get_ready([self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3]]);
                                output(header.0);
                                output(header.1);
                                output_le_i16(header.2, |byte:u8|{output(byte)});
                                output_le_i16(header.3, |byte:u8|{output(byte)});
                                output_le_i16(header.4, |byte:u8|{output(byte)});
                                output_le_i16(header.5, |byte:u8|{output(byte)});
                                output_le_i16(header.6, |byte:u8|{output(byte)});
                                output_le_i16(header.7, |byte:u8|{output(byte)});
                                self.bytes_yield += 14;
                            }
                        },
                    }
                } else {
                    self.buffer[self.bufsize] = sample;
                    self.bufsize += 1;
                    match self.channels {
                        Mono(ref mut enc) => {
                            if self.bufsize >= 2 {
                                self.bufsize = 0;
                                output (
                                    enc.compress_sample(self.buffer[0]) |
                                    enc.compress_sample(self.buffer[1]) << 4
                                );
                                self.bytes_yield += 1;
                            }
                            if self.bytes_yield >= self.max_block_bytes() {
                                enc.unready();
                                self.bytes_yield = 0;
                            }
                        },
                        Stereo(ref mut enc) => {
                            if self.bufsize >= 4 {
                                self.bufsize = 0;
                                output (
                                    enc.compress_sample(self.buffer[0]) |
                                    enc.compress_sample(self.buffer[1]) << 4
                                );
                                output (
                                    enc.compress_sample(self.buffer[2]) |
                                    enc.compress_sample(self.buffer[3]) << 4
                                );
                                self.bytes_yield += 2;
                            }
                            if self.bytes_yield >= self.max_block_bytes() {
                                enc.unready();
                                self.bytes_yield = 0;
                            }
                        },
                    }
                }
            }
            Ok(())
        }

        fn get_required_fmt_chunk_size(&mut self) -> usize {
            16 + 2 + AdpcmMsData::sizeof()
        }

        fn modify_fmt_chunk(&self, fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
            fmt_chunk.block_align = BLOCK_SIZE as u16 * fmt_chunk.channels;
            fmt_chunk.bits_per_sample = 4;
            fmt_chunk.byte_rate = fmt_chunk.sample_rate * 8 / (fmt_chunk.channels as u32 * fmt_chunk.bits_per_sample as u32);
            if let Some(ref mut extension) = fmt_chunk.extension {
                if let ExtensionData::AdpcmMs(ref mut adpcm_ms) = extension.data {
                    adpcm_ms.samples_per_block = (BLOCK_SIZE as u16 - 7 * fmt_chunk.channels) * fmt_chunk.channels * 2;
                    adpcm_ms.num_coeff = 7;
                    adpcm_ms.coeffs = self.coeff_table;
                    Ok(())
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, format!("Wrong extension data stored in the `fmt ` chunk for ADPCM-IMA")))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("For ADPCM-IMA, must store the extension data in the `fmt ` chunk")))
            }
        }
        fn flush(&mut self, _output: impl FnMut(u8)) -> Result<(), io::Error> {
            Ok(())
        }
    }


// for (i = 0; i < avctx->channels; i++) {
//     int predictor = 0;
//     *dst++ = predictor;
//     c->status[i].coeff1 = ff_adpcm_AdaptCoeff1[predictor];
//     c->status[i].coeff2 = ff_adpcm_AdaptCoeff2[predictor];
// }
// for (i = 0; i < avctx->channels; i++) {
//     if (c->status[i].idelta < 16)
//         c->status[i].idelta = 16;
//     bytestream_put_le16(&dst, c->status[i].idelta);
// }
// for (i = 0; i < avctx->channels; i++)
//     c->status[i].sample2= *samples++;
// for (i = 0; i < avctx->channels; i++) {
//     c->status[i].sample1 = *samples++;
//     bytestream_put_le16(&dst, c->status[i].sample1);
// }
// for (i = 0; i < avctx->channels; i++)
//     bytestream_put_le16(&dst, c->status[i].sample2);
// 
// if (avctx->trellis > 0) {
//     int n = avctx->block_align - 7 * avctx->channels;
//     FF_ALLOC_OR_GOTO(avctx, buf, 2 * n, error);
//     if (avctx->channels == 1) {
//         adpcm_compress_trellis(avctx, samples, buf, &c->status[0], n);
//         for (i = 0; i < n; i += 2)
//             *dst++ = (buf[i] << 4) | buf[i + 1];
//     } else {
//         adpcm_compress_trellis(avctx, samples,     buf,     &c->status[0], n);
//         adpcm_compress_trellis(avctx, samples + 1, buf + n, &c->status[1], n);
//         for (i = 0; i < n; i++)
//             *dst++ = (buf[i] << 4) | buf[n + i];
//     }
//     av_free(buf);
// } else {
//     for (i = 7 * avctx->channels; i < avctx->block_align; i++) {
//         int nibble;
//         nibble  = adpcm_ms_compress_sample(&c->status[ 0], *samples++) << 4;
//         nibble |= adpcm_ms_compress_sample(&c->status[st], *samples++);
//         *dst++  = nibble;
//     }
// }
// break;
    /*

    #[derive(Clone, Copy)]
    pub struct EncoderBlock {
        predictor: u8,
        delta: i16,
        sample1: i16,
        sample2: i16,
        nibbles: [u8; NIBBLE_BUFFER_SIZE],
        num_nibbles: usize
    }

    impl EncoderBlock {
        pub fn new() -> Self {
            Self {
                predictor: 0,
                delta: 0,
                sample1: 0,
                sample2: 0,
                nibbles: [0u8; NIBBLE_BUFFER_SIZE],
                num_nibbles: 0,
            }
        }

        pub fn to_le_bytes(&self) -> Vec<u8> {
            let mut ret = Vec::<u8>::with_capacity(256);
            ret.push(self.predictor);
            ret.extend(&self.delta.to_le_bytes());
            ret.extend(&self.sample1.to_le_bytes());
            ret.extend(&self.sample2.to_le_bytes());
            ret.extend(&self.nibbles);
            ret
        }

        pub fn is_full(&self) -> bool {
            self.num_nibbles as usize >= self.nibbles.len()
        }

        pub fn push_nibble(&mut self, nibble: u8) -> Result<(), io::Error> {
            if !self.is_full() {
                self.nibbles[self.num_nibbles as usize] = nibble;
                self.num_nibbles += 1;
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::StorageFull, format!("The nibble buffer is full.")))
            }
        }

        pub fn fill_nibble(&mut self) {
            while !self.is_full() {
                self.nibbles[self.num_nibbles as usize] = 0;
                self.num_nibbles += 1;
            }
        }

        pub fn clear(&mut self) {
            self.num_nibbles = 0;
        }
    }

    #[derive(Clone, Copy)]
    pub struct Encoder {
        coeff_table: [AdpcmCoeffSet; 7],
        block: EncoderBlock,
        delta: i16,
        sample1: i16,
        sample2: i16,
        nibble_flag: bool,
        input_buffer: [i16; SAMPLES_PER_BLOCK as usize],
        num_samples: u16,
        total_samples: u64,
        is_first_block: bool,
    }

    impl Encoder {
        pub fn is_full(&self) -> bool {
            self.num_samples as usize >= self.input_buffer.len()
        }

        pub fn push_sample(&mut self, sample: i16) -> Result<(), io::Error> {
            if !self.is_full() {
                self.input_buffer[self.num_samples as usize] = sample;
                self.num_samples += 1;
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::StorageFull, format!("The nibble buffer is full.")))
            }
        }

        pub fn fill_samples(&mut self) {
            while !self.is_full() {
                self.input_buffer[self.num_samples as usize] = 0;
                self.num_samples += 1;
            }
        }

        pub fn clear(&mut self) {
            self.num_samples = 0;
        }
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                coeff_table: DEF_COEFF_TABLE,
                block: EncoderBlock::new(),
                delta: 0,
                sample1: 0,
                sample2: 0,
                nibble_flag: false,
                input_buffer: [0i16; SAMPLES_PER_BLOCK as usize],
                num_samples: 0,
                total_samples: 0,
                is_first_block: true,
            }
        }

        // 编码逻辑：每次吃一整个大块，吃饱后拉出同样的一个大块，以此循环。
        // 输入 None 后停止循环，此时使用 `flush()` 可以拉出最后一个大块。
        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                while !self.is_full() { // 先吃满一整个块
                    match input() {
                        Some(sample) => {
                            self.push_sample(sample)?;
                            self.total_samples += 1;
                        },
                        None => return Ok(()),
                    }
                }
                let index = AdpcmCoeffSet::calculate_coefficient(&self.input_buffer).get_closest_coefficient_index(&self.coeff_table);
                let coeff = self.coeff_table[index as usize];
                self.block.sample2 = self.input_buffer[0];
                self.block.sample1 = self.input_buffer[1];
                if self.is_first_block {
                    self.delta = ((coeff.coeff1 as i32 * self.block.sample1 as i32 +
                                   coeff.coeff2 as i32 * self.block.sample2 as i32) / 256) as i16 - self.input_buffer[2];
                    self.delta /= 4;
                    if self.delta <= 0 {self.delta = -self.delta + 1;}
                    self.is_first_block = false;
                }
                self.block.delta = self.delta;
                self.block.predictor = index;
                self.sample1 = self.block.sample1;
                self.sample2 = self.block.sample2;
                let mut nibble = 0u8;
                let mut i = 3usize;
                while i < SAMPLES_PER_BLOCK as usize {
                    let predictor = ((coeff.coeff1 as i32 * self.sample1 as i32 + coeff.coeff2 as i32 * self.sample2 as i32) / 256) as i16;
                    let sample_diff = self.input_buffer[i] - predictor;
                    let mut error_delta = (sample_diff / self.delta).clamp(-8, 7) as i8;
                    let remainder = sample_diff % self.delta;
                    if remainder > self.delta / 2 {error_delta += 1;}
                    error_delta = error_delta.clamp(-8, 7);
                    let new_sample = predictor + error_delta as i16 * self.delta;
                    self.delta = (self.delta as i32 * ADAPTATIONTABLE[trim_to_nibble(error_delta) as usize] as i32 / 256) as i16;
                    if self.delta < 1 {self.delta = 1}
                    self.sample2 = self.sample1;
                    self.sample1 = new_sample;
                    i += 1;
                    if !self.nibble_flag {
                        self.nibble_flag = true;
                        nibble = trim_to_nibble(error_delta);
                    } else {
                        self.nibble_flag = false;
                        nibble = (nibble << 4) | trim_to_nibble(error_delta);
                        if !self.block.is_full() {
                            self.block.push_nibble(nibble)?;
                        } else {
                            for nibble in self.block.to_le_bytes() {
                                output(nibble);
                            }
                            self.block.clear();
                            self.clear();
                        }
                    }
                }
            }
        }
        fn get_required_fmt_chunk_size(&mut self) -> usize {
            16 + 2 + AdpcmImaData::sizeof();
        }
        fn yield_extension_data(&self, channels: u16) -> Option<FmtExtension> {
            Some(FmtExtension::new_adpcm_ms(AdpcmMsData{
                samples_per_block: (SAMPLES_PER_BLOCK * channels as usize) as u16,
                num_coeff: 7,
                coeffs: self.coeff_table,
            }))
        }
        fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            self.fill_samples();
            self.encode(
                || -> Option<i16> {None},
                |nibble: u8|{output(nibble)})?;
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct DecoderBlock {
        pub predictor: u8,
        pub delta: i32,
        pub sample1: i32,
        pub sample2: i32,
        pub coeff: AdpcmCoeffSet,
    }

    impl DecoderBlock {
        pub fn new() -> Self {
            Self {
                predictor: 0,
                delta: 0,
                sample1: 0,
                sample2: 0,
                coeff: AdpcmCoeffSet::new(),
            }
        }

        pub fn expand_nibble(&mut self, nibble: u8) -> i16 {
            let predictor = ((self.sample1 as i32 * self.coeff.coeff1 as i32 +
                              self.sample2 as i32 * self.coeff.coeff2 as i32) / 256) +
                (if nibble & 0x08 != 0 {nibble - 0x10} else {nibble}) as i32 * self.delta as i32;

            self.sample2 = self.sample1;
            self.sample1 = predictor.clamp(-32768, 32767) as i16;

            //FFmpeg 的源码里，delta 是 i32，它的数值可能会变得夸张的大，还得做限制
            self.delta = ((ADAPTATIONTABLE[nibble as usize] as i32 * self.delta as i32) >> 8).clamp(16, 32767) as i16;

            // 返回值
            self.sample1
        }
    }

    #[derive(Debug, Clone)]
    pub struct Decoder {
        coeff_table: [AdpcmCoeffSet; 7],
        samples_per_block: u16,
        block: DecoderBlock,
        buffer: [u8; HEADER_SIZE as usize],
        bufsize: usize,
        header_init: bool,
        bytes_read: usize,
    }

    impl AdpcmDecoder for Decoder {
        fn new(extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            // 从 `fmt ` 块的扩展块里读取初始系数和系数表，以及块大小。
            let adpcm_ms = if let Some(extension_data) = extension_data {
                if let ExtensionData::AdpcmMs(adpcm_ms) = extension_data.data {
                    Ok(adpcm_ms)
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, format!("ADPCM-MS: When parsing `fmt ` chunk extension data, the data is not for ADPCM-MS, got {:?}", extension_data)))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, format!("ADPCM-MS: When parsing `fmt ` chunk, the extension data is needed")))
            }?;
            if adpcm_ms.num_coeff != 7 {
                // 系数表其实是钦定的，但是钦定的系数表也要写入到 `fmt ` 的扩展块里，并且解码的时候也要从扩展块里读取它。
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("ADPCM-MS: When parsing `fmt ` chunk extension data, `num_coeff` must be 7 ")));
            }
            Self {
                coeff_table: adpcm_ms.coeffs,
                samples_per_block: adpcm_ms.samples_per_block,
                block: DecoderBlock::new(),
                buffer: [0u8; HEADER_SIZE as usize],
                bufsize: 0,
                header_init: false,
                bytes_read: 0,
            }
        }

        // 解码就不需要分块了，只要读取了头部就可以一直解码。
        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            loop {
                if !self.header_init {
                    while self.bufsize < HEADER_SIZE {
                        if let Some(nibble) = input() {
                            self.buffer[self.bufsize as usize] = nibble;
                            self.bufsize += 1;
                            self.bytes_read += 1;
                        } else {
                            return Ok(())
                        }
                    }
                    self.block.predictor = self.buffer[0];
                    self.block.delta = i16::from_le_bytes([self.buffer[1], self.buffer[2]]) as i32;
                    self.block.sample1 = i16::from_le_bytes([self.buffer[3], self.buffer[4]]);
                    self.block.sample2 = i16::from_le_bytes([self.buffer[5], self.buffer[6]]);
                    if self.block.predictor as usize >= self.coeff_table.len() {
                        self.bufsize = 0;
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("`block.predictor` = {:?}", self.block)));
                    }
                    self.block.coeff = self.coeff_table[self.block.predictor as usize];
                    self.header_init = true;
                    output(self.block.sample2 as i16);
                    output(self.block.sample1 as i16);
                }
                if self.header_init {
                    if let Some(nibble) = input() {
                        self.bytes_read += 1;

                        // 看了一下 FFmpeg 的源码，一个 nibble 可以展开为两个样本，而对于立体声的情况，一个 nibble 展开的是两个声道
                        output(self.block.expand_nibble(nibble >> 4));
                        output(self.block.expand_nibble(nibble & 0x0F));

                        // 读了一个块的数据了，恢复状态重新读头
                        if self.bytes_read >= BLOCK_SIZE {
                            self.bufsize = 0;
                            self.bytes_read = 0;
                            self.header_init = false;
                        }
                    } else {
                        return Ok(())
                    }
                }
            }
        }
        fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            if self.bytes_read > 0 && self.bytes_read < BLOCK_SIZE as usize {
                let mut zeroes = Vec::<u8>::new();
                zeroes.resize(BLOCK_SIZE as usize - self.bytes_read, 0);
                let mut iter = zeroes.into_iter();
                self.decode(
                    || -> Option<u8> {iter.next()},
                    |sample: i16|{output(sample)})?;
            }
            Ok(())
        }
    }

    impl std::fmt::Debug for EncoderBlock {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("EncoderBlock")
                .field("predictor", &self.predictor)
                .field("delta", &self.delta)
                .field("sample1", &self.sample1)
                .field("sample2", &self.sample2)
                .field("nibbles", &format_args!("[u8; {}]", self.nibbles.len()))
                .field("num_nibbles", &self.num_nibbles)
                .finish()
        }
    }

    impl std::fmt::Debug for Encoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Encoder")
                .field("coeff_table", &self.coeff_table)
                .field("block", &self.block)
                .field("input_buffer", &format_args!("[i16; {}]", self.input_buffer.len()))
                .field("num_samples", &self.num_samples)
                .finish()
        }
    }
    */
}

