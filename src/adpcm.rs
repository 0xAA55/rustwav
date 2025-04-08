// https://github.com/superctr/adpcm/tree/master
#![allow(dead_code)]

use std::{io, fmt::Debug};
use crate::FmtExtension;

pub trait AdpcmEncoder: Debug {
    fn new() -> Self;
    fn encode(&mut self, input: impl FnMut() -> Option<i16>, output: impl FnMut(u8)) -> Result<(), io::Error>;
    fn get_interleave_bytes(&self) -> usize {
        4
    }
    fn get_header_bytes(&self) -> usize {
        0
    }
    fn get_block_size(&self) -> u16 {
        512
    }
    fn yield_extension_data(channels: u16) -> Option<FmtExtension> {
        None
    }
    fn flush(&mut self, output: impl FnMut(u8)) -> Result<(), io::Error> {
        Ok(())
    }
}

pub trait AdpcmDecoder: Debug {
    fn new(extension_data: Option<FmtExtension>) -> Result<Self, io::Error>;
    fn decode(&mut self, input: impl FnMut() -> Option<u8>, output: impl FnMut(i16)) -> Result<(), io::Error>;
    fn get_interleave_bytes(&self) -> usize {
        4
    }
    fn get_header_bytes(&self) -> usize {
        0
    }
    fn get_block_size(&self) -> u16 {
        512
    }
    fn flush(&mut self, output: impl FnMut(i16)) -> Result<(), io::Error> {
        Ok(())
    }
}

pub trait AdpcmCodec: AdpcmEncoder + AdpcmDecoder {}
impl<T> AdpcmCodec for T where T: AdpcmEncoder + AdpcmDecoder{}

pub fn test(encoder: &mut impl AdpcmEncoder, decoder: &mut impl AdpcmDecoder, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
    encoder.encode(
        ||-> Option<i16> { input() },
        |code: u8| {
            let buf = vec![code];
            let mut iter = buf.into_iter();
            decoder.decode(
                || -> Option<u8> { iter.next() },
                |sample: i16|{ output(sample) }).unwrap()
        }
    )
}

pub type AdpcmEncoderBS      = bs::Encoder;
pub type AdpcmEncoderOKI     = oki::Encoder;
pub type AdpcmEncoderOKI6258 = oki6258::Encoder;
pub type AdpcmEncoderYMA     = yma::Encoder;
pub type AdpcmEncoderYMB     = ymb::Encoder;
pub type AdpcmEncoderYMZ     = ymz::Encoder;
pub type AdpcmEncoderAICA    = aica::Encoder;
pub type AdpcmEncoderIMA     = ima::Encoder;
pub type AdpcmEncoderMS      = ms::Encoder;

pub type AdpcmDecoderBS      = bs::Decoder;
pub type AdpcmDecoderOKI     = oki::Decoder;
pub type AdpcmDecoderOKI6258 = oki6258::Decoder;
pub type AdpcmDecoderYMA     = yma::Decoder;
pub type AdpcmDecoderYMB     = ymb::Decoder;
pub type AdpcmDecoderYMZ     = ymz::Decoder;
pub type AdpcmDecoderAICA    = aica::Decoder;
pub type AdpcmDecoderIMA     = ima::Decoder;
pub type AdpcmDecoderMS      = ms::Decoder;

pub type EncBS      = AdpcmEncoderBS;
pub type EncOKI     = AdpcmEncoderOKI;
pub type EncOKI6258 = AdpcmEncoderOKI6258;
pub type EncYMA     = AdpcmEncoderYMA;
pub type EncYMB     = AdpcmEncoderYMB;
pub type EncYMZ     = AdpcmEncoderYMZ;
pub type EncAICA    = AdpcmEncoderAICA;
pub type EncIMA     = AdpcmEncoderIMA;
pub type EncMS      = AdpcmEncoderMS;

