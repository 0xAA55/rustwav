# RustWAV

因为不爽 hound 库，它的接口太差了，功能也不行，迭代器也不行。因此决定自己重新造 WAV 的轮子。

## Language 语言

[English](Readme.md) | 简体中文

## 特点

### 音频读取器：
* 支持超过 4GB 的 WAV 音频文件的读取。
* 支持 PCM、PCM-aLaw、PCM-muLaw、ADPCM-MS、ADPCM-IMA、ADPCM-YAMAHA、MP3、Opus 等内嵌格式。
* 支持 Resampler 可协助用于修改采样率。
* 支持 `Chunk` 随机分布存储的 WAV 文件的读取。
* 能根据提供的 **泛型参数** ，生成对应的迭代器用于获取音频帧，每个音频帧里的样本格式都是转换好的泛型类型，转换的过程 **严格按照样本格式的数值范围来进行伸缩** 。
	* 泛型参数支持 `i8` `i16` `i24` `i32` `i64` `u8` `u16` `u24` `u32` `u64` `f32` `f64`
	* 不论原始音频格式是怎么存储的，迭代器都可以将其转换为上述的这些泛型格式。
	* 如果原始音频格式和你提供的泛型类型完全相同，则完全不会发生任何转换。
* 能支持同时生成多个迭代器，每个迭代器都是独立的，各自从各自的位置读取音频帧。
* 能读取音频里面的乐曲信息相关元数据。
	* 由于在有些系统比如 Windows 里，音乐的元数据的字符串编码是按照系统编码来的（比如代码页 936 编码格式 GB2312），我对此专门做了处理。
        * 一旦是在 Windows 编译，则调用 `GetACP()` 获取当前代码页，根据代码页获取编码，然后使用 `encoding` 库将其转换为 UTF-8 格式的字符串。
    * 支持 id3 元数据。
* 允许直接使用具有 `Read + Seek` 的 `trait` 作为数据输入来创建音频读取器。在这种情况下，音频读取器会生成一个临时文件用于存储音频的 `data` 部分。
	* 临时文件使用的是操作系统特有的“句柄一旦关闭即可删除文件”的特性，因此就算程序中途退出了，这个临时文件也会被自动删除。
	* 如果是使用文件路径来创建音频读取器，则不会生成任何临时文件。
* 除非明显的函数参数输入错误，否则无任何 `panic!`

### 音频写入器
* 支持超过 4GB 的 WAV 音频文件的写入。
* 支持 PCM、PCM-aLaw、PCM-muLaw、ADPCM-MS、ADPCM-IMA、ADPCM-YAMAHA、MP3、Opus 等内嵌格式。
* 写入音频的函数 `write_frame()` 支持 **泛型参数** ，将输入的音频样本进行编码后存储。
* 能够写入乐曲信息相关元数据，还能从别的音频读取器照搬所有的乐曲信息相关元数据。
* 除非明显的函数参数输入错误，否则无任何 `panic!`

### 其它特性
* 每个音频帧的数据处理单声道和立体声，我们还支持以下各种声道的各种组合：
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

* 大多数内部结构体支持直接 `dbg!()` 输出。

## 用法（示例代码）

