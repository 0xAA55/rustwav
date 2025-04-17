# RustWAV

I was dissatisfied with the hound library - its API was poor, functionality limited, and iterator implementation subpar. Thus, I decided to reinvent the WAV wheel myself.

## Language 语言

English | [简体中文](Readme-CN.md)

## Features

### Audio Reader:
* Supports reading WAV audio files over 4GB in size.
* Supports embedded formats including PCM, PCM-aLaw, PCM-muLaw, ADPCM-MS, ADPCM-IMA, ADPCM-YAMAHA, MP3, Opus, etc.
* Resampler support assists in modifying sample rates.
* Supports reading WAV files with randomly distributed Chunk storage.
* Generates corresponding iterators via **generic parameters** to retrieve audio frames, with sample formats in each frame **strictly converted to specified generic types according to their numerical ranges**.
    * Supported generic types: `i8`, `i16`, `i24`, `i32`, `i64`, `u8`, `u16`, `u24`, `u32`, `u64`, `f32`, `f64`
    * Regardless of original audio storage format, iterators can convert to the above generic formats.
    * No conversion occurs when original format matches the specified generic type.
* Supports creating multiple independent iterators, each reading audio frames from their own positions.
* Reads music metadata information:
    * Special handling for system-specific string encodings (e.g., Code Page 936/GB2312 on Windows):
        * On Windows builds, calls `GetACP()` to detect code page, retrieves corresponding encoding, and converts to UTF-8 using the encoding crate.
    * Supports ID3 metadata.
* Allows creating audio readers using any Read + Seek trait implementer as input. In this mode, a temporary file stores the audio's `data` section:
    * Leverages OS-specific "delete-on-close" behavior for temporary files - automatically cleaned up even if the program crashes.
    * No temporary files created when using file paths to initialize readers.
* No panic! except for explicit parameter errors.


### Audio Writer
* Supports writing WAV audio files over 4GB in size.
* Supports embedded formats including PCM, PCM-aLaw, PCM-muLaw, ADPCM-MS, ADPCM-IMA, ADPCM-YAMAHA, MP3, Opus, etc.
* write_frame() function accepts **generic parameters**, encoding input samples for storage.
* Writes music metadata and can copy all metadata from other audio readers.
* No panic! except for explicit parameter errors.

### Other Features
* Supports channel configurations including but not limited to:

    * `FrontLeft`
    * `FrontRight`
    * `FrontCenter`
    * `LowFreq`
    * `BackLeft`
    * `BackRight`
    * `FrontLeftOfCenter`
    * `FrontRightOfCenter`
    * `BackCenter`
    * `SideLeft`
    * `SideRight`
    * `TopCenter`
    * `TopFrontLeft`
    * `TopFrontCenter`
    * `TopFrontRight`
    * `TopBackLeft`
    * `TopBackCenter`
    * `TopBackRight`

* Most internal structs support direct `dbg!()` output.

