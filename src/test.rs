
pub mod errors;
pub mod savagestr;
pub mod readwrite;
pub mod sampleutils;
pub mod filehasher;
pub mod adpcm;
pub mod encoders;
pub mod decoders;
pub mod wavcore;
pub mod wavreader;
pub mod wavwriter;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::{EncoderToImpl, Encoder, PcmEncoder, AdpcmEncoder};
pub use decoders::{Decoder, PcmDecoder};
pub use adpcm::{AdpcmCodecTypes};
pub use adpcm::{AdpcmEncoderBS, AdpcmEncoderOKI, AdpcmEncoderOKI6258, AdpcmEncoderYMA, AdpcmEncoderYMB, AdpcmEncoderYMZ, AdpcmEncoderAICA};
pub use adpcm::{AdpcmDecoderBS, AdpcmDecoderOKI, AdpcmDecoderOKI6258, AdpcmDecoderYMA, AdpcmDecoderYMB, AdpcmDecoderYMZ, AdpcmDecoderAICA};
pub use adpcm::{EncBS, EncOKI, EncOKI6258, EncYMA, EncYMB, EncYMZ, EncAICA};
pub use adpcm::{DecBS, DecOKI, DecOKI6258, DecYMA, DecYMB, DecYMZ, DecAICA};
pub use wavcore::{DataFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtChunkExtension, BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, WaveIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use filehasher::FileHasher;

#[cfg(feature = "mp3dec")]
pub use decoders::MP3::Mp3Decoder;

#[cfg(feature = "mp3enc")]
pub use encoders::MP3::Mp3Encoder;

use std::env::args;
use std::error::Error;
use std::process::ExitCode;

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
        bits_per_sample: 16, // 设置样本位数
        sample_format: SampleFormat::Int, // 使用有符号整数
    };

    use FileSizeOption::{NeverLargerThan4GB, AllowLargerThan4GB, ForceUse4GBFormat};

    // 音频写入器，将音频信息写入到 arg2 文件
    let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Adpcm, NeverLargerThan4GB).unwrap();

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