pub type DecBS      = AdpcmDecoderBS;
pub type DecOKI     = AdpcmDecoderOKI;
pub type DecOKI6258 = AdpcmDecoderOKI6258;
pub type DecYMA     = AdpcmDecoderYMA;
pub type DecYMB     = AdpcmDecoderYMB;
pub type DecYMZ     = AdpcmDecoderYMZ;
pub type DecAICA    = AdpcmDecoderAICA;
pub type DecIMA     = AdpcmDecoderIMA;
pub type DecMS      = AdpcmDecoderMS;

pub mod bs {
    // Encode and decode algorithms for
    // Brian Schmidt's ADPCM used in QSound DSP

    // 2018-2019 by superctr.
    // 2025 by 0xAA55

    use std::io;

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    // step ADPCM algorithm
    fn bs_step(step: i8, history: &mut i16, step_size: &mut i16) -> i16 {
        const ADPCM_TABLE: [i16; 16] = [
            154, 154, 128, 102, 77, 58, 58, 58, // 2.4, 2.4, 2.0, 1.6, 1.2, 0.9, 0.9, 0.9
            58, 58, 58, 58, 77, 102, 128, 154   // 0.9, 0.9, 0.9, 0.9, 1.2, 1.6, 2.0, 2.4
        ];
        
        let scale = *step_size as i32;
        let mut delta = ((1 + (step << 1).abs() as i32) * scale) >> 1;
        let out = *history as i32;
        if step <= 0 {
            delta = -delta;
        }
        let out = ((out + delta).clamp(-32768, 32767)) as i16;
        let scale = (scale * ADPCM_TABLE[(8 + step) as usize] as i32) >> 6;
        *step_size = scale.clamp(1, 2000) as i16;
        *history = out;
        out
    }

    // step high pass filter
    fn bs_hpf_step(input: i16, history: &mut i16, state: &mut i32) -> i16 {
        *state = (*state >> 2) + input as i32 - *history as i32;
        *history = input;
        let out = (*state >> 1) + input as i32;
        out.clamp(-32768, 32767) as i16
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub step_size: i16,
        pub history: i16,
        pub buf_sample: u8,
        pub nibble: u8,
        pub filter_history: i16,
        pub filter_state: i32,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                step_size: 10,
                history: 0,
                buf_sample: 0,
                nibble: 0,
                filter_history: 0,
                filter_state: 0,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                let step = bs_hpf_step(sample, &mut self.filter_history, &mut self.filter_state);
                let step = ((step / self.step_size) >> 1).clamp(-8, 7) as i8;
                if self.nibble != 0 {
                    output(self.buf_sample | (step as u8 & 0xF));
                } else {
                    self.buf_sample = (step as u8 & 0xF) << 4;
                }
                self.nibble ^= 1;
                bs_step(step, &mut self.history, &mut self.step_size);
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub step_size: i16,
        pub history: i16,
        pub nibble: u8,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                step_size: 10,
                history: 0,
                nibble: 0,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                output(bs_step(step, &mut self.history, &mut self.step_size));
            }
            Ok(())
        }
    }
}

pub mod oki {
    // Encode and decode algorithms for
    // OKI ADPCM

    // Only difference between MSM6295 and MSM6258 is that the nibbles are swapped.
    // MSM6295 reads from MSB to LSB. MSM6258 reads from LSB to MSB.

    // Dialogic 'VOX' PCM reads from MSB to LSB, therefore should use the MSM6295 functions.

    // 2019-2022 by superctr.
    // 2025 by 0xAA55

    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    const OKI_STEP_TABLE: [u16; 49] = [
        16, 17, 19, 21, 23, 25, 28, 31,
        34, 37, 41, 45, 50, 55, 60, 66,
        73, 80, 88, 97, 107,118,130,143,
        157,173,190,209,230,253,279,307,
        337,371,408,449,494,544,598,658,
        724,796,876,963,1060,1166,1282,1411,1552
    ];

