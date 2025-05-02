use std::{io, fmt::Debug};
use crate::wavcore::{FmtChunk};

/// * This is used for the encoder to decide while the new sample arrives, which channel the sample should be put in
#[derive(Debug, Clone, Copy)]
pub enum CurrentChannel {
    Left,
    Right,
}

/// ## The `AdpcmEncoder` trait for all of the ADPCM encoders to implement
/// * This thing is able to encode samples, write the `fmt ` chunk, update statistics, and finish encoding.
pub trait AdpcmEncoder: Debug {
    fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized;

    /// * The `encode()` function uses two closures to input samples and output the encoded data.
    /// * If you have samples to encode, just feed it through the `input` closure. When it has encoded data to excrete, it will call the `output()` closure to give you back the encoded data.
    /// * It will endlessly ask for new samples to encode through calling `input` closure, give it a `None` and let it return.
    /// * You can continue encoding by calling `encode()` again and feeding the samples through the `input` closure.
    /// * Typically usage is to use an iterator to feed data through the `input` closure, when no data to feed, the `iter.next()` returns `None` to break the loop.
    /// * If you have the iterator, it is convenient to use `iter.as_ref().take()` to break the waveform into segments, and feeding each segment to the encoder works too.
    ///   And you have the convenience to modify each segment of the audio data e.g. do some resampling (first call the taken data's `collect()` method to get a `Vec`, then do your process, and turn the `Vec` into another iterator to feed the encoder), etc.
    fn encode(&mut self, input: impl FnMut() -> Option<i16>, output: impl FnMut(u8)) -> Result<(), io::Error>;

    /// * Call this method if you want to create a `fmt ` chunk for a WAV file.
    /// * The `fmt ` chunk is new and it's without any statistics data inside it.
    /// * Later after encoding, you should call `modify_fmt_chunk()` to update the `fmt ` chunk for the encoder to save some statistics data.
    fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16) -> Result<FmtChunk, io::Error> {
        let block_align = (bits_per_sample as u32 * channels as u32 / 8) as u16;
        Ok(FmtChunk {
            format_tag: 1,
            channels,
            sample_rate,
            byte_rate: sample_rate * block_align as u32,
            block_align,
            bits_per_sample,
            extension: None,
        })
    }

    /// * After encoding, call this function to update the `fmt ` chunk for the encoder to save some statistics data.
    fn modify_fmt_chunk(&self, _fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
        Ok(())
    }

    /// * Flush the encoder. The encoder may have some half-bytes (nibbles) in the cache, or the encoded data size is not a full block.
    /// * Normally the `flush()` method will feed zero samples for the encoder to let it excrete.
    fn flush(&mut self, _output: impl FnMut(u8)) -> Result<(), io::Error> {
        Ok(())
    }
}

/// ## The `AdpcmDecoder` trait for all of the ADPCM decoders to implement
/// * To create this thing, you need the WAV `fmt ` chunk data for the encoder to get the critical data.
pub trait AdpcmDecoder: Debug {
    fn new(fmt_chunk: FmtChunk) -> Result<Self, io::Error> where Self: Sized;

    /// * Get the block size for each block stored in the `data` chunk of the WAV file.
    /// * When it's needed to seek for a sample, first seek to the block, then decode the block to get the sample.
    fn get_block_size(&self) -> usize;

    /// * How many audio frames are encoded in a block of data. An audio frame is an array that contains one sample for every channel.
    fn frames_per_block(&self) -> usize;

    /// * This function is called before seeking, it resets the decoder to prevent it excretes unexpected samples.
    fn reset_states(&mut self);

    /// * The `decode()` function uses two closures to input data and output the decoded samples.
    /// * When there's data to be decoded, feed it through the `input()` closure by wrapping it to `Some(data)`.
    /// * When the decoder wants to excrete samples, it calls `output()` closure to give you the sample.
    /// * The function will endlessly call `input()` to get the encoded data. Feed it a `None` will let the function return.
    /// * To continue decoding, just call the function again and feed it `Some(data)` through the `input()` closure.
    fn decode(&mut self, input: impl FnMut() -> Option<u8>, output: impl FnMut(i16)) -> Result<(), io::Error>;

    /// * The `flush()` function causes the decoder to excrete the last samples.
    fn flush(&mut self, _output: impl FnMut(i16)) -> Result<(), io::Error> {
        Ok(())
    }
}

/// ### An example test function to test the `AdpcmEncoder` and the `AdpcmDecoder`.
/// * Normally it won't be called by you, but if you want to test how lossy the ADPCM algorithm is, this can help.
#[allow(dead_code)]
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
pub type AdpcmEncoderMS      = ms::Encoder;
pub type AdpcmEncoderYAMAHA  = yamaha::Encoder;

pub type AdpcmDecoderIMA     = ima::Decoder;
pub type AdpcmDecoderMS      = ms::Decoder;
pub type AdpcmDecoderYAMAHA  = yamaha::Decoder;

pub type EncIMA     = AdpcmEncoderIMA;
pub type EncMS      = AdpcmEncoderMS;
pub type EncYAMAHA  = AdpcmEncoderYAMAHA;

pub type DecIMA     = AdpcmDecoderIMA;
pub type DecMS      = AdpcmDecoderMS;
pub type DecYAMAHA  = AdpcmDecoderYAMAHA;

pub mod ima {
    use std::{io, cmp::min, mem};

    use super::{AdpcmEncoder, AdpcmDecoder, CurrentChannel};
    use crate::copiablebuf::{CopiableBuffer};
    use crate::wavcore::{FmtChunk, FmtExtension, ExtensionData, AdpcmImaData};

    #[derive(Debug)]
    pub enum ImaAdpcmError {
        InvalidArgument(String),
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
    const HEADER_SIZE: usize = 4;
    const INTERLEAVE_BYTES: usize = 4;
    const INTERLEAVE_SAMPLES: usize = INTERLEAVE_BYTES * 2;
    const NIBBLE_BUFFER_SIZE: usize = HEADER_SIZE + INTERLEAVE_BYTES;

    /// ## The core encoder of ADPCM-IMA for mono-channel
    #[derive(Debug, Clone, Copy)]
    pub struct EncoderCore {
        prev_sample: i16,
        stepsize_index: i8,
        nibble: u8,
        half_byte_written: bool,
        header_written: bool,
        num_outputs: usize,
    }

    impl EncoderCore{
        /// Set all of the members to zero.
        pub fn new() -> Self {
            Self {
                prev_sample: 0,
                stepsize_index: 0,
                nibble: 0,
                half_byte_written: false,
                header_written: false,
                num_outputs: 0,
            }
        }

        /// * Encode one sample, get a nibble
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

        /// ### Encoder logic:
        /// 1. Initially outputs 4 bytes of the decoder's state machine register values.
        /// 2. Processes samples by converting two raw samples into one encoded unit combined by two nibbles (a byte).
        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                if !self.header_written {
                    // Write the four bytes header
                    let buf = self.prev_sample.to_le_bytes();
                    output(buf[0]);
                    output(buf[1]);
                    output(self.stepsize_index as u8);
                    output(0);
                    self.num_outputs += 4;
                    self.header_written = true;
                }
                if !self.half_byte_written {
                    self.nibble = self.encode_sample(sample);
                    self.half_byte_written = true;
                } else {
                    self.nibble |= self.encode_sample(sample) << 4;
                    self.half_byte_written = false;
                    output(self.nibble);
                    self.num_outputs += 1;
                    if self.num_outputs >= BLOCK_SIZE {
                        // Reaches the block size limit; resets the encoder.
                        self.prev_sample = sample;
                        self.header_written = false;
                        self.num_outputs = 0;
                    }
                }
            }
            Ok(())
        }

