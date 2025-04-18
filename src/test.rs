mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod copiablebuf;
mod filehasher;
mod adpcm;
mod xlaw;
mod encoders;
mod decoders;
mod wavcore;
mod wavreader;
mod wavwriter;
mod resampler;
mod hacks;

pub mod utils;

pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use readwrite::{Reader, Writer};
pub use wavcore::{Spec, SampleFormat, DataFormat, AdpcmSubFormat};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use resampler::Resampler;
pub use errors::{AudioReadError, AudioError, AudioWriteError};

use std::env::args;
use std::error::Error;
use std::process::ExitCode;

const FORMATS: [(&str, DataFormat); 8] = [
        ("pcm", DataFormat::Pcm),
        ("pcm-alaw", DataFormat::PcmALaw),
        ("pcm-ulaw", DataFormat::PcmMuLaw),
        ("adpcm-ms", DataFormat::Adpcm(AdpcmSubFormat::Ms)),
        ("adpcm-ima", DataFormat::Adpcm(AdpcmSubFormat::Ima)),
        ("adpcm-yamaha", DataFormat::Adpcm(AdpcmSubFormat::Yamaha)),
        ("mp3", DataFormat::Mp3),
        ("opus", DataFormat::Opus),
];

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

    let mut data_format = DataFormat::Unspecified;
    for format in FORMATS {
        if arg1 == format.0 {
            data_format = format.1;
            break;
        }
    }

    if data_format == DataFormat::Unspecified {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unknown format `{arg1}`. Please input one of these:\n{}", FORMATS.iter().map(|(s, _v)|{s.to_string()}).collect::<Vec<String>>().join(", "))).into());
    }

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
