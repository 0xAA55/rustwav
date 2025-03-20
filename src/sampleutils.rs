
pub trait SampleUtils{
    // 无符号升位数
    fn u8_to_u16(v: u8) -> u16{
        let v = v as u16;
        (v << 8) | v
    }
    fn u16_to_u32(v: u16) -> u32{
        let v = v as u32;
        (v << 16) | v
    }
    fn u32_to_u64(v: u32) -> u64{
        let v = v as u64;
        (v << 32) | v
    }
    fn u8_to_u32(v: u8) -> u32{
        u16_to_u32(u8_to_u16(v))
    }
    fn u8_to_u64(v: u8) -> u64{
        u16_to_u64(u8_to_u16(v))
    }
    fn u16_to_u64(v: u16) -> u64{
        u32_to_u64(u16_to_u32(v))
    }

    // 有符号升位数
    fn i8_to_i16(v: i8) -> i16{
        let v = v as i16;
        (v << 8) | (v & 0xFF)
    }
    fn i16_to_i32(v: i16) -> i32{
        let v = v as i32;
        (v << 16) | (v & 0xFFFF)
    }
    fn i32_to_i64(v: i32) -> i64{
        let v = v as i64;
        (v << 32) | (v & 0xFFFFFFFF)
    }
    fn i8_to_i32(v: i8) -> i32{
        i16_to_i32(i8_to_i16(v))
    }
    fn i8_to_i64(v: i8) -> i64{
        i16_to_i64(i8_to_i16(v))
    }
    fn i16_to_i64(v: i16) -> i64{
        i32_to_i64(i16_to_i32(v))
    }

    // 无符号降位数
    fn u16_to_u8(v: u16) -> u8{
        (v >> 8) as u8
    }
    fn u32_to_u8(v: u32) -> u8{
        (v >> 24) as u8
    }
    fn u32_to_u16(v: u32) -> u16{
        (v >> 16) as u16
    }
    fn u64_to_u32(v: u64) -> u32{
        (v >> 32) as u32
    }
    fn u64_to_u16(v: u64) -> u16{
        (v >> 48) as u16
    }
    fn u64_to_u8(v: u64) -> u8{
        (v >> 56) as u8
    }

    // 有符号降位数
    fn i16_to_i8(v: i16) -> i8{
        (v >> 8) as i8
    }
    fn i32_to_i8(v: i32) -> i8{
        (v >> 24) as i8
    }
    fn i32_to_i16(v: i32) -> i16{
        (v >> 16) as i16
    }
    fn i64_to_i32(v: i64) -> i32{
        (v >> 32) as i32
    }
    fn i64_to_i16(v: i64) -> i16{
        (v >> 48) as i16
    }
    fn i64_to_i8(v: i64) -> i8{
        (v >> 56) as i8
    }

    // 同位数u变i
    fn u8_to_i8(v: u8) -> i8{
        v.wrapping_add(0x80u8) as i8
    }
    fn u16_to_i16(v: u16) -> i16{
        v.wrapping_add(0x8000u16) as i16
    }
    fn u32_to_i32(v: u32) -> i32{
        v.wrapping_add(0x80000000u32) as i32
    }
    fn u64_to_i64(v: u64) -> i64{
        v.wrapping_add(0x80000000_00000000u64) as i64
    }

    // 同位数i变u
    fn i8_to_u8(v: i8) -> u8{
        v.wrapping_sub(0x80i8) as u8
    }
    fn i16_to_u16(v: i16) -> u16{
        v.wrapping_sub(0x8000i16) as u16
    }
    fn i32_to_u32(v: i32) -> u32{
        v.wrapping_sub(0x80000000i32) as u32
    }
    fn i64_to_u64(v: i64) -> u64{
        v.wrapping_sub(0x80000000_00000000i64) as u64
    }

