
mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod adpcm;
mod codecs;
mod wavcore;
mod wavreader;
mod wavwriter;

pub use wavcore::DataFormat;
pub use wavreader::WaveReader;
pub use wavwriter::{WaveWriter, Spec, FileSizeOption, SampleFormat};
