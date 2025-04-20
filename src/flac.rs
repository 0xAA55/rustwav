#![allow(dead_code)]

use std::{error, io::{self, ErrorKind}, fmt::{self, Debug, Display, Formatter}, slice, ffi::{CStr, c_void}, ptr};

use libflac_sys::*;

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderError {
    pub code: u32,
    pub message: &'static str,
    pub function: &'static str,
}

impl FlacEncoderError {
    pub fn new(code: u32, message: &'static str, function: &'static str) -> Self {
        Self {
            code,
            message,
            function,
        }
    }
}

impl error::Error for FlacEncoderError {}

impl Display for FlacEncoderError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let code = self.code;
        let message = self.message;
        let function = self.function;
        write!(f, "Code: {code}, function: {function}, message: {message}")?;
        Ok(())
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderParams {
    pub verify_decoded: bool,
    pub compression: FlacCompression,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_rate: u32,
    pub total_samples_estimate: u64,
}

impl FlacEncoderParams {
    pub fn new() -> Self {
        Self {
            verify_decoded: false,
            compression: FlacCompression::Level5,
            channels: 2,
            bits_per_sample: 16,
            sample_rate: 44100,
            total_samples_estimate: 0,
        }
    }
}

pub struct FlacEncoder<W, S, T>
where 
    W: FnMut(&[u8]) -> Result<(), io::Error>,
    S: FnMut(u64) -> Result<(), io::Error>,
    T: FnMut() -> Result<u64, io::Error>
{
    encoder: *mut FLAC__StreamEncoder,
    params: FlacEncoderParams,
    on_write: W,
    on_seek: S,
    on_tell: T,
}

impl<W, S, T> FlacEncoder<W, S, T>
where 
    W: FnMut(&[u8]) -> Result<(), io::Error>,
    S: FnMut(u64) -> Result<(), io::Error>,
    T: FnMut() -> Result<u64, io::Error> {
    pub fn new(
        on_write: W,
        on_seek: S,
        on_tell: T,
        params: &FlacEncoderParams
    ) -> Result<Box<Self>, FlacEncoderError> {

        // Must put `self` into a `box` to prevent pointer changes.
        let mut ret = Box::new(Self {
            encoder: unsafe {FLAC__stream_encoder_new()},
            params: *params,
            on_write,
            on_seek,
            on_tell,
        });
        ret.init()?;

        Ok(ret)
    }

    fn get_status(&self) -> (u32, &'static str) {
        unsafe {
            let state = FLAC__stream_encoder_get_state(self.encoder);
            let desc = FLAC__stream_encoder_get_resolved_state_string(self.encoder);
            let desc = CStr::from_ptr(desc).to_str().unwrap();
            (state, desc)
        }
    }

    fn get_status_as_error(&self, function: &'static str) -> Result<(), FlacEncoderError> {
        let (state, desc) = self.get_status();
        Err(FlacEncoderError::new(state, desc, function))
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
            if FLAC__stream_encoder_set_bits_per_sample(self.encoder, self.params.bits_per_sample as u32) == 0 {
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
            if FLAC__stream_encoder_init_stream(self.encoder,
                    Some(Self::write_callback),
                    Some(Self::seek_callback),
                    Some(Self::tell_callback),
                    Some(Self::metadata_callback),

                    // At this time, `self` must be put into a `Box` so the pointer of `self` won't change.
                    self.as_mut_ptr() as *mut c_void,
                ) != 0 {
                return self.get_status_as_error("FLAC__stream_encoder_init_stream");
            }
        }
        Ok(())
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

    pub fn write_monos(&mut self, monos: &[i16]) -> Result<(), FlacEncoderError> {
        if monos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => unsafe {
                let samples: Vec<i32> = monos.iter().map(|s|{*s as i32}).collect();
                if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr() as *const i32, monos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            2 => self.write_stereos(&monos.iter().map(|mono| -> (i16, i16){(*mono, *mono)}).collect::<Vec<(i16, i16)>>()),
            o => self.write_frames(&monos.iter().map(|mono| -> Vec<i16> {(0..o).map(|_|{*mono}).collect()}).collect::<Vec<Vec<i16>>>()),
        }
    }

    pub fn write_stereos(&mut self, stereos: &[(i16, i16)]) -> Result<(), FlacEncoderError> {
        if stereos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => self.write_monos(&stereos.iter().map(|(l, r): &(i16, i16)| -> i16 {((*l as i32 + *r as i32) / 2) as i16}).collect::<Vec<i16>>()),
            2 => unsafe {
                let samples: Vec<i32> = stereos.iter().flat_map(|(l, r): &(i16, i16)| -> [i32; 2] {[*l as i32, *r as i32]}).collect();
                if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr() as *const i32, stereos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            o => panic!("Can't turn stereo audio into {o} channels audio."),
        }
    }

    pub fn write_frames(&mut self, frames: &[Vec<i16>]) -> Result<(), FlacEncoderError> {
        if frames.is_empty() {return Ok(())}
        let samples: Vec<i32> = frames.iter().flat_map(|frame: &Vec<i16>| -> Vec<i32> {
            if frame.len() != self.params.channels as usize {
                panic!("On FlacEncoder::write_frames(): a frame size {} does not match the encoder channels.", frame.len())
            } else {frame.iter().map(|s|{*s as i32}).collect()}
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
            if FLAC__stream_encoder_finish(self.encoder) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_finish");
            }
            Ok(())
        }
    }

    fn on_drop(&mut self) {
        unsafe {
            if !self.is_finalized() {
                match self.finish() {
                    Ok(_) => (),
                    Err(e) => eprintln!("Failed to `finish()`. {:?}", e),
                }
                FLAC__stream_encoder_delete(self.encoder);
                self.encoder = ptr::null_mut();
            }
        };
    }

    pub fn finalize(mut self) -> Result<(), FlacEncoderError> {
        self.on_drop();
        Ok(())
    }

    pub fn is_finalized(&self) -> bool {
        self.encoder == ptr::null_mut()
    }
}

impl<W, S, T> Drop for FlacEncoder<W, S, T>
where 
    W: FnMut(&[u8]) -> Result<(), io::Error>,
    S: FnMut(u64) -> Result<(), io::Error>,
    T: FnMut() -> Result<u64, io::Error> {
    fn drop(&mut self) {
        self.on_drop();
    }
}



