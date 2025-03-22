use std::error::Error;

use crate::errors::*;
use crate::audiocore::{Spec};

pub trait AudioWriter {
    fn spec(&self) -> Spec;
    fn write(&mut self, frame: &Vec<f32>) -> Result<(), Box<dyn Error>>;
    fn finalize(&mut self) -> Result<(), Box<dyn Error>>;

    fn check_channels(&self, frame: &Vec<f32>) -> Result<(), Box<dyn Error>> {
        if frame.len() != self.spec().channels as usize {
            Err(AudioWriteError::ChannelCountNotMatch.into())
        } else {
            Ok(())
        }
    }
}
