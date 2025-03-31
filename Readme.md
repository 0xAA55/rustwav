# RustWAV

Frustrated with the poor interface, limited functionality, and inadequate iterators of the hound library, I decided to reinvent the WAV wheel myself.

## Language 语言

English | [简体中文](Readme-CN.md)

## Features

### Audio Reader:
* Supports reading WAV files larger than 4GB
* Supports reading WAV files with MP3 as it's content rather than PCM
* Supports reading WAV files with randomly distributed chunks
* Generates iterators for audio frames through **generic parameters**, with samples strictly converted to specified types within their numerical ranges:
  * Generic parameter support: `i8` `i16` `i24` `i32` `i64` `u8` `u16` `u24` `u32` `u64` `f32` `f64`
  * Converts any native audio format to these generic types
  * Zero conversion when input format matches generic type
  * Cached format converter for consistent performance when using same generic type
* Supports multiple independent iterators reading from different positions
* Reads music metadata:
  * Windows-specific handling: Uses `GetACP()` to detect code page (e.g., GB2312 code page 936), then converts to UTF-8 via `encoding` crate
  * Writers always use UTF-8 for metadata
  * Decodes id3 metadata
* Accepts any `Read + Seek` input, creating temporary files with OS-managed deletion on handle close
* No panics unless caused by obvious parameter errors

### Audio Writer
* Supports writing WAV files larger than 4GB
* Lets you specify storage format during file creation
* `write_frame()` automatically converts input samples to storage format:
  * Cached converter for consistent performance with same generic type
* Thread-safe frame writing
* Preserves and migrates all metadata from readers
* Automatically finalizes headers/data blocks/ds64 on drop (or via `finalize()`)
* No panics unless caused by obvious parameter errors

### Other Features
* Supports various channel configurations including:
  * `FrontLeft`, `FrontRight`, `FrontCenter`, `LowFreq`
  * `BackLeft`, `BackRight`, `FrontLeftOfCenter`, `FrontRightOfCenter`
  * `BackCenter`, `SideLeft`, `SideRight`, `TopCenter`
  * `TopFrontLeft`, `TopFrontCenter`, `TopFrontRight`
  * `TopBackLeft`, `TopBackCenter`, `TopBackRight`
* Most internal structs support `Dbg!()` output

## Usage Example
```rust
use std::env::args;
use std::process::ExitCode;
use std::error::Error;

use wavreader::WaveReader;
use wavwriter::{WaveWriter, Spec, FileSizeOption, SampleFormat};

// read `arg1` as input, do some conversion then write to `arg2`
fn test(arg1: &str, arg2: &str) -> Result<(), Box<dyn Error>> {

    // This is the wave reader.
    let mut wavereader = WaveReader::open(arg1).unwrap();

    let orig_spec = wavereader.spec();

    let spec = Spec {
        channels: orig_spec.channels,
        channel_mask: orig_spec.channel_mask,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 24, // Changed to 24 bits per sample
        sample_format: SampleFormat::Int, // Use signed int
    };

    // This is the wave writer.
    let mut wavewriter = WaveWriter::create(arg2, &spec, FileSizeOption::ForceUse4GBFormat).unwrap();

    // Use iterator to read audio frames from the reader.
    // Note that `iter` has a generic type that is the output format of the iterator.
    // The iterator will convert the original sample format to the generic type.
    for frame in wavereader.iter::<f32>()? {

        // Write every audio frame to the writer
        wavewriter.write_frame(&frame)?;
    }

    wavewriter.migrate_metadata_from_reader(&wavereader);

    // 输出调试信息
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
```