    pub fn oki_step(step: u8, history: &mut i16, step_hist: &mut u8, oki_highpass: bool) -> i16
    {
        const ADJUST_TABLE: [i8; 8] = [
            -1,-1,-1,-1,2,4,6,8
        ];

        let step_size = OKI_STEP_TABLE[*step_hist as usize] as i16;
        let mut delta = (step_size >> 3) as i16;
        if step & 1 != 0 {
            delta += step_size >> 2;
        }
        if step & 2 != 0 {
            delta += step_size >> 1;
        }
        if step & 4 != 0 {
            delta += step_size;
        }
        if step & 8 != 0 {
            delta = -delta;
        }

        let out: i32;
        if oki_highpass {
            out = (((delta as i32) << 8) + ((*history as i32) * 245)) >> 8;
        } else {
            out = (*history + delta) as i32;
        }
        let out = out.clamp(-2048, 2047) as i16; // Saturate output
        *history = out;
        let adjusted_step = *step_hist as i8 + ADJUST_TABLE[(step & 7) as usize];
        *step_hist = adjusted_step.clamp(0, 48) as u8;
        out
    }

    pub fn oki_encode_step(input: i16, history: &mut i16, step_hist: &mut u8, oki_highpass: bool) -> u8 {
        let mut step_size = OKI_STEP_TABLE[*step_hist as usize] as i16;
        let mut delta = input - *history;
        let mut adpcm_sample: u8 = if delta < 0 { 8 } else { 0 };
        delta = delta.abs();
        for bit in (0..3).rev() {
            if delta >= step_size {
                adpcm_sample |= 1 << bit;
                delta -= step_size;
            }
            step_size >>= 1;
        }
        oki_step(adpcm_sample as u8, history, step_hist, oki_highpass);
        adpcm_sample
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub history: i16,
        pub step_hist: u8,
        pub buf_sample: u8,
        pub nibble: u8,
        pub oki_highpass: bool,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                history: 0,
                step_hist: 0,
                buf_sample: 0,
                nibble: 0,
                oki_highpass: false,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let mut sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                if sample < 0x7FF8 {
                    sample += 8;
                }
                sample >>= 4;
                let step = oki_encode_step(sample, &mut self.history, &mut self.step_hist, self.oki_highpass);
                if self.nibble != 0 {
                    output(self.buf_sample | (step & 0xF));
                } else {
                    self.buf_sample = (step & 0xF) << 4;
                }
                self.nibble ^= 1;
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub history: i16,
        pub step_hist: u8,
        pub nibble: u8,
        pub oki_highpass: bool,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                history: 0,
                step_hist: 0,
                nibble: 0,
                oki_highpass: false,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                output(oki_step(step as u8, &mut self.history, &mut self.step_hist, self.oki_highpass) << 4);
            }
            Ok(())
        }
    }
}

pub mod oki6258 {
    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    use super::oki::oki_encode_step;
    use super::oki::oki_step;

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub history: i16,
        pub step_hist: u8,
        pub buf_sample: u8,
        pub nibble: u8,
        pub oki_highpass: bool,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                history: 0,
                step_hist: 0,
                buf_sample: 0,
                nibble: 0,
                oki_highpass: false,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let mut sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                if sample < 0x7FF8 {
                    sample += 8;
                }
                sample >>= 4;
                let step = oki_encode_step(sample, &mut self.history, &mut self.step_hist, self.oki_highpass);
                if self.nibble != 0 {
                    output(self.buf_sample | ((step & 0xF) << 4));
                } else {
                    self.buf_sample = step & 0xF;
                }
                self.nibble ^= 1;
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub history: i16,
        pub step_hist: u8,
        pub nibble: u8,
        pub oki_highpass: bool,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                history: 0,
                step_hist: 0,
                nibble: 4,
                oki_highpass: false,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                output(oki_step(step as u8, &mut self.history, &mut self.step_hist, self.oki_highpass) << 4);
            }
            Ok(())
        }
    }
}

pub mod yma {
    // Encode and decode algorithms for
    // Yamaha ADPCM-A

    // 2019 by superctr.
    // 2025 by 0xAA55

    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    const YMA_STEP_TABLE: [u16; 49] = [
        16, 17, 19, 21, 23, 25, 28, 31,
        34, 37, 41, 45, 50, 55, 60, 66,
        73, 80, 88, 97, 107,118,130,143,
        157,173,190,209,230,253,279,307,
        337,371,408,449,494,544,598,658,
        724,796,876,963,1060,1166,1282,1411,1552
    ];

