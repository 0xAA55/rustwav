# RustWAV

因为不爽 hound 库，它的接口太差了，功能也不行，迭代器也不行。因此决定自己重新造 WAV 的轮子。

## Language 语言

[English](Readme.md) | 简体中文

## 特点

### 音频读取器：
* 支持超过 4GB 的 WAV 音频文件的读取。
* 支持 `Chunk` 随机分布存储的 WAV 文件的读取。
* 能根据提供的 **泛型参数** ，生成对应的迭代器用于获取音频帧，每个音频帧里的样本格式都是转换好的泛型类型，转换的过程 **严格按照样本格式的数值范围来进行伸缩** 。
	* 泛型参数支持 `i8` `i16` `i24` `i32` `i64` `u8` `u16` `u24` `u32` `u64` `f32` `f64`
	* 不论原始音频格式是怎么存储的，迭代器都可以将其转换为上述的这些泛型格式。
	* 如果原始音频格式和你提供的泛型类型完全相同，则完全不会发生任何转换。
	* 除非你频繁改变 **泛型参数类型** ，否则「读取器」会使用固定的格式转换器对读取的音频数据进行格式转换，因此能高效读取并转换音频样本格式。
* 能支持 **同时生成多个迭代器** ，每个迭代器都是独立的，各自从各自的位置读取音频帧。
* 能读取音频里面的乐曲信息相关元数据。
	* 由于在有些系统比如 Windows 里，音乐的元数据的字符串编码是按照系统编码来的（比如代码页 936 编码格式 GB2312），我对此专门做了处理。
        * 一旦是在 Windows 编译，我会调用 `GetACP()` 获取当前代码页，根据代码页获取编码，然后使用 `encoding` 库将其转换为 UTF-8 格式的字符串。
        * 但是在音频写入器则会按 UTF-8 格式写入音乐的元数据信息。
    * 支持 id3 元数据。
* 允许直接使用具有 `Read + Seek` 的 `trait` 作为数据输入来创建音频读取器。在这种情况下，音频读取器会生成一个临时文件用于存储音频的 `data` 部分。
	* 临时文件使用的是操作系统特有的“句柄一旦关闭即可删除文件”的特性，因此就算程序中途退出了，这个临时文件也会被自动删除。
	* 如果是使用文件路径来创建音频读取器，则不会生成任何临时文件。
* 除非明显的函数参数输入错误，否则无任何 `panic!`

### 音频写入器
* 支持超过 4GB 的 WAV 音频文件的写入。
* 创建音频文件的时候指定存储的音频文件内部的具体格式。
* 写入音频的函数 `write_frame()` 根据输入的 **泛型参数** ，将输入的音频样本数据 **自动转换** 为音频文件本身指定的样本格式进行存储。
	* 除非你频繁改变 **泛型参数类型** ，否则「写入器」会使用固定的格式转换器对要写入的音频数据进行格式转换，因此能高效转换并写入音频样本格式。
* 写入音频的函数 `write_frame()` 是线程安全的。
* 能够写入乐曲信息相关元数据，还能从别的音频读取器照搬所有的乐曲信息相关元数据。
* 音频写入器会在被销毁的时候自动完成对文件头和 `data` 块、`ds64` 块的信息补充包括填写文件大小等。
	* 也可以通过手动调用 `finalize()` 进行优雅的最终处理。
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

* 大多数内部结构体支持直接 `Dbg!()` 输出。

## 用法（示例代码）

```rust
use std::env::args;
use std::process::ExitCode;
use std::error::Error;

use wavreader::WaveReader;
use wavwriter::{WaveWriter, Spec, FileSizeOption, SampleFormat};

// test：读取 arg1 的音频文件，写入到 arg2 的音频文件
fn test(arg1: &str, arg2: &str) -> Result<(), Box<dyn Error>> {

    // 读取 arg1 的音频文件，得到一个 WaveReader 的实例
    let mut wavereader = WaveReader::open(arg1).unwrap();

    // 获取原本音频文件的数据参数
    let orig_spec = wavereader.spec();

    // 这里可以修改参数，能改变样本的位数和格式等。
    // WAV 实际支持的样本的位数和格式有限。
    let spec = Spec {
        channels: orig_spec.channels,
        channel_mask: orig_spec.channel_mask,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 24, // 音频改成 24 位
        sample_format: SampleFormat::Int, // 使用有符号整数
    };

    // 音频写入器，将音频信息写入到 arg2 文件
    let mut wavewriter = WaveWriter::create(arg2, &spec, FileSizeOption::ForceUse4GBFormat).unwrap();

    // 使用迭代器读取 WaveReader 的音频，注意迭代器支持一个泛型参数，此处设置的是 f32
    // 迭代器会自动把读取到的原始音频格式按照这个泛型格式做转换，并使样本的数值符合样本数据类型的范围
    for frame in wavereader.iter::<f32>()? {

        // 音频写入器写入每个音频帧
        wavewriter.write_frame(&frame)?;
    }

    // 音频写入器从音频读取器那里读取音乐元数据过来
    wavewriter.migrate_metadata_from_reader(&wavereader);

    // 输出调试信息
    dbg!(&wavereader);
    dbg!(&wavewriter);

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {return ExitCode::from(1);}

    // 输入 args[1]，输出 output.wav
    match test(&args[1], "output.wav") {
        Ok(_) => ExitCode::from(0),
        Err(e) => {
            println!("Error: {}", e);
            ExitCode::from(2)
        },
    }
}
```
