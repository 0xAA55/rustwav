pub struct SampleUtils;

impl SampleUtils {

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

    pub fn i32_to_i16(value: i32) -> i16 {
        (value >> 16) as i16
    }

    pub fn i32_to_i8(value: i32) -> i8 {
        (value >> 24) as i8
    }

    pub fn i16_to_i8(value: i16) -> i8 {
        (value >> 8) as i8
    }

    pub fn u32_to_u16(value: u32) -> u16 {
        (value >> 16) as u16
    }

    pub fn u32_to_u8(value: u32) -> u8 {
        (value >> 24) as u8
    }

    pub fn u16_to_u8(value: u16) -> u8 {
        (value >> 8) as u8
    }

    pub fn i32_to_u32(value: i32) -> u32 {
        value.wrapping_add(0x80000000u32 as i32) as u32
    }

    pub fn u32_to_i32(value: u32) -> i32 {
        (value as i32).wrapping_sub(0x80000000u32 as i32)
    }

    pub fn i32_to_f32(value: i32) -> f32 {
        value as f32 / i32::MAX as f32
    }

    pub fn clamp(value: f32) -> f32 {
        if value > 1.0 {
            1.0
        } else if value < -1.0 {
            -1.0
        } else {
            value
        }
    }

    pub fn f32_to_i32(value: f32) -> i32 {
        (Self::clamp(value) * i32::MAX as f32) as i32
    }

    pub fn f32_to_i16(value: f32) -> i16 {
        Self::i32_to_i16(Self::f32_to_i32(value))
    }

    pub fn f32_to_u8(value: f32) -> u8 {
        Self::u32_to_u8(Self::i32_to_u32(Self::f32_to_i32(value)))
    }
}