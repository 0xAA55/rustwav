#![allow(dead_code)]

use std::{fs::File, io::{Seek, Read, Write}};
use tempfile::tempfile;
use bincode::{Encode, Decode, encode_to_vec, decode_from_slice};

#[derive(Clone, Encode, Decode)]
pub enum WaveForm {
    None,
    Mono(Vec<f32>),
    Stereo((Vec<f32>, Vec<f32>))
}

#[derive(Debug, Clone)]
pub enum Error {
    Empty,
    ChannelNotMatch,
    LengthNotMatch,
    SerializeError,
    DeserializeError,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
       match self {
           Error::Empty => write!(f, "No waveform data given"),
           Error::ChannelNotMatch => write!(f, "The channel count is not match"),
           Error::LengthNotMatch => write!(f, "The two channels have different lengths"),
           Error::SerializeError => write!(f, "Serialize error"),
           Error::DeserializeError => write!(f, "Deserialize error"),
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

impl WaveForm {
    pub fn len(&self) -> Result<usize, Error> {
        match self {
            WaveForm::None => Ok(0),
            WaveForm::Mono(mono) => Ok(mono.len()),
            WaveForm::Stereo((chnl1, chnl2)) => {
                if chnl1.len() != chnl2.len() {
                    Err(Error::LengthNotMatch)
                } else {
                    Ok(chnl1.len())
                }
            },
        }
    }

    // 立体声转单声道
    pub fn to_mono(&self) -> Result<WaveForm, Error> {
        match self {
            WaveForm::None => Err(Error::Empty),
            WaveForm::Mono(mono) => Ok(WaveForm::Mono(mono.clone())),
            WaveForm::Stereo((chnl1, chnl2)) => Ok(WaveForm::Mono(vecf32_mix(&chnl1, &chnl2)?)),
        }
    }

    pub fn to_stereo(&self) -> Result<WaveForm, Error> {
        match self {
            WaveForm::None => Err(Error::Empty),
            WaveForm::Mono(mono) => Ok(WaveForm::Stereo((mono.clone(), mono.clone()))),
            WaveForm::Stereo(stereo) => Ok(WaveForm::Stereo(stereo.clone())),
        }
    }

    // 使 WaveForm 内容的长度增长到指定值，如果没有内容就不增长。
    pub fn resize(&mut self, target_size: usize) {
        match self {
            WaveForm::None => {
                // 未知声道的 chunk 自动变为单声道 chunk
                let mut mono = Vec::<f32>::new();
                mono.resize(target_size, 0.0);
                *self = WaveForm::Mono(mono);
            },
            WaveForm::Mono(mono) => {
                mono.resize(target_size, 0.0);
            },
            WaveForm::Stereo((chnl1, chnl2)) => {
                chnl1.resize(target_size, 0.0);
                chnl2.resize(target_size, 0.0);
            },
        }
    }

    pub fn resized(&self, target_size: usize) -> WaveForm {
        match self {
            WaveForm::None => {
                // 未知声道的 chunk 自动变为单声道 chunk
                let mut mono = Vec::<f32>::new();
                mono.resize(target_size, 0.0);
                WaveForm::Mono(mono)
            },
            WaveForm::Mono(mono) => {
                let mut mono = mono.clone();
                mono.resize(target_size, 0.0);
                WaveForm::Mono(mono)
            },
            WaveForm::Stereo((chnl1, chnl2)) => {
                let (mut chnl1, mut chnl2) = (chnl1.clone(), chnl2.clone());
                chnl1.resize(target_size, 0.0);
                chnl2.resize(target_size, 0.0);
                WaveForm::Stereo((chnl1, chnl2))
            },
        }
    }

    pub fn get_stereo_channels(&self) -> Result<(Vec<f32>, Vec<f32>), Error> {
        match self {
            WaveForm::None => Err(Error::Empty),
            WaveForm::Mono(mono) => Ok((mono.to_vec(), mono.to_vec())),
            WaveForm::Stereo(stereo) => Ok(stereo.clone()),
        }
    }

    // 拼接两个 WaveForm，会检查其中的类型，不允许不同类型的进行拼接，但是 None 可以参与拼接。
    pub fn extended(&self, chunk: &WaveForm) -> Result<WaveForm, Error> {
        match self {
            WaveForm::None => {
                // 一个空的音频数据加上一个有的音频数据，直接变为有的音频数据
                Ok(chunk.clone())
            },
            WaveForm::Mono(mono) => {
                match chunk {
                    WaveForm::None => Ok(self.clone()),
                    WaveForm::Mono(mono2) => {
                        let mut mono = mono.clone();
                        mono.extend(mono2);
                        Ok(WaveForm::Mono(mono))
                    },
                    WaveForm::Stereo(stereo) => {
                        let (mut chnl1, mut chnl2) = self.get_stereo_channels()?;
                        let (chnl1_2, chnl2_2) = stereo;
                        chnl1.extend(chnl1_2);
                        chnl2.extend(chnl2_2);
                        Ok(WaveForm::Stereo((chnl1, chnl2)))
                    }
                }
            }
            WaveForm::Stereo(stereo) => {
                let (mut chnl1, mut chnl2) = stereo.clone();
                let (chnl1_2, chnl2_2) = chunk.get_stereo_channels()?;
                chnl1.extend(chnl1_2);
                chnl2.extend(chnl2_2);
                Ok(WaveForm::Stereo((chnl1, chnl2)))
            }
        }
    }

    pub fn extend(&mut self, chunk: &WaveForm) -> Result<(), Error> {
        match self {
            WaveForm::None => {
                // 一个空的音频数据加上一个有的音频数据，直接变为有的音频数据
                *self = chunk.clone();
                Ok(())
            },
            WaveForm::Mono(mono) => {
                match chunk {
                    WaveForm::None => Ok(()),
                    WaveForm::Mono(mono2) => {
                        mono.extend(mono2);
                        Ok(())
                    },
                    WaveForm::Stereo(stereo) => {
                        let (ref mut chnl1, ref mut chnl2) = self.get_stereo_channels()?;
                        let (chnl1_2, chnl2_2) = stereo;
                        chnl1.extend(chnl1_2);
                        chnl2.extend(chnl2_2);
                        Ok(())
                    }
                }
            }
            WaveForm::Stereo(stereo) => {
                let (ref mut chnl1, ref mut chnl2) = stereo;
                let (chnl1_2, chnl2_2) = chunk.get_stereo_channels()?;
                chnl1.extend(chnl1_2);
                chnl2.extend(chnl2_2);
                Ok(())
            }
        }
    }

    pub fn split(&self, at: usize) -> (WaveForm, WaveForm) {
        match self {
            WaveForm::None => (WaveForm::None, WaveForm::None),
            WaveForm::Mono(mono) => {
                (WaveForm::Mono(mono[0..at].to_vec()),
                 WaveForm::Mono(mono[at..].to_vec()))
            },
            WaveForm::Stereo((chnl1, chnl2)) => {
                (WaveForm::Stereo((chnl1[0..at].to_vec(), chnl2[0..at].to_vec())),
                 WaveForm::Stereo((chnl1[at..].to_vec(), chnl2[at..].to_vec())))
            },
        }
    }

    // 叠加两个 chunk 的值
    pub fn add_to(&self, chunk: &WaveForm) -> Result<WaveForm, Error> {
        match (self, chunk) {
            (WaveForm::None, WaveForm::None) => {
                Ok(WaveForm::None)
            },
            (WaveForm::Mono(mono1), WaveForm::Mono(mono2)) => {
                Ok(WaveForm::Mono(vecf32_add(mono1, mono2)?))
            },
            (WaveForm::Stereo((chnl1_1, chnl2_1)), WaveForm::Stereo((chnl1_2, chnl2_2))) => {
                Ok(WaveForm::Stereo((vecf32_add(chnl1_1, chnl1_2)?, vecf32_add(chnl2_1, chnl2_2)?)))
            },
            _ => Err(Error::ChannelNotMatch),
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>, Error> {
        match encode_to_vec(self, bincode::config::standard()) {
            Ok(bytes) => Ok(bytes),
            Err(_) => Err(Error::SerializeError),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, Error> {
        match decode_from_slice(data, bincode::config::standard()) {
            Ok((ret, _size)) => Ok(ret),
            Err(_) => Err(Error::DeserializeError),
        }
    }

    pub fn to_tempfile(self) -> Result<File, Box<dyn std::error::Error>> {
        let mut ret = tempfile()?;
        ret.write_all(&self.into_bytes()?)?;
        ret.flush()?;
        Ok(ret)
    }

    pub fn restore_from_tempfile(mut file: File) -> Result<Self, Box<dyn std::error::Error>> {
        let mut buf = Vec::<u8>::new();
        file.rewind()?;
        file.read_to_end(&mut buf)?;
        Ok(Self::from_bytes(&buf)?)
    }
}