```rust
use sampleutils::{SampleType, SampleFrom, i24, u24};
use readwrite::{Reader, Writer};
use wavcore::{Spec, SampleFormat, DataFormat, AdpcmSubFormat};
use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
use wavwriter::{FileSizeOption, WaveWriter};
use resampler::Resampler;
use errors::{AudioReadError, AudioError, AudioWriteError};

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

#[allow(unused_imports)]
use FileSizeOption::{NeverLargerThan4GB, AllowLargerThan4GB, ForceUse4GBFormat};

fn transfer_audio_from_decoder_to_encoder(decoder: &mut WaveReader, encoder: &mut WaveWriter) {
    // The fft size can be any number greater than the sample rate of the encoder or the decoder.
    // It is for the resampler. A greater number results in better resample quality, but the process could be slower.
    // In most cases, the audio sampling rate is about 11025 to 48000, so 65536 is the best number for the resampler.
    const FFT_SIZE: usize = 65536;

    // This is the resampler, if the decoder's sample rate is different than the encode sample rate, use the resampler to help stretch or compress the waveform.
    // Otherwise, it's not needed there.
    let mut resampler = Resampler::new(FFT_SIZE);

    // The decoding audio spec
    let decode_spec = *decoder.spec();

    // The encoding audio spec
    let encode_spec = *encoder.spec();

    // The number of channels must match
    assert_eq!(encode_spec.channels, decode_spec.channels);

    // Process size is for the resampler to process the waveform, it is the length of the source waveform slice.
    let process_size = resampler.get_process_size(FFT_SIZE, decode_spec.sample_rate, encode_spec.sample_rate);

    // There are three types of iterators for three types of audio channels: mono, stereo, and more than 2 channels of audio.
    // Usually, the third iterator can handle all numbers of channels, but it's the slowest iterator.
    match encode_spec.channels {
        1 => {
            let mut iter = decoder.mono_iter::<f32>().unwrap();
            loop {
                let block: Vec<f32> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_mono(&mut resampler, &block, decode_spec.sample_rate, encode_spec.sample_rate);
                encoder.write_monos(&block).unwrap();
            }
        },
        2 => {
            let mut iter = decoder.stereo_iter::<f32>().unwrap();
            loop {
                let block: Vec<(f32, f32)> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_stereo(&mut resampler, &block, decode_spec.sample_rate, encode_spec.sample_rate);
                encoder.write_stereos(&block).unwrap();
            }
        },
        _ => {
            let mut iter = decoder.frame_iter::<f32>().unwrap();
            loop {
                let block: Vec<Vec<f32>> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_frames(&mut resampler, &block, decode_spec.sample_rate, encode_spec.sample_rate);
                encoder.write_frames(&block).unwrap();
            }
        }
    }
}

// The `test()` function
// arg1: the format, e.g. "pcm"
// arg2: the input file to parse and decode, tests the decoder for the input file.
// arg3: the output file to encode, test the encoder.
// arg4: re-decode arg3 and encode to pcm to test the decoder.
fn test(arg1: &str, arg2: &str, arg3: &str, arg4: &str) -> Result<(), Box<dyn Error>> {
    let mut data_format = DataFormat::Unspecified;
    for format in FORMATS {
        if arg1 == format.0 {
            data_format = format.1;
            break;
        }
    }

    // Failed to match the data format
    if data_format == DataFormat::Unspecified {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unknown format `{arg1}`. Please input one of these:\n{}", FORMATS.iter().map(|(s, _v)|{s.to_string()}).collect::<Vec<String>>().join(", "))).into());
    }

    println!("======== TEST 1 ========");

    // This is the decoder
    let mut wavereader = WaveReader::open(arg2).unwrap();

    let orig_spec = *wavereader.spec();

    // The spec for the encoder
    let spec = Spec {
        channels: orig_spec.channels,
        channel_mask: 0,
        sample_rate: 48000, // Specify a sample rate to test the resampler
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // This is the encoder
    let mut wavewriter = WaveWriter::create(arg3, &spec, data_format, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader, &mut wavewriter);

    // Get the metadata from the decoder
    wavewriter.migrate_metadata_from_reader(&wavereader);

    // It's not needed to call `finalize()` after use, but calling it will free the memory and resources immediately.
    wavewriter.finalize();

    // Show debug info
    dbg!(&wavereader);
    dbg!(&wavewriter);

    println!("======== TEST 2 ========");

    let spec2 = Spec {
        channels: spec.channels,
        channel_mask: 0,
        sample_rate: 44100, // Changed to another sample rate to test the resampler.
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut wavereader_2 = WaveReader::open(arg3).unwrap();
    let mut wavewriter_2 = WaveWriter::create(arg4, &spec2, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader_2, &mut wavewriter_2);

    // Get the metadata from the decoder
    wavewriter_2.migrate_metadata_from_reader(&wavereader_2);

    // It's not needed to call `finalize()` after use, but calling it will free the memory and resources immediately.
    wavewriter_2.finalize();

    // Show debug info
    dbg!(&wavereader_2);
    dbg!(&wavewriter_2);

    Ok(())
}

#[test]
fn testrun() {
    for format in FORMATS {
        test(format.0, "test.wav", "output.wav", "output2.wav").unwrap();
    }
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
