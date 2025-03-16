
use std::{env, fs::File, io::{BufReader, BufWriter}, error::Error, process::ExitCode};

use hound::{SampleFormat, WavSpec, WavReader, WavWriter};

use rayon::iter::{ParallelIterator, ParallelBridge};

mod waveform;
use waveform::WaveForm;

mod freqproc;
use freqproc::FreqProcessor;

#[derive(Debug, Clone)]
pub enum WriterError {
    WaveFormLengthError,
}

impl std::error::Error for WriterError {}

impl std::fmt::Display for WriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           WriterError::WaveFormLengthError => write!(f, "The length of the chunk to write must be longer than the window size."),
       }
    }
}

// 整数补位
fn integer_upcast(value: i32, bits: u16) -> i32 {
    let value = value as u32;
    (match bits {
         8 => {let value = 0xFF & value; (value << 24) | (value << 16) | (value << 8) | value},
         16 => {let value = 0xFFFF & value; (value << 16) | value},
         24 => {let value = 0xFFFFFF & value; (value << 8) | (value >> 16)},
         32 => value,
         _ => panic!("`bits` should be 8, 16, 24, 32, got {}", bits),
     }) as i32
}

fn integers_upcast(values: &[i32], bits: u16) -> Vec<i32> {
    let mut ret = Vec::<i32>::with_capacity(values.len());
    for value in values.iter() {
        ret.push(integer_upcast(*value, bits));
    }
    ret
}

fn convert_i2f(value: i32) -> f32 {
    value as f32 / i32::MAX as f32
}

fn convert_samples_i2f(src_samples: &[i32]) -> Vec<f32> {
    let mut ret = Vec::<f32>::with_capacity(src_samples.len());
    for sample in src_samples.iter() {
        ret.push(convert_i2f(*sample));
    }
    ret
}

fn unzip_samples(src_samples: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let size = src_samples.len() / 2;
    assert_eq!(size * 2, src_samples.len());
    let mut ret1 = Vec::<f32>::with_capacity(size);
    let mut ret2 = Vec::<f32>::with_capacity(size);
    for i in 0..size {
        ret1.push(src_samples[i * 2]);
        ret2.push(src_samples[i * 2 + 1]);
    }
    (ret1, ret2)
}

fn zip_samples(src_samples: &(Vec<f32>, Vec<f32>)) -> Vec<f32> {
    let (chnl1, chnl2) = src_samples;
    assert_eq!(chnl1.len(), chnl2.len());
    let size = chnl1.len();
    let mut ret = Vec::<f32>::with_capacity(size);
    for i in 0..size {
        ret.push(chnl1[i]);
        ret.push(chnl2[i]);
    }
    ret
}

trait WaveReader {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> where Self: Sized;
    fn spec(&self) -> &WavSpec;
    fn set_chunk_size(&mut self, chunk_size: usize);
}

trait WaveWriter {
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, Box<dyn Error>> where Self: Sized;
    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>>;
    fn finalize(&mut self) -> Result<(), Box<dyn Error>>;

    fn set_window_size(&mut self, _window_size: usize) {
        panic!("`set_window_size()` is not implemented here.");
    }
}

struct WaveReaderSimple {
    reader: WavReader<BufReader<File>>,
    spec: WavSpec,
    chunk_size: usize,
}

impl WaveReader for WaveReaderSimple {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> {
        let reader = WavReader::open(input_file)?;
        let spec = reader.spec();
        Ok(Self {
            reader,
            spec,
            chunk_size: 0,
        })
    }

    fn spec(&self) -> &WavSpec {
        &self.spec
    }

    fn set_chunk_size(&mut self, chunk_size: usize) {
        self.chunk_size = chunk_size;
    }
}

impl Iterator for WaveReaderSimple {
    type Item = WaveForm;

    // 被当作迭代器使用时，返回一块块的音频数据
    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        // 分浮点数格式和整数格式分别处理（整数）
        match self.spec.sample_format {
            SampleFormat::Int =>
                // 整数要转换为浮点数，并且不同长度的整数要标准化到相同的长度
                match self.spec.channels {
                    1 => {
                        let mono: Vec<i32> = self.reader.samples::<i32>().take(self.chunk_size).flatten().collect();
                        if mono.is_empty() { return None; }
                        let mono = integers_upcast(&mono, self.spec.bits_per_sample);
                        let mono = convert_samples_i2f(&mono);
                        Some(Self::Item::Mono(mono))
                    },
                    2 => {
                        let stereo: Vec<i32> = self.reader.samples::<i32>().take(self.chunk_size * 2).flatten().collect();
                        if stereo.is_empty() { return None; }
                        let stereo = integers_upcast(&stereo, self.spec.bits_per_sample);
                        let stereo = convert_samples_i2f(&stereo);
                        Some(Self::Item::Stereo(unzip_samples(&stereo)))
                    },
                    other => panic!("Unsupported channel number: {}", other)
                },
            SampleFormat::Float =>
                // 浮点数不用转换
                match self.spec.channels {
                    1 => {
                        let mono: Vec<f32> = self.reader.samples::<f32>().take(self.chunk_size).flatten().collect();
                        if mono.is_empty() { return None; }
                        Some(Self::Item::Mono(mono))
                    },
                    2 => {
                        let stereo: Vec<f32> = self.reader.samples::<f32>().take(self.chunk_size * 2).flatten().collect();
                        if stereo.is_empty() { return None; }
                        Some(Self::Item::Stereo(unzip_samples(&stereo)))
                    },
                    other => panic!("Unsupported channel number: {}", other)
                },
        }
    }
}

