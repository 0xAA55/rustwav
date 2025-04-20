mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod copiablebuf;
mod filehasher;
mod adpcm;
mod xlaw;
mod flac;
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
pub use wavcore::{Spec, SampleFormat, DataFormat};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter, FrameIntoIter, StereoIntoIter, MonoIntoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use resampler::Resampler;
pub use errors::{AudioReadError, AudioError, AudioWriteError};
pub use wavcore::{AdpcmSubFormat};
pub use wavcore::{Mp3EncoderOptions, Mp3Channels, Mp3Quality, Mp3Bitrate, Mp3VbrMode};
pub use wavcore::{OpusEncoderOptions, OpusBitrate, OpusEncoderSampleDuration};
