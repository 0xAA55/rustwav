use std::{any::TypeId, error::Error};

use crate::errors::*;
use crate::audiocore::*;
use crate::sampleutils::*;
use crate::aw_abstract::AudioWriter;
use crate::wavwriter::WaveWriter;

impl AudioWriter for WaveWriter {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn write<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        self.check_channels(frame)?;
        if TypeId::of::<S>() == TypeId::of::<u8>() {
            self.packer_u8.save_sample(self.writer, frame)?;
        } else if TypeId::of::<S>() == TypeId::of::<i16>() {
            self.packer_s16.save_sample(self.writer, frame)?;
        } else if TypeId::of::<S>() == TypeId::of::<i24>() {
            self.packer_s24.save_sample(self.writer, frame)?;
        } else if TypeId::of::<S>() == TypeId::of::<i32>() {
            self.packer_s32.save_sample(self.writer, frame)?;
        } else if TypeId::of::<S>() == TypeId::of::<f32>() {
            self.packer_f32.save_sample(self.writer, frame)?;
        } else if TypeId::of::<S>() == TypeId::of::<f64>() {
            self.packer_f64.save_sample(self.writer, frame)?;
        } else {
            return Err(AudioWriteError::UnsupportedFormat.into);
        }

        self.num_frames += 1;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.update_header()
    }
}