    pub fn yma_step(step: u8, history: &mut i16, step_hist: &mut u8) -> i16 {
        const DELTA_TABLE: [i8; 16] = [
            1,3,5,7,9,11,13,15, -1,-3,-5,-7,-9,-11,-13,-15
        ];
        const ADJUST_TABLE: [i8; 8] = [
            -1,-1,-1,-1,2,5,7,9
        ];
        let step_size = YMA_STEP_TABLE[*step_hist as usize];
        let delta = DELTA_TABLE[(step & 0xF) as usize] as i16 * step_size as i16 / 8;
        let out = (*history + delta) & 0xFFF; // No saturation
        let out = out | if out & 0x800 != 0 { 0xf000u16 as i16 } else { 0 };
        *history = out;
        let adjusted_step = *step_hist as i8 + ADJUST_TABLE[(step & 7) as usize]; // Different adjust table
        *step_hist = adjusted_step.clamp(0, 48) as u8;
        out
    }

    pub fn yma_encode_step(input: i16, history: &mut i16, step_hist: &mut u8) -> u8 {
        let mut step_size = YMA_STEP_TABLE[*step_hist as usize] as i16;
        let mut delta = input - *history;
        let mut adpcm_sample = if delta < 0 { 8 } else { 0 };
        if delta < 0 {
            adpcm_sample = 8;
        }
        delta = delta.abs();
        for bit in (0..3).rev() {
            if delta >= step_size {
                adpcm_sample |= 1 << bit;
                delta -= step_size;
            }
            step_size >>= 1;
        }
        yma_step(adpcm_sample, history, step_hist);
        adpcm_sample
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub history: i16,
        pub step_hist: u8,
        pub buf_sample: u8,
        pub nibble: u8,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                history: 0,
                step_hist: 0,
                buf_sample: 0,
                nibble: 0,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let mut sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                if sample < 0x7FF8 {
                    sample += 8;
                }
                sample >>= 4;
                let step = yma_encode_step(sample, &mut self.history, &mut self.step_hist);
                if self.nibble != 0 {
                    output(self.buf_sample | (step & 0xF));
                } else {
                    self.buf_sample = (step & 0xF) << 4;
                }
                self.nibble ^= 1;
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub history: i16,
        pub step_hist: u8,
        pub nibble: u8,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                history: 0,
                step_hist: 0,
                nibble: 0,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                output(yma_step(step as u8, &mut self.history, &mut self.step_hist) << 4);
            }
            Ok(())
        }
    }
}

pub mod ymb {
    // Encode and decode algorithms for
    // Y8950/YM2608/YM2610 ADPCM-B

    // 2019 by superctr.
    // 2025 by 0xAA55

    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    pub fn ymb_step(step: u8, history: &mut i16, step_size: &mut i16) -> i16 {
        const STEP_TABLE: [i32; 8] = [
            57, 57, 57, 57, 77, 102, 128, 153
        ];

        let sign = (step & 8) as i32;
        let delta = (step & 7) as i32;
        let diff = ((1 + (delta << 1)) * *step_size as i32) >> 3;
        let mut newval = *history as i32;
        let nstep = (STEP_TABLE[delta as usize] * *step_size as i32) >> 6;
        if sign > 0 {
            newval -= diff;
        } else {
            newval += diff;
        }
        //step_size = nstep.clamp(511, 32767);
        *step_size = nstep.clamp(127, 24576) as i16;
        let newval = newval.clamp(-32768, 32767) as i16;
        *history = newval;
        newval
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub step_size: i16,
        pub history: i16,
        pub buf_sample: u8,
        pub nibble: u8,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                step_size: 127,
                history: 0,
                buf_sample: 0,
                nibble: 0,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                let step = ((sample & -8) - self.history) as i32;
                let mut adpcm_sample = ((step.abs() << 16) / ((self.step_size as i32) << 14)) as u32;
                adpcm_sample = adpcm_sample.clamp(0, 7);
                if step < 0 {
                    adpcm_sample |= 8;
                }
                if self.nibble != 0 {
                    output(self.buf_sample | (adpcm_sample & 0xF) as u8);
                } else {
                    self.buf_sample = ((adpcm_sample & 0xF) << 4) as u8;
                }
                self.nibble ^= 1;
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub step_size: i16,
        pub history: i16,
        pub nibble: u8,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                step_size: 127,
                history: 0,
                nibble: 0,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                output(ymb_step(step as u8, &mut self.history, &mut self.step_size));
            }
            Ok(())
        }
    }
}