        /// * Continue feeding zeroes to the encoder until it finishes processing a whole block of data.
        pub fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            let aligned_size = ((self.num_outputs - 1) / INTERLEAVE_BYTES + 1) * INTERLEAVE_BYTES;
            let pad_size = aligned_size - self.num_outputs;
            if pad_size != 0 {
                let mut iter = vec![0i16; pad_size].into_iter();
                self.encode(|| -> Option<i16> {iter.next()}, |nibble: u8| {output(nibble)})?
            }
            Ok(())
        }
    }

    impl Default for EncoderCore {
        fn default() -> Self {
            Self::new()
        }
    }

    type EncoderSampleBuffer = CopiableBuffer<i16, INTERLEAVE_SAMPLES>;
    type EncoderNibbleBuffer = CopiableBuffer<u8, NIBBLE_BUFFER_SIZE>;

    /// ### A wrapper for the `EncoderCore` to encode stereo audio.
    #[derive(Debug, Clone, Copy)]
    pub struct StereoEncoder {
        current_channel: CurrentChannel,
        core_l: EncoderCore,
        core_r: EncoderCore,
        sample_l: EncoderSampleBuffer,
        sample_r: EncoderSampleBuffer,
        nibble_l: EncoderNibbleBuffer,
        nibble_r: EncoderNibbleBuffer,
    }

    /// ## The ADPCM-IMA Encoder for you to use.
    #[derive(Debug, Clone)]
    pub enum Encoder {
        Mono(EncoderCore),
        Stereo(StereoEncoder),
    }

    impl StereoEncoder {
        pub fn new() -> Self {
            Self {
                current_channel: CurrentChannel::Left,
                core_l: EncoderCore::new(),
                core_r: EncoderCore::new(),
                sample_l: EncoderSampleBuffer::new(),
                sample_r: EncoderSampleBuffer::new(),
                nibble_l: EncoderNibbleBuffer::new(),
                nibble_r: EncoderNibbleBuffer::new(),
            }
        }

        /// * This `encode()` function will cache a very small amount of samples and data.
        /// * Call `flush()` to let it excrete all of the cached data.
        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                match self.current_channel{
                    CurrentChannel::Left => {
                        self.current_channel = CurrentChannel::Right;
                        self.sample_l.push(sample);
                    },
                    CurrentChannel::Right => {
                        self.current_channel = CurrentChannel::Left;
                        self.sample_r.push(sample);
                    },
                }
                if self.sample_l.is_full() && self.sample_r.is_full() {
                    let mut iter_l = mem::replace(&mut self.sample_l, EncoderSampleBuffer::new()).into_iter();
                    let mut iter_r = mem::replace(&mut self.sample_r, EncoderSampleBuffer::new()).into_iter();
                    self.core_l.encode(|| -> Option<i16> {iter_l.next()}, |nibble:u8|{self.nibble_l.push(nibble)})?;
                    self.core_r.encode(|| -> Option<i16> {iter_r.next()}, |nibble:u8|{self.nibble_r.push(nibble)})?;
                }
                while self.nibble_l.len() >= INTERLEAVE_BYTES && self.nibble_r.len() >= INTERLEAVE_BYTES {
                    for i in 0..INTERLEAVE_BYTES {output(self.nibble_l[i]);}
                    for i in 0..INTERLEAVE_BYTES {output(self.nibble_r[i]);}
                    self.nibble_l = self.nibble_l.into_iter().skip(INTERLEAVE_BYTES).collect();
                    self.nibble_r = self.nibble_r.into_iter().skip(INTERLEAVE_BYTES).collect();
                }
            }
            Ok(())
        }

        /// * Let the encoder excrete all of the data, finish encoding.
        pub fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while !self.sample_l.is_empty() || !self.sample_r.is_empty() {
                let mut iter = [0i16].into_iter();
                self.encode(|| -> Option<i16> {iter.next()}, |nibble:u8|{output(nibble)})?;
            }
            Ok(())
        }
    }

    impl Default for StereoEncoder {
        fn default() -> Self {
            Self::new()
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
                Encoder::Mono(enc) => enc.encode(|| -> Option<i16> {input()}, |nibble:u8|{output(nibble)}),
                Encoder::Stereo(enc) => enc.encode(|| -> Option<i16> {input()}, |nibble:u8|{output(nibble)}),
            }
        }

        fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            match self {
                Encoder::Mono(enc) => enc.flush(|nibble:u8|{output(nibble)}),
                Encoder::Stereo(enc) => enc.flush(|nibble:u8|{output(nibble)}),
            }
        }

        fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16) -> Result<FmtChunk, io::Error> {
            assert_eq!(bits_per_sample, 4);
            let block_align = BLOCK_SIZE as u16 * channels;
            Ok(FmtChunk {
                format_tag: 0x0011,
                channels,
                sample_rate,
                byte_rate: sample_rate * bits_per_sample as u32 * channels as u32 / 8,
                block_align,
                bits_per_sample,
                extension: Some(FmtExtension::new_adpcm_ima(AdpcmImaData{
                    samples_per_block: (BLOCK_SIZE as u16 - HEADER_SIZE as u16) * channels * 2,
                })),
            })
        }

        fn modify_fmt_chunk(&self, fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
            fmt_chunk.block_align = BLOCK_SIZE as u16 * fmt_chunk.channels;
            fmt_chunk.bits_per_sample = 4;
            fmt_chunk.byte_rate = fmt_chunk.sample_rate * 8 / (fmt_chunk.channels as u32 * fmt_chunk.bits_per_sample as u32);
            if let Some(extension) = fmt_chunk.extension {
                if let ExtensionData::AdpcmIma(mut adpcm_ima) = extension.data {
                    adpcm_ima.samples_per_block = (BLOCK_SIZE as u16 - 4) * fmt_chunk.channels * 2;
                    Ok(())
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, format!("Wrong extension data stored in the `fmt ` chunk for ADPCM-IMA: {:?}", extension)))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, "For ADPCM-IMA, the extension data in the `fmt ` chunk is needed".to_owned()))
            }
        }
    }

    type DecoderNibbleBuffer = CopiableBuffer<u8, INTERLEAVE_BYTES>;
    type DecoderSampleBuffer = CopiableBuffer<i16, INTERLEAVE_SAMPLES>;

    /// ### Decoder logic:
    /// * Data is stored as interleaved u32 values across channels.
    /// * For each channel, the first u32 initializes the decoder state.
    /// * Each subsequent u32 (4 bytes) decodes into 8 compressed nibbles (4-bit samples).
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderCore {
        sample_val: i16,
        stepsize_index: i8,
        ready: bool,
        nibble_buffer: DecoderNibbleBuffer,
        input_count: usize,
        block_size: usize,
    }

    impl DecoderCore {
        pub fn new(fmt_chunk: FmtChunk) -> Self {
            Self {
                sample_val: 0,
                stepsize_index: 0,
                ready: false,
                nibble_buffer: DecoderNibbleBuffer::new(),
                input_count: 0,
                block_size: (fmt_chunk.block_align / fmt_chunk.channels) as usize,
            }
        }

        /// Decode one sample
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

        /// * This `encode()` function needs 4 initial bytes to initialize, after being initialized, every 4 bytes decodes to 8 samples.
        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            while let Some(byte) = input() {
                if !self.ready {
                    // Consumes 4 bytes to initialize the decoder state and generates the first decoded sample.
                    self.nibble_buffer.push(byte);
                    self.input_count += 1;
                    if self.nibble_buffer.is_full() {
                        self.sample_val = i16::from_le_bytes([self.nibble_buffer[0], self.nibble_buffer[1]]);
                        self.stepsize_index = self.nibble_buffer[2] as i8;
                        if self.nibble_buffer[3] != 0 {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Reserved byte for ADPCM-IMA must be zero, not 0x{:02x}", self.nibble_buffer[3])));
                        }
                        self.nibble_buffer.clear();
                        self.ready = true;
                        output(self.sample_val);
                    }
                } else {
                    self.nibble_buffer.push(byte);
                    self.input_count += 1;
                    // Every 4 bytes (8 nibbles) decode to 8 samples.
                    if self.nibble_buffer.is_full() {
                        let (b1, b2, b3, b4) = (self.nibble_buffer[0], self.nibble_buffer[1], self.nibble_buffer[2], self.nibble_buffer[3]);
                        output(self.decode_sample(b1 & 0xF));
                        output(self.decode_sample(b1 >> 4));
                        output(self.decode_sample(b2 & 0xF));
                        output(self.decode_sample(b2 >> 4));
                        output(self.decode_sample(b3 & 0xF));
                        output(self.decode_sample(b3 >> 4));
                        output(self.decode_sample(b4 & 0xF));
                        output(self.decode_sample(b4 >> 4));
                        self.nibble_buffer.clear();
                        if self.input_count >= self.block_size {
                            self.unready()
                        }
                    }
                }
            }
            Ok(())
        }

        /// * Check if one block of data was fully decoded (at the beginning of a new block)
        pub fn on_new_block(&self) -> bool {
            (self.ready, self.input_count == 0) == (false, true)
        }

        /// * Uninitialize the decoder. Every time it finishes decoding a block, it needs to be uninitialized.
        /// * Another usage is for seeking. Before seeking, `unready()` should be called to uninitialize the decoder.
        pub fn unready(&mut self) {
            self.sample_val = 0;
            self.stepsize_index = 0;
            self.nibble_buffer.clear();
            self.input_count = 0;
            self.ready = false;
        }

        /// * Continuous feeding the decoder zero data until it finished decoding a whole block.
        pub fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            while !self.on_new_block() {
                let mut iter = [0u8].into_iter();
                self.decode(|| -> Option<u8> {iter.next()}, |sample: i16| {output(sample)})?;
            }
            Ok(())
        }
    }

    /// ### A wrapper for the `DecoderCore` to decode stereo audio.
    #[derive(Debug, Clone, Copy)]
    pub struct StereoDecoder {
        current_channel: CurrentChannel,
        core_l: DecoderCore,
        core_r: DecoderCore,
        nibble_l: DecoderNibbleBuffer,
        nibble_r: DecoderNibbleBuffer,
        sample_l: DecoderSampleBuffer,
        sample_r: DecoderSampleBuffer,
        block_size: usize,
    }

    /// ## The ADPCM-IMA Decoder for you to use.
    #[derive(Debug, Clone, Copy)]
    pub enum Decoder {
        Mono(DecoderCore),
        Stereo(StereoDecoder),
    }

    impl StereoDecoder {
        pub fn new(fmt_chunk: FmtChunk) -> Self {
            Self {
                current_channel: CurrentChannel::Left,
                core_l: DecoderCore::new(fmt_chunk),
                core_r: DecoderCore::new(fmt_chunk),
                nibble_l: DecoderNibbleBuffer::new(),
                nibble_r: DecoderNibbleBuffer::new(),
                sample_l: DecoderSampleBuffer::new(),
                sample_r: DecoderSampleBuffer::new(),
                block_size: (fmt_chunk.block_align / fmt_chunk.channels) as usize,
            }
        }

        /// * Uninitialize the decoder. Every time it finishes decoding a block, it needs to be uninitialized.
        /// * Another usage is for seeking. Before seeking, `unready()` should be called to uninitialize the decoder.
        pub fn unready(&mut self) {
            self.current_channel = CurrentChannel::Left;
            self.core_l.unready();
            self.core_r.unready();
            self.nibble_l.clear();
            self.nibble_r.clear();
            self.sample_l.clear();
            self.sample_r.clear();
        }

        /// * This `encode()` function needs 8 initial bytes to initialize 2 decoder cores, after being initialized, every 8 bytes decode to 16 samples.
        /// * The output samples are interleaved by channels.
        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            while let Some(nibble) = input() {
                match self.current_channel{
                    CurrentChannel::Left => {
                        self.nibble_l.push(nibble);
                        if self.nibble_l.is_full() {
                            self.current_channel = CurrentChannel::Right;
                        }
                    },
                    CurrentChannel::Right => {
                        self.nibble_r.push(nibble);
                        if self.nibble_r.is_full() {
                            self.current_channel = CurrentChannel::Left;
                            // It's time to process
                        }
                    },
                }
                if self.nibble_l.is_full() && self.nibble_r.is_full() {
                    let mut iter_l = mem::take(&mut self.nibble_l).into_iter();
                    let mut iter_r = mem::take(&mut self.nibble_r).into_iter();
                    self.core_l.decode(|| -> Option<u8> {iter_l.next()}, |sample:i16|{self.sample_l.push(sample)})?;
                    self.core_r.decode(|| -> Option<u8> {iter_r.next()}, |sample:i16|{self.sample_r.push(sample)})?;
                }
                let iter_l = mem::take(&mut self.sample_l).into_iter();
                let iter_r = mem::take(&mut self.sample_r).into_iter();
                for stereo in iter_l.zip(iter_r) {
                    output(stereo.0);
                    output(stereo.1);
                }
            }
            Ok(())
        }

        /// * Flush both decoder cores.
        /// Continuous feeding the decoder zero data until it finished decoding a whole block.
        pub fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            while !self.core_l.on_new_block() || !self.core_r.on_new_block() {
                let mut iter = [0u8].into_iter();
                self.decode(|| -> Option<u8> {iter.next()}, |sample: i16| {output(sample)})?;
            }
            Ok(())
        }
    }

    impl AdpcmDecoder for Decoder {
        fn new(fmt_chunk: FmtChunk) -> Result<Self, io::Error> where Self: Sized {
            match fmt_chunk.channels {
                1 => Ok(Decoder::Mono(DecoderCore::new(fmt_chunk))),
                2 => Ok(Decoder::Stereo(StereoDecoder::new(fmt_chunk))),
                other => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Wrong channel number \"{other}\" for ADPCM-IMA decoder."))),
            }
        }
        fn get_block_size(&self) -> usize {
            match self {
                Decoder::Mono(dec) => dec.block_size,
                Decoder::Stereo(dec) => dec.block_size,
            }
        }

        /// Each byte stores two 4-bit samples (packed as high/low nibbles).
        /// Effective decodable bytes per block: BLOCK_SIZE - HEADER_SIZE.
        /// Mono: Block size = BLOCK_SIZE.
        /// Stereo: Block size doubles (2×BLOCK_SIZE), but two samples (L+R) form one audio frame.
        /// Thus, total samples = (BLOCK_SIZE - HEADER_SIZE) × 2 samples (1 frame per stereo pair).
        fn frames_per_block(&self) -> usize {
            (self.get_block_size() - HEADER_SIZE) * 2
        }
        fn reset_states(&mut self) {
            match self {
                Decoder::Mono(dec) => dec.unready(),
                Decoder::Stereo(dec) => dec.unready(),
            }
        }
        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error>{
            match self {
                Decoder::Mono(dec) => dec.decode(|| -> Option<u8> {input()}, |sample:i16|{output(sample)}),
                Decoder::Stereo(dec) => dec.decode(|| -> Option<u8> {input()}, |sample:i16|{output(sample)}),
            }
        }
        fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            match self {
                Decoder::Mono(dec) => dec.flush(|sample:i16|{output(sample)}),
                Decoder::Stereo(dec) => dec.flush(|sample:i16|{output(sample)}),
            }
        }
    }
}


