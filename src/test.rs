
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
pub mod utils;

pub use errors::{AudioError, AudioReadError, AudioWriteError};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, SharedWriter, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::EncoderToImpl;
pub use encoders::{Encoder, PcmEncoder, AdpcmEncoderWrap};
pub use decoders::{Decoder, PcmDecoder, AdpcmDecoderWrap};
pub use adpcm::{AdpcmCodec};
pub use adpcm::{AdpcmEncoder, AdpcmEncoderBS, AdpcmEncoderOKI, AdpcmEncoderOKI6258, AdpcmEncoderYMA, AdpcmEncoderYMB, AdpcmEncoderYMZ, AdpcmEncoderAICA, AdpcmEncoderIMA};
pub use adpcm::{AdpcmDecoder, AdpcmDecoderBS, AdpcmDecoderOKI, AdpcmDecoderOKI6258, AdpcmDecoderYMA, AdpcmDecoderYMB, AdpcmDecoderYMZ, AdpcmDecoderAICA, AdpcmDecoderIMA};
pub use adpcm::{EncBS, EncOKI, EncOKI6258, EncYMA, EncYMB, EncYMZ, EncAICA, EncIMA};
pub use adpcm::{DecBS, DecOKI, DecOKI6258, DecYMA, DecYMB, DecYMZ, DecAICA, DecIMA};
pub use wavcore::{DataFormat, AdpcmSubFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtChunkExtension, FmtChunkAdpcmData, FmtChunkExtensible};
pub use wavcore::{BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
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
    #[allow(unused_imports)]
    use FileSizeOption::{NeverLargerThan4GB, AllowLargerThan4GB, ForceUse4GBFormat};

    println!("======== TEST 1 ========");

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

    // 音频写入器，将音频信息写入到 arg2 文件
    let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Adpcm(AdpcmSubFormat::Ima), NeverLargerThan4GB).unwrap();
    // let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Mp3, NeverLargerThan4GB).unwrap();

    match spec.channels {
        1 => {
            for stereo in wavereader.mono_iter::<i16>()? {
                wavewriter.write_mono(stereo)?;
            }
        },
        2 => {
            for stereo in wavereader.stereo_iter::<i16>()? {
                wavewriter.write_stereo(stereo)?;
            }
        },
        _ => {
            for frame in wavereader.frame_iter::<i16>()? {
                wavewriter.write_frame(&frame)?;
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

    let mut wavereader_2 = WaveReader::open(arg2).unwrap();
    let mut wavewriter_2 = WaveWriter::create("output2.wav", &spec, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    match spec.channels {
        1 => {
            for stereo in wavereader_2.mono_iter::<i16>()? {
                wavewriter_2.write_mono(stereo)?;
            }
        },
        2 => {
            for stereo in wavereader_2.stereo_iter::<i16>()? {
                wavewriter_2.write_stereo(stereo)?;
            }
        },
        _ => {
            for frame in wavereader_2.frame_iter::<i16>()? {
                wavewriter_2.write_frame(&frame)?;
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
