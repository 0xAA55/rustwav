
mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod filehasher;
mod adpcm;
mod encoders;
mod decoders;
mod wavcore;
mod wavreader;
mod wavwriter;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::{EncoderBasic, Encoder, PcmEncoder};
pub use decoders::{Decoder, PcmDecoder};
pub use wavcore::{DataFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtChunkExtension, BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, WaveIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
