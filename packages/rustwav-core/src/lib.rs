mod wavcore;
mod wavreader;
mod wavwriter;
mod adpcm;

#[macro_use]
mod hacks;

/// * The encoders for the `WaveWriter`, each of these provides the same API for it to use. You can use it too.
pub mod encoders;

/// * The decoders for the `WaveReader`, each of these provides the same API for it to use. You can use it too.
pub mod decoders;

/// * The resampler
#[doc(inline)]
pub use resampler;

#[doc(hidden)]
pub use sampletypes::{i24, u24};

pub use sampletypes::{SampleFrom, SampleType};
pub use wavreader::{WaveDataSource, WaveReader};
pub use wavwriter::WaveWriter;

/// * Errors returned from most of the function in this library.
pub mod errors;

/// * Utilities for IO
pub use io_utils;

/// * The utility for both you and me to convert waveform format and do resampling and convert sample types.
pub use audioutils;

/// * The downmixer
pub use downmixer;

/// * Misc utilities
pub mod utils {
    /// * A utility for you to manipulate data bitwise, mainly to concatenate data in bits or to split data from a specific bit position.
    #[doc(inline)]
    pub use revorbis::bitwise::BitwiseData;

    /// * Copiable buffer, a tinier `Vec`, uses a fixed-size array to store a variable number of items.
    pub use copiablebuf::CopiableBuffer;

    /// * File hasher to calculate the hash for a section of a file, the hash is `u64` size. The `Write` trait was implemented for it.
    pub use filehasher::FileHasher;

    // * A string encode/decode library that sometimes do things savagely
    pub use savagestr::{SavageStringCodecs, StringCodecMaps};

    /// * A function to gather all of the needed chunks from a `WaveReader` and constructs the `cue` data full of the info.
    /// * WAV files seldom contain the `cue` data, normally the cue data is separated into a `.cue` file.
    pub use crate::wavcore::create_full_info_cue_data;
}

/// * Iterators for `WaveReader` to decode audio samples.
pub mod iterators {
    pub use crate::wavreader::{FrameIntoIter, FrameIter, MonoIntoIter, MonoIter, StereoIntoIter, StereoIter};
}

/// * WAV file format specs
pub mod format_specs {
    pub use crate::wavcore::{DataFormat, SampleFormat, Spec, WaveSampleType};

    /// * All of the supported WAV format tags
    pub mod format_tags {
        pub use crate::wavcore::format_tags::*;
    }

    /// * All of the supported WAV format GUIDs from the extensible data from the `fmt ` chunk.
    pub mod guids {
        pub use crate::wavcore::guids::*;
    }
}

/// * Encoder creation options
pub mod options {
    pub use crate::wavwriter::FileSizeOption;

    #[doc(inline)]
    pub use crate::wavcore::AdpcmSubFormat;

    #[doc(inline)]
    pub use crate::wavcore::flac::{FlacCompression, FlacEncoderParams};

    #[doc(inline)]
    pub use crate::wavcore::mp3::{Mp3Bitrate, Mp3Channels, Mp3EncoderOptions, Mp3Quality, Mp3VbrMode};

    #[doc(inline)]
    pub use crate::wavcore::opus::{OpusBitrate, OpusEncoderOptions, OpusEncoderSampleDuration};

    #[doc(inline)]
    pub use crate::wavcore::oggvorbis::{OggVorbisEncoderParams, OggVorbisMode, OggVorbisBitrateStrategy};
}

/// * WAV chunks
pub mod chunks {
    pub use crate::wavcore::{
        FmtChunk,
        SlntChunk,
        BextChunk,
        InstChunk,
        AcidChunk,
        TrknChunk,
        CueChunk,
        PlstChunk,
        SmplChunk,
        ListChunk,
        Id3,
        JunkChunk,
        FullInfoCuePoint,
        ListInfo,
        AdtlChunk,
        LablChunk,
        NoteChunk,
        LtxtChunk,
        FileChunk,
    };

    /// * WAV `fmt ` chunk extension data
    pub mod ext {
        pub use crate::wavcore::{
            FmtExtension,
            ExtensionData,
            AdpcmMsData,
            AdpcmImaData,
            Mp3Data,
            VorbisHeaderData,
            OggVorbisData,
            OggVorbisWithHeaderData,
            ExtensibleData,
        };
    }
}

use resampler::Resampler;

/// * Transfer audio from the decoder to the encoder with resampling.
/// * This allows to transfer of audio from the decoder to a different sample rate encoder.
pub fn transfer_audio_from_decoder_to_encoder(decoder: &mut WaveReader, encoder: &mut WaveWriter) {
    // The decoding audio spec
    let decode_spec = decoder.spec();

    // The encoding audio spec
    let encode_spec = encoder.spec();

    let decode_channels = decode_spec.channels;
    let encode_channels = encode_spec.channels;
    let decode_sample_rate = decode_spec.sample_rate;
    let encode_sample_rate = encode_spec.sample_rate;

    // Get the best FFT size for the resampler.
    let fft_size = Resampler::get_rounded_up_fft_size(std::cmp::max(encode_sample_rate, decode_sample_rate));

    // This is the resampler, if the decoder's sample rate is different than the encode sample rate, use the resampler to help stretch or compress the waveform.
    // Otherwise, it's not needed there.
    let resampler = Resampler::new(fft_size);

    // The number of channels must match
    assert_eq!(encode_channels, decode_channels);

    // Process size is for the resampler to process the waveform, it is the length of the source waveform slice.
    let process_size = resampler.get_process_size(fft_size, decode_sample_rate, encode_sample_rate);

    // There are three types of iterators for three types of audio channels: mono, stereo, and more than 2 channels of audio.
    // Usually, the third iterator can handle all numbers of channels, but it's the slowest iterator.
    match encode_channels {
        1 => {
            let mut iter = decoder.mono_iter::<f32>().unwrap();
            loop {
                let block: Vec<f32> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = audioutils::do_resample_mono(
                    &resampler,
                    &block,
                    decode_sample_rate,
                    encode_sample_rate,
                );
                encoder.write_mono_channel(&block).unwrap();
            }
        }
        2 => {
            let mut iter = decoder.stereo_iter::<f32>().unwrap();
            loop {
                let block: Vec<(f32, f32)> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = audioutils::do_resample_stereo(
                    &resampler,
                    &block,
                    decode_sample_rate,
                    encode_sample_rate,
                );
                encoder.write_stereos(&block).unwrap();
            }
        }
        _ => {
            let mut iter = decoder.frame_iter::<f32>().unwrap();
            loop {
                let block: Vec<Vec<f32>> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = audioutils::do_resample_frames(
                    &resampler,
                    &block,
                    decode_sample_rate,
                    encode_sample_rate,
                );
                encoder.write_frames(&block).unwrap();
            }
        }
    }
}
