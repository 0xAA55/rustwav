pub use rustwav_core::*;

use std::{env::args, process::ExitCode};

/// * A function dedicated to testing WAV encoding and decoding. This function is actually a `main()` function for a command-line program that parses `args` and returns an `ExitCode`.
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
test_fn!(test_nakedvorbis, 9);
test_fn!(test_oggvorbis1, 10);
test_fn!(test_oggvorbis2, 11);
test_fn!(test_oggvorbis3, 12);
test_fn!(test_oggvorbis1p, 13);
test_fn!(test_oggvorbis2p, 14);
test_fn!(test_oggvorbis3p, 15);
