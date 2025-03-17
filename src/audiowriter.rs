use std::error::Error;

use crate::waveform::WaveForm;
use crate::audioreader::Spec;

#[derive(Debug, Clone)]
pub enum WriterError {
    WaveFormLengthError,
}

impl std::error::Error for WriterError {}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           WriterError::WaveFormLengthError => write!(f, "The length of the chunk to write must be longer than the window size."),
       }
    }
}

pub trait AudioWriter {
    fn create(_output_file: &str, _spec: &Spec) -> Result<Self, Box<dyn Error>> where Self: Sized {
        panic!("Not implemented for `create()`");
    }
    fn upgrade(_writer: Box<dyn AudioWriter>) -> Result<Self, Box<dyn Error>> where Self: Sized {
        panic!("Not implemented for `upgrade()`");
    }
    fn set_window_size(&mut self, _window_size: usize) {
        panic!("Not implemented for `set_window_size`");
    }
    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>>;
    fn finalize(&mut self) -> Result<(), Box<dyn Error>>;
}
