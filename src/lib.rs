pub use rustwav_core::*;

use std::{env::args, process::ExitCode};

/// * The list for the command line program to parse the argument and we have the pre-filled encoder initializer parameter structs for each format.
#[allow(clippy::large_const_arrays)]
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
            bitrate: Some(OggVorbisBitrateStrategy::Vbr(160000)),
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

/// * The `test()` function
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
test_fn!(test_nakedvorbis, 9);
test_fn!(test_oggvorbis1, 10);
test_fn!(test_oggvorbis2, 11);
test_fn!(test_oggvorbis3, 12);
test_fn!(test_oggvorbis1p, 13);
test_fn!(test_oggvorbis2p, 14);
test_fn!(test_oggvorbis3p, 15);


/// * A function dedicated to testing WAV encoding and decoding. This function is actually a `main()` function for a command-line program that parses `args` and returns an `ExitCode`.
/// * The usage is `arg0 [format] [test.wav] [output.wav]`
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
