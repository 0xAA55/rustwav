
use std::sync::Arc;
use rustfft::{FftPlanner, Fft, num_complex::Complex};

#[derive(Debug, Clone, Copy)]
pub enum ResamplerError {
	SizeError(String),
}

#[derive(Debug, Clone)]
pub struct Resampler {
    fft_forward: Arc<dyn Fft<f64>>,
    fft_inverse: Arc<dyn Fft<f64>>,
    forward_size: usize,
    inverse_size: usize,
    normalize_scaler: f64,
}

fn get_average(complexes: [Complex<f64>]) -> Complex<f64> {
	let sum = complexes.iter().copied().sum();
	let scaler = 1.0 / complexes.len() as f64;
	Complex<f64> {
		re: sum.re * scaler,
		im: sum.im * scaler,
	}
}

fn interpolate(c1: Complex<f64>, c2: Complex<f64>, s: f64) -> Complex<f64> {
	c1 + (c2 - c1) * s
}

// Resampler 工作方式
// 对于音频的拉长：
// 1. 输入的音频是它本来的长度，末尾补零使其长度达到目标长度
// 2. 进行 FFT 变换，得到频域
// 3. 在频域里，把频率值往低处按比例压缩
// 4. 进行 FFT 逆变换，得到拉长的音频
// 对于音频的压缩：
// 1. 输入音频
// 2. 进行 FFT 变换
// 3. 在频域里，把频率值往高处按比例拉长
// 4. 进行 FFT 逆变换，得到的音频的音调增高了，但是长度没变
// 5. 剪切音频，使其缩短长度
impl Resampler {
	pub fn new(input_size: usize, output_size: usize) -> Result<Self, ResamplerError> {
		let mut planner = FftPlanner::new();
		if input_size & 1 != 0 || output_size & 1 != 0 {
			Err(ResamplerError::SizeError(format!("The input size and the output size must be times of 2, got {input_size} and {output_size}")))
		} else {
	        Ok(Self {
	            fft_forward: planner.plan_fft_forward(input_size),
	            fft_inverse: planner.plan_fft_inverse(output_size),
	            forward_size: input_size,
	            inverse_size: output_size,
	            normalize_scaler: 1.0 / output_size as f64,
	        })
		}
	}

	// 输入的样本
	pub fn resample(&self, samples: &[f32]) -> Result<Vec<f32>, ResamplerError> {
		if self.forward_size == self.inverse_size {
			return Ok(samples.to_vec());
		}
		let mut fftbuf: Vec<Complex<f64>> = samples.iter().map(|sample: &f32| -> Complex<f64> {Complex{re: sample as f64, im: 0.0}}).collect();

		// 末尾补零
		if fftbuf.len() < self.forward_size {
			fftbuf.resize(self.forward_size, Complex{re: 0.0, im: 0.0});
		}

		// 进行 FFT 正向变换
		self.fft_forward.process(&mut fftbuf);

		// 准备进行插值
		let mut fftdst = vec![Complex<f64>{re: 0.0, im: 0.0}; self.inverse_size];
		let forward_half = self.forward_size / 2;
		let inverse_half = self.inverse_size / 2;
		if self.forward_size > self.inverse_size {
			// 输入大小大于输出大小，意味着音频的缩短
			for i in 0..forward_half {

			}
		} else {
			// 输入大小小于输出大小，意味着音频的伸长
			for i = 0..inverse_half {
				
			}
		}
	}
}
