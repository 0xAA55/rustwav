
use std::{io::{BufWriter}, {fs::File}, {error::Error}};
use hound::{WavSpec, WavWriter};

use crate::waveform::WaveForm;
use crate::sampleutils::SampleUtils;
use crate::audioreader::Spec;
use crate::audiowriter::AudioWriter;

pub struct WaveWriterSimple {
    writer: WavWriter<BufWriter<File>>,
}

impl AudioWriter for WaveWriterSimple {
    fn create(output_file: &str, spec: &Spec) -> Result<Self, Box<dyn Error>> {
        let writer = WavWriter::create(output_file, WavSpec {
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        })?;
        Ok(Self {
            writer,
        })
    }

    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>> {
        match channels_data {
            WaveForm::Mono(mono) => {
                for sample in mono.into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            WaveForm::Stereo(stereo) => {
                for sample in SampleUtils::zip_samples(&stereo).into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            _ => panic!("Must not give `WaveForm::None` for a `WaveWriterSimple` to write."),
        }
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        Ok(self.writer.flush()?)
    }
}