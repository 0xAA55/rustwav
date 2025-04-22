#![allow(dead_code)]

use std::{any::type_name, io::{self, ErrorKind}, fmt::{self, Debug, Display, Formatter}, slice, ffi::{CStr, c_void}, ptr};

use libflac_sys::*;

#[derive(Debug, Clone, Copy)]
pub enum FlacCompression {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
    Level6 = 6,
    Level7 = 7,
    Level8 = 8
}

pub trait FlacError {
    fn get_code(&self) -> u32;
    fn get_message(&self) -> &'static str;
    fn get_function(&self) -> &'static str;
    fn get_message_from_code(&self) -> &'static str;

    fn format(&self, f: &mut Formatter) -> fmt::Result {
        let code = self.get_code();
        let message = self.get_message();
        let function = self.get_function();
        write!(f, "Code: {code}, function: {function}, message: {message}")?;
        Ok(())
    }
}

macro_rules! impl_FlacError {
    ($error:ty) => {
        impl FlacError for $error {
            fn get_code(&self) -> u32 {self.code}
            fn get_message(&self) -> &'static str {self.message}
            fn get_function(&self) -> &'static str {self.function}
            fn get_message_from_code(&self) -> &'static str {
                Self::get_message_from_code(self.get_code())
            }
        }

        impl std::error::Error for $error {}

        impl Display for $error {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                <$error as FlacError>::format(self, f)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacEncoderError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamEncoderStateString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacEncoderError);

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderInitError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacEncoderInitError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamEncoderInitStatusString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacEncoderInitError);

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderParams {
    pub verify_decoded: bool,
    pub compression: FlacCompression,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u32,
    pub total_samples_estimate: u64,
}

impl FlacEncoderParams {
    pub fn new() -> Self {
        Self {
            verify_decoded: false,
            compression: FlacCompression::Level5,
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            total_samples_estimate: 0,
        }
    }
}

pub struct FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    encoder: *mut FLAC__StreamEncoder,
    params: FlacEncoderParams,
    on_write: Wr,
    on_seek: Sk,
    on_tell: Tl,
}

