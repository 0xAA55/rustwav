use std::error::Error;

use crate::waveform::WaveForm;

#[derive(Clone)]
pub enum SampleFormat {
    Float,
    Int,
}

#[derive(Clone)]
pub struct Spec {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

pub trait AudioReader: Iterator<Item = WaveForm> + Send {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> where Self: Sized {
        panic!("Abstract function called with param input_file = {input_file}.");
    }
    fn spec(&self) -> Spec;
    fn get_chunk_size(&self) -> usize;
    fn set_chunk_size(&mut self, chunk_size: usize);
}
