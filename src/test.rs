
mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod filehasher;
mod adpcm;
mod encoders;
mod decoders;
mod wavcore;
mod wavreader;
mod wavwriter;

use std::env::args;
use std::process::ExitCode;
use std::error::Error;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, StringIO};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::{EncoderBasic, Encoder, PcmEncoder};
pub use decoders::{Decoder, PcmDecoder};
pub use wavcore::{DataFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtChunkExtension, BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, WaveIter};
pub use wavwriter::{FileSizeOption, WaveWriter};

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
    let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::PCM_Int, FileSizeOption::ForceUse4GBFormat).unwrap();

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