impl<Wr, Sk, Tl> FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    pub fn new(
        on_write: Wr,
        on_seek: Sk,
        on_tell: Tl,
        params: &FlacEncoderParams
    ) -> Result<Self, FlacEncoderError> {
        let ret = Self {
            encoder: unsafe {FLAC__stream_encoder_new()},
            params: *params,
            on_write,
            on_seek,
            on_tell,
        };
        if ret.encoder.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__stream_encoder_new"))
        } else {
            Ok(ret)
        }
    }

    fn get_status_as_result(&self, function: &'static str) -> Result<(), FlacEncoderError> {
        let code = unsafe {FLAC__stream_encoder_get_state(self.encoder)};
        if code == 0 {
            Ok(())
        } else {
            Err(FlacEncoderError::new(code, function))
        }
    }

    fn get_status_as_error(&self, function: &'static str) -> Result<(), FlacEncoderError> {
        let code = unsafe {FLAC__stream_encoder_get_state(self.encoder)};
        Err(FlacEncoderError::new(code, function))
    }

    fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }

    fn init(&mut self) -> Result<(), FlacEncoderError> {
        unsafe {
            if FLAC__stream_encoder_set_verify(self.encoder, if self.params.verify_decoded {1} else {0}) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_verify");
            }
            if FLAC__stream_encoder_set_compression_level(self.encoder, self.params.compression as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_compression_level");
            }
            if FLAC__stream_encoder_set_channels(self.encoder, self.params.channels as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_channels");
            }
            if FLAC__stream_encoder_set_bits_per_sample(self.encoder, self.params.bits_per_sample) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_bits_per_sample");
            }
            if FLAC__stream_encoder_set_sample_rate(self.encoder, self.params.sample_rate) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_sample_rate");
            }
            if self.params.total_samples_estimate > 0 {
                if FLAC__stream_encoder_set_total_samples_estimate(self.encoder, self.params.total_samples_estimate) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_set_total_samples_estimate");
                }
            }
            let ret = FLAC__stream_encoder_init_stream(self.encoder,
                Some(Self::write_callback),
                Some(Self::seek_callback),
                Some(Self::tell_callback),
                Some(Self::metadata_callback),
                self.as_mut_ptr() as *mut c_void,
            );
            if ret != 0 {
                return Err(FlacEncoderError {
                    code: ret,
                    message: FlacEncoderInitError::get_message_from_code(ret),
                    function: "FLAC__stream_encoder_init_stream",
                });
            }
        }
        self.get_status_as_result("FlacEncoderUnmovable::Init()")
    }

    unsafe extern "C" fn write_callback(_encoder: *const FLAC__StreamEncoder, buffer: *const u8, bytes: usize, _samples: u32, _current_frame: u32, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_write)(slice::from_raw_parts(buffer, bytes)) {
            Ok(_) => FLAC__STREAM_ENCODER_WRITE_STATUS_OK,
            Err(e) => {
                eprintln!("On `write_callback()`: {:?}", e);
                FLAC__STREAM_ENCODER_WRITE_STATUS_FATAL_ERROR
            },
        }
    }

    unsafe extern "C" fn seek_callback(_encoder: *const FLAC__StreamEncoder, absolute_byte_offset: u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_seek)(absolute_byte_offset) {
            Ok(_) => FLAC__STREAM_ENCODER_SEEK_STATUS_OK,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_ENCODER_SEEK_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_ENCODER_SEEK_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn tell_callback(_encoder: *const FLAC__StreamEncoder, absolute_byte_offset: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_tell)() {
            Ok(offset) => {
                *absolute_byte_offset = offset;
                FLAC__STREAM_ENCODER_TELL_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_ENCODER_TELL_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_ENCODER_TELL_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn metadata_callback(_encoder: *const FLAC__StreamEncoder, metadata: *const FLAC__StreamMetadata, client_data: *mut c_void) {
        let _this = &mut *(client_data as *mut Self);
        let _meta = &*(metadata as *const FLAC__StreamMetadata);
    }

    pub fn write_monos(&mut self, monos: &[i32]) -> Result<(), FlacEncoderError> {
        if monos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => unsafe {
                if FLAC__stream_encoder_process_interleaved(self.encoder, monos.as_ptr() as *const i32, monos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            2 => self.write_stereos(&monos.iter().map(|mono| -> (i32, i32){(*mono, *mono)}).collect::<Vec<(i32, i32)>>()),
            o => self.write_frames(&monos.iter().map(|mono| -> Vec<i32> {(0..o).map(|_|{*mono}).collect()}).collect::<Vec<Vec<i32>>>()),
        }
    }

    pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), FlacEncoderError> {
        if stereos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => self.write_monos(&stereos.iter().map(|(l, r): &(i32, i32)| -> i32 {((*l as i64 + *r as i64) / 2) as i32}).collect::<Vec<i32>>()),
            2 => unsafe {
                let samples: Vec<i32> = stereos.iter().flat_map(|(l, r): &(i32, i32)| -> [i32; 2] {[*l, *r]}).collect();
                if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr() as *const i32, stereos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            o => panic!("Can't turn stereo audio into {o} channels audio."),
        }
    }

    pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), FlacEncoderError> {
        if frames.is_empty() {return Ok(())}
        let samples: Vec<i32> = frames.iter().flat_map(|frame: &Vec<i32>| -> Vec<i32> {
            if frame.len() != self.params.channels as usize {
                panic!("On FlacEncoderUnmovable::write_frames(): a frame size {} does not match the encoder channels.", frame.len())
            } else {frame.to_vec()}
        }).collect();
        unsafe {
            if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr() as *const i32, frames.len() as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
            }
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), FlacEncoderError> {
        unsafe {
            if FLAC__stream_encoder_finish(self.encoder) != 0 {
                Ok(())
            } else {
                self.get_status_as_error("FLAC__stream_encoder_finish")
            }
        }
    }

    fn on_drop(&mut self) {
        unsafe {
            if !self.is_finalized() {
                FLAC__stream_encoder_delete(self.encoder);
                self.encoder = ptr::null_mut();
            }
        };
    }

    pub fn finalize(mut self) -> Result<(), FlacEncoderError> {
        self.finish()
    }

    pub fn is_finalized(&self) -> bool {
        self.encoder.is_null()
    }
}

impl<Wr, Sk, Tl> Debug for FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacEncoderUnmovable<{}, {}, {}>", type_name::<Wr>(), type_name::<Sk>(), type_name::<Tl>()))
            .field("encoder", &self.encoder)
            .field("params", &self.params)
            .field("on_write", &format_args!("0x{:x}", &self.on_write as *const Wr as usize))
            .field("on_seek", &format_args!("0x{:x}", &self.on_seek as *const Sk as usize))
            .field("on_tell", &format_args!("0x{:x}", &self.on_tell as *const Tl as usize))
            .finish()
    }
}

