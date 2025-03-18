pub struct SampleUtils;

impl SampleUtils {

    // 整数补位
    pub fn si_to_i32(value: i32, bits: u16) -> i32 {
        let value = value as u32;
        let ret = match bits {
            8 => {let value = 0xFF & value; (value << 24) | (value << 16) | (value << 8) | value},
            16 => {let value = 0xFFFF & value; (value << 16) | value},
            24 => {let value = 0xFFFFFF & value; (value << 8) | (value >> 16)},
            32 => value,
            _ => panic!("`bits` should be 8, 16, 24, 32, got {}", bits),
        };
        ret as i32
    }

    pub fn ui_to_u32(value: u32, bits: u16) -> u32 {
        match bits {
            8 => {let value = 0xFF & value; (value << 24) | (value << 16) | (value << 8) | value},
            16 => {let value = 0xFFFF & value; (value << 16) | value},
            24 => {let value = 0xFFFFFF & value; (value << 8) | (value >> 16)},
            32 => value,
            _ => panic!("`bits` should be 8, 16, 24, 32, got {}", bits),
        }
    }

    pub fn si_to_f32(value: i32, bits: u16) -> f32 {
        Self::i32_to_f32(Self::si_to_i32(value, bits))
    }

    pub fn ui_to_f32(value: u32, bits: u16) -> f32 {
        Self::i32_to_f32(Self::u32_to_i32(Self::ui_to_u32(value, bits)))
    }

    pub fn u8_to_f32(value: u8) -> f32 {
        Self::ui_to_f32(value as u32, 8)
    }

    pub fn i16_to_f32(value: i16) -> f32 {
        Self::si_to_f32(value as i32, 16)
    }

    // 整数除位
    pub fn i32_to_i16(value: i32) -> i16 {
        (value >> 16) as i16
    }

    pub fn i32_to_i8(value: i32) -> i8 {
        (value >> 24) as i8
    }

    pub fn i16_to_i8(value: i16) -> i8 {
        (value >> 8) as i8
    }

    // 整数符号转换
    pub fn i32_to_u32(value: i32) -> u32 {
        value.wrapping_add(0x80000000u32 as i32) as u32
    }

    pub fn u32_to_i32(value: u32) -> i32 {
        (value as i32).wrapping_sub(0x80000000u32 as i32)
    }

    // 整数转小数
    pub fn i32_to_f32(value: i32) -> f32 {
        value as f32 / i32::MAX as f32
    }

    pub fn i32s_to_f32s(value: &[i32]) -> Vec<f32> {
        let mut ret = Vec::<f32>::with_capacity(value.len());
        for i in value.iter() {
            ret.push(Self::i32_to_f32(*i));
        }
        ret
    }

    pub fn integers_to_f32s(src_samples: &[i32], bits: u16) -> Vec<f32> {
        let mut ret = Vec::<f32>::with_capacity(src_samples.len());
        for sample in src_samples.iter() {
            ret.push(Self::i32_to_f32(Self::si_to_i32(*sample, bits)));
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
}