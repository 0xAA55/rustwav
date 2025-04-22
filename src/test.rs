mod errors;
mod savagestr;
mod readwrite;
mod sampleutils;
mod copiablebuf;
mod filehasher;
mod adpcm;
mod xlaw;
mod wavcore;
mod wavreader;
mod wavwriter;
mod resampler;
mod hacks;

pub mod utils;
pub mod encoders;
pub mod decoders;

#[cfg(feature = "flac")]
mod flac;

pub use sampleutils::{SampleType, SampleFrom, i24, u24};
pub use readwrite::{Reader, Writer};
pub use wavcore::{Spec, SampleFormat, DataFormat};
pub use wavreader::{WaveDataSource, WaveReader, FrameIter, StereoIter, MonoIter, FrameIntoIter, StereoIntoIter, MonoIntoIter};
pub use wavwriter::{FileSizeOption, WaveWriter};
pub use resampler::Resampler;
pub use errors::{AudioReadError, AudioError, AudioWriteError};
pub use wavcore::{AdpcmSubFormat};
pub use wavcore::{Mp3EncoderOptions, Mp3Channels, Mp3Quality, Mp3Bitrate, Mp3VbrMode};
pub use wavcore::{OpusEncoderOptions, OpusBitrate, OpusEncoderSampleDuration};

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
        ("mp3", DataFormat::Mp3(Mp3EncoderOptions{
            channels: Mp3Channels::NotSet,
            quality: Mp3Quality::Best,
            bitrate: Mp3Bitrate::Kbps320,
            vbr_mode: Mp3VbrMode::Off,
            id3tag: None,
        })),
        ("opus", DataFormat::Opus(OpusEncoderOptions{
            bitrate: OpusBitrate::Max,
            encode_vbr: false,
            samples_cache_duration: OpusEncoderSampleDuration::MilliSec60,
        })),
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
    let decode_spec = decoder.spec();

    // The encoding audio spec
    let encode_spec = encoder.spec();

    let decode_channels = decode_spec.channels;
    let encode_channels = encode_spec.channels;
    let decode_sample_rate = decode_spec.sample_rate;
    let encode_sample_rate = encode_spec.sample_rate;

    // The number of channels must match
    assert_eq!(encode_channels, decode_channels);

    // Process size is for the resampler to process the waveform, it is the length of the source waveform slice.
    let process_size = resampler.get_process_size(FFT_SIZE, decode_sample_rate, encode_sample_rate);

    // There are three types of iterators for three types of audio channels: mono, stereo, and more than 2 channels of audio.
    // Usually, the third iterator can handle all numbers of channels, but it's the slowest iterator.
    match encode_channels {
        1 => {
            let mut iter = decoder.mono_iter::<f32>().unwrap();
            loop {
                let block: Vec<f32> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block = utils::do_resample_mono(&mut resampler, &block, decode_sample_rate, encode_sample_rate);
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
                let block = utils::do_resample_stereo(&mut resampler, &block, decode_sample_rate, encode_sample_rate);
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
                let block = utils::do_resample_frames(&mut resampler, &block, decode_sample_rate, encode_sample_rate);
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

    let orig_spec = wavereader.spec();

    // The spec for the encoder
    let mut spec = Spec {
        channels: orig_spec.channels,
        channel_mask: 0,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    match data_format {
        DataFormat::Mp3(ref mut options) => {
            match spec.channels {
                1 => options.channels = Mp3Channels::Mono,
                2 => options.channels = Mp3Channels::JointStereo,
                o => panic!("MP3 format can't encode {o} channels audio."),
            }
        },
        DataFormat::Opus(ref options) => {
            spec.sample_rate = options.get_rounded_up_sample_rate(spec.sample_rate);
        },
        _ => (),
    }

    // This is the encoder
    let mut wavewriter = WaveWriter::create(arg3, &spec, data_format, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader, &mut wavewriter);

    // Get the metadata from the decoder
    wavewriter.migrate_metadata_from_reader(&wavereader, true);

    // Show debug info
    dbg!(&wavereader);
    dbg!(&wavewriter);

    drop(wavereader);
    drop(wavewriter);

    println!("======== TEST 2 ========");

    let spec2 = Spec {
        channels: spec.channels,
        channel_mask: 0,
        sample_rate: orig_spec.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut wavereader_2 = WaveReader::open(arg3).unwrap();
    let mut wavewriter_2 = WaveWriter::create(arg4, &spec2, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    // Transfer audio samples from the decoder to the encoder
    transfer_audio_from_decoder_to_encoder(&mut wavereader_2, &mut wavewriter_2);

    // Get the metadata from the decoder
    wavewriter_2.migrate_metadata_from_reader(&wavereader_2, true);


    // Show debug info
    dbg!(&wavereader_2);
    dbg!(&wavewriter_2);

    drop(wavereader_2);
    drop(wavewriter_2);

    Ok(())
}

#[test]
fn testrun() {
    for format in FORMATS {
        test(format.0, "test.wav", "output.wav", "output2.wav").unwrap();
    }
}

#[allow(dead_code)]
fn test_normal() -> ExitCode {
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

use std::{fs::File, io::{self, SeekFrom, BufReader, BufWriter}, cmp::Ordering};
use crate::wavcore::{ListChunk, ListInfo};
use crate::readwrite::{SharedReader, SharedWriter};
use crate::flac::{FlacEncoder, FlacEncoderParams, FlacCompression, FlacDecoder};
#[allow(unused_imports)]
use crate::flac::FlacError;

#[allow(dead_code)]
fn test_flac() -> ExitCode {
    const ALLOW_SEEK: bool = true;

    let args: Vec<String> = args().collect();
    if args.len() < 4 {return ExitCode::from(1);}

    println!("======== TEST 1 ========");
    let mut wavereader = WaveReader::open(&args[1]).unwrap();
    let spec1 = wavereader.spec();

    let mut params = FlacEncoderParams::new();
    params.verify_decoded = false;
    params.compression = FlacCompression::Level8;
    params.channels = spec1.channels;
    params.sample_rate = spec1.sample_rate;
    params.total_samples_estimate = wavereader.get_fact_data();

    dbg!(&params);

    let writer = SharedWriter::new(BufWriter::new(File::create(&args[2]).unwrap()));

    let on_write = |encoded: &[u8]| -> Result<(), io::Error> {
        writer.escorted_write(|writer|{
            writer.write_all(encoded)
        })
    };
    let on_seek = |position: u64| -> Result<(), io::Error> {
        if ALLOW_SEEK {
            writer.escorted_write(|writer|{
                writer.seek(SeekFrom::Start(position))?;
                Ok(())
            })
        } else {
            Err(io::Error::new(io::ErrorKind::NotSeekable, format!("Not seekable.")))
        }
    };
    let on_tell = || -> Result<u64, io::Error> {
        if ALLOW_SEEK {
            writer.escorted_work(|writer|{
                writer.stream_position()
            })
        } else {
            Err(io::Error::new(io::ErrorKind::NotSeekable, format!("Not seekable.")))
        }
    };

    let mut encoder = FlacEncoder::new(
        on_write,
        on_seek,
        on_tell,
        &params,
    ).unwrap();

    #[cfg(feature = "id3")]
    if let Some(id3_tag) = wavereader.get_id3__chunk() {
        encoder.migrate_metadata_from_id3(&id3_tag).unwrap();
    }
    if let Some(list) = wavereader.get_list_chunk() {
        match list {
            ListChunk::Info(_) => {
                if let Some(data) = list.get_artist() {
                    encoder.insert_comments("ARTIST", data).unwrap();
                }
                if let Some(data) = list.get_comment() {
                    encoder.insert_comments("COMMENT", data).unwrap();
                }
                if let Some(data) = list.get_copyright() {
                    encoder.insert_comments("COPYRIGHT", data).unwrap();
                }
                if let Some(data) = list.get_create_date() {
                    encoder.insert_comments("DATE", data).unwrap();
                }
                if let Some(data) = list.get_genre() {
                    encoder.insert_comments("GENRE", data).unwrap();
                }
                if let Some(data) = list.get_source() {
                    encoder.insert_comments("ORGANIZATION", data).unwrap();
                }
                if let Some(data) = list.get_name() {
                    encoder.insert_comments("TITLE", data).unwrap();
                }
            },
            ListChunk::Adtl(adtls) => {
                use wavcore::AdtlChunk;
                // Don't know how to read these data, just print them if a test WAV file contains them.
                for adtl in adtls.iter() {
                    match adtl{
                        AdtlChunk::Labl(labl) => println!("labl: {:?}", labl),
                        AdtlChunk::Note(note) => println!("note: {:?}", note),
                        AdtlChunk::Ltxt(ltxt) => println!("ltxt: {:?}", ltxt),
                    }
                }
            },
        }
    }

    let process_size = 65536;
    match spec1.channels {
        1 => {
            let mut iter = wavereader.mono_iter::<i16>().unwrap();
            loop {
                let block: Vec<i16> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block: Vec<i32> = block.into_iter().map(|sample: i16| -> i32 {sample as i32}).collect();
                encoder.write_monos(&block).unwrap();
            }
        },
        2 => {
            let mut iter = wavereader.stereo_iter::<i16>().unwrap();
            loop {
                let block: Vec<(i16, i16)> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block: Vec<(i32, i32)> = block.into_iter().map(|(l, r): (i16, i16)| -> (i32, i32) {(l as i32, r as i32)}).collect();
                encoder.write_stereos(&block).unwrap();
            }
        },
        _ => {
            let mut iter = wavereader.frame_iter::<i16>().unwrap();
            loop {
                let block: Vec<Vec<i16>> = iter.by_ref().take(process_size).collect();
                if block.is_empty() {
                    break;
                }
                let block: Vec<Vec<i32>> = block.into_iter().map(|frame: Vec<i16>| -> Vec<i32> {frame.into_iter().map(|sample: i16| -> i32 {sample as i32}).collect()}).collect();
                encoder.write_frames(&block).unwrap();
            }
        },
    }

    encoder.finalize().unwrap();
    println!("======== TEST 2 ========");

    const FFT_SIZE: usize = 65536;
    let mut resampler = Resampler::new(FFT_SIZE);

    let spec2 = Spec {
        channels: spec1.channels,
        channel_mask: 0,
        sample_rate: spec1.sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let reader = SharedReader::new(BufReader::new(File::open(&args[2]).unwrap()));
    let mut wavewriter = WaveWriter::create(&args[3], &spec2, DataFormat::Pcm, NeverLargerThan4GB).unwrap();

    let length = reader.escorted_work(|reader| -> Result<u64, io::Error> {
        let cur = reader.stream_position()?;
        reader.seek(SeekFrom::End(0))?;
        let ret = reader.stream_position()?;
        reader.seek(SeekFrom::Start(cur))?;
        Ok(ret)
    }).unwrap();

    let mut frames_buffer = Vec::<Vec<i32>>::new();
    let mut cur_sample_rate = 0;

    let on_read = |buffer: &mut [u8]| -> (usize, flac::ReadStatus) {
        reader.escorted_work(|reader| -> Result<(usize, flac::ReadStatus), io::Error> {
            let to_read = buffer.len();
            Ok(match reader.read(buffer) {
                Ok(size) => {
                    match size.cmp(&to_read) {
                        Ordering::Equal => (size, flac::ReadStatus::GoOn),
                        Ordering::Less => (size, flac::ReadStatus::Eof),
                        Ordering::Greater => panic!("`reader.read()` returns a size greater than the desired size."),
                    }
                },
                Err(e) => {
                    eprintln!("on_read(): {:?}", e);
                    (0, flac::ReadStatus::Abort)
                }
            })
        }).unwrap()
    };
    let on_seek = |position: u64|-> Result<(), io::Error> {
        if ALLOW_SEEK {
            reader.escorted_read(|reader|{
                reader.seek(SeekFrom::Start(position))?;
                Ok(())
            })
        } else {
            Err(io::Error::new(io::ErrorKind::NotSeekable, format!("Not seekable.")))
        }
    };
    let on_tell = || -> Result<u64, io::Error> {
        if ALLOW_SEEK {
            reader.escorted_work(|reader|{
                reader.stream_position()
            })
        } else {
            Err(io::Error::new(io::ErrorKind::NotSeekable, format!("Not seekable.")))
        }
    };
    let on_length = || -> Result<u64, io::Error> {
        if ALLOW_SEEK {
            Ok(length)
        } else {
            Err(io::Error::new(io::ErrorKind::NotSeekable, format!("Not seekable.")))
        }
    };
    let on_eof = || -> bool {
        reader.escorted_work(|reader| -> Result<bool, io::Error>{
            Ok(reader.stream_position()? >= length)
        }).unwrap()
    };
    let on_write = |frames: &[Vec<i32>], sample_info: &flac::SamplesInfo| -> Result<(), io::Error> {
        let sample_rate = sample_info.sample_rate;
        if cur_sample_rate == 0 {cur_sample_rate = sample_rate;}
        let process_size = resampler.get_process_size(FFT_SIZE, cur_sample_rate, spec2.sample_rate);
        frames_buffer.extend(frames.to_vec());
        let mut iter = frames_buffer.iter();
        loop {
            // Sample rate transition handling:
            // ------------------------------------------
            // - If sample rate remains unchanged (cur_sample_rate == sample_rate):
            //   - Maintain input sample continuity without truncation (direct stream passthrough).
            //
            // - If sample rate changes (cur_sample_rate ≠ sample_rate):
            //   - Process all buffered samples to completion under the current rate.
            //   - Begin processing new samples at the new sample rate.
            let block: Vec<Vec<i32>> = iter.by_ref().take(process_size).cloned().collect();
            if cur_sample_rate == sample_rate {
                if block.len() < process_size {
                    frames_buffer = block;
                    break;
                }
            } else {
                if block.is_empty() {
                    break;
                }
            }
            let block = utils::do_resample_frames(&mut resampler, &block, cur_sample_rate, spec2.sample_rate);
            wavewriter.write_frames(&block)?;
        }
        cur_sample_rate = sample_rate;
        Ok(())
    };
    let on_error = |error: flac::DecoderError| {
        eprintln!("on_error({error})");
    };

    let mut decoder = FlacDecoder::new(
        on_read,
        on_seek,
        on_tell,
        on_length,
        on_eof,
        on_write,
        on_error,
        true,
        true,
        flac::AudioForm::FrameArray,
    ).unwrap();

    const BY_BLOCKS: bool = false;
    if BY_BLOCKS {
        while !on_eof() {
            match decoder.decode() {
                Ok(go_on) => {
                    if !go_on {
                        break;
                    }
                },
                Err(e) => {
                    if e.get_code() == 4 {
                        break;
                    } else {
                        panic!("{:?}", e);
                    }
                }
            }
        }
    } else {
        decoder.decode_all().unwrap();
    }

    decoder.finalize().unwrap();

    if frames_buffer.len() > 0 {
        let block = utils::do_resample_frames(&mut resampler, &frames_buffer, cur_sample_rate, spec2.sample_rate);
        wavewriter.write_frames(&block).unwrap();
        frames_buffer.clear();
    }

    wavewriter.finalize().unwrap();

    println!("======== TEST FINISHED ========");

    ExitCode::from(0)
}

fn main() -> ExitCode {
    test_flac()
}