impl<Wr, Sk, Tl> Drop for FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn drop(&mut self) {
        self.on_drop();
    }
}

pub struct FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    encoder: Box<FlacEncoderUnmovable<Wr, Sk, Tl>>,
}

impl<Wr, Sk, Tl> FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    pub fn new(
        on_write: Wr,
        on_seek: Sk,
        on_tell: Tl,
        params: &FlacEncoderParams
    ) -> Result<Self, FlacEncoderError> {
        let mut ret = Self {
            encoder: Box::new(FlacEncoderUnmovable::new(on_write, on_seek, on_tell, params)?)
        };
        ret.encoder.init()?;
        Ok(ret)
    }

    pub fn write_monos(&mut self, monos: &[i32]) -> Result<(), FlacEncoderError> {
        self.encoder.write_monos(monos)
    }

    pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), FlacEncoderError> {
        self.encoder.write_stereos(stereos)
    }

    pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), FlacEncoderError> {
        self.encoder.write_frames(frames)
    }

    pub fn finish(&mut self) -> Result<(), FlacEncoderError> {
        self.encoder.finish()
    }

    pub fn finalize(self) -> Result<(), FlacEncoderError> {
        Ok(())
    }
}

impl<Wr, Sk, Tl> Debug for FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacEncoder<{}, {}, {}>", type_name::<Wr>(), type_name::<Sk>(), type_name::<Tl>()))
            .field("encoder", &self.encoder)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlacDecoderError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacDecoderError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamDecoderStateString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacDecoderError);

#[derive(Debug, Clone, Copy)]
pub struct FlacDecoderInitError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacDecoderInitError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamDecoderInitStatusString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacDecoderInitError);

#[derive(Debug, Clone, Copy)]
pub enum ReadStatus {
    GoOn,
    Eof,
    Abort,
}

impl Display for ReadStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::GoOn => write!(f, "go_on"),
            Self::Eof => write!(f, "eof"),
            Self::Abort => write!(f, "abort"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DecoderError {
    LostSync,
    BadHeader,
    FrameCrcMismatch,
    UnparseableStream,
    BadMetadata,
    OutOfBounds,
    MissingFrame,
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::LostSync => write!(f, "FLAC: An error in the stream caused the decoder to lose synchronization."),
            Self::BadHeader => write!(f, "FLAC: The decoder encountered a corrupted frame header."),
            Self::FrameCrcMismatch => write!(f, "FLAC: The frame's data did not match the CRC in the footer."),
            Self::UnparseableStream => write!(f, "FLAC: The decoder encountered reserved fields in use in the stream."),
            Self::BadMetadata => write!(f, "FLAC: The decoder encountered a corrupted metadata block."),
            Self::OutOfBounds => write!(f, "FLAC: The decoder encountered a otherwise valid frame in which the decoded samples exceeded the range offered by the stated bit depth."),
            Self::MissingFrame => write!(f, "FLAC: Two adjacent frames had frame numbers increasing by more than 1 or sample numbers increasing by more than the blocksize, indicating that one or more frame/frames was missing between them. The decoder will sent out one or more ´fake' constant subframes to fill up the gap."),
        }
    }
}

