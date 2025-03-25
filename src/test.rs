
mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod wavcore;
mod wavreader;
mod wavwriter;

use std::{env, process::ExitCode};

use wavreader::WaveReader;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {return ExitCode::from(1);}

    let wavereader = WaveReader::open(&args[1]).unwrap();
    dbg!(wavereader);

    ExitCode::from(0)
}
