use std::error::Error;

#[derive(Debug)]
pub enum AudioError {
    CantGuessChannelMask(u16), // 无法猜出声道掩码
    ChannelNotMatchMask, // 声道数不和声道掩码匹配
}

impl Error for AudioError {}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           AudioError::CantGuessChannelMask(channels) => write!(f, "Can't guess channel mask for channels = {}.", channels),
           AudioError::ChannelNotMatchMask => write!(f, "The number of the channels doesn't match the channel mask."),
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

pub enum SpeakerPosition {
    FrontLeft,
    FrontRight,
    FrontCenter,
    LowFreq,
    BackLeft,
    BackRight,
    FrontLeftOfCenter,
    FrontRightOfCenter,
    BackCenter,
    SideLeft,
    SideRight,
    TopCenter,
    TopFrontLeft,
    TopFrontCenter,
    TopFrontRight,
    TopBackLeft,
    TopBackCenter,
    TopBackRight,
}

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

    pub fn guess_channel_mask(channels: u16) -> Result<u32, AudioError> {
        use ChannelMaskValues::*;
        match channels {
            1 => Ok(FrontCenter),
            2 => Ok(FrontLeft | FrontRight),
            other => Err(AudioError::CantGuessChannelMask(other)),
        }
    }

    pub which_channel_which_speaker(&self) -> Result<Vec<ChannelType>, AudioError> {
        let masks = [
            ChannelMaskValues::FrontLeft,
            ChannelMaskValues::FrontRight,
            ChannelMaskValues::FrontCenter,
            ChannelMaskValues::LowFreq,
            ChannelMaskValues::BackLeft,
            ChannelMaskValues::BackRight,
            ChannelMaskValues::FrontLeftOfCenter,
            ChannelMaskValues::FrontRightOfCenter,
            ChannelMaskValues::BackCenter,
            ChannelMaskValues::SideLeft,
            ChannelMaskValues::SideRight,
            ChannelMaskValues::TopCenter,
            ChannelMaskValues::TopFrontLeft,
            ChannelMaskValues::TopFrontCenter,
            ChannelMaskValues::TopFrontRight,
            ChannelMaskValues::TopBackLeft,
            ChannelMaskValues::TopBackCenter,
            ChannelMaskValues::TopBackRight,
        ];
        let enums = [
            SpeakerPosition::FrontLeft,
            SpeakerPosition::FrontRight,
            SpeakerPosition::FrontCenter,
            SpeakerPosition::LowFreq,
            SpeakerPosition::BackLeft,
            SpeakerPosition::BackRight,
            SpeakerPosition::FrontLeftOfCenter,
            SpeakerPosition::FrontRightOfCenter,
            SpeakerPosition::BackCenter,
            SpeakerPosition::SideLeft,
            SpeakerPosition::SideRight,
            SpeakerPosition::TopCenter,
            SpeakerPosition::TopFrontLeft,
            SpeakerPosition::TopFrontCenter,
            SpeakerPosition::TopFrontRight,
            SpeakerPosition::TopBackLeft,
            SpeakerPosition::TopBackCenter,
            SpeakerPosition::TopBackRight,
        ];
        let mut ret = Vec::<SpeakerPosition>::new();
        for (i, m) in masks.iter().enumerate() {
            if self.channel_mask & m == m {ret.push(enums[i]);}
        }
        return if ret.len() == self.channels {
            Ok(ret)
        } else {
            Err(AudioError::ChannelNotMatchMask)
        }
    }
}


