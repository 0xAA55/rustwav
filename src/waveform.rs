#![allow(dead_code)]

#[derive(Clone, bincode::Encode, bincode::Decode)]
pub enum WaveFormChannels {
    None,
    Mono(Vec<f32>),
    Stereo((Vec<f32>, Vec<f32>))
}

#[derive(Debug, Clone)]
pub enum Error {
    Empty,
    ChannelNotMatch,
    LengthNotMatch,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           Error::Empty => write!(f, "No waveform data given"),
           Error::ChannelNotMatch => write!(f, "The channel count is not match"),
           Error::LengthNotMatch => write!(f, "The two channels have different lengths"),
       }
    }
}

fn vecf32_add(v1: &[f32], v2: &[f32]) -> Result<Vec<f32>, Error> {
    if v1.len() != v2.len() {return Err(Error::LengthNotMatch);}
    let mut v3 = v1.to_owned();
    for i in 0..v3.len() {
        v3[i] += v2[i];
    }
    Ok(v3)
}

fn vecf32_mix(v1: &[f32], v2: &[f32]) -> Result<Vec<f32>, Error> {
    if v1.len() != v2.len() {return Err(Error::LengthNotMatch);}
    let mut v3 = v1.to_owned();
    for i in 0..v3.len() {
        v3[i] = (v3[i] + v2[i]) * 0.5;
    }
    Ok(v3)
}

impl WaveFormChannels {
    pub fn len(&self) -> Result<usize, Error> {
        match self {
            WaveFormChannels::None => Ok(0),
            WaveFormChannels::Mono(mono) => Ok(mono.len()),
            WaveFormChannels::Stereo((chnl1, chnl2)) => {
                if chnl1.len() != chnl2.len() {
                    Err(Error::LengthNotMatch)
                } else {
                    Ok(chnl1.len())
                }
            },
        }
    }

    // 立体声转单声道
    pub fn to_mono(&self) -> Result<WaveFormChannels, Error> {
        match self {
            WaveFormChannels::None => Err(Error::Empty),
            WaveFormChannels::Mono(mono) => Ok(WaveFormChannels::Mono(mono.clone())),
            WaveFormChannels::Stereo((chnl1, chnl2)) => Ok(WaveFormChannels::Mono(vecf32_mix(&chnl1, &chnl2)?)),
        }
    }

    pub fn to_stereo(&self) -> Result<WaveFormChannels, Error> {
        match self {
            WaveFormChannels::None => Err(Error::Empty),
            WaveFormChannels::Mono(mono) => Ok(WaveFormChannels::Stereo((mono.clone(), mono.clone()))),
            WaveFormChannels::Stereo(stereo) => Ok(WaveFormChannels::Stereo(stereo.clone())),
        }
    }

    // 使 WaveFormChannels 内容的长度增长到指定值，如果没有内容就不增长。
    pub fn resize(&mut self, target_size: usize) {
        match self {
            WaveFormChannels::None => {
                // 未知声道的 chunk 自动变为单声道 chunk
                let mut mono = Vec::<f32>::new();
                mono.resize(target_size, 0.0);
                *self = WaveFormChannels::Mono(mono);
            },
            WaveFormChannels::Mono(mono) => {
                mono.resize(target_size, 0.0);
            },
            WaveFormChannels::Stereo((chnl1, chnl2)) => {
                chnl1.resize(target_size, 0.0);
                chnl2.resize(target_size, 0.0);
            },
        }
    }

    pub fn resized(&self, target_size: usize) -> WaveFormChannels {
        match self {
            WaveFormChannels::None => {
                // 未知声道的 chunk 自动变为单声道 chunk
                let mut mono = Vec::<f32>::new();
                mono.resize(target_size, 0.0);
                WaveFormChannels::Mono(mono)
            },
            WaveFormChannels::Mono(mono) => {
                let mut mono = mono.clone();
                mono.resize(target_size, 0.0);
                WaveFormChannels::Mono(mono)
            },
            WaveFormChannels::Stereo((chnl1, chnl2)) => {
                let (mut chnl1, mut chnl2) = (chnl1.clone(), chnl2.clone());
                chnl1.resize(target_size, 0.0);
                chnl2.resize(target_size, 0.0);
                WaveFormChannels::Stereo((chnl1, chnl2))
            },
        }
    }

    pub fn get_stereo_channels(&self) -> Result<(Vec<f32>, Vec<f32>), Error> {
        match self {
            WaveFormChannels::None => Err(Error::Empty),
            WaveFormChannels::Mono(mono) => Ok((mono.to_vec(), mono.to_vec())),
            WaveFormChannels::Stereo(stereo) => Ok(stereo.clone()),
        }
    }

