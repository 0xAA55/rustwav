mod copiablebuf;
mod filehasher;
mod readwrite;
mod savagestr;
mod wavcore;
mod wavreader;
mod wavwriter;
mod xlaw;
mod adpcm;

#[macro_use]
mod hacks;

#[cfg(any(feature = "vorbis", feature = "oggvorbis"))]
mod ogg;

#[cfg(any(feature = "vorbis", feature = "oggvorbis"))]
mod vorbis;

/// ## The utility for both you and me to convert waveform format and do resampling and convert sample types.
#[macro_use]
pub mod audioutils;

/// ## The utility for both you and me to do bitvise manipulations.
pub mod bitwise;

/// ## The encoders for the `WaveWriter`, each of these provides the same API for it to use. You can use it too.
pub mod encoders;

/// ## The decoders for the `WaveReader`, each of these provides the same API for it to use. You can use it too.
pub mod decoders;

/// ## Errors returned from most of the function in this library.
pub mod errors;

/// ## The downmixer
pub mod downmixer;

/// ## The resampler
pub use resampler;

#[doc(hidden)]
pub use sampletypes::{i24, u24};

pub use sampletypes::{SampleFrom, SampleType};
pub use wavreader::{WaveDataSource, WaveReader};
pub use wavwriter::WaveWriter;

pub mod io_utils {
    pub use crate::readwrite::{
        Reader,
        Writer,
        ReadBridge,
        WriteBridge,
        SharedReader,
        CursorVecU8,
        SharedCursor,
        CombinedReader,
        DishonestReader,
        WriterWithCursor,
        SharedBorrowedReader,
        SharedBorrowedWriter,
        SharedWriterWithCursor,
        string_io,
    };
}

pub mod utils {
    pub use crate::copiablebuf::CopiableBuffer;
    pub use crate::filehasher::FileHasher;
    pub use crate::bitwise::BitwiseData;
    pub use crate::wavcore::create_full_info_cue_data;
}

pub mod iterators {
    pub use crate::wavreader::{FrameIntoIter, FrameIter, MonoIntoIter, MonoIter, StereoIntoIter, StereoIter};
}

pub mod format_specs {
    pub use crate::wavcore::{DataFormat, SampleFormat, Spec, WaveSampleType};
    pub mod format_tags {
        pub use crate::wavcore::format_tags::*;
    }
    pub mod guids {
        pub use crate::wavcore::guids::*;
    }
}

pub mod options {
    pub use crate::wavwriter::FileSizeOption;

    pub use crate::wavcore::AdpcmSubFormat;

    #[cfg(feature = "flac")]
    pub use flac::{FlacCompression, FlacEncoderParams};

    #[cfg(not(feature = "flac"))]
    pub use crate::wavcore::flac::{FlacCompression, FlacEncoderParams};

    #[doc(inline)]
    pub use crate::encoders::{Mp3Bitrate, Mp3Channels, Mp3EncoderOptions, Mp3Quality, Mp3VbrMode};

    #[doc(inline)]
    pub use crate::encoders::{OpusBitrate, OpusEncoderOptions, OpusEncoderSampleDuration};

    #[doc(inline)]
    pub use crate::encoders::{OggVorbisEncoderParams, OggVorbisMode, OggVorbisBitrateStrategy};

}

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
    pub mod ext {
        pub use crate::wavcore::{
            FmtExtension,
            ExtensionData,
            AdpcmMsData,
            AdpcmImaData,
            Mp3Data,
            OggVorbisData,
            OggVorbisWithHeaderData,
            ExtensibleData,
        };
    }
}

use std::{env::args, error::Error, process::ExitCode};

use format_specs::*;
use options::*;
use resampler::Resampler;

