
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
pub mod copiablebuf;
pub mod hacks;

pub use errors::{AudioError, AudioReadError, AudioWriteError, IOErrorInfo};
pub use savagestr::{StringCodecMaps, SavageStringCodecs};
pub use readwrite::{Reader, Writer, string_io};
pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use encoders::EncoderToImpl;
pub use encoders::{Encoder, PcmEncoder, AdpcmEncoderWrap};
pub use decoders::{Decoder, PcmDecoder, AdpcmDecoderWrap};
pub use adpcm::{AdpcmEncoder, AdpcmEncoderIMA, AdpcmEncoderMS};
pub use adpcm::{AdpcmDecoder, AdpcmDecoderIMA, AdpcmDecoderMS};
pub use adpcm::{EncIMA, EncMS};
pub use adpcm::{DecIMA, DecMS};
pub use wavcore::{DataFormat, AdpcmSubFormat, Spec, SampleFormat, WaveSampleType, SpeakerPosition};
pub use wavcore::{GUID, GUID_PCM_FORMAT, GUID_IEEE_FLOAT_FORMAT};
pub use wavcore::{ChunkWriter, ChunkHeader};
pub use wavcore::{FmtChunk, FmtExtension, ExtensionData, AdpcmMsData, AdpcmImaData, ExtensibleData, Mp3Data};
pub use wavcore::{BextChunk, SmplChunk, SmplSampleLoop, InstChunk, CueChunk, Cue, ListChunk, AdtlChunk, LablChunk, NoteChunk, LtxtChunk, AcidChunk, JunkChunk, Id3};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use filehasher::FileHasher;
pub use copiablebuf::{CopiableBuffer, CopiableBufferIter, CopiableBufferIterMut, CopiableBufferIntoIter};

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

    let transfer_by_blocks = true;
    let transfer_block_size = 1024usize;

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
    let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Adpcm(AdpcmSubFormat::Ms), NeverLargerThan4GB).unwrap();
    // let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Mp3, NeverLargerThan4GB).unwrap();

    if transfer_by_blocks {
        let mut frame_transfered = 0usize;
        match spec.channels {
            1 => {
                loop {
                    let iter = wavereader.mono_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<i16>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter.write_monos(&block)?;
                    frame_transfered += transfer_block_size;
                }
            },
            2 => {
                loop {
                    let iter = wavereader.stereo_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<(i16, i16)>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter.write_stereos(&block)?;
                    frame_transfered += transfer_block_size;
                }
            },
            _ => {
                loop {
                    let iter = wavereader.frame_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<Vec<i16>>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter.write_frames(&block)?;
                    frame_transfered += transfer_block_size;
                }
            }
        }
    } else {
        match spec.channels {
            1 => {
                for mono in wavereader.mono_iter::<i16>()? {
                    wavewriter.write_mono(mono)?;
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

    if transfer_by_blocks {
        let mut frame_transfered = 0usize;
        match spec.channels {
            1 => {
                loop {
                    let iter = wavereader_2.mono_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<i16>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter_2.write_monos(&block)?;
                    frame_transfered += transfer_block_size;
                }
            },
            2 => {
                loop {
                    let iter = wavereader_2.stereo_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<(i16, i16)>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter_2.write_stereos(&block)?;
                    frame_transfered += transfer_block_size;
                }
            },
            _ => {
                loop {
                    let iter = wavereader_2.frame_iter::<i16>()?;
                    let block = iter.skip(frame_transfered).take(transfer_block_size).collect::<Vec<Vec<i16>>>();
                    if block.len() == 0 {
                        break;
                    }
                    wavewriter_2.write_frames(&block)?;
                    frame_transfered += transfer_block_size;
                }
            }
        }
    } else {
        match spec.channels {
            1 => {
                for mono in wavereader_2.mono_iter::<i16>()? {
                    wavewriter_2.write_mono(mono)?;
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
