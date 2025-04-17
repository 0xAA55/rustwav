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