/// ## The list for the command line program to parse the argument and we have the pre-filled encoder initializer parameter structs for each format.
pub const FORMATS: [(&str, DataFormat); 16] = [
    ("pcm", DataFormat::Pcm),
    ("pcm-alaw", DataFormat::PcmALaw),
    ("pcm-ulaw", DataFormat::PcmMuLaw),
    ("adpcm-ms", DataFormat::Adpcm(AdpcmSubFormat::Ms)),
    ("adpcm-ima", DataFormat::Adpcm(AdpcmSubFormat::Ima)),
    ("adpcm-yamaha", DataFormat::Adpcm(AdpcmSubFormat::Yamaha)),
    (
        "mp3",
        DataFormat::Mp3(Mp3EncoderOptions {
            channels: Mp3Channels::NotSet,
            quality: Mp3Quality::Best,
            bitrate: Mp3Bitrate::Kbps320,
            vbr_mode: Mp3VbrMode::Off,
            id3tag: None,
        }),
    ),
    (
        "opus",
        DataFormat::Opus(OpusEncoderOptions {
            bitrate: OpusBitrate::Max,
            encode_vbr: false,
            samples_cache_duration: OpusEncoderSampleDuration::MilliSec60,
        }),
    ),
    (
        "flac",
        DataFormat::Flac(FlacEncoderParams {
            verify_decoded: false,
            compression: FlacCompression::Level8,
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 32,
            total_samples_estimate: 0,
        }),
    ),
    (
        "vorbis",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::NakedVorbis,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Vbr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis1",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::OriginalStreamCompatible,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Vbr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis2",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::HaveIndependentHeader,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Vbr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis3",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::HaveNoCodebookHeader,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Vbr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis1p",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::OriginalStreamCompatible,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Abr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis2p",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::HaveIndependentHeader,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Abr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
    (
        "oggvorbis3p",
        DataFormat::OggVorbis(OggVorbisEncoderParams {
            mode: OggVorbisMode::HaveNoCodebookHeader,
            channels: 2,
            sample_rate: 44100,
            stream_serial: None,
            bitrate: Some(OggVorbisBitrateStrategy::Abr(320_000)),
            minimum_page_data_size: None,
        }),
    ),
];

/// ## The fft size can be any number greater than the sample rate of the encoder or the decoder.
/// * It is for the resampler. A greater number results in better resample quality, but the process could be slower.
/// * In most cases, the audio sampling rate is about `11025` to `48000`, so `65536` is the best number for the resampler.
pub fn get_rounded_up_fft_size(sample_rate: u32) -> usize {
    for i in 0..31 {
        let fft_size = 1usize << i;
        if fft_size >= sample_rate as usize {
            return fft_size;
        }
    }
    0x1_00000000_usize
}

/// ## Transfer audio from the decoder to the encoder with resampling.
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
    let fft_size = get_rounded_up_fft_size(std::cmp::max(encode_sample_rate, decode_sample_rate));

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

/// ## The `test()` function
/// * arg1: the format, e.g. "pcm"
/// * arg2: the input file to parse and decode, tests the decoder for the input file.
/// * arg3: the output file to encode, test the encoder.
/// * arg4: re-decode arg3 and encode to pcm to test the decoder.
pub fn test(arg1: &str, arg2: &str, arg3: &str, arg4: &str) -> Result<(), Box<dyn Error>> {
    let mut data_format = DataFormat::Unspecified;
    for format in FORMATS {
        if arg1 == format.0 {
            data_format = format.1;
            break;
        }
    }

    // Failed to match the data format
    if data_format == DataFormat::Unspecified {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Unknown format `{arg1}`. Please input one of these:\n{}",
                FORMATS
                    .iter()
                    .map(|(s, _v)| { s.to_string() })
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
        )
        .into());
    }

    println!("======== TEST 1 ========");
    println!("{:?}", data_format);

    // This is the decoder
    let mut wavereader = WaveReader::open(arg2).unwrap();

    let orig_spec = wavereader.spec();

    // The spec for the encoder
    let mut spec = Spec {
        channels: orig_spec.channels,
        channel_mask: 0,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    match data_format {
        DataFormat::Mp3(ref mut options) => match spec.channels {
            1 => options.channels = Mp3Channels::Mono,
            2 => options.channels = Mp3Channels::JointStereo,
            o => panic!("MP3 format can't encode {o} channels audio."),
        },
        DataFormat::Opus(ref options) => {
            spec.sample_rate = options.get_rounded_up_sample_rate(spec.sample_rate);
        }
        DataFormat::Flac(ref mut options) => {
            options.channels = spec.channels;
            options.sample_rate = spec.sample_rate;
            options.bits_per_sample = spec.bits_per_sample as u32;
        }
        DataFormat::OggVorbis(ref mut options) => {
            options.channels = spec.channels;
            options.sample_rate = spec.sample_rate;
        }
        _ => (),
    }

    // Just to let you know, WAV file can be larger than 4 GB
    #[allow(unused_imports)]
    use options::FileSizeOption::{AllowLargerThan4GB, ForceUse4GBFormat, NeverLargerThan4GB};

    // This is the encoder
    let mut wavewriter = WaveWriter::create(arg3, spec, data_format, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader, &mut wavewriter);

    // Get the metadata from the decoder
    wavewriter.inherit_metadata_from_reader(&wavereader, true);

    // Show debug info
    dbg!(&wavereader);
    dbg!(&wavewriter);

    drop(wavereader);
    drop(wavewriter);

    println!("======== TEST 2 ========");

    let spec2 = Spec {
        channels: spec.channels,
        channel_mask: 0,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut wavereader_2 = WaveReader::open(arg3).unwrap();
    let mut wavewriter_2 = WaveWriter::create(arg4, spec2, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader_2, &mut wavewriter_2);

    // Get the metadata from the decoder
    wavewriter_2.inherit_metadata_from_reader(&wavereader_2, true);

    // Show debug info
    dbg!(&wavereader_2);
    dbg!(&wavewriter_2);

    drop(wavereader_2);
    drop(wavewriter_2);

    Ok(())
}

/// ## A function dedicated to testing WAV encoding and decoding. This function is actually a `main()` function for a command-line program that parses `args` and returns an `ExitCode`.
/// * The usage is `arg0 [format] [test.wav] [output.wav] [output2.wav]`
/// * It decodes the `test.wav` and encodes it to `output.wav` by `format`
/// * Then it re-decode `output.wav` to `output2.wav`
/// * This can test both encoders and decoders with the specified format to see if they behave as they should.
#[allow(dead_code)]
pub fn test_wav() -> ExitCode {
    let args: Vec<String> = args().collect();
    if args.len() < 5 {
        return ExitCode::from(1);
    }
    let input_wav = &args[1];
    let output_wav = &args[2];
    let reinput_wav = &args[3];
    let reoutput_wav = &args[4];
    match test(input_wav, output_wav, reinput_wav, reoutput_wav) {
        Ok(_) => ExitCode::from(0),
        Err(e) => {
            eprintln!("{:?}", e);
            ExitCode::from(2)
        }
    }
}

macro_rules! test_fn {
    ($name:ident, $index:expr) => {
        #[test]
        pub fn $name() {
            let fmt = FORMATS[$index].0;
            test(
                fmt,
                "test.wav",
                &format!("{fmt}_test_encode.wav"),
                &format!("{fmt}_test_decode.wav"),
            )
            .unwrap();
        }
    };
}

test_fn!(test_pcm, 0);
test_fn!(test_pcm_alaw, 1);
test_fn!(test_pcm_ulaw, 2);
test_fn!(test_adpcm_ms, 3);
test_fn!(test_adpcm_ima, 4);
test_fn!(test_adpcm_yamaha, 5);
test_fn!(test_mp3, 6);
test_fn!(test_opus, 7);
test_fn!(test_flac, 8);
test_fn!(test_vorbis, 9);
test_fn!(test_oggvorbis1, 10);
test_fn!(test_oggvorbis2, 11);
test_fn!(test_oggvorbis3, 12);
test_fn!(test_oggvorbis1p, 13);
test_fn!(test_oggvorbis2p, 14);
test_fn!(test_oggvorbis3p, 15);
