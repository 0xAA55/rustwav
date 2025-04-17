
use std::{cmp::min, sync::Arc, fmt::{self, Debug, Formatter}};
use rustfft::{FftPlanner, Fft, num_complex::Complex};

#[derive(Debug, Clone)]
pub enum ResamplerError {
    SizeError(String),
}

#[derive(Clone)]
pub struct Resampler {
    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,
    fft_size: usize,
    normalize_scaler: f64,
}

fn get_average(complexes: &[Complex<f64>]) -> Complex<f64> {
    let sum: Complex<f64> = complexes.iter().copied().sum();
    let scaler = 1.0 / complexes.len() as f64;
    Complex::<f64> {
        re: sum.re * scaler,
        im: sum.im * scaler,
    }
}

fn interpolate(c1: Complex<f64>, c2: Complex<f64>, s: f64) -> Complex<f64> {
    c1 + (c2 - c1) * s
}

// Resampler 工作方式
// 对于音频的拉长：
//   1. 输入的音频是它本来的长度，末尾补零使其长度达到目标长度
//   2. 进行 FFT 变换，得到频域
//   3. 在频域里，把频率值往低处按比例压缩
//   4. 进行 FFT 逆变换，得到拉长的音频
// 对于音频的压缩：
//   1. 输入音频
//   2. 进行 FFT 变换
//   3. 在频域里，把频率值往高处按比例拉长
//   4. 进行 FFT 逆变换，得到的音频的音调增高了，但是长度没变
//   5. 剪切音频，使其缩短长度
// 由此可见：FFT 变换的长度要选最长的
impl Resampler {
    pub fn new(fft_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        if fft_size & 1 != 0 {
            panic!("The input size and the output size must be times of 2, got {fft_size}");
        }
        Self {
            fft_forward: planner.plan_fft_forward(fft_size),
            fft_inverse: planner.plan_fft_inverse(fft_size),
            fft_size,
            normalize_scaler: 1.0 / fft_size as f64,
        }
    }

    // desired_length：想要得到的音频长度，不能超过 FFT size
    // 当 samples.len() 小于 desired_length 的时候，说明你是要拉长音频到 desired_length
    // 当 samples.len() 大于 desired_length 的时候，说明你要压缩音频到 desired_length
    pub fn resample_core(&self, samples: &[f32], desired_length: usize) -> Result<Vec<f32>, ResamplerError> {
        const INTERPOLATE_UPSCALE: bool = true;
        const INTERPOLATE_DNSCALE: bool = true;

        let input_size = samples.len();
        if input_size == desired_length {
            return Ok(samples.to_vec());
        }

        if desired_length > self.fft_size {
            return Err(ResamplerError::SizeError(format!("The desired size {desired_length} must not exceed the FFT size {}", self.fft_size)));
        }

        let mut fftbuf: Vec<Complex<f64>> = samples.iter().map(|sample: &f32| -> Complex<f64> {Complex{re: *sample as f64, im: 0.0}}).collect();

        if fftbuf.len() <= self.fft_size {
            fftbuf.resize(self.fft_size, Complex{re: 0.0, im: 0.0});
        } else {
            return Err(ResamplerError::SizeError(format!("The input size {} must not exceed the FFT size {}", fftbuf.len(), self.fft_size)));
        }

        // 进行 FFT 正向变换
        self.fft_forward.process(&mut fftbuf);

        // 准备进行插值
        let mut fftdst = vec![Complex::<f64>{re: 0.0, im: 0.0}; self.fft_size];

        let half = self.fft_size / 2;
        let back = self.fft_size - 1;
        let scaling = desired_length as f64 / input_size as f64;
        if input_size > desired_length {
            // 输入大小大于输出大小，意味着音频的缩短
            // 意味着频域的拉伸
            for i in 0..half {
                let scaled = i as f64 * scaling;
                let i1 = scaled.trunc() as usize;
                let i2 = i1 + 1;
                let s = scaled.fract();
                if INTERPOLATE_DNSCALE {
                    fftdst[i] = interpolate(fftbuf[i1], fftbuf[i2], s);
                    fftdst[back - i] = interpolate(fftbuf[back - i1], fftbuf[back - i2], s);
                } else {
                    fftdst[i] = fftbuf[i1];
                    fftdst[back - i] = fftbuf[back - i1];
                }
            }
        } else {
            // 输入大小小于输出大小，意味着音频的伸长
            // 意味着频域的压缩
            for i in 0..half {
                let i1 = (i as f64 * scaling).trunc() as usize;
                let i2 = ((i + 1) as f64 * scaling).trunc() as usize;
                if i2 >= half {break;}
                let j1 = back - i2;
                let j2 = back - i1;
                if INTERPOLATE_UPSCALE {
                    fftdst[i] = get_average(&fftbuf[i1..i2]);
                    fftdst[back - i] = get_average(&fftbuf[j1..j2]);
                } else {
                    fftdst[i] = fftbuf[i1];
                    fftdst[back - i] = fftbuf[back - i1];
                }
            }
        }

        // 进行 FFT 逆向变换
        self.fft_inverse.process(&mut fftdst);

        // 切割大小
        fftdst.truncate(desired_length);

        // 标准化输出
        Ok(fftdst.into_iter().map(|c| -> f32 {(c.re * self.normalize_scaler) as f32}).collect())
    }

