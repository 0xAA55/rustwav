
use crate::waveform::Error;

pub struct SampleUtils;

impl SampleUtils {

    // 整数补位
    pub fn integer_upcast(value: i32, bits: u16) -> i32 {
        let value = value as u32;
        (match bits {
             8 => {let value = 0xFF & value; (value << 24) | (value << 16) | (value << 8) | value},
             16 => {let value = 0xFFFF & value; (value << 16) | value},
             24 => {let value = 0xFFFFFF & value; (value << 8) | (value >> 16)},
             32 => value,
             _ => panic!("`bits` should be 8, 16, 24, 32, got {}", bits),
         }) as i32
    }

    pub fn convert_i2f(value: i32) -> f32 {
        value as f32 / i32::MAX as f32
    }

    pub fn integer_upcast_to_float(value: i32, bits: u16) -> f32 {
        Self::convert_i2f(Self::integer_upcast(value, bits))
    }

    pub fn integers_upcast_to_floats(src_samples: &[i32], bits: u16) -> Vec<f32> {
        let mut ret = Vec::<f32>::with_capacity(src_samples.len());
        for sample in src_samples.iter() {
            ret.push(Self::integer_upcast_to_float(*sample, bits));
        }
        ret
    }

    pub fn unzip_samples(src_samples: &[f32]) -> (Vec<f32>, Vec<f32>) {
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

    pub fn zip_samples(src_samples: &(Vec<f32>, Vec<f32>)) -> Vec<f32> {
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

    // 两个向量各元素各自相叠加
    pub fn samples_add(v1: &[f32], v2: &[f32]) -> Result<Vec<f32>, Error> {
        if v1.len() != v2.len() {return Err(Error::LengthNotMatch);}
        let mut v3 = v1.to_owned();
        for i in 0..v3.len() {
            v3[i] += v2[i];
        }
        Ok(v3)
    }

    // 两个向量各元素求平均值
    pub fn samples_mix(v1: &[f32], v2: &[f32]) -> Result<Vec<f32>, Error> {
        if v1.len() != v2.len() {return Err(Error::LengthNotMatch);}
        let mut v3 = v1.to_owned();
        for i in 0..v3.len() {
            v3[i] = (v3[i] + v2[i]) * 0.5;
        }
        Ok(v3)
    }
}