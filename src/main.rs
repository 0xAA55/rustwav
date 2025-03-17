mod audioreader;
mod audiowriter;
mod wavreader;
mod wavwriter;
mod sampleutils;
mod waveform;
mod freqproc;
mod windowedreader;
mod windowedwriter;

use std::{env, fs::File, error::Error, process::ExitCode};

use rayon::iter::{ParallelIterator, ParallelBridge};

use freqproc::FreqProcessor;
use waveform::WaveForm;
use wavreader::WaveReaderSimple;
use wavwriter::WaveWriterSimple;
use audioreader::{AudioReader, Spec};
use audiowriter::AudioWriter;
use windowedreader::WindowedAudioReader;
use windowedwriter::WindowedAudioComposer;

pub trait IteratorWithAudioReader: Iterator<Item = WaveForm> + AudioReader + Send {}
impl<T> IteratorWithAudioReader for T where T: Iterator<Item = WaveForm> + AudioReader + Send {}

fn wave_reader_create(input_file: &str, do_hann_window: bool) -> Result<Box<dyn AudioReader>, Box<dyn Error>> {
    let mut reader: Box<dyn AudioReader> = Box::new(WaveReaderSimple::open(input_file)?);
    if do_hann_window {
        reader = Box::new(WindowedAudioReader::upgrade(reader)?);
    }
    Ok(reader)
}

fn wave_writer_create(output_file: &str, spec: &Spec, do_hann_window: bool) -> Result<Box<dyn AudioWriter>, Box<dyn Error>> {
    let mut writer: Box<dyn AudioWriter> = Box::new(WaveWriterSimple::create(output_file, spec)?);
    if do_hann_window {
        writer = Box::new(WindowedAudioComposer::upgrade(writer)?);
    }
    Ok(writer)
}

// 处理单个块
fn process_chunk(freq_processor: &FreqProcessor, chunk_index: usize, chunk: WaveForm, do_hann_window: bool, target_freq: f64) -> WaveForm {
    match chunk {
        WaveForm::Mono(mono) => {
            let mono_process = freq_processor.proc(chunk_index, &mono, target_freq, do_hann_window);
            WaveForm::Mono(mono_process)
        },
        WaveForm::Stereo((chnl1, chnl2)) => {
            let chnl1_process = freq_processor.proc(chunk_index, &chnl1, target_freq, do_hann_window);
            let chnl2_process = freq_processor.proc(chunk_index, &chnl2, target_freq, do_hann_window);
            WaveForm::Stereo((chnl1_process, chnl2_process))
        },
        WaveForm::None => WaveForm::None,
    }
}

fn process_wav_file(
    output_file: &str, // 输出文件
    input_file: &str,  // 输入文件
    target_freq: f64,  // 调音的目标频率
    section_duration: f64, // 调音的分节长度
    do_hann_window: bool, // 是否进行汉宁窗处理
    concurrent: bool, // 是否并行处理
) -> Result<(), Box<dyn Error>> {

    // 打开源文件，以一边分节读取处理一边保存的策略进行音频的处理。
    let mut reader = wave_reader_create(input_file, do_hann_window)?;

    // 获取音频文件规格
    let sample_rate = reader.spec().sample_rate;

    // 打开写入文件
    let mut writer = wave_writer_create(output_file, &reader.spec(), do_hann_window)?;

    // 根据输入文件的采样率，计算出调音的分节包含的样本数量
    let section_sample_count = (sample_rate as f64 * section_duration * 0.5) as usize * 2;

    // 设置块大小
    reader.set_chunk_size(section_sample_count);

    if do_hann_window {
        // 设置窗口大小
        writer.set_window_size(section_sample_count / 2);
    }

    // 初始化 FFT 模块
    let freq_processor = FreqProcessor::new(section_sample_count, sample_rate);

    // 进行处理
    if concurrent {
        // 先并行处理，返回索引和数据
        let mut indexed_all_samples: Vec::<(usize, File)> = reader.enumerate().par_bridge().map(|(i, chunk)| -> Option<(usize, File)> {
            Some((i, process_chunk(&freq_processor, i, chunk, do_hann_window, target_freq).to_tempfile().unwrap()))
        }).collect::<Vec<Option<(usize, File)>>>().into_iter().flatten().collect();

        // 排序
        indexed_all_samples.sort_by_key(|k| k.0);

        // 按顺序存储所有数据
        for (_, file) in indexed_all_samples {
            writer.write(WaveForm::restore_from_tempfile(file).unwrap()).unwrap();
        }
    }
    else {
        // 不并行处理，则一边处理一边存储
        for (i, chunk) in reader.enumerate() {
            writer.write(process_chunk(&freq_processor, i, chunk, do_hann_window, target_freq))?;
        }
    }

    writer.finalize().unwrap();
    Ok(())
}

fn usage() {
    println!("Usage: evoice <input.wav> <output.wav> <target freq> [window size]");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {usage(); return ExitCode::from(1);}
    let input_file = args[1].clone();
    let output_file = args[2].clone();
    let target_freq = args[3].parse::<f64>().unwrap();
    let mut window_size = 0.1;
    if args.len() > 5 {window_size = args[4].parse::<f64>().unwrap();}
    match process_wav_file(&output_file, &input_file, target_freq, window_size, true, true) {
        Ok(_) => ExitCode::from(0),
        _ => ExitCode::from(2),
    }
}