    pub fn get_process_size(&self, orig_size: usize, src_sample_rate: u32, dst_sample_rate: u32) -> usize {
        const MAX_INFRASOUND_FREQ: usize = 20;
        // 处理单元大小要改成按每秒多少个块的方式来处理，而块的数量正好符合最大次声波频率的时候，杂音就会消失。
        // 调用 `self.get_desired_length()` 可以推导出根据目标采样率计算出来的处理后的块大小。
        if src_sample_rate == dst_sample_rate {
            min(self.fft_size, orig_size)
        } else {
            min(self.fft_size, src_sample_rate as usize / MAX_INFRASOUND_FREQ)
        }
    }

    pub fn get_desired_length(&self, proc_size: usize, src_sample_rate: u32, dst_sample_rate: u32) -> usize {
        min(self.fft_size, proc_size * dst_sample_rate as usize / src_sample_rate as usize)
    }

    pub fn resample(&mut self, input: &[f32], src_sample_rate: u32, dst_sample_rate: u32, max_lengthen_rate: u32) -> Result<Vec<f32>, ResamplerError> {
        if src_sample_rate == dst_sample_rate {
            Ok(input.to_vec())
        } else if src_sample_rate > dst_sample_rate {
            // 源采样率高于目标采样率，说明要压缩波形
            let desired_length = self.fft_size * dst_sample_rate as usize / src_sample_rate as usize;
            self.resample_core(&input, desired_length)
        } else {
            // 源采样率低于目标采样率，说明要拉长波形
            let desired_length = (self.fft_size * dst_sample_rate as usize / src_sample_rate as usize) / max_lengthen_rate as usize;
            let proc_size = self.fft_size / max_lengthen_rate as usize;
            let mut iter = input.into_iter();
            let mut ret = Vec::<f32>::new();
            loop {
                let chunk: Vec<f32> = iter.by_ref().take(proc_size).copied().collect();
                if chunk.len() == 0 {
                    break;
                }
                let result = self.resample_core(&chunk, desired_length).unwrap();
                ret.extend(&result);
            }
            Ok(ret)
        }
    }

    pub fn get_fft_size(&self) -> usize {
        self.fft_size
    }
}

impl Debug for Resampler {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("Resampler")
            .field("fft_forward", &format_args!("..."))
            .field("fft_inverse", &format_args!("..."))
            .field("fft_size", &self.fft_size)
            .field("normalize_scaler", &self.normalize_scaler)
            .finish()
    }
}