    // 降位数u变i
    fn u16_to_i8(v: u16) -> i8{
        i16_to_i8(u16_to_i16(v))
    }
    fn u32_to_i16(v: u32) -> i16{
        i32_to_i16(u32_to_i32(v))
    }
    fn u32_to_i8(v: u32) -> i8{
        i32_to_i8(u32_to_i32(v))
    }
    fn u64_to_i32(v: u64) -> i32{
        i64_to_i32(u64_to_i64(v))
    }
    fn u64_to_i16(v: u64) -> i16{
        i64_to_i16(u64_to_i64(v))
    }
    fn u64_to_i8(v: u64) -> i8{
        i64_to_i8(u64_to_i64(v))
    }

    // 降位数i变u
    fn i16_to_u8(v: i16) -> u8{
        u16_to_u8(i16_to_u16(v))
    }
    fn i32_to_u16(v: i32) -> u16{
        u32_to_u16(i32_to_u32(v))
    }
    fn i32_to_u8(v: i32) -> u8{
        u32_to_u8(i32_to_u32(v))
    }
    fn i64_to_u32(v: i64) -> u32{
        u64_to_u32(i64_to_u64(v))
    }
    fn i64_to_u16(v: i64) -> u16{
        u64_to_u16(i64_to_u64(v))
    }
    fn i64_to_u8(v: i64) -> u8{
        u64_to_u8(i64_to_u64(v))
    }

    // 升位数u变i
    fn u8_to_i16(v: u8) -> i16{
        u16_to_i16(u8_to_u16(v))
    }
    fn u8_to_i32(v: u8) -> i32{
        u32_to_i32(u8_to_u32(v))
    }
    fn u16_to_i32(v: u16) -> i32{
        u32_to_i32(u16_to_u32(v))
    }
    fn u32_to_i64(v: u32) -> i64{
        u64_to_i64(u32_to_u64(v))
    }
    fn u16_to_i64(v: u16) -> i64{
        u64_to_i64(u16_to_u64(v))
    }
    fn u8_to_i64(v: u8) -> i64{
        u64_to_i64(u8_to_u64(v))
    }

    // 有符号转浮点
    fn i64_to_f32(v: i64) -> f32{
        (v as f32) / (i64::MAX as f32)
    }
    fn i32_to_f32(v: i32) -> f32{
        (v as f32) / (i32::MAX as f32)
    }
    fn i16_to_f32(v: i16) -> f32{
        (v as f32) / (i16::MAX as f32)
    }
    fn i8_to_f32(v: i8) -> f32{
        (v as f32) / (i8::MAX as f32)
    }
    fn i64_to_f64(v: i64) -> f64{
        (v as f64) / (i64::MAX as f64)
    }
    fn i32_to_f64(v: i32) -> f64{
        (v as f64) / (i32::MAX as f64)
    }
    fn i16_to_f64(v: i16) -> f64{
        (v as f64) / (i16::MAX as f64)
    }
    fn i8_to_f64(v: i8) -> f64{
        (v as f64) / (i8::MAX as f64)
    }

    // 无符号转浮点
    fn u64_to_f32(v: u64) -> f32{
        i64_to_f32(u64_to_i64(v))
    }
    fn u32_to_f32(v: u32) -> f32{
        i32_to_f32(u32_to_i32(v))
    }
    fn u16_to_f32(v: u16) -> f32{
        i16_to_f32(u16_to_i16(v))
    }
    fn u8_to_f32(v: u8) -> f32{
        i8_to_f32(u8_to_i8(v))
    }
    fn u64_to_f64(v: u64) -> f64{
        i64_to_f64(u64_to_i64(v))
    }
    fn u32_to_f64(v: u32) -> f64{
        i32_to_f64(u32_to_i32(v))
    }
    fn u16_to_f64(v: u16) -> f64{
        i16_to_f64(u16_to_i16(v))
    }
    fn u8_to_f64(v: u8) -> f64{
        i8_to_f64(u8_to_i8(v))
    }

    fn clampf(v: f32) -> f32 {
        if v > 1.0 {
            1.0
        } else if v < -1.0 {
            -1.0
        } else {
            v
        }
    }

    fn clampd(v: f64) -> f64 {
        if v > 1.0 {
            1.0
        } else if v < -1.0 {
            -1.0
        } else {
            v
        }
    }

