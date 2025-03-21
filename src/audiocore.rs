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

#[derive(Clone, Copy, Debug)]
pub enum SpeakerPosition {
    FrontLeft = 0x1,
    FrontRight = 0x2,
    FrontCenter = 0x4,
    LowFreq = 0x8,
    BackLeft = 0x10,
    BackRight = 0x20,
    FrontLeftOfCenter = 0x40,
    FrontRightOfCenter = 0x80,
    BackCenter = 0x100,
    SideLeft = 0x200,
    SideRight = 0x400,
    TopCenter = 0x800,
    TopFrontLeft = 0x1000,
    TopFrontCenter = 0x2000,
    TopFrontRight = 0x4000,
    TopBackLeft = 0x8000,
    TopBackCenter = 0x10000,
    TopBackRight = 0x20000,
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

    pub fn guess_channel_mask(channels: u16) -> Result<u32, AudioError> {
        match channels {
            1 => Ok(SpeakerPosition::FrontCenter as u32),
            2 => Ok((SpeakerPosition::FrontLeft as u32) | (SpeakerPosition::FrontRight as u32)),
            other => Err(AudioError::CantGuessChannelMask(other)),
        }
    }

    pub fn which_channel_which_speaker(&self) -> Result<Vec<SpeakerPosition>, AudioError> {
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
        for (i, m) in enums.iter().enumerate() {
            let m = *m as u32;
            if self.channel_mask & m == m {ret.push(enums[i]);}
        }
        return if ret.len() == self.channels.into() {
            Ok(ret)
        } else {
            Err(AudioError::ChannelNotMatchMask)
        }
    }
}