    // 拼接两个 WaveFormChannels，会检查其中的类型，不允许不同类型的进行拼接，但是 None 可以参与拼接。
    pub fn extended(&self, chunk: &WaveFormChannels) -> Result<WaveFormChannels, Error> {
        match self {
            WaveFormChannels::None => {
                // 一个空的音频数据加上一个有的音频数据，直接变为有的音频数据
                Ok(chunk.clone())
            },
            WaveFormChannels::Mono(mono) => {
                match chunk {
                    WaveFormChannels::None => Ok(self.clone()),
                    WaveFormChannels::Mono(mono2) => {
                        let mut mono = mono.clone();
                        mono.extend(mono2);
                        Ok(WaveFormChannels::Mono(mono))
                    },
                    WaveFormChannels::Stereo(stereo) => {
                        let (mut chnl1, mut chnl2) = self.get_stereo_channels()?;
                        let (chnl1_2, chnl2_2) = stereo;
                        chnl1.extend(chnl1_2);
                        chnl2.extend(chnl2_2);
                        Ok(WaveFormChannels::Stereo((chnl1, chnl2)))
                    }
                }
            }
            WaveFormChannels::Stereo(stereo) => {
                let (mut chnl1, mut chnl2) = stereo.clone();
                let (chnl1_2, chnl2_2) = chunk.get_stereo_channels()?;
                chnl1.extend(chnl1_2);
                chnl2.extend(chnl2_2);
                Ok(WaveFormChannels::Stereo((chnl1, chnl2)))
            }
        }
    }

    pub fn extend(&mut self, chunk: &WaveFormChannels) -> Result<(), Error> {
        match self {
            WaveFormChannels::None => {
                // 一个空的音频数据加上一个有的音频数据，直接变为有的音频数据
                *self = chunk.clone();
                Ok(())
            },
            WaveFormChannels::Mono(mono) => {
                match chunk {
                    WaveFormChannels::None => Ok(()),
                    WaveFormChannels::Mono(mono2) => {
                        mono.extend(mono2);
                        Ok(())
                    },
                    WaveFormChannels::Stereo(stereo) => {
                        let (ref mut chnl1, ref mut chnl2) = self.get_stereo_channels()?;
                        let (chnl1_2, chnl2_2) = stereo;
                        chnl1.extend(chnl1_2);
                        chnl2.extend(chnl2_2);
                        Ok(())
                    }
                }
            }
            WaveFormChannels::Stereo(stereo) => {
                let (ref mut chnl1, ref mut chnl2) = stereo;
                let (chnl1_2, chnl2_2) = chunk.get_stereo_channels()?;
                chnl1.extend(chnl1_2);
                chnl2.extend(chnl2_2);
                Ok(())
            }
        }
    }

    pub fn split(&self, at: usize) -> (WaveFormChannels, WaveFormChannels) {
        match self {
            WaveFormChannels::None => (WaveFormChannels::None, WaveFormChannels::None),
            WaveFormChannels::Mono(mono) => {
                (WaveFormChannels::Mono(mono[0..at].to_vec()),
                 WaveFormChannels::Mono(mono[at..].to_vec()))
            },
            WaveFormChannels::Stereo((chnl1, chnl2)) => {
                (WaveFormChannels::Stereo((chnl1[0..at].to_vec(), chnl2[0..at].to_vec())),
                 WaveFormChannels::Stereo((chnl1[at..].to_vec(), chnl2[at..].to_vec())))
            },
        }
    }

    // 叠加两个 chunk 的值
    pub fn add_to(&self, chunk: &WaveFormChannels) -> Result<WaveFormChannels, Error> {
        match (self, chunk) {
            (WaveFormChannels::None, WaveFormChannels::None) => {
                Ok(WaveFormChannels::None)
            },
            (WaveFormChannels::Mono(mono1), WaveFormChannels::Mono(mono2)) => {
                Ok(WaveFormChannels::Mono(vecf32_add(mono1, mono2)?))
            },
            (WaveFormChannels::Stereo((chnl1_1, chnl2_1)), WaveFormChannels::Stereo((chnl1_2, chnl2_2))) => {
                Ok(WaveFormChannels::Stereo((vecf32_add(chnl1_1, chnl1_2)?, vecf32_add(chnl2_1, chnl2_2)?)))
            },
            _ => Err(Error::ChannelNotMatch),
        }
    }
}