## Usage Example
```rust
use std::env::args;
use std::error::Error;
use std::process::ExitCode;

// test：读取 arg1 的音频文件，写入到 arg2 的音频文件
fn test(arg1: &str, arg2: &str, arg3: &str, arg4: &str) -> Result<(), Box<dyn Error>> {
    #[allow(unused_imports)]
    use FileSizeOption::{NeverLargerThan4GB, AllowLargerThan4GB, ForceUse4GBFormat};

    let transfer_block_size = 65536usize;

    let mut resampler = Resampler::new(transfer_block_size);

    println!("======== TEST 1 ========");

    // 读取 arg1 的音频文件，得到一个 WaveReader 的实例
    let mut wavereader = WaveReader::open(arg2).unwrap();

    // 获取原本音频文件的数据参数
    let orig_spec = *wavereader.spec();

    // 这里可以修改参数，能改变样本的位数和格式等。
    // WAV 实际支持的样本的位数和格式有限。
    let spec = Spec {
        channels: orig_spec.channels,
        channel_mask: 0,
        sample_rate: 48000,
        bits_per_sample: 16, // 设置样本位数
        sample_format: SampleFormat::Int, // 使用有符号整数
    };

    let data_format = match arg1{
        "pcm" => DataFormat::Pcm,
        "pcm-alaw" => DataFormat::PcmALaw,
        "pcm-ulaw" => DataFormat::PcmMuLaw,
        "adpcm-ms" => DataFormat::Adpcm(AdpcmSubFormat::Ms),
        "adpcm-ima" => DataFormat::Adpcm(AdpcmSubFormat::Ima),
        "adpcm-yamaha" => DataFormat::Adpcm(AdpcmSubFormat::Yamaha),
        "mp3" => DataFormat::Mp3,
        "opus" => DataFormat::Opus,
        other => {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unknown format `{other}`. Please input one of these:\n{}",
                [
                    "pcm",
                    "pcm-alaw",
                    "pcm-ulaw",
                    "adpcm-ms",
                    "adpcm-ima",
                    "adpcm-yamaha",
                    "mp3",
                    "opus",
                ].join(", ")
            )).into());
        },
    };

    // 音频写入器，将音频信息写入到 arg2 文件
    let mut wavewriter = WaveWriter::create(arg3, &spec, data_format, NeverLargerThan4GB).unwrap();

    let process_size = resampler.get_process_size(transfer_block_size, orig_spec.sample_rate, spec.sample_rate);
    match spec.channels {
        1 => {
            let mut iter = wavereader.mono_iter::<i16>()?;
            loop {
                let block: Vec<i16> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_mono(&mut resampler, &block, orig_spec.sample_rate, spec.sample_rate);
                wavewriter.write_monos(&block)?;
            }
        },
        2 => {
            let mut iter = wavereader.stereo_iter::<i16>()?;
            loop {
                let block: Vec<(i16, i16)> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_stereo(&mut resampler, &block, orig_spec.sample_rate, spec.sample_rate);
                wavewriter.write_stereos(&block)?;
            }
        },
        _ => {
            let mut iter = wavereader.frame_iter::<i16>()?;
            loop {
                let block: Vec<Vec<i16>> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_frames(&mut resampler, &block, orig_spec.sample_rate, spec.sample_rate);
                wavewriter.write_frames(&block)?;
            }
        }
    }

    // 音频写入器从音频读取器那里读取音乐元数据过来
    wavewriter.migrate_metadata_from_reader(&wavereader);
    wavewriter.finalize()?;

    // 输出调试信息
    dbg!(&wavereader);
    dbg!(&wavewriter);

    println!("======== TEST 2 ========");

    let spec2 = Spec {
        channels: spec.channels,
        channel_mask: 0,
        sample_rate: 44100,
        bits_per_sample: 16, // 设置样本位数
        sample_format: SampleFormat::Int, // 使用有符号整数
    };

    let mut wavereader_2 = WaveReader::open(arg3).unwrap();
    let mut wavewriter_2 = WaveWriter::create(arg4, &spec2, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    let process_size = resampler.get_process_size(transfer_block_size, spec.sample_rate, spec2.sample_rate);
    match spec2.channels {
        1 => {
            let mut iter = wavereader_2.mono_iter::<i16>()?;
            loop {
                let block: Vec<i16> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_mono(&mut resampler, &block, spec.sample_rate, spec2.sample_rate);
                wavewriter_2.write_monos(&block)?;
            }
        },
        2 => {
            let mut iter = wavereader_2.stereo_iter::<i16>()?;
            loop {
                let block: Vec<(i16, i16)> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_stereo(&mut resampler, &block, spec.sample_rate, spec2.sample_rate);
                wavewriter_2.write_stereos(&block)?;
            }
        },
        _ => {
            let mut iter = wavereader_2.frame_iter::<i16>()?;
            loop {
                let block: Vec<Vec<i16>> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_frames(&mut resampler, &block, spec.sample_rate, spec2.sample_rate);
                wavewriter_2.write_frames(&block)?;
            }
        }
    }

    // 音频写入器从音频读取器那里读取音乐元数据过来
    wavewriter_2.migrate_metadata_from_reader(&wavereader_2);
    wavewriter_2.finalize()?;

    // 输出调试信息
    dbg!(&wavereader_2);
    dbg!(&wavewriter_2);

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = args().collect();
    if args.len() < 5 {return ExitCode::from(1);}

    match test(&args[1], &args[2], &args[3], &args[4]) {
        Ok(_) => ExitCode::from(0),
        Err(e) => {
            println!("Error: {}", e);
            ExitCode::from(2)
        },
    }
}
```