pub struct FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>, // monos, sample_rate
    Er: FnMut(DecoderError) {
    decoder: *mut FLAC__StreamDecoder,
    on_read: Rd,
    on_seek: Sk,
    on_tell: Tl,
    on_length: Ln,
    on_eof: Ef,
    on_write: Wr,
    on_error: Er,
    md5_checking: bool,
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    pub fn new(
        on_read: Rd,
        on_seek: Sk,
        on_tell: Tl,
        on_length: Ln,
        on_eof: Ef,
        on_write: Wr,
        on_error: Er,
        md5_checking: bool,
    ) -> Result<Self, FlacDecoderError> {
        let ret = Self {
            decoder: unsafe {FLAC__stream_decoder_new()},
            on_read,
            on_seek,
            on_tell,
            on_length,
            on_eof,
            on_write,
            on_error,
            md5_checking,
        };

        if ret.decoder.is_null() {
            Err(FlacDecoderError::new(FLAC__STREAM_DECODER_MEMORY_ALLOCATION_ERROR, "FLAC__stream_decoder_new"))
        } else {
            Ok(ret)
        }
    }

    fn get_status_as_result(&self, function: &'static str) -> Result<(), FlacDecoderError> {
        let code = unsafe {FLAC__stream_decoder_get_state(self.decoder)};
        if code == 0 {
            Ok(())
        } else {
            Err(FlacDecoderError::new(code, function))
        }
    }

    fn get_status_as_error(&self, function: &'static str) -> Result<(), FlacDecoderError> {
        let code = unsafe {FLAC__stream_decoder_get_state(self.decoder)};
        Err(FlacDecoderError::new(code, function))
    }

    fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }

    fn init(&mut self) -> Result<(), FlacDecoderError> {
        unsafe {
            if FLAC__stream_decoder_set_md5_checking(self.decoder, self.md5_checking as i32) == 0 {
                return self.get_status_as_error("FLAC__stream_decoder_set_md5_checking");
            }
            let ret = FLAC__stream_decoder_init_stream(
                self.decoder,
                Some(Self::read_callback),
                Some(Self::seek_callback),
                Some(Self::tell_callback),
                Some(Self::length_callback),
                Some(Self::eof_callback),
                Some(Self::write_callback),
                Some(Self::metadata_callback),
                Some(Self::error_callback),
                self.as_mut_ptr() as *mut c_void,
            );
            if ret != 0 {
                return Err(FlacDecoderError {
                    code: ret,
                    message: FlacDecoderInitError::get_message_from_code(ret),
                    function: "FLAC__stream_decoder_init_stream",
                });
            }
        }
        self.get_status_as_result("FlacDecoderUnmovable::Init()")
    }

    unsafe extern "C" fn read_callback(_decoder: *const FLAC__StreamDecoder, buffer: *mut u8, bytes: *mut usize, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        if *bytes == 0 {
            FLAC__STREAM_DECODER_READ_STATUS_ABORT
        } else {
            let buf = slice::from_raw_parts_mut(buffer, *bytes);
            let (bytes_read, status) = (this.on_read)(buf);
            let ret = match status{
                ReadStatus::GoOn => FLAC__STREAM_DECODER_READ_STATUS_CONTINUE,
                ReadStatus::Eof => FLAC__STREAM_DECODER_READ_STATUS_END_OF_STREAM,
                ReadStatus::Abort => FLAC__STREAM_DECODER_READ_STATUS_ABORT,
            };

            *bytes = bytes_read;
            ret as u32
        }
    }

    unsafe extern "C" fn seek_callback(_decoder: *const FLAC__StreamDecoder, absolute_byte_offset: u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_seek)(absolute_byte_offset) {
            Ok(_) => FLAC__STREAM_DECODER_SEEK_STATUS_OK,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_SEEK_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_SEEK_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn tell_callback(_decoder: *const FLAC__StreamDecoder, absolute_byte_offset: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_tell)() {
            Ok(offset) => {
                *absolute_byte_offset = offset;
                FLAC__STREAM_DECODER_TELL_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_TELL_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_TELL_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn length_callback(_decoder: *const FLAC__StreamDecoder, stream_length: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_length)() {
            Ok(length) => {
                *stream_length = length;
                FLAC__STREAM_DECODER_LENGTH_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_LENGTH_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_LENGTH_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn eof_callback(_decoder: *const FLAC__StreamDecoder, client_data: *mut c_void) -> i32 {
        let this = &mut *(client_data as *mut Self);
        if (this.on_eof)() {1} else {0}
    }

    unsafe extern "C" fn write_callback(_decoder: *const FLAC__StreamDecoder, frame: *const FLAC__Frame, buffer: *const *const i32, client_data: *mut c_void) -> u32 {
        // Decoder output handling:
        // ------------------------------------------
        // - The decoder provides audio as `i32` values, but only the lower `bits_per_sample` bits are valid.
        //   Example: For `bits_per_sample = 8`, the valid range is -128 to +127 (8-bit signed PCM).
        //
        // - Signal normalization: Scale the decoded samples to utilize the full dynamic range
        //   of the `i32` type (i.e., from `i32::MIN` (-2147483648) to `i32::MAX` (2147483647)),
        //   while preserving waveform integrity and avoiding clipping.
        //
        //   Implementation logic:
        //   1. Left-shift the decoded samples by (32 - bits_per_sample) bits
        //      - e.g., 8-bit → shift left by 24 bits (to occupy upper 8 bits)
        //   2. Retain original sign and magnitude relationships.
        fn scale_to_i32(sample: i32, bits: u32) -> i32 {
            assert!(bits <= 32);
            if bits == 32 {
                sample
            } else {
                fn scale_to_unsigned(sample: i32, bits: u32) -> u32 {
                    let mask = (1u32 << bits) - 1;
                    let mid_number = 1u32 << (bits - 1);
                    ((sample as u32).wrapping_add(mid_number) & mask) << (32 - bits)
                }
                let mut lower_fill = scale_to_unsigned(sample, bits);
                let mut result = (sample as u32) << (32 - bits);
                while lower_fill > 0 {
                    lower_fill >>= bits;
                    result |= lower_fill;
                }
                result as i32
            }
        }

        let this = &mut *(client_data as *mut Self);
        let frame = *frame;
        let samples = frame.header.blocksize;
        let channels = frame.header.channels;
        let sample_rate = frame.header.sample_rate;
        let bits_per_sample = frame.header.bits_per_sample;

        let mut ret = vec![Vec::<i32>::with_capacity(channels as usize); samples as usize];
        for i in 0..channels {
            let channel = *buffer.add(i as usize);
            for s in 0..samples {
                ret[s as usize].push(scale_to_i32(*channel.add(s as usize), bits_per_sample));
            }
        }

        match (this.on_write)(&ret, sample_rate) {
            Ok(_) => FLAC__STREAM_DECODER_WRITE_STATUS_CONTINUE,
            Err(e) => {
                eprintln!("On `write_callback()`: {:?}", e);
                FLAC__STREAM_DECODER_WRITE_STATUS_ABORT
            },
        }
    }

    unsafe extern "C" fn metadata_callback(_decoder: *const FLAC__StreamDecoder, metadata: *const FLAC__StreamMetadata, client_data: *mut c_void) {
        let _this = &mut *(client_data as *mut Self);
        let _meta = &*(metadata as *const FLAC__StreamMetadata);
    }

    unsafe extern "C" fn error_callback(_decoder: *const FLAC__StreamDecoder, status: u32, client_data: *mut c_void) {
        let this = &mut *(client_data as *mut Self);
        (this.on_error)(match status {
            FLAC__STREAM_DECODER_ERROR_STATUS_LOST_SYNC => DecoderError::LostSync,
            FLAC__STREAM_DECODER_ERROR_STATUS_BAD_HEADER => DecoderError::BadHeader,
            FLAC__STREAM_DECODER_ERROR_STATUS_FRAME_CRC_MISMATCH => DecoderError::FrameCrcMismatch,
            FLAC__STREAM_DECODER_ERROR_STATUS_UNPARSEABLE_STREAM => DecoderError::UnparseableStream,
            // FLAC__STREAM_DECODER_ERROR_STATUS_BAD_METADATA => DecoderError::BadMetadata,
            // FLAC__STREAM_DECODER_ERROR_STATUS_MISSING_FRAME => DecoderError::MissingFrame,
            o => panic!("Unknown value of `FLAC__StreamDecoderErrorStatus`: {o}"),
        });
    }

    pub fn decode(&mut self) -> Result<bool, FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_process_single(self.decoder) != 0} {
            Ok(true)
        } else {
            match self.get_status_as_result("FLAC__stream_decoder_process_single") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }

    pub fn decode_all(&mut self) -> Result<bool, FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_process_until_end_of_stream(self.decoder) != 0} {
            Ok(true)
        } else {
            match self.get_status_as_result("FLAC__stream_decoder_process_until_end_of_stream") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }

    pub fn finish(&mut self) -> Result<(), FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_finish(self.decoder) != 0} {
            Ok(())
        } else {
            self.get_status_as_result("FLAC__stream_decoder_finish")
        }
    }

    fn on_drop(&mut self) {
        unsafe {
            if !self.is_finalized() {
                FLAC__stream_decoder_delete(self.decoder);
                self.decoder = ptr::null_mut();
            }
        };
    }

    pub fn finalize(mut self) -> Result<(), FlacDecoderError> {
        self.finish()
    }

    pub fn is_finalized(&self) -> bool {
        self.decoder.is_null()
    }
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Debug for FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacDecoderUnmovable<{}, {}, {}, {}, {}, {}, {}>",
                type_name::<Rd>(),
                type_name::<Sk>(),
                type_name::<Tl>(),
                type_name::<Ln>(),
                type_name::<Ef>(),
                type_name::<Wr>(),
                type_name::<Er>()))
            .field("decoder", &self.decoder)
            .field("on_read", &format_args!("0x{:x}", &self.on_read as *const Rd as usize))
            .field("on_seek", &format_args!("0x{:x}", &self.on_seek as *const Sk as usize))
            .field("on_tell", &format_args!("0x{:x}", &self.on_tell as *const Tl as usize))
            .field("on_length", &format_args!("0x{:x}", &self.on_length as *const Ln as usize))
            .field("on_eof", &format_args!("0x{:x}", &self.on_eof as *const Ef as usize))
            .field("on_write", &format_args!("0x{:x}", &self.on_write as *const Wr as usize))
            .field("on_error", &format_args!("0x{:x}", &self.on_error as *const Er as usize))
            .field("md5_checking", &self.md5_checking)
            .finish()
    }
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Drop for FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn drop(&mut self) {
        self.on_drop();
    }
}

