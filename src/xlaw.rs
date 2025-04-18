#![allow(dead_code)]

use std::fmt::{self, Debug, Display, Formatter};

#[derive(Debug, Clone, Copy)]
pub enum XLaw {
    ALaw,
    MuLaw,
}

impl Display for XLaw {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::ALaw => write!(f, "ALaw"),
            Self::MuLaw => write!(f, "MuLaw"),
       }
    }
}

fn alaw_to_linear(a_val: u8) -> i32 {
    let a_val = a_val ^ 0x55;
    let t = a_val as i32 & 0x0F;
    let seg = (a_val & 0x70) >> 4;
    let t = if seg != 0 {
        (t + t + 1 + 32) << (seg + 2)
    } else {
        (t + t + 1     ) << 3
    };

    if a_val & 0x80 != 0 {
        t
    } else {
        -t
    }
}

fn ulaw_to_linear(u_val: u8) -> i32 {
    let u_val = !u_val;
    let t = ((u_val as i32 & 0x0f) << 3) + 0x84;
    let t = t << ((u_val as i32 & 0x70) >> 4);
    if u_val & 0x80 != 0 {
        0x84 - t
    } else {
        t - 0x84
    }
}

#[derive(Clone, Copy)]
pub struct PcmXLawEncoder {
    which_law: XLaw,
    linear_to_xlaw: [u8; 16384]
}

impl Debug for PcmXLawEncoder{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("PcmXLawEncoder")
            .field("which_law", &self.which_law)
            .field("linear_to_xlaw", &format_args!("[u8; {}]", self.linear_to_xlaw.len()))
            .finish()
    }
}

impl PcmXLawEncoder {
    fn build_linear_to_xlaw_table(mut xlaw2linear: impl FnMut(u8) -> i32, mask: u8) -> [u8; 16384] {
        let mut linear_to_xlaw = [0u8; 16384];
        linear_to_xlaw[8192] = mask;

        let mut j = 1usize;
        for i in 0..127 {
            let v1 = xlaw2linear(i ^ mask);
            let v2 = xlaw2linear((i + 1) ^ mask);
            let v = ((v1 + v2 + 4) >> 3) as usize;
            while j < v {
                linear_to_xlaw[8192 - j] = i ^ (mask ^ 0x80);
                linear_to_xlaw[8192 + j] = i ^ mask;
                j += 1;
            }
        }
        while j < 8192 {
            linear_to_xlaw[8192 - j] = 127 ^ (mask ^ 0x80);
            linear_to_xlaw[8192 + j] = 127 ^ mask;
            j += 1;
        }
        linear_to_xlaw[0] = linear_to_xlaw[1];
        linear_to_xlaw
    }

    pub fn get_which_law(&self) -> XLaw {
        self.which_law
    }

    pub fn new(which_law: XLaw) -> Self {
        match which_law {
            XLaw::ALaw => Self::new_alaw(),
            XLaw::MuLaw => Self::new_ulaw(),
        }
    }

    pub fn new_alaw() -> Self {
        Self {
            which_law: XLaw::ALaw,
            linear_to_xlaw: Self::build_linear_to_xlaw_table(alaw_to_linear, 0xd5),
        }
    }

    pub fn new_ulaw() -> Self {
        Self {
            which_law: XLaw::MuLaw,
            linear_to_xlaw: Self::build_linear_to_xlaw_table(ulaw_to_linear, 0xff),
        }
    }

    pub fn encode(&self, sample: i16) -> u8 {
        self.linear_to_xlaw[((sample as u16).wrapping_add(0x8000) >> 2) as usize]
    }
}

#[derive(Clone, Copy)]
pub struct PcmXLawDecoder {
    which_law: XLaw,
    table: [i16; 256]
}

impl Debug for PcmXLawDecoder{
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("PcmXLawDecoder")
            .field("which_law", &self.which_law)
            .field("table", &format_args!("[i16; {}]", self.table.len()))
            .finish()
    }
}

impl PcmXLawDecoder {
    pub fn get_which_law(&self) -> XLaw {
        self.which_law
    }

    pub fn new(which_law: XLaw) -> Self {
        match which_law {
            XLaw::ALaw => Self::new_alaw(),
            XLaw::MuLaw => Self::new_ulaw(),
        }
    }

    pub fn new_alaw() -> Self {
        Self {
            which_law: XLaw::ALaw,
            table: core::array::from_fn(|i| {alaw_to_linear(i as u8) as i16}),
        }
    }

    pub fn new_ulaw() -> Self {
        Self {
            which_law: XLaw::MuLaw,
            table: core::array::from_fn(|i| {ulaw_to_linear(i as u8) as i16}),
        }
    }

    pub fn decode(&self, byte: u8) -> i16 {
        self.table[byte as usize]
    }
}