pub mod ymz {
    // Encode and decode algorithms for
    // YMZ280B / AICA ADPCM.

    // The only difference between YMZ280B and AICA ADPCM is that the nibbles are swapped.

    // 2019 by superctr.
    // 2025 by 0xAA55

    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    pub fn ymz_step(step: u8, history: &mut i16, step_size: &mut i16) -> i16 {
        const STEP_TABLE: [i32; 8] = [
            230, 230, 230, 230, 307, 409, 512, 614
        ];

        let sign = (step & 8) as i32;
        let delta = (step & 7) as i32;
        let diff = ((1 + (delta << 1)) * *step_size as i32) >> 3;
        let mut newval = *history as i32;
        let nstep = (STEP_TABLE[delta as usize] * *step_size as i32) >> 8;
        // Only found in the official AICA encoder
        // but it's possible all chips (including ADPCM-B) does this.
        let diff = diff.clamp(0, 32767);
        if sign > 0 {
            newval -= diff;
        } else {
            newval += diff;
        }
        //step_size = nstep.clamp(511, 32767);
        *step_size = nstep.clamp(127, 24576) as i16;
        let newval = newval.clamp(-32768, 32767) as i16;
        *history = newval;
        newval
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub step_size: i16,
        pub history: i16,
        pub buf_sample: u8,
        pub nibble: u8,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                step_size: 127,
                history: 0,
                buf_sample: 0,
                nibble: 0,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                let step = ((sample & -8) - self.history) as i32;
                let mut adpcm_sample = ((step.abs() << 16) / ((self.step_size as i32) << 14)) as u32;
                adpcm_sample = adpcm_sample.clamp(0, 7);
                if step < 0 {
                    adpcm_sample |= 8;
                }
                if self.nibble != 0 {
                    output(self.buf_sample | (adpcm_sample & 0xF) as u8);
                } else {
                    self.buf_sample = ((adpcm_sample & 0xF) << 4) as u8;
                }
                self.nibble ^= 1;
                ymz_step(adpcm_sample as u8, &mut self.history, &mut self.step_size);
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        pub step_size: i16,
        pub history: i16,
        pub nibble: u8,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                step_size: 127,
                history: 0,
                nibble: 0,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble != 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                self.history = self.history * 254 / 256; // High pass
                output(ymz_step(step as u8, &mut self.history, &mut self.step_size));
            }
            Ok(())
        }
    }
}

pub mod aica {
    use std::{io::{self}};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    use super::ymz::ymz_step;

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        pub step_size: i16,
        pub history: i16,
        pub buf_sample: u8,
        pub nibble: u8,
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                step_size: 127,
                history: 0,
                buf_sample: 0,
                nibble: 0,
            }
        }

        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            loop {
                let sample = match input() {
                    Some(sample) => sample,
                    None => break,
                };
                let step = ((sample & -8) - self.history) as i32;
                let mut adpcm_sample = ((step.abs() << 16) / ((self.step_size as i32) << 14)) as u32;
                adpcm_sample = adpcm_sample.clamp(0, 7);
                if step < 0 {
                    adpcm_sample |= 8;
                }
                if self.nibble == 0 {
                    output(self.buf_sample | (adpcm_sample << 4) as u8);
                } else {
                    self.buf_sample = (adpcm_sample & 0xF) as u8;
                }
                self.nibble ^= 1;
                ymz_step(adpcm_sample as u8, &mut self.history, &mut self.step_size);
            }
            Ok(())
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        step_size: i16,
        history: i16,
        nibble: u8,
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                step_size: 127,
                history: 0,
                nibble: 4,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            let mut byte = match input() {
                Some(byte) => byte,
                None => return Ok(()),
            };
            let mut quit = false;
            while !quit {
                let step = (byte as i8) << self.nibble;
                let step = step >> 4;
                if self.nibble == 0 {
                    byte = match input() {
                        Some(byte) => byte,
                        None => {
                            quit = true;
                            byte
                        },
                    }
                }
                self.nibble ^= 4;
                self.history = self.history * 254 / 256; // High pass
                output(ymz_step(step as u8, &mut self.history, &mut self.step_size));
            }
            Ok(())
        }
    }
}