pub mod ms {
    // MS-ADPCM
    // https://ffmpeg.org/doxygen/3.1/adpcmenc_8c_source.html
    // https://ffmpeg.org/doxygen/3.1/adpcm_8c_source.html
    use std::io;

    use super::{AdpcmEncoder, AdpcmDecoder, CurrentChannel};
    use crate::copiablebuf::{CopiableBuffer};
    use crate::wavcore::{FmtChunk, FmtExtension, ExtensionData, AdpcmMsData};

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
    }
    
    impl Default for AdpcmCoeffSet {
        fn default() -> Self {
            Self::new()
        }
    }

    /// ## The core encoder of ADPCM-MS for mono-channel
    #[derive(Debug, Clone, Copy)]
    pub struct EncoderCore {
        sample1: i16,
        sample2: i16,
        coeff: AdpcmCoeffSet,
        delta: i32,
        ready: bool,
    }

    impl EncoderCore {
        pub fn new() -> Self {
            Self {
                sample1: 0,
                sample2: 0,
                coeff: AdpcmCoeffSet::new(),
                delta: 16,
                ready: false,
            }
        }

        /// * If the encoder is ready, put samples to get the encoded data.
        pub fn is_ready(&self) -> bool {
            self.ready
        }

        /// * Uninitialize the decoder. Every time it finishes decoding a block, it needs to be uninitialized.
        /// * Another usage is for seeking. Before seeking, `unready()` should be called to uninitialize the decoder.
        pub fn unready(&mut self) {
            self.ready = false;
        }

        /// * Encode one sample to a nibble
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
            let nibble = (nibble + bias) / self.delta;
            let nibble = nibble.clamp(-8, 7) & 0x0F;
            let predictor = predictor + if nibble & 0x08 != 0 {nibble | 0xFFFFFFF0u32 as i32} else {nibble} * self.delta;
            self.sample2 = self.sample1;
            self.sample1 = predictor.clamp(-32768, 32767) as i16;
            self.delta = (ADAPTATIONTABLE[nibble as usize] as i32 * self.delta) >> 8;
            if self.delta < 16 {
                self.delta = 16;
            }

            nibble as u8
        }

        /// * To be initialized, some data is needed for the encoder.
        /// * The encoder excretes 4 fields of data as the header of a block of audio data.
        pub fn get_ready(&mut self, samples: &[i16; 2], coeff_table: &[AdpcmCoeffSet; 7]) -> (u8, i16, i16, i16) {
            let predictor = 0u8;
            self.coeff = coeff_table[predictor as usize];
            self.sample2 = samples[0];
            self.sample1 = samples[1];
            self.ready = true;
            (predictor, self.delta as i16, self.sample1, self.sample2)
        }
    }

    impl Default for EncoderCore {
        fn default() -> Self {
            Self::new()
        }
    }

    fn output_le_i16(val: i16, mut output: impl FnMut(u8)) {
        let bytes = val.to_le_bytes();
        output(bytes[0]);
        output(bytes[1]);
    }

    /// ### A wrapper for the `EncoderCore` to encode stereo audio.
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

        /// * If the encoder is ready, put samples to get the encoded data.
        pub fn is_ready(&self) -> bool {
            self.ready
        }

        /// * Uninitialize the decoder. Every time it finishes decoding a block, it needs to be uninitialized.
        /// * Another usage is for seeking. Before seeking, `unready()` should be called to uninitialize the decoder.
        pub fn unready(&mut self) {
            self.core_l.unready();
            self.core_r.unready();
            self.current_channel = CurrentChannel::Left;
            self.ready = false;
        }

        /// * Encode one sample to a nibble, interleaved by channels
        pub fn compress_sample(&mut self, sample: i16) -> u8 {
            match self.current_channel {
                CurrentChannel::Left => {
                    self.current_channel = CurrentChannel::Right;
                    self.core_l.compress_sample(sample)
                },
                CurrentChannel::Right => {
                    self.current_channel = CurrentChannel::Left;
                    self.core_r.compress_sample(sample)
                },
            }
        }

        /// * To be initialized, some data is needed for the encoder.
        /// * The encoder excretes 4 fields of data as the header of a block of audio data.
        /// * There are two of the encoder cores to be initialized, so 8 fields of data are to be excreted.
        pub fn get_ready(&mut self, samples: &[i16; 4], coeff_table: &[AdpcmCoeffSet; 7]) -> (u8, u8, i16, i16, i16, i16, i16, i16) {
            let ready1 = self.core_l.get_ready(&[samples[0], samples[2]], coeff_table);
            let ready2 = self.core_r.get_ready(&[samples[1], samples[3]], coeff_table);
            self.ready = true;
            (
                ready1.0, ready2.0,
                ready1.1, ready2.1,
                ready1.2, ready2.2,
                ready1.3, ready2.3
            )
        }
    }

    impl Default for StereoEncoder {
        fn default() -> Self {
            Self::new()
        }
    }

    type EncoderBuffer = CopiableBuffer<i16, 4>;

    /// ## The encoder is for your use
    #[derive(Debug, Clone, Copy)]
    pub struct Encoder{
        channels: Channels,
        coeff_table: [AdpcmCoeffSet; 7],
        bytes_yield: usize,
        buffer: EncoderBuffer,
    }

    impl Encoder {
        pub fn is_ready(&self) -> bool {
            match self.channels {
                Channels::Mono(enc) => enc.is_ready(),
                Channels::Stereo(enc) => enc.is_ready(),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Channels{
        Mono(EncoderCore),
        Stereo(StereoEncoder),
    }

    impl AdpcmEncoder for Encoder {
        fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized {
            match channels {
                1 => {
                    Ok(Self {
                        channels: Channels::Mono(EncoderCore::new()),
                        coeff_table: DEF_COEFF_TABLE,
                        bytes_yield: 0,
                        buffer: EncoderBuffer::new(),
                    })
                },
                2 => {
                    Ok(Self {
                        channels: Channels::Stereo(StereoEncoder::new()),
                        coeff_table: DEF_COEFF_TABLE,
                        bytes_yield: 0,
                        buffer: EncoderBuffer::new(),
                    })
                },
                o => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Channels must be 1 or 2, not {o}"))),
            }
        }

        /// * When uninitialized, it asks for data to initialize the encoder cores.
        /// * On initialization, every encoder cores excrete its data for the header of the block.
        /// * When ready to encode, for stereo, each byte contains 2 nibbles, one for the left channel, and another for the right channel.
        /// * Same as other `encode()` functions, feed it `Some(sample)` keep it continuously asking for new samples, and feed it `None` to let it return.
        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                self.buffer.push(sample);
                if !self.is_ready() {
                    match self.channels {
                        Channels::Mono(ref mut enc) => {
                            if self.buffer.len() == 2 {
                                let header = enc.get_ready(&[self.buffer[0], self.buffer[1]], &self.coeff_table);
                                output(header.0);
                                output_le_i16(header.1, |byte:u8|{output(byte)});
                                output_le_i16(header.2, |byte:u8|{output(byte)});
                                output_le_i16(header.3, |byte:u8|{output(byte)});
                                self.buffer.clear();
                                self.bytes_yield += 7;
                            }
                        },
                        Channels::Stereo(ref mut enc) => {
                            if self.buffer.len() >= 4 {
                                let header = enc.get_ready(&[self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3]], &self.coeff_table);
                                output(header.0);
                                output(header.1);
                                output_le_i16(header.2, |byte:u8|{output(byte)});
                                output_le_i16(header.3, |byte:u8|{output(byte)});
                                output_le_i16(header.4, |byte:u8|{output(byte)});
                                output_le_i16(header.5, |byte:u8|{output(byte)});
                                output_le_i16(header.6, |byte:u8|{output(byte)});
                                output_le_i16(header.7, |byte:u8|{output(byte)});
                                self.buffer.clear();
                                self.bytes_yield += 14;
                            }
                        },
                    }
                } else {
                    match self.channels {
                        Channels::Mono(ref mut enc) => {
                            if self.buffer.len() == 2 {
                                let h = enc.compress_sample(self.buffer[0]);
                                let l = enc.compress_sample(self.buffer[1]);
                                output(l | (h << 4));
                                self.buffer.clear();
                                self.bytes_yield += 1;
                            }
                            if self.bytes_yield == BLOCK_SIZE {
                                enc.unready();
                                self.bytes_yield = 0;
                            }
                        },
                        Channels::Stereo(ref mut enc) => {
                            if self.buffer.len() == 4 {
                                let h1 = enc.compress_sample(self.buffer[0]);
                                let l1 = enc.compress_sample(self.buffer[1]);
                                let h2 = enc.compress_sample(self.buffer[2]);
                                let l2 = enc.compress_sample(self.buffer[3]);
                                output(l1 | (h1 << 4));
                                output(l2 | (h2 << 4));
                                self.buffer.clear();
                                self.bytes_yield += 2;
                            }
                            if self.bytes_yield == BLOCK_SIZE * 2 {
                                enc.unready();
                                self.bytes_yield = 0;
                            }
                        },
                    }
                }
            }
            Ok(())
        }

        /// * The ADPCM-MS has specific `fmt ` chunk extension data to store the coeff table.
        fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16) -> Result<FmtChunk, io::Error> {
            if bits_per_sample != 4 {
                eprintln!("For ADPCM-MS, bits_per_sample bust be 4, the value `{bits_per_sample}` is ignored.");
            }
            let bits_per_sample = 4u16;
            let block_align = BLOCK_SIZE as u16 * channels;
            Ok(FmtChunk {
                format_tag: 0x0002,
                channels,
                sample_rate,
                byte_rate: sample_rate * bits_per_sample as u32 * channels as u32 / 8,
                block_align,
                bits_per_sample,
                extension: Some(FmtExtension::new_adpcm_ms(AdpcmMsData{
                    samples_per_block: (BLOCK_SIZE as u16 - HEADER_SIZE as u16) * 2 * channels,
                    num_coeff: self.coeff_table.len() as u16,
                    coeffs: self.coeff_table,
                })),
            })
        }

        /// * When encoding is ended, call this to update statistics data for the `fmt ` chunk.
        fn modify_fmt_chunk(&self, fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
            fmt_chunk.block_align = BLOCK_SIZE as u16 * fmt_chunk.channels;
            fmt_chunk.bits_per_sample = 4;
            fmt_chunk.byte_rate = fmt_chunk.sample_rate * fmt_chunk.channels as u32 * fmt_chunk.bits_per_sample as u32 / 8;
            if let Some(extension) = fmt_chunk.extension {
                if let ExtensionData::AdpcmMs(mut adpcm_ms) = extension.data {
                    adpcm_ms.samples_per_block = (BLOCK_SIZE as u16 - HEADER_SIZE as u16) * 2 * fmt_chunk.channels;
                    adpcm_ms.num_coeff = self.coeff_table.len() as u16;
                    adpcm_ms.coeffs = self.coeff_table;
                    Ok(())
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidData, format!("Wrong extension data stored in the `fmt ` chunk for ADPCM-MS: {:?}", extension)))
                }
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidData, "For ADPCM-MS, must store the extension data in the `fmt ` chunk".to_owned()))
            }
        }

        /// * Excrete the last data.
        fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while self.bytes_yield != 0 {
                let mut iter = [0i16].into_iter();
                self.encode(|| -> Option<i16> {iter.next()}, |nibble: u8| {output(nibble)})?;
            }
            Ok(())
        }
    }

    /// ## This is the decoder core for mono-channel decoding
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderCore {
        sample1: i16,
        sample2: i16,
        coeff: AdpcmCoeffSet,
        delta: i32,
        ready: bool,
        coeff_table: [AdpcmCoeffSet; 7],
        header_buffer: CopiableBuffer<u8, HEADER_SIZE>,
        bytes_eaten: usize,
        max_bytes_can_eat: usize,
    }

    /// * The header data for the decoder to initialize.
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderBreakfast {
        predictor: u8,
        delta: i16,
        sample1: i16,
        sample2: i16,
    }

    impl DecoderCore{
        /// * If `fmt_chunk` doesn't have the extension data, use the default coeff table.
        pub fn new(fmt_chunk: FmtChunk) -> Self {
            Self {
                sample1: 0,
                sample2: 0,
                coeff: AdpcmCoeffSet::new(),
                delta: 0,
                ready: false,
                coeff_table: match fmt_chunk.extension {
                    None => DEF_COEFF_TABLE,
                    Some(extension) => {
                        if extension.ext_len < 12 {
                            DEF_COEFF_TABLE
                        } else {
                            match extension.data {
                                ExtensionData::AdpcmMs(adpcm_ms) => {
                                    if adpcm_ms.num_coeff != 7 {
                                        DEF_COEFF_TABLE
                                    } else {
                                        adpcm_ms.coeffs
                                    }
                                },
                                _ => DEF_COEFF_TABLE,
                            }
                        }
                    },
                },
                header_buffer: CopiableBuffer::<u8, 7>::new(),
                bytes_eaten: 0,
                max_bytes_can_eat: fmt_chunk.block_align as usize,
            }
        }

        /// * Uncompress a nibble to a sample
        pub fn expand_nibble(&mut self, nibble: u8) -> i16 {
            let predictor = (
                self.sample1 as i32 * self.coeff.get(1) as i32 +
                self.sample2 as i32 * self.coeff.get(2) as i32) / 256;
            let nibble = nibble as i32;
            let predictor = predictor + if (nibble & 0x08) != 0 {nibble.wrapping_sub(0x10)} else {nibble} * self.delta;

            self.sample2 = self.sample1;
            self.sample1 = predictor.clamp(-32768, 32767) as i16;
            self.delta = ((ADAPTATIONTABLE[nibble as usize] as i32 * self.delta) >> 8).clamp(16, i32::MAX / 768);

            self.sample1
        }

        /// * Check if breakfast was eaten, and ready to decode.
        pub fn is_ready(&self) -> bool {
            self.ready
        }

        /// * Puke the breakfast, and reset to the initial state.
        pub fn unready(&mut self) {
            self.header_buffer.clear();
            self.bytes_eaten = 0;
            self.ready = false;
        }

        /// * How many bytes were eaten, should not exceed the block size in the `fmt ` chunk.
        #[allow(dead_code)]
        pub fn get_bytes_eaten(&self) -> usize {
            self.bytes_eaten
        }

        /// * The block size in the `fmt ` chunk.
        pub fn get_max_bytes_can_eat(&self) -> usize {
            self.max_bytes_can_eat
        }

        /// * Provide the header data of the ADPCM-MS for the decoder core as the breakfast.
        pub fn get_ready(&mut self, breakfast: &DecoderBreakfast, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            if breakfast.predictor > 6 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("When decoding ADPCM-MS: predictor is {} and it's greater than 6", breakfast.predictor)));
            }
            self.coeff = self.coeff_table[breakfast.predictor as usize];
            self.delta = breakfast.delta as i32;
            self.sample1 = breakfast.sample1;
            self.sample2 = breakfast.sample2;
            self.ready = true;
            self.bytes_eaten += 7;
            output(breakfast.sample2);
            output(breakfast.sample1);
            Ok(())
        }
    }

    impl DecoderBreakfast {
        pub fn from_bytes_mono(bytes: &[u8; 7]) -> Self {
            Self {
                predictor: bytes[0],
                delta: i16::from_le_bytes([bytes[1], bytes[2]]),
                sample1: i16::from_le_bytes([bytes[3], bytes[4]]),
                sample2: i16::from_le_bytes([bytes[5], bytes[6]]),
            }
        }

        pub fn from_bytes_stereo(bytes: &[u8; 14]) -> (Self, Self) {
            let l = Self::from_bytes_mono(&[bytes[0], bytes[2], bytes[3], bytes[6], bytes[7], bytes[10], bytes[11]]);
            let r = Self::from_bytes_mono(&[bytes[1], bytes[4], bytes[5], bytes[8], bytes[9], bytes[12], bytes[13]]);
            (l, r)
        }
    }


    /// ## The decoder with two cores to decode the stereo audio.
    #[derive(Debug, Clone, Copy)]
    pub struct StereoDecoder {
        core_l: DecoderCore,
        core_r: DecoderCore,
        bytes_eaten: usize,
        max_bytes_can_eat: usize,
        ready: bool,
    }

    /// ## The decoder is for your use
    #[derive(Debug, Clone, Copy)]
    pub enum Decoder {
        Mono(DecoderCore),
        Stereo(StereoDecoder),
    }

    impl StereoDecoder {
        pub fn new(fmt_chunk: FmtChunk) -> Self {
            Self {
                core_l: DecoderCore::new(fmt_chunk),
                core_r: DecoderCore::new(fmt_chunk),
                bytes_eaten: 0,
                max_bytes_can_eat: fmt_chunk.block_align as usize,
                ready: false,
            }
        }

        /// * Check if breakfast was eaten, and ready to decode.
        pub fn is_ready(&self) -> bool {
            self.ready
        }

        /// * Puke the breakfast, and reset to the initial state.
        pub fn unready(&mut self) {
            self.core_l.unready();
            self.core_r.unready();
            self.bytes_eaten = 0;
            self.ready = false;
        }

        /// * How many bytes were eaten, should not exceed the block size in the `fmt ` chunk.
        #[allow(dead_code)]
        pub fn get_bytes_eaten(&self) -> usize {
            self.bytes_eaten
        }

        /// * The block size in the `fmt ` chunk.
        pub fn get_max_bytes_can_eat(&self) -> usize {
            self.max_bytes_can_eat
        }

        /// * Provide the header data of the ADPCM-MS for the decoder cores as the breakfast.
        pub fn get_ready(&mut self, breakfast_l: &DecoderBreakfast, breakfast_r: &DecoderBreakfast, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut sample_buffer = CopiableBuffer::<i16, 4>::new();
            self.core_l.get_ready(breakfast_l, |sample:i16|{sample_buffer.push(sample);})?;
            self.core_r.get_ready(breakfast_r, |sample:i16|{sample_buffer.push(sample);})?;
            output(sample_buffer[0]);
            output(sample_buffer[2]);
            output(sample_buffer[1]);
            output(sample_buffer[3]);
            self.bytes_eaten += 14;
            self.ready = true;
            Ok(())
        }
    }

    impl Decoder {
        pub fn new(fmt_chunk: FmtChunk) -> Result<Self, io::Error> {
            match fmt_chunk.channels {
                1 => Ok(Self::Mono(DecoderCore::new(fmt_chunk))),
                2 => Ok(Self::Stereo(StereoDecoder::new(fmt_chunk))),
                other => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Wrong channel number \"{other}\" for ADPCM-MS decoder."))),
            }
        }

        /// * Check if breakfast was eaten, and ready to decode.
        #[allow(dead_code)]
        pub fn is_ready(&self) -> bool {
            match self {
                Self::Mono(mono) => mono.is_ready(),
                Self::Stereo(stereo) => stereo.is_ready(),
            }
        }

        /// * Puke the breakfast, and reset to the initial state.
        pub fn unready(&mut self) {
            match self {
                Self::Mono(mono) => mono.unready(),
                Self::Stereo(stereo) => stereo.unready(),
            }
        }

        /// * How many bytes were eaten, should not exceed the block size in the `fmt ` chunk.
        #[allow(dead_code)]
        pub fn get_bytes_eaten(&self) -> usize {
            match self {
                Self::Mono(mono) => mono.get_bytes_eaten(),
                Self::Stereo(stereo) => stereo.get_bytes_eaten(),
            }
        }

        /// * The block size in the `fmt ` chunk.
        pub fn get_max_bytes_can_eat(&self) -> usize {
            match self {
                Self::Mono(mono) => mono.get_max_bytes_can_eat(),
                Self::Stereo(stereo) => stereo.get_max_bytes_can_eat(),
            }
        }

        /// * Get the number of the channels
        pub fn get_channels(&self) -> usize {
            match self {
                Self::Mono(_) => 1,
                Self::Stereo(_) => 2,
            }
        }
    }

    impl AdpcmDecoder for Decoder {
        fn new(fmt_chunk: FmtChunk) -> Result<Self, io::Error> where Self: Sized {
            Self::new(fmt_chunk)
        }

        fn get_block_size(&self) -> usize {
            self.get_max_bytes_can_eat() / self.get_channels()
        }

        /// Each byte stores two 4-bit samples (packed as high/low nibbles).
        /// Effective decodable bytes per block: BLOCK_SIZE - HEADER_SIZE.
        /// Mono: Block size = BLOCK_SIZE.
        /// Stereo: Block size doubles (2×BLOCK_SIZE), but two samples (L+R) form one audio frame.
        /// Thus, total samples = (BLOCK_SIZE - HEADER_SIZE) × 2 samples (1 frame per stereo pair).
        fn frames_per_block(&self) -> usize {
            (self.get_block_size() - HEADER_SIZE) * 2
        }

        fn reset_states(&mut self) {
            self.unready()
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            while let Some(byte) = input() {
                match self {
                    Self::Mono(mono) => {
                        if !mono.is_ready() {
                            mono.header_buffer.push(byte);
                            if mono.header_buffer.is_full() {
                                let breakfast = DecoderBreakfast::from_bytes_mono(mono.header_buffer.get_array());
                                mono.get_ready(&breakfast, |sample:i16|{output(sample)})?;
                            }
                        } else {
                            output(mono.expand_nibble(byte >> 4));
                            output(mono.expand_nibble(byte & 0x0F));
                            mono.bytes_eaten += 1;
                            if mono.bytes_eaten >= mono.max_bytes_can_eat {
                                mono.unready();
                            }
                        }
                    },
                    Self::Stereo(stereo) => {
                        if !stereo.is_ready() {
                            if !stereo.core_l.header_buffer.is_full() {
                                stereo.core_l.header_buffer.push(byte);
                            } else {
                                stereo.core_r.header_buffer.push(byte);
                            }
                            if stereo.core_r.header_buffer.is_full() {
                                let bytes = stereo.core_l.header_buffer.into_iter().chain(stereo.core_r.header_buffer.into_iter()).collect::<CopiableBuffer<u8, 14>>();
                                let (breakfast_l, breakfast_r) = DecoderBreakfast::from_bytes_stereo(&bytes.into_array());
                                stereo.get_ready(&breakfast_l, &breakfast_r, |sample:i16|{output(sample)})?;
                            }
                        } else {
                            output(stereo.core_l.expand_nibble(byte >> 4));
                            output(stereo.core_r.expand_nibble(byte & 0x0F));
                            stereo.bytes_eaten += 1;
                            if stereo.bytes_eaten >= stereo.max_bytes_can_eat {
                                stereo.unready();
                            }
                        }
                    },
                }
            }
            Ok(())
        }

        fn flush(&mut self, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            match self {
                Self::Mono(mono) => {
                    if mono.bytes_eaten > 0 && mono.bytes_eaten < mono.max_bytes_can_eat {
                        let mut food = vec![0; mono.max_bytes_can_eat - mono.bytes_eaten].into_iter();
                        self.decode(|| -> Option<u8> {food.next()}, |sample:i16|{output(sample)})?;
                    }
                },
                Self::Stereo(stereo) => {
                    if stereo.bytes_eaten > 0 && stereo.bytes_eaten < stereo.max_bytes_can_eat {
                        let mut food = vec![0; stereo.max_bytes_can_eat - stereo.bytes_eaten].into_iter();
                        self.decode(|| -> Option<u8> {food.next()}, |sample:i16|{output(sample)})?;
                    }
                },
            }
            Ok(())
        }
    }
}