pub struct FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    decoder: Box<FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>>,
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    pub fn new(
        on_read: Rd,
        on_seek: Sk,
        on_tell: Tl,
        on_length: Ln,
        on_eof: Ef,
        on_write: Wr,
        on_error: Er,
        md5_checking: bool,
    ) -> Result<Self, FlacDecoderError> {
        let mut ret = Self {
            decoder: Box::new(FlacDecoderUnmovable::<Rd, Sk, Tl, Ln, Ef, Wr, Er>::new(
                on_read,
                on_seek,
                on_tell,
                on_length,
                on_eof,
                on_write,
                on_error,
                md5_checking,
            )?),
        };
        ret.decoder.init()?;
        Ok(ret)
    }

    pub fn decode(&mut self) -> Result<bool, FlacDecoderError> {
        self.decoder.decode()
    }

    pub fn decode_all(&mut self) -> Result<bool, FlacDecoderError> {
        self.decoder.decode_all()
    }

    pub fn finish(&mut self) -> Result<(), FlacDecoderError> {
        self.decoder.finish()
    }

    pub fn finalize(self) -> Result<(), FlacDecoderError> {
        Ok(())
    }

    pub fn is_finalized(&self) -> bool {
        self.decoder.is_finalized()
    }
}


impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Debug for FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], u32) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacDecoder<{}, {}, {}, {}, {}, {}, {}>",
                type_name::<Rd>(),
                type_name::<Sk>(),
                type_name::<Tl>(),
                type_name::<Ln>(),
                type_name::<Ef>(),
                type_name::<Wr>(),
                type_name::<Er>()))
            .field("decoder", &self.decoder)
            .finish()
    }
}