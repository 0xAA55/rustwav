use std::{any::TypeId, error::Error};

use crate::errors::*;
use crate::audiocore::*;
use crate::sampleutils::*;
use crate::aw_abstract::AudioWriter;
use crate::wavwriter::{WaveWriter, WaveSampleType};

impl AudioWriter for WaveWriter {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn write<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        self.check_channels(frame)?;

        let mut sample_type = WaveSampleType::U8;

        let type_id_of_s = TypeId::of::<S>();
        if type_id_of_s == TypeId::of::<u8>() {
            sample_type = WaveSampleType::U8;
        } else if type_id_of_s == TypeId::of::<i16>() {
            sample_type = WaveSampleType::S16;
        } else if type_id_of_s == TypeId::of::<i24>() {
            sample_type = WaveSampleType::S24;
        } else if type_id_of_s == TypeId::of::<i32>() {
            sample_type = WaveSampleType::S32;
        } else if type_id_of_s == TypeId::of::<f32>() {
            sample_type = WaveSampleType::F32;
        } else if type_id_of_s == TypeId::of::<f64>() {
            sample_type = WaveSampleType::F64;
        } else {
            return Err(AudioWriteError::UnsupportedFormat.into);
        }

        self.save_frame::<S>(frame, sample_type)?;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.update_header()
    }
}