pub mod yamaha {
    use std::{io, cmp::min};

    use super::{AdpcmEncoder, AdpcmDecoder};
    use crate::copiablebuf::{CopiableBuffer};
    use crate::wavcore::{FmtChunk};

    const BLOCK_SIZE: usize = 1024;

    const YAMAHA_INDEXSCALE: [i16; 16] = [
        230, 230, 230, 230, 307, 409, 512, 614,
        230, 230, 230, 230, 307, 409, 512, 614
    ];

    const YAMAHA_DIFFLOOKUP: [i8; 16] = [
         1,  3,  5,  7,  9,  11,  13,  15,
        -1, -3, -5, -7, -9, -11, -13, -15
    ];

    #[derive(Debug, Clone, Copy)]
    struct YamahaCodecCore {
        predictor: i32,
        step: i32,
    }

    impl YamahaCodecCore {
        pub fn new() -> Self {
            Self {
                predictor: 0,
                step: 127,
            }
        }

        /// * Compress one sample to a nibble
        pub fn compress_sample(&mut self, sample: i16) -> u8 {
            let delta = sample as i32 - self.predictor;
            let nibble = min(7, delta.abs() * 4 / self.step) + if delta < 0 {8} else {0};
            self.predictor += (self.step * YAMAHA_DIFFLOOKUP[nibble as usize] as i32 / 8).clamp(-32768, 32767);
            self.step = ((self.step * YAMAHA_INDEXSCALE[nibble as usize] as i32) >> 8).clamp(127, 24576);
            nibble as u8
        }

