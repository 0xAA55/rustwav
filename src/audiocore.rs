use std::error::Error;

#[derive(Debug)]
pub enum GuessError {
    CantGuessChannelMask(channels), // 无法猜出声道掩码
}

impl Error for GuessError {}

impl std::fmt::Display for GuessError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           GuessError::CantGuessChannelMask(channels) => write!(f, "Can't guess channel mask for channels = {}.", channels),
       }
    }
}

pub trait ChannelMaskValues {
    const FrontLeft: u32 = 0x1;
    const FrontRight: u32 = 0x2;
    const FrontCenter: u32 = 0x4;
    const LowFreq: u32 = 0x8;
    const BackLeft: u32 = 0x10;
    const BackRight: u32 = 0x20;
    const FrontLeftOfCenter: u32 = 0x40;
    const FrontRightOfCenter: u32 = 0x80;
    const BackCenter: u32 = 0x100;
    const SideLeft: u32 = 0x200;
    const SideRight: u32 = 0x400;
    const TopCenter: u32 = 0x800;
    const TopFrontLeft: u32 = 0x1000;
    const TopFrontCenter: u32 = 0x2000;
    const TopFrontRight: u32 = 0x4000;
    const TopBackLeft: u32 = 0x8000;
    const TopBackCenter: u32 = 0x10000;
    const TopBackRight: u32 = 0x20000;
}

#[derive(Clone, Copy, Debug)]
pub enum SampleFormat {
    Unknown,
    Float,
    UInt,
    Int,
}

#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub channels: u16,
    pub channel_mask: u32,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

impl Spec {
    pub fn new() -> Self {
        Self {
            channels: 0,
            channel_mask: 0,
            sample_rate: 0,
            bits_per_sample: 0,
            sample_format: SampleFormat::Unknown,
        }
    }

    pub fn guess_channel_mask(channels: u16) -> Option<u32, GuessError> {
        match channels {
            1 => Ok(1),
            2 => Ok(3),
            other => Err(GuessError::CantGuessChannelMask(other)),
        }
    }
}

