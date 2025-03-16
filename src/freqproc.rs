
use std::{sync::Arc};

use rustfft::{FftPlanner, Fft, num_complex::Complex};

pub struct FreqProcessor {
    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,
    sample_rate: u32,
    section_sample_count: usize,
    normalize_scaler: f64,
}

impl FreqProcessor {
    pub fn new(section_sample_count: usize, sample_rate: u32) -> Self {
        let mut planner = FftPlanner::new();
        Self {
            fft_forward: planner.plan_fft_forward(section_sample_count),
            fft_inverse: planner.plan_fft_inverse(section_sample_count),
            sample_rate,
            section_sample_count,
            normalize_scaler: 1.0 / section_sample_count as f64,
        }
    }

    pub fn length(x: f64, y: f64) -> f64 {
        (x * x + y * y).sqrt()
    }

    pub fn lerp(a: f64, b: f64, s: f64) -> f64 {
        a + (b - a) * s
    }

    // 检测音调算法1
    pub fn tone_detect(&self, fftbuf: &[Complex::<f64>]) -> f64 {
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
            let weight = Self::length(front.re, front.im) + Self::length(back.re, back.im);
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
    pub fn tone_modify(fftbuf: Vec::<Complex::<f64>>, modi: f64) -> Vec::<Complex::<f64>> {
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
    pub fn apply_hann_window(samples: &mut [f32]) {
        let num_samples = samples.len();
        let pi = std::f32::consts::PI;
        for (i, sample) in samples.iter_mut().enumerate().take(num_samples) {
            let progress = i as f32 / (num_samples - 1) as f32;
            *sample *= 0.5 - 0.5 * (2.0 * pi * progress).cos();
        }
    }

    // 音频处理
    pub fn proc(&self, chunk_index: usize, samples: &[f32], target_freq: f64, do_hann_window: bool) -> Vec<f32> {

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
        let avr_freq = Self::lerp(avr_freq, target_freq, 0.75);

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