        /// * Uncompress a nibble to a sample
        pub fn expand_nibble(&mut self, nibble: u8) -> i16 {
            self.predictor += (self.step * YAMAHA_DIFFLOOKUP[nibble as usize] as i32 / 8).clamp(-32768, 32767);
            self.step = ((self.step * YAMAHA_INDEXSCALE[nibble as usize] as i32) >> 8).clamp(127, 24576);
            self.predictor as i16
        }

        /// * Compress two samples, combine two encoded nibbles to a bytes
        pub fn encode_sample(&mut self, samples: &[i16; 2]) -> u8 {
            let l = self.compress_sample(samples[0]);
            let h = self.compress_sample(samples[1]);
            l | (h << 4)
        }

        /// * Decode a byte as two nibbles, and return two samples.
        pub fn decode_sample(&mut self, nibble: u8) -> [i16; 2] {
            [self.expand_nibble(nibble & 0x0F), self.expand_nibble(nibble >> 4)]
        }
    }

    /// ## The mono-channel encoder
    #[derive(Debug, Clone, Copy)]
    pub struct EncoderMono {
        core: YamahaCodecCore,
        buffer: CopiableBuffer<i16, 2>,
    }

    impl EncoderMono {
        pub fn new() -> Self {
            Self {
                core: YamahaCodecCore::new(),
                buffer: CopiableBuffer::<i16, 2>::new(),
            }
        }

