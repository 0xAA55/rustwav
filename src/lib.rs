
pub mod errors;
pub mod savagestr;
pub mod readwrite;
pub mod sampleutils;
pub mod filehasher;
pub mod adpcm;
pub mod encoders;
pub mod decoders;
pub mod wavcore;
pub mod wavreader;
pub mod wavwriter;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::{EncoderToImpl, Encoder, PcmEncoder};
pub use decoders::{Decoder, PcmDecoder};
pub use adpcm::{AdpcmEncoderTypes, AdpcmEncoderBS, AdpcmEncoderOKI, AdpcmEncoderOKI6258, AdpcmEncoderYMA, AdpcmEncoderYMB, AdpcmEncoderYMZ, AdpcmEncoderAICA};
pub use wavcore::{DataFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtChunkExtension, BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, WaveIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use filehasher::FileHasher;

#[cfg(feature = "mp3dec")]
pub use decoders::MP3::Mp3Decoder;

#[cfg(feature = "mp3enc")]
pub use encoders::MP3::Mp3Encoder;