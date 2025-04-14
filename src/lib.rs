
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
pub mod copiablebuf;

pub use errors::{AudioError, AudioReadError, AudioWriteError, IOErrorInfo};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::EncoderToImpl;
pub use encoders::{Encoder, PcmEncoder, AdpcmEncoderWrap};
pub use decoders::{Decoder, PcmDecoder, AdpcmDecoderWrap};
pub use adpcm::{AdpcmEncoder, AdpcmEncoderIMA, AdpcmEncoderMS};
pub use adpcm::{AdpcmDecoder, AdpcmDecoderIMA, AdpcmDecoderMS};
pub use adpcm::{EncIMA, EncMS};
pub use adpcm::{DecIMA, DecMS};
pub use wavcore::{DataFormat, AdpcmSubFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtExtension, ExtensionData, AdpcmMsData, AdpcmImaData, ExtensibleData, Mp3Data};
pub use wavcore::{BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use filehasher::FileHasher;
pub use copiablebuf::{CopiableBuffer, CopiableBufferIter, CopiableBufferIterMut, CopiableBufferIntoIter};

#[cfg(feature = "mp3dec")]
pub use decoders::MP3::Mp3Decoder;

#[cfg(feature = "mp3enc")]
pub use encoders::MP3::Mp3Encoder;