
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
pub mod resampler;
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
pub use resampler::Resampler;

#[cfg(feature = "mp3dec")]
pub use decoders::mp3::Mp3Decoder;

#[cfg(feature = "mp3enc")]
pub use encoders::mp3::Mp3Encoder;

#[cfg(feature = "opus")]
pub use encoders::opus::OpusEncoder;

use std::env::args;
use std::error::Error;
use std::process::ExitCode;

fn do_resample<S>(resampler: &mut Resampler, input: &[S], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<S>
where S: SampleType {
    if src_sample_rate == dst_sample_rate {
        input.to_vec()
    } else if src_sample_rate > dst_sample_rate {
        // 源采样率高于目标采样率，说明要压缩波形
        let input = utils::sample_conv::<S, f32>(input);
        let desired_length = resampler.get_fft_size() * dst_sample_rate as usize / src_sample_rate as usize;
        let f32_result = resampler.resample(&input, desired_length).unwrap();
        utils::sample_conv::<f32, S>(&f32_result)
    } else {
        // 源采样率低于目标采样率，说明要拉长波形
        let input = utils::sample_conv::<S, f32>(input);
        let desired_length = (resampler.get_fft_size() * dst_sample_rate as usize / src_sample_rate as usize) / 4;
        let proc_size = resampler.get_fft_size() / 4;
        let mut iter = input.into_iter();
        let mut ret = Vec::<S>::new();
        loop {
            let chunk: Vec<f32> = iter.by_ref().take(proc_size).collect();
            if chunk.len() == 0 {
                break;
            }
            let f32_result = resampler.resample(&chunk, desired_length).unwrap();
            ret.extend(utils::sample_conv::<f32, S>(&f32_result));
        }
        ret
    }
}

// test：读取 arg1 的音频文件，写入到 arg2 的音频文件
fn test(arg1: &str, arg2: &str) -> Result<(), Box<dyn Error>> {
    #[allow(unused_imports)]
    use FileSizeOption::{NeverLargerThan4GB, AllowLargerThan4GB, ForceUse4GBFormat};

    let transfer_by_blocks = true;
    let transfer_block_size = 4096usize;

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

    dbg!(&spec);

    // 音频写入器，将音频信息写入到 arg2 文件
    let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Adpcm(AdpcmSubFormat::Ms), NeverLargerThan4GB).unwrap();
    // let mut wavewriter = WaveWriter::create(arg2, &spec, DataFormat::Mp3, NeverLargerThan4GB).unwrap();

    if transfer_by_blocks {
        match spec.channels {
            1 => {
                let mut iter = wavereader.mono_iter::<i16>()?;
                loop {
                    let block: Vec<i16> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter.write_monos(&block)?;
                }
            },
            2 => {
                let mut iter = wavereader.stereo_iter::<i16>()?;
                loop {
                    let block: Vec<(i16, i16)> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter.write_stereos(&block)?;
                }
            },
            _ => {
                let mut iter = wavereader.frame_iter::<i16>()?;
                loop {
                    let block: Vec<Vec<i16>> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter.write_frames(&block)?;
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

    let spec = Spec {
        channels: spec.channels,
        channel_mask: 0,
        sample_rate: 48000,
        bits_per_sample: 16, // 设置样本位数
        sample_format: SampleFormat::Int, // 使用有符号整数
    };

    let mut wavereader_2 = WaveReader::open(arg2).unwrap();
    let mut wavewriter_2 = WaveWriter::create("output2.wav", &spec, DataFormat::Opus, NeverLargerThan4GB).unwrap();

    if transfer_by_blocks {
        match spec.channels {
            1 => {
                let mut iter = wavereader_2.mono_iter::<i16>()?;
                loop {
                    let block: Vec<i16> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter_2.write_monos(&block)?;
                }
            },
            2 => {
                let mut iter = wavereader_2.stereo_iter::<i16>()?;
                loop {
                    let block: Vec<(i16, i16)> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter_2.write_stereos(&block)?;
                }
            },
            _ => {
                let mut iter = wavereader_2.frame_iter::<i16>()?;
                loop {
                    let block: Vec<Vec<i16>> = iter.by_ref().take(transfer_block_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    wavewriter_2.write_frames(&block)?;
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