        /// * Encode two samples, combine two encoded nibbles to a bytes
        pub fn encode_sample(&mut self, samples: &[i16; 2]) -> u8 {
            self.core.encode_sample(samples)
        }

        /// * The `encode()` function, uses two closures to eat/excrete data.
        /// * It will endlessly ask for new samples, and feed it `None` to let the function return.
        /// * To continue encoding, call it again and feed it data.
        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) {
            while let Some(sample) = input() {
                self.buffer.push(sample);
                if self.buffer.is_full() {
                    output(self.encode_sample(&{*self.buffer.get_array()}));
                    self.buffer.clear();
                }
            }
        }

        /// * Excrete the last data
        pub fn flush(&mut self, mut output: impl FnMut(u8)) {
            if self.buffer.is_empty() {
                return;
            }
            while !self.buffer.is_full() {
                self.buffer.push(0);
            }
            output(self.core.encode_sample(&{*self.buffer.get_array()}));
            self.buffer.clear();
        }
    }

    impl Default for EncoderMono {
        fn default() -> Self {
            Self::new()
        }
    }

    /// ## The stereo-channel encoder
    #[derive(Debug, Clone, Copy)]
    pub struct EncoderStereo {
        core_l: YamahaCodecCore,
        core_r: YamahaCodecCore,
        buffer: CopiableBuffer<i16, 2>,
    }

    impl EncoderStereo {
        pub fn new() -> Self {
            Self {
                core_l: YamahaCodecCore::new(),
                core_r: YamahaCodecCore::new(),
                buffer: CopiableBuffer::<i16, 2>::new(),
            }
        }

        /// * The function distributes two samples to each encoder core and returns a byte containing two nibbles for each channel.
        pub fn encode_sample(&mut self, samples: &[i16; 2]) -> u8 {
            let l = self.core_l.compress_sample(samples[0]);
            let h = self.core_r.compress_sample(samples[1]);
            l | (h << 4)
        }

        /// * The `encode()` function, uses two closures to eat/excrete data.
        /// * It will endlessly ask for new samples, and feed it `None` to let the function return.
        /// * To continue encoding, call it again and feed it data.
        pub fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) {
            while let Some(sample) = input() {
                self.buffer.push(sample);
                if self.buffer.is_full() {
                    output(self.encode_sample(&{*self.buffer.get_array()}));
                    self.buffer.clear();
                }
            }
        }

        /// * Excrete the last data
        pub fn flush(&mut self, mut output: impl FnMut(u8)) {
            if self.buffer.is_empty() {
                return;
            }
            while !self.buffer.is_full() {
                self.buffer.push(0);
            }
            output(self.encode_sample(&{*self.buffer.get_array()}));
            self.buffer.clear();
        }
    }

    impl Default for EncoderStereo {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Encoder {
        Mono(EncoderMono),
        Stereo(EncoderStereo),
    }

    impl AdpcmEncoder for Encoder {
        fn new(channels: u16) -> Result<Self, io::Error> where Self: Sized {
            match channels {
                1 => Ok(Self::Mono(EncoderMono::new())),
                2 => Ok(Self::Stereo(EncoderStereo::new())),
                o => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Channels must be 1 or 2, not {o}"))),
            }
        }

        /// * The `encode()` function, uses two closures to eat/excrete data.
        /// * It will endlessly ask for new samples, and feed it `None` to let the function return.
        /// * To continue encoding, call it again and feed it data.
        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            match self {
                Self::Mono(enc) => enc.encode(||{input()},|nibble|{output(nibble)}),
                Self::Stereo(enc) => enc.encode(||{input()},|nibble|{output(nibble)}),
            }
            Ok(())
        }

        fn new_fmt_chunk(&mut self, channels: u16, sample_rate: u32, bits_per_sample: u16) -> Result<FmtChunk, io::Error> {
            if bits_per_sample != 4 {
                eprintln!("For ADPCM-YAMAHA, bits_per_sample bust be 4, the value `{bits_per_sample}` is ignored.");
            }
            let bits_per_sample = 4u16;
            let block_align = BLOCK_SIZE as u16;
            Ok(FmtChunk {
                format_tag: 0x0020,
                channels,
                sample_rate,
                byte_rate: sample_rate * bits_per_sample as u32 * channels as u32 / 8,
                block_align,
                bits_per_sample,
                extension: None,
            })
        }

        fn modify_fmt_chunk(&self, _fmt_chunk: &mut FmtChunk) -> Result<(), io::Error> {
            Ok(())
        }

        fn flush(&mut self, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            match self {
                Self::Mono(enc) => enc.flush(|nibble|{output(nibble)}),
                Self::Stereo(enc) => enc.flush(|nibble|{output(nibble)}),
            }
            Ok(())
        }
    }

    /// ## The mono-channel decoder
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderMono {
        core: YamahaCodecCore,
    }

    impl DecoderMono {
        pub fn new() -> Self {
            Self {
                core: YamahaCodecCore::new(),
            }
        }

        /// * Each byte contains two nibbles to decode to 2 samples.
        pub fn decode_sample(&mut self, nibble: u8) -> [i16; 2] {
            self.core.decode_sample(nibble)
        }

        /// * The `decode()` function, uses two closures to eat/excrete data.
        /// * It will endlessly ask for new data, and feed it `None` to let the function return.
        /// * To continue encoding, call it again and feed it data.
        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) {
            while let Some(nibble) = input() {
                self.decode_sample(nibble).into_iter().for_each(|sample|{output(sample)});
            }
        }
    }

    impl Default for DecoderMono {
        fn default() -> Self {
            Self::new()
        }
    }

    /// ## The stereo-channel decoder
    #[derive(Debug, Clone, Copy)]
    pub struct DecoderStereo {
        core_l: YamahaCodecCore,
        core_r: YamahaCodecCore,
    }

    impl DecoderStereo {
        pub fn new() -> Self {
            Self {
                core_l: YamahaCodecCore::new(),
                core_r: YamahaCodecCore::new(),
            }
        }

        /// * Each byte contains two nibbles to decode to 2 samples for left and right channels.
        pub fn decode_sample(&mut self, nibble: u8) -> [i16; 2] {
            [self.core_l.expand_nibble(nibble & 0x0F), self.core_r.expand_nibble(nibble >> 4)]
        }

        /// * The `decode()` function, uses two closures to eat/excrete data.
        /// * It will endlessly ask for new data, and feed it `None` to let the function return.
        /// * To continue encoding, call it again and feed it data.
        pub fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) {
            while let Some(nibble) = input() {
                self.decode_sample(nibble).into_iter().for_each(|sample|{output(sample)});
            }
        }
    }

    impl Default for DecoderStereo {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Decoder {
        Mono(DecoderMono),
        Stereo(DecoderStereo),
    }

    impl Decoder {
        pub fn get_channels(&self) -> usize {
            match self {
                Self::Mono(_) => 1,
                Self::Stereo(_) => 2,
            }
        }
    }

    impl AdpcmDecoder for Decoder {
        fn new(fmt_chunk: FmtChunk) -> Result<Self, io::Error> where Self: Sized {
            match fmt_chunk.channels {
                1 => Ok(Self::Mono(DecoderMono::new())),
                2 => Ok(Self::Stereo(DecoderStereo::new())),
                o => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Channels must be 1 or 2, not {o}"))),
            }
        }

        fn get_block_size(&self) -> usize {
            BLOCK_SIZE
        }

        fn frames_per_block(&self) -> usize {
            BLOCK_SIZE * 2 / self.get_channels()
        }

        fn reset_states(&mut self) {
            match self {
                Self::Mono(dec) => *dec = DecoderMono::new(),
                Self::Stereo(dec) => *dec = DecoderStereo::new(),
            }
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            match self {
                Self::Mono(dec) => dec.decode(||{input()},|sample|{output(sample)}),
                Self::Stereo(dec) => dec.decode(||{input()},|sample|{output(sample)}),
            }
            Ok(())
        }

        fn flush(&mut self, _output: impl FnMut(i16)) -> Result<(), io::Error> {
            Ok(())
        }
    }
}
