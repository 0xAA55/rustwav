
use std::{env, io::{BufReader, BufWriter, Seek, Read, Write}, fs::File, sync::Arc, error::Error, process::ExitCode};

use tempfile::tempfile;

use hound::{SampleFormat, WavSpec, WavReader, WavWriter};

use rustfft::{FftPlanner, Fft, num_complex::Complex};

use rayon::iter::{ParallelIterator, ParallelBridge};

fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

fn lerp(a: f64, b: f64, s: f64) -> f64 {
    a + (b - a) * s
}

struct FreqProcessor {
    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,
    sample_rate: u32,
    section_sample_count: usize,
    normalize_scaler: f64,
}

impl FreqProcessor {
    fn new(section_sample_count: usize, sample_rate: u32) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            fft_forward: planner.plan_fft_forward(section_sample_count),
            fft_inverse: planner.plan_fft_inverse(section_sample_count),
            sample_rate,
            section_sample_count,
            normalize_scaler: 1.0 / section_sample_count as f64,
        }
    }

    // 检测音调算法1
    fn tone_detect(&self, fftbuf: &[Complex::<f64>]) -> f64 {
        let max_freq = self.sample_rate as f64 / 2.0;
        let min_freq = self.sample_rate as f64 / self.section_sample_count as f64;
        let freq_range = max_freq - min_freq;
        let half = self.section_sample_count / 2;
        let last = self.section_sample_count - 1;

        // 权重
        let mut weights = Vec::<f64>::new();
        weights.resize(half, 0.0);

        let mut max_weight = 0.0;
        for i in 0..half {
            let front = &fftbuf[i];
            let back = &fftbuf[last - i];
            // 注释掉的代码被用于压低高频部分的权重值，这次要想办法测试出图，用图表来实际查看频谱图长啥样。
            //let progress = i as f64 / half as f64;
            let weight = length(front.re, front.im) + length(back.re, back.im);
            //let weight = weight * (1.0 - progress);
            if weight > max_weight {max_weight = weight}
            weights[i] = weight;
        }

        let mut weighted_freq_sum = 0.0;
        let mut weight_sum = 0.0;

        // 进行加权平均数计算
        for (i, weight) in weights.iter().enumerate().take(half) {
            // 标准化权重值
            let weight = *weight / max_weight;

            // 当前 i 值对应的频率值
            let freq_of_i = i as f64 * freq_range / half as f64 + min_freq;

            // 总和
            weighted_freq_sum += weight * freq_of_i;

            // 权重总和
            weight_sum += weight;
        }
        weighted_freq_sum / weight_sum
    }

    // 改变音调
    fn tone_modify(fftbuf: Vec::<Complex::<f64>>, modi: f64) -> Vec::<Complex::<f64>> {
        let size = fftbuf.len();
        let last = size - 1;
        let half = size / 2;
        let mut ret = Vec::<Complex::<f64>>::new();
        ret.resize(size, Complex{re: 0.0, im: 0.0});

        // 对 FFT 频域数据进行重采样，音调的变化按 modi 值进行倍乘。
        // modi 大于 1.0 的时候，new_freq 的增长速度大于 i，会超出 half，这个时候直接退出循环即可。
        // 否则 new_freq 的增长速度小于 i，达不到 half，此处不必处理。
        for i in 0..half {
            let sample_position = (i as f64 * modi) as usize;
            if sample_position < half {
                ret[i] = fftbuf[sample_position];
                ret[last - i] = fftbuf[last - sample_position];
            } else {
                break;
            }
        }

        ret
    }

    // 对一段样本施加汉宁窗使其便于叠加
    fn apply_hann_window(samples: &mut [f32]) {
        let num_samples = samples.len();
        let pi = std::f32::consts::PI;
        for (i, sample) in samples.iter_mut().enumerate().take(num_samples) {
            let progress = i as f32 / (num_samples - 1) as f32;
            *sample *= 0.5 - 0.5 * (2.0 * pi * progress).cos();
        }
    }

    // 音频处理
    fn proc(&self, samples: &[f32], target_freq: f64, do_hann_window: bool) -> Vec<f32> {

        // 将音频样本转换为复数用于 FFT 计算。
        let mut fftbuf = Vec::<Complex::<f64>>::with_capacity(self.section_sample_count);
        for sample in samples.iter() {
            fftbuf.push(Complex{re: (*sample) as f64, im: 0.0});
        }

        // 确保 FFT 缓冲区大小达到需求
        if fftbuf.len() < self.section_sample_count {
            fftbuf.resize(self.section_sample_count, Complex{re: 0.0, im: 0.0});
        }

        // 进行 FFT 转换
        self.fft_forward.process(&mut fftbuf);

        // 检测平均音调
        let avr_freq = self.tone_detect(&fftbuf);

        // 做插值，使检测到的平均音调比较接近目标音调值，以免调音处理用力过度
        let avr_freq = lerp(avr_freq, target_freq, 0.75);

        // 改变音调
        let freq_mod = avr_freq / target_freq;
        let mut fftbuf = Self::tone_modify(fftbuf, freq_mod);

        // 转换回来
        self.fft_inverse.process(&mut fftbuf);

        // 重新采样回来
        let mut result = Vec::<f32>::with_capacity(self.section_sample_count);
        for complex in fftbuf.into_iter() {

            // 标准化值，只取实数部分
            result.push((complex.re * self.normalize_scaler) as f32);
        }

        // 采样回来后，因为音调发生了变化，波形的长度其实是发生了变化的。
        // 如果波形缩短，则需要做额外的处理。
        if freq_mod > 1.0 {
            let real_size = (self.section_sample_count as f64 * avr_freq) as usize;
            result.resize(real_size, 0.0);
            loop {
                if result.len() < self.section_sample_count {
                    result.extend(&result.clone());
                } else {
                    break;
                }
            }
            result.resize(self.section_sample_count, 0.0);
        }

        // 施加汉宁窗
        if do_hann_window {
            Self::apply_hann_window(&mut result);
        }
        result
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

#[derive(Clone, bincode::Encode, bincode::Decode)]
enum WaveFormChannels {
    None,
    Mono(Vec<f32>),
    Stereo((Vec<f32>, Vec<f32>))
}

fn vecf32_add(v1: &[f32], v2: &[f32]) -> Vec<f32> {
    assert_eq!(v1.len(), v2.len());
    let mut v3 = v1.to_owned();
    for i in 0..v3.len() {
        v3[i] += v2[i];
    }
    v3
}

fn chunk_get_len(chunk: &WaveFormChannels) -> usize {
    match chunk {
        WaveFormChannels::None => 0,
        WaveFormChannels::Mono(mono) => mono.len(),
        WaveFormChannels::Stereo((chnl1, chnl2)) => {
            assert_eq!(chnl1.len(), chnl2.len());
            chnl1.len()
        },
    }
}

// 使 WaveFormChannels 内容的长度增长到指定值，如果没有内容就不增长。
fn chunk_extend(chunk: WaveFormChannels, target_size: usize) -> WaveFormChannels {
    match chunk {
        WaveFormChannels::None => WaveFormChannels::None,
        WaveFormChannels::Mono(mono) => {
            let mut mono = mono;
            mono.resize(target_size, 0.0);
            WaveFormChannels::Mono(mono)
        },
        WaveFormChannels::Stereo((chnl1, chnl2)) => {
            let (mut chnl1, mut chnl2) = (chnl1, chnl2);
            chnl1.resize(target_size, 0.0);
            chnl2.resize(target_size, 0.0);
            WaveFormChannels::Stereo((chnl1, chnl2))
        },
    }
}

// 拼接两个 WaveFormChannels，会检查其中的类型，不允许不同类型的进行拼接，但是 None 可以参与拼接。
fn chunk_concat(chunk1: &WaveFormChannels, chunk2: &WaveFormChannels) -> WaveFormChannels {
    match chunk1 {
        // 1 是 None，返回 2
        WaveFormChannels::None => chunk2.clone(),

        // 1 是 Mono，根据 2 来判断
        WaveFormChannels::Mono(mono) => match chunk2 {

            // 2 是 None，返回 1
            WaveFormChannels::None => chunk1.clone(),

            // 2 是 Mono，进行拼接
            WaveFormChannels::Mono(mono2) => {
                let mut cmono = mono.clone();
                cmono.extend(mono2);
                WaveFormChannels::Mono(cmono)
            },

            // 2 是 Stereo，类型不同，不能拼接。
            WaveFormChannels::Stereo(_) => panic!("Must concat same type `WaveFormChannels`."),
        },

        // 1 是 Stereo，根据 2 来判断
        WaveFormChannels::Stereo(stereo) => match chunk2 {

            // 2 是 None，返回 1
            WaveFormChannels::None => chunk1.clone(),

            // 2 是 Mono，类型不同，不能拼接。
            WaveFormChannels::Mono(_) => panic!("Must concat same type `WaveFormChannels`."),

            // 2 是 Stereo，进行拼接
            WaveFormChannels::Stereo((chnl1_2, chnl2_2)) => {
                let (mut chnl1_1, mut chnl2_1) = stereo.clone();
                chnl1_1.extend(chnl1_2);
                chnl2_1.extend(chnl2_2);
                WaveFormChannels::Stereo((chnl1_1, chnl2_1))
            },
        },
    }
}

fn chunk_split(chunk: &WaveFormChannels, at: usize) -> (WaveFormChannels, WaveFormChannels) {
    match chunk {
        WaveFormChannels::None => (WaveFormChannels::None, WaveFormChannels::None),
        WaveFormChannels::Mono(mono) => {
            (WaveFormChannels::Mono(mono[0..at].to_vec()),
             WaveFormChannels::Mono(mono[at..].to_vec()))
        },
        WaveFormChannels::Stereo((chnl1, chnl2)) => {
            (WaveFormChannels::Stereo((chnl1[0..at].to_vec(), chnl2[0..at].to_vec())),
             WaveFormChannels::Stereo((chnl1[at..].to_vec(), chnl2[at..].to_vec())))
        },
    }
}

// 叠加两个 chunk 的值
fn chunks_add(chunk1: &WaveFormChannels, chunk2: &WaveFormChannels) -> WaveFormChannels {
    match (chunk1, chunk2) {
        (WaveFormChannels::None, WaveFormChannels::None) => {
            WaveFormChannels::None
        },
            (WaveFormChannels::Mono(mono1), WaveFormChannels::Mono(mono2)) => {
             WaveFormChannels::Mono(vecf32_add(mono1, mono2))
        },
            (WaveFormChannels::Stereo((chnl1_1, chnl2_1)), WaveFormChannels::Stereo((chnl1_2, chnl2_2))) => {
             WaveFormChannels::Stereo((vecf32_add(chnl1_1, chnl1_2), vecf32_add(chnl2_1, chnl2_2))),
        },
        _ => panic!("Two chunks to add must have same channel type."),
    }
}

trait WaveReader {
    fn open(input_file: &str) -> Result<Self, hound::Error> where Self: Sized;
    fn spec(&self) -> &WavSpec;
    fn set_chunk_size(&mut self, chunk_size: usize);
}

trait WaveWriter {
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, hound::Error> where Self: Sized;
    fn write(&mut self, channels_data: WaveFormChannels) -> Result<(), hound::Error>;
    fn finalize(&mut self) -> Result<(), hound::Error>;

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
    fn open(input_file: &str) -> Result<Self, hound::Error> {
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
    type Item = WaveFormChannels;

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
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, hound::Error> {
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

    fn write(&mut self, channels_data: WaveFormChannels) -> Result<(), hound::Error> {
        match channels_data {
            WaveFormChannels::Mono(mono) => {
                for sample in mono.into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            WaveFormChannels::Stereo(stereo) => {
                for sample in zip_samples(&stereo).into_iter() {
                    self.writer.write_sample(sample)?
                }
            },
            _ => panic!("Must not give `WaveFormChannels::None` for a `WaveWriterSimple` to write."),
        }
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), hound::Error>
    {
        self.writer.flush()
    }
}

struct WaveReaderWindowed {
    reader: WaveReaderSimple,
    spec: WavSpec,
    last_chunk: WaveFormChannels,
    chunk_size: usize,
}

impl WaveReader for WaveReaderWindowed {
    fn open(input_file: &str) ->  Result<Self, hound::Error> {
        let reader = WaveReaderSimple::open(input_file)?;
        let spec = reader.spec;
        Ok(Self {
            reader,
            spec,
            last_chunk: WaveFormChannels::None,
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
    type Item = WaveFormChannels;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        let new_chunk = self.reader.next();
        match new_chunk {
            None => { // 新块没有数据了
                match self.last_chunk {
                    // 旧的块都无了，则返回 None 结束迭代
                    WaveFormChannels::None => None,

                    // 旧的块还有，延长旧的块为全块大小，将其返回，然后让旧的块变为无。
                    _ => {
                        let ret = chunk_extend(self.last_chunk.clone(), self.chunk_size);
                        self.last_chunk = WaveFormChannels::None;
                        Some(ret)
                    },
                }
            },
            Some(chunk) => { // 新块有数据
                // 不论如何，延长到半块大小
                let chunk = chunk_extend(chunk, self.reader.chunk_size);
                match self.last_chunk {
                    // 没有旧块，但是有新块，说明是第一次迭代。此时应当再次迭代，才能组成一个完整的块。
                    WaveFormChannels::None => {
                        self.last_chunk = chunk;
                        self.next()
                    },
                    // 有新块，有旧块
                    _ => {
                        let ret = chunk_concat(&self.last_chunk, &chunk);
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
    window: WaveFormChannels,
}

impl WaveWriter for WaveWindowedWriter {
    fn create(output_file: &str, spec: &WavSpec) -> Result<Self, hound::Error> {
        let writer = WaveWriterSimple::create(output_file, spec)?;
        Ok(Self {
            writer,
            window_size: 0,
            window: WaveFormChannels::None,
        })
    }

    fn set_window_size(&mut self, window_size: usize) {
        self.window_size = window_size;
    }

    fn write(&mut self, channels_data: WaveFormChannels) -> Result<(), hound::Error> {
        // 策略：
        // 输入的音频被设计为窗口长度的两倍。
        // 平时存储上一个输入的音频的后半部分，音频来了以后，合并前半部分，写入文件。
        // 然后再存储新音频的后半部分。
        // 最后要调用一次 `finalize()`，把存储的最后一段音频的后半部分写入。
        if chunk_get_len(&channels_data) < self.window_size {
            panic!("Channel data to write must be greater than the window size.")
        }
        // 按窗口大小分割音频
        let (first, second) = chunk_split(&channels_data, self.window_size);
        if let WaveFormChannels::None = self.window {
            // 第一次写入，直接写入前半段，存住后半段
            self.writer.write(first)?;
        } else {
            // 写入叠加窗口
            self.writer.write(chunks_add(&first, &self.window))?;
        }
        self.window = second;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), hound::Error>
    {
        self.writer.write(self.window.clone())?;
        self.writer.finalize()
    }
}

trait IteratorWithWaveReader: Iterator<Item = WaveFormChannels> + WaveReader + Send {}
impl<T> IteratorWithWaveReader for T where T: Iterator<Item = WaveFormChannels> + WaveReader + Send {}

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
fn process_chunk(freq_processor: &FreqProcessor, chunk: WaveFormChannels, do_hann_window: bool, target_freq: f64) -> WaveFormChannels {
    match chunk {
        WaveFormChannels::Mono(mono) => {
            let mono_process = freq_processor.proc(&mono, target_freq, do_hann_window);
            WaveFormChannels::Mono(mono_process)
        },
        WaveFormChannels::Stereo((chnl1, chnl2)) => {
            let chnl1_process = freq_processor.proc(&chnl1, target_freq, do_hann_window);
            let chnl2_process = freq_processor.proc(&chnl2, target_freq, do_hann_window);
            WaveFormChannels::Stereo((chnl1_process, chnl2_process))
        },
        WaveFormChannels::None => WaveFormChannels::None,
    }
}

// 将一个 chunk 序列化为字节数据用于暂存到硬盘
fn chunk_to_bytes(chunk: &WaveFormChannels) -> Vec<u8> {
    bincode::encode_to_vec(chunk, bincode::config::standard()).unwrap()
}

// 将一个 chunk 从字节数据恢复出来
fn chunk_from_bytes(data: &[u8]) -> WaveFormChannels {
    let (ret, _size) = bincode::decode_from_slice(data, bincode::config::standard()).unwrap();
    ret
}

// 暂存 chunk 到临时文件，这个文件句柄需要保留。关闭文件导致临时文件被自动删除。
fn make_cached(chunk: WaveFormChannels) -> File {
    let mut ret = tempfile().expect("Could not open the temp file.");
    ret.write_all(&chunk_to_bytes(&chunk)).expect("Could not write to the temp file.");
    ret.flush().expect("Could not flush the temp file.");
    ret
}

// 从临时文件取回 chunk 数据。
fn read_back(file: &mut File) -> WaveFormChannels {
    let mut buf = Vec::<u8>::new();
    file.rewind().expect("Could not rewind the temp file.");
    file.read_to_end(&mut buf).expect("Could not read from the temp file.");
    chunk_from_bytes(&buf)
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
            Some((i, make_cached(process_chunk(&freq_processor, chunk, do_hann_window, target_freq))))
        }).collect::<Vec<Option<(usize, File)>>>().into_iter().flatten().collect();

        // 排序
        indexed_all_samples.sort_by_key(|k| k.0);

        // 按顺序存储所有数据
        for (_, mut file) in indexed_all_samples {
            writer.write(read_back(&mut file))?;
        }
    }
    else {
        // 不并行处理，则一边处理一边存储
        for chunk in reader {
            writer.write(process_chunk(&freq_processor, chunk, do_hann_window, target_freq))?;
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