    // 浮点转有符号整数
    fn f32_to_i64(v: f32) -> i64{
        (clampf(v) * (i64::MAX as f32)) as i64
    }
    fn f32_to_i32(v: f32) -> i32{
        (clampf(v) * (i32::MAX as f32)) as i32
    }
    fn f32_to_i16(v: f32) -> i16{
        (clampf(v) * (i16::MAX as f32)) as i16
    }
    fn f32_to_i8(v: f32) -> i8{
        (clampf(v) * (i8::MAX as f32)) as i8
    }
    fn f64_to_i64(v: f64) -> i64{
        (clampd(v) * (i64::MAX as f64)) as i64
    }
    fn f64_to_i32(v: f64) -> i32{
        (clampd(v) * (i32::MAX as f64)) as i32
    }
    fn f64_to_i16(v: f64) -> i16{
        (clampd(v) * (i16::MAX as f64)) as i16
    }
    fn f64_to_i8(v: f64) -> i8{
        (clampd(v) * (i8::MAX as f64)) as i8
    }

    // 浮点转无符号整数
    fn f32_to_u64(v: f32) -> u64{
        i64_to_u64(f32_to_i64(v))
    }
    fn f32_to_u32(v: f32) -> u32{
        i32_to_u32(f32_to_i32(v))
    }
    fn f32_to_u16(v: f32) -> u16{
        i16_to_u16(f32_to_i16(v))
    }
    fn f32_to_u8(v: f32) -> u8{
        i8_to_u8(f32_to_i8(v))
    }
    fn f64_to_u64(v: f64) -> u64{
        i64_to_u64(f64_to_i64(v))
    }
    fn f64_to_u32(v: f64) -> u32{
        i32_to_u32(f64_to_i32(v))
    }
    fn f64_to_u16(v: f64) -> u16{
        i16_to_u16(f64_to_i16(v))
    }
    fn f64_to_u8(v: f64) -> u8{
        i8_to_u8(f64_to_i8(v))
    }

    // 自己转自己
    fn i8_to_i8(v: i8) -> i8{
        v
    }
    fn i16_to_i16(v: i16) -> i16{
        v
    }
    fn i32_to_i32(v: i32) -> i32{
        v
    }
    fn i64_to_i64(v: i64) -> i64{
        v
    }
    fn u8_to_u8(v: u8) -> u8{
        v
    }
    fn u16_to_u16(v: u16) -> u16{
        v
    }
    fn u32_to_u32(v: u32) -> u32{
        v
    }
    fn u64_to_u64(v: u64) -> u64{
        v
    }
    fn f32_to_f32(v: f32) -> f32{
        v
    }
    fn f64_to_f64(v: f64) -> f64{
        v
    }

    // 特殊情况：i24 的读取。读取三个字节，然后转换为 i32
    fn i24_le_to_i32(i24_le: &[u8; 3]) -> i32 {
        let mut i32_le = [0u8; 4];
        i32_le[0] = i24_le[2];
        i32_le[1] = i24_le[0];
        i32_le[2] = i24_le[1];
        i32_le[3] = i24_le[2];
        i32::from_le_bytes(i32_le)
    }

    fn i24_be_to_i32(i24_be: &[u8; 3]) -> i32 {
        let mut i32_be = [0u8; 4];
        i32_be[0] = i24_be[0];
        i32_be[1] = i24_be[1];
        i32_be[2] = i24_be[2];
        i32_be[3] = i24_be[0];
        i32::from_be_bytes(i32_be)
    }

    // 将 i32 转换为 i24（的字节）
    fn i32_to_i24_le(v: i32) -> [u8; 3] {
        let i32_le = v.to_le_bytes();
        let mut ret = [0u8; 3];
        ret[0] = i32_le[1];
        ret[1] = i32_le[2];
        ret[2] = i32_le[3];
        ret
    }

    fn i32_to_i24_be(v: i32) -> [u8; 3] {
        let i32_be = v.to_be_bytes();
        let mut ret = [0u8; 3];
        ret[0] = i32_be[0];
        ret[1] = i32_be[1];
        ret[2] = i32_be[2];
        ret
    }
}
