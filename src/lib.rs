
pub mod errors;
pub mod savagestr;
pub mod readwrite;
pub mod sampleutils;
pub mod filehasher;
pub mod adpcm;
pub mod xlaw;
pub mod encoders;
pub mod decoders;
pub mod wavcore;
pub mod wavreader;
pub mod wavwriter;
pub mod utils;
pub mod copiablebuf;
pub mod resampler;
pub mod hacks;

pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use readwrite::{Reader, Writer};
pub use wavcore::{Spec, SampleFormat, DataFormat, AdpcmSubFormat};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use resampler::Resampler;
pub use errors::{AudioReadError, AudioError, AudioWriteError};
