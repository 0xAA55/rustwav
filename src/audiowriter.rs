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

    // 允许用户输入任何格式为 S 的样本组成的音频帧，内部我们根据 spec 会做具体的类型转换。
    fn write<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        self.check_channels(frame)?;
        self.save_frame::<S>(frame)?;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.update_header()
    }
}
