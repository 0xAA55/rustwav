
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
pub mod utils;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::EncoderToImpl;
pub use encoders::{Encoder, PcmEncoder, AdpcmEncoderWrap};
pub use decoders::{Decoder, PcmDecoder, AdpcmDecoderWrap};
pub use adpcm::{AdpcmCodec};
pub use adpcm::{AdpcmEncoder, AdpcmEncoderBS, AdpcmEncoderOKI, AdpcmEncoderOKI6258, AdpcmEncoderYMA, AdpcmEncoderYMB, AdpcmEncoderYMZ, AdpcmEncoderAICA, AdpcmEncoderIMA, AdpcmEncoderMS};
pub use adpcm::{AdpcmDecoder, AdpcmDecoderBS, AdpcmDecoderOKI, AdpcmDecoderOKI6258, AdpcmDecoderYMA, AdpcmDecoderYMB, AdpcmDecoderYMZ, AdpcmDecoderAICA, AdpcmDecoderIMA, AdpcmDecoderMS};
pub use adpcm::{EncBS, EncOKI, EncOKI6258, EncYMA, EncYMB, EncYMZ, EncAICA, EncIMA, EncMS};
pub use adpcm::{DecBS, DecOKI, DecOKI6258, DecYMA, DecYMB, DecYMZ, DecAICA, DecIMA, DecMS};
pub use wavcore::{DataFormat, AdpcmSubFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtExtension, ExtensionData, AdpcmMsData, AdpcmImaData, Extensible};
pub use wavcore::{BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use filehasher::FileHasher;

#[cfg(feature = "mp3dec")]
pub use decoders::MP3::Mp3Decoder;

#[cfg(feature = "mp3enc")]
pub use encoders::MP3::Mp3Encoder;