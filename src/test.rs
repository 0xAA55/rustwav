
mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod wavcore;
mod wavreader;
mod wavwriter;

use std::env::args;
use std::process::ExitCode;
use std::error::Error;

use wavreader::WaveReader;
use wavwriter::{WaveWriter, Spec, SampleFormat};

fn test(arg1: &str, arg2: &str) -> Result<(), Box<dyn Error>> {
    let mut wavereader = WaveReader::open(arg1).unwrap();

    let spec = Spec {
        channels: 2,
        channel_mask: Spec::guess_channel_mask(2)?,
        sample_rate: wavereader.spec().sample_rate,
        bits_per_sample: 8,
        sample_format: SampleFormat::UInt,
    };

    let mut wavewriter = WaveWriter::create(arg2, &spec, true).unwrap();

    for frame in wavereader.iter::<f32>()? {
        wavewriter.write_sample(&frame)?;
    }

    dbg!(&wavereader);
    dbg!(&wavewriter);

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {return ExitCode::from(1);}

    match test(&args[1], "output.wav") {
        Ok(_) => ExitCode::from(0),
        Err(e) => {
            println!("Error: {}", e);
            ExitCode::from(2)
        },
    }
}