struct WaveWriterSimple {
    writer: WavWriter<BufWriter<File>>,
}

impl WaveWriter for WaveWriterSimple {
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, Box<dyn Error>> {
        let writer = WavWriter::create(output_file, WavSpec {
            channels: spec.channels,
            sample_rate: spec.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        })?;
        Ok(Self {
            writer,
        })
    }

    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>> {
        match channels_data {
            WaveForm::Mono(mono) => {
                for sample in mono.into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            WaveForm::Stereo(stereo) => {
                for sample in zip_samples(&stereo).into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            _ => panic!("Must not give `WaveForm::None` for a `WaveWriterSimple` to write."),
        }
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        Ok(self.writer.flush()?)
    }
}

struct WaveReaderWindowed {
    reader: WaveReaderSimple,
    spec: WavSpec,
    last_chunk: WaveForm,
    chunk_size: usize,
}

impl WaveReader for WaveReaderWindowed {
    fn open(input_file: &str) -> Result<Self, Box<dyn Error>> {
        let reader = WaveReaderSimple::open(input_file)?;
        let spec = reader.spec;
        Ok(Self {
            reader,
            spec,
            last_chunk: WaveForm::None,
            chunk_size: 0,
        })
    }

    fn spec(&self) -> &WavSpec {
        &self.spec
    }

    fn set_chunk_size(&mut self, chunk_size: usize) {
        self.reader.set_chunk_size(chunk_size / 2);
        self.chunk_size = self.reader.chunk_size * 2;
    }
}

impl Iterator for WaveReaderWindowed {
    type Item = WaveForm;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        let new_chunk = self.reader.next();
        match new_chunk {
            None => { // 新块没有数据了
                match self.last_chunk {
                    // 旧的块都无了，则返回 None 结束迭代
                    WaveForm::None => None,

                    // 旧的块还有，延长旧的块为全块大小，将其返回，然后让旧的块变为无。
                    _ => {
                        let ret = self.last_chunk.resized(self.chunk_size);
                        self.last_chunk = WaveForm::None;
                        Some(ret)
                    },
                }
            },
            Some(chunk) => { // 新块有数据
                // 不论如何，延长到半块大小
                let chunk = chunk.resized(self.reader.chunk_size);
                match self.last_chunk {
                    // 没有旧块，但是有新块，说明是第一次迭代。此时应当再次迭代，才能组成一个完整的块。
                    WaveForm::None => {
                        self.last_chunk = chunk;
                        self.next()
                    },
                    // 有新块，有旧块
                    _ => {
                        let ret = self.last_chunk.extended(&chunk).unwrap();
                        self.last_chunk = chunk;
                        Some(ret)
                    },
                }
            },
        }
    }
}

struct WaveWindowedWriter {
    writer: WaveWriterSimple,
    window_size: usize,
    window: WaveForm,
}

impl WaveWriter for WaveWindowedWriter {
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, Box<dyn Error>> {
        let writer = WaveWriterSimple::create(output_file, spec)?;
        Ok(Self {
            writer,
            window_size: 0,
            window: WaveForm::None,
        })
    }

    fn set_window_size(&mut self, window_size: usize) {
        self.window_size = window_size;
    }

    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>> {
        // 策略：
        // 输入的音频被设计为窗口长度的两倍。
        // 平时存储上一个输入的音频的后半部分，音频来了以后，合并前半部分，写入文件。
        // 然后再存储新音频的后半部分。
        // 最后要调用一次 `finalize()`，把存储的最后一段音频的后半部分写入。
        if channels_data.len().unwrap() < self.window_size {
            return Err(WriterError::WaveFormLengthError.into());
        }
        // 按窗口大小分割音频
        let (first, second) = channels_data.split(self.window_size);
        if let WaveForm::None = self.window {
            // 第一次写入，直接写入前半段，存住后半段
            self.writer.write(first)?;
        } else {
            // 写入叠加窗口
            self.writer.write(first.add_to(&self.window)?)?;
        }
        self.window = second;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.writer.write(self.window.clone())?;
        self.writer.finalize()
    }
}

trait IteratorWithWaveReader: Iterator<Item = WaveForm> + WaveReader + Send {}
impl<T> IteratorWithWaveReader for T where T: Iterator<Item = WaveForm> + WaveReader + Send {}

fn wave_reader_create(input_file: &str, do_hann_window: bool) -> Box<dyn IteratorWithWaveReader> {
    if do_hann_window {
        Box::new(WaveReaderWindowed::open(input_file).unwrap())
    } else {
        Box::new(WaveReaderSimple::open(input_file).unwrap())
    }
}

fn wave_writer_create(output_file: &str, spec: &WavSpec, do_hann_window: bool) -> Box<dyn WaveWriter> {
    if do_hann_window {
        Box::new(WaveWindowedWriter::create(output_file, spec).unwrap())
    } else {
        Box::new(WaveWriterSimple::create(output_file, spec).unwrap())
    }
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
    let mut reader = wave_reader_create(input_file, do_hann_window);

    // 获取音频文件规格
    let sample_rate = reader.spec().sample_rate;

    // 打开写入文件
    let mut writer = wave_writer_create(output_file, reader.spec(), do_hann_window);

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
