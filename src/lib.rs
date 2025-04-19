mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod copiablebuf;
mod filehasher;
mod adpcm;
mod xlaw;
mod encoders;
mod decoders;
mod wavcore;
mod wavreader;
mod wavwriter;
mod resampler;
mod hacks;

pub mod utils;

pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use readwrite::{Reader, Writer};
pub use wavcore::{Spec, SampleFormat, DataFormat, AdpcmSubFormat, Mp3EncoderOptions, Mp3Channels, Mp3Quality, Mp3Bitrate, Mp3VbrMode};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use resampler::Resampler;
pub use errors::{AudioReadError, AudioError, AudioWriteError};