pub mod ima {
    use std::{io::{self}, cmp::min};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    #[derive(Debug)]
    pub enum ImaAdpcmError {
        InvalidArgument(String), // 参数错误
    }

    impl std::error::Error for ImaAdpcmError {}

    impl std::fmt::Display for ImaAdpcmError {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            match self {
                Self::InvalidArgument(info) => write!(f, "Invalid arguments: {info}"),
            }
        }
    }

    const IMAADPCM_INDEX_TABLE: [i8; 16] = [
        -1, -1, -1, -1, 2, 4, 6, 8, 
        -1, -1, -1, -1, 2, 4, 6, 8 
    ];

    const IMAADPCM_STEPSIZE_TABLE: [u16; 89] = [
        7,     8,     9,     10,    11,    12,    13,    14, 
        16,    17,    19,    21,    23,    25,    28,    31, 
        34,    37,    41,    45,    50,    55,    60,    66,
        73,    80,    88,    97,    107,   118,   130,   143, 
        157,   173,   190,   209,   230,   253,   279,   307,
        337,   371,   408,   449,   494,   544,   598,   658,
        724,   796,   876,   963,   1060,  1166,  1282,  1411, 
        1552,  1707,  1878,  2066,  2272,  2499,  2749,  3024,
        3327,  3660,  4026,  4428,  4871,  5358,  5894,  6484,
        7132,  7845,  8630,  9493,  10442, 11487, 12635, 13899,
        15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
        32767
    ];

    const MAX_BLOCK_SIZE: u16 = 512;
    const INTERLEAVE_BYTES: u16 = 4;

    #[derive(Debug, Clone, Copy)]
    pub struct Encoder {
        prev_sample: i16,
        stepsize_index: i8,
        nibble: [u8; 2],
        nibble_index: u8,
        header_written: bool,
        num_outputs: u16,
    }

    impl Encoder{
        // 编一个码
        pub fn encode_sample(&mut self, sample: i16) -> u8 {
            let mut prev = self.prev_sample as i32;
            let idx = self.stepsize_index;
            let stepsize = IMAADPCM_STEPSIZE_TABLE[idx as usize] as i32;
            let diff = sample as i32 - prev;
            let sign = diff < 0;
            let diffabs = diff.abs();
            let mut nibble = min((diffabs << 2) / stepsize, 7) as u8;
            if sign {
                nibble |= 8;
            }
            let delta = (nibble & 7) as i32;
            let qdiff = (stepsize * ((delta << 1) + 1)) >> 3;
            if sign {
                prev -= qdiff;
            } else {
                prev += qdiff;
            }
            prev = prev.clamp(-32768, 32767);
            let idx = (idx + IMAADPCM_INDEX_TABLE[nibble as usize]).clamp(0, 88);
            self.prev_sample = prev as i16;
            self.stepsize_index = idx;
            nibble
        }
    }

    impl AdpcmEncoder for Encoder {
        fn new() -> Self {
            Self {
                prev_sample: 0,
                stepsize_index: 0,
                nibble: [0u8; 2],
                nibble_index: 0,
                header_written: false,
                num_outputs: 0,
            }
        }

        // 编码器逻辑
        // 一开始输出 4 字节的头部信息
        // 然后每两个样本转一个码
        fn encode(&mut self, mut input: impl FnMut() -> Option<i16>, mut output: impl FnMut(u8)) -> Result<(), io::Error> {
            while let Some(sample) = input() {
                if !self.header_written {
                    // 写出 4 字节头部
                    let buf = self.prev_sample.to_le_bytes();
                    output(buf[0]);
                    output(buf[1]);
                    output(self.stepsize_index as u8);
                    output(0);
                    self.num_outputs += 4;
                    self.header_written = true;
                }
                self.nibble[self.nibble_index as usize] = self.encode_sample(sample);
                self.nibble_index += 1;
                if self.nibble_index >= 2 {
                    self.nibble_index = 0;
                    output(self.nibble[0] | (self.nibble[1] << 4));
                    self.num_outputs += 1;
                    if self.num_outputs >= MAX_BLOCK_SIZE {
                        // 到达块大小上限，重置编码器
                        self.prev_sample = sample;
                        self.header_written = false;
                        self.num_outputs = 0;
                    }
                }
            }
            Ok(())
        }

        fn get_interleave_bytes(&self) -> usize {
            INTERLEAVE_BYTES as usize
        }
        fn get_header_bytes(&self) -> usize {
            4
        }
        fn get_block_size(&self) -> u16 {
            MAX_BLOCK_SIZE
        }
        fn flush(&mut self, output: impl FnMut(u8)) -> Result<(), io::Error> {
            let pad = INTERLEAVE_BYTES - self.num_outputs % INTERLEAVE_BYTES;
            if pad != 0 && pad != INTERLEAVE_BYTES {
                let mut pad = Vec::<i16>::new();
                pad.resize(pad_size, 0);
                let iter = pad.into_iter();
                self.encode(
                    || -> Option<i16> {
                        iter.next()
                    },
                    |nibble: u8| {
                        output(nibble)
                    })?
            }
            Ok(())
        }
    }

    // 解码器逻辑
    // data 里面是交错存储的 u32
    // 对于每个声道，第一个 u32 用于初始化解码器
    // 之后的每个 u32 相当于 4 个字节，能解出 8 个码
    #[derive(Debug, Clone, Copy)]
    pub struct Decoder {
        sample_val: i16,
        stepsize_index: i8,
        ready: bool,
        buffer: [u8; 4],
        bufsize: u8,
        input_count: u16,
    }

    impl Decoder{
        pub fn get_num_samples(fact_data: &Vec<u8>) -> Result<u64, ImaAdpcmError> {
            match fact_data.len() {
                4 => Ok(u32::from_le_bytes([fact_data[0], fact_data[1], fact_data[2], fact_data[3]]) as u64),
                8 => Ok(u64::from_le_bytes([fact_data[0], fact_data[1], fact_data[2], fact_data[3], fact_data[4], fact_data[5], fact_data[6], fact_data[7]])),
                other => Err(ImaAdpcmError::InvalidArgument(format!("fact data size should be 4 or 8, not {other}."))),
            }
        }

        // 解一个码
        pub fn decode_sample(&mut self, nibble: u8) -> i16 {
            let mut predict = self.sample_val as i32;
            let idx = self.stepsize_index;
            let stepsize = IMAADPCM_STEPSIZE_TABLE[idx as usize] as i32;
            let idx = (idx + IMAADPCM_INDEX_TABLE[nibble as usize]).clamp(0, 88);
            let delta = (nibble & 7) as i32;
            let qdiff = (stepsize * ((delta << 1) + 1)) >> 3;
            if (nibble & 8) != 0 {
                predict -= qdiff;
            } else {
                predict += qdiff;
            }
            predict = predict.clamp(-32768, 32767);
            self.sample_val = predict as i16;
            self.stepsize_index = idx;
            self.sample_val
        }

        fn push_buf(&mut self, byte: u8) {
            self.buffer[self.bufsize as usize] = byte;
            self.bufsize += 1;
        }
    }

    impl AdpcmDecoder for Decoder {
        fn new(_extension_data: Option<FmtExtension>) -> Result<Self, io::Error> {
            Ok(Self {
                sample_val: 0,
                stepsize_index: 0,
                ready: false,
                buffer: [0u8; INTERLEAVE_BYTES],
                bufsize: 0,
                input_count: 0,
            })
        }

        fn decode(&mut self, mut input: impl FnMut() -> Option<u8>, mut output: impl FnMut(i16)) -> Result<(), io::Error> {
            loop {
                if !self.ready {
                    // 先吃四个字节用来初始化，并输出第一个样本。
                    while self.bufsize < INTERLEAVE_BYTES {
                        match input() {
                            Some(byte) => {
                                self.push_buf(byte);
                                self.input_count += 1;
                            },
                            None => return Ok(()),
                        }
                    }
                    self.sample_val = i16::from_le_bytes([self.buffer[0], self.buffer[1]]);
                    self.stepsize_index = self.buffer[2] as i8;
                    if self.buffer[3] != 0 {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Reserved byte for ADPCM-IMA must be zero."));
                    }
                    self.bufsize = 0;
                    self.ready = true;
                    output(self.sample_val);
                }
                if self.ready {
                    // 完成初始化后，每吃一个字节输出两个样本。
                    while self.bufsize < INTERLEAVE_BYTES {
                        match input() {
                            Some(byte) => {
                                self.push_buf(byte);
                                self.input_count += 1;
                            },
                            None => return Ok(()),
                        }
                    }
                    // 每读取 4 个字节解 8 个码
                    let (b1, b2, b3, b4) = (self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3]);
                    output(self.decode_sample((b1 >> 0) & 0xF));
                    output(self.decode_sample((b1 >> 4) & 0xF));
                    output(self.decode_sample((b2 >> 0) & 0xF));
                    output(self.decode_sample((b2 >> 4) & 0xF));
                    output(self.decode_sample((b3 >> 0) & 0xF));
                    output(self.decode_sample((b3 >> 4) & 0xF));
                    output(self.decode_sample((b4 >> 0) & 0xF));
                    output(self.decode_sample((b4 >> 4) & 0xF));
                    self.bufsize = 0;
                    if self.input_count >= MAX_BLOCK_SIZE {
                        self.input_count = 0;
                        self.ready = false;
                    }
                }
            }
        }

        fn get_interleave_bytes(&self) -> usize {
            INTERLEAVE_BYTES as usize
        }
        fn get_header_bytes(&self) -> usize {
            4
        }
        fn get_block_size(&self) -> u16 {
            MAX_BLOCK_SIZE
        }
        fn yield_extension_data(channels: u16) -> Option<FmtExtension> {
            Some(FmtExtension::new_adpcm_ima(ExtensionData::AdpcmIma::new(self.get_block_size() * channels)))
        }
        fn flush(&mut self, output: impl FnMut(i16)) -> Result<(), io::Error> {
            if (self.ready, self.bufsize > 0, self.bufsize < INTERLEAVE_BYTES) == (true, true, true) {
                let pad_size = INTERLEAVE_BYTES - self.bufsize;
                let mut pad = Vec::<u8>::new();
                pad.resize(pad_size, 0);
                let iter = pad.into_iter();
                self.decode(
                    || -> Option<u8> {
                        iter.next()
                    },
                    |sample: i16| {
                        output(sample)
                    })?
            } else {
                Ok(())
            }
        }
    }
}

pub mod ms {
    // 巨硬的 ADPCM
    use std::{io::{self}, cmp::min};

    use super::AdpcmEncoder;
    use super::AdpcmDecoder;

    const AdaptationTable: [i32; 16] = [
        230, 230, 230, 230, 307, 409, 512, 614,
        768, 614, 512, 409, 307, 230, 230, 230
    ];

    #[derive(Debug, Clone, Copy)]
    struct AdpcmCoefSet {
        coef1: i16,
        coef2: i16,
    };

    const AdpcmCoefSet: [AdpcmCoefSet; 7] = [
        AdpcmCoefSet{coef1: 256, coef2: 0   },
        AdpcmCoefSet{coef1: 512, coef2: -256},
        AdpcmCoefSet{coef1: 0  , coef2: 0   },
        AdpcmCoefSet{coef1: 192, coef2: 64  },
        AdpcmCoefSet{coef1: 240, coef2: 0   },
        AdpcmCoefSet{coef1: 460, coef2: -208},
        AdpcmCoefSet{coef1: 392, coef2: -232},
    ]


}
