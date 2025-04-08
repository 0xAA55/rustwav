#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fmt::Debug, mem, cmp::min};

use crate::adpcm;
use crate::{AudioError, AudioReadError};
use crate::{Spec, WaveSampleType, FmtChunk};
use crate::{SampleType, i24, u24};
use crate::Reader;

// 解码器，解码出来的样本格式是 S
pub trait Decoder<S>: Debug
    where S: SampleType {

    // 必须实现
    fn get_channels(&self) -> u16;
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError>;

    // 可选实现
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(match self.decode_frame()? {
                Some(samples) => Some((samples[0], samples[0])),
                None => None,
            }),
            2 => Ok(match self.decode_frame()? {
                Some(samples) => Some((samples[0], samples[1])),
                None => None,
            }),
            other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 2 channels."))),
        }
    }

    // 可选实现
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(match self.decode_frame()? {
                Some(samples) => Some(samples[0]),
                None => None,
            }),
            2 => Ok(match self.decode_frame()? {
                Some(samples) => Some(samples[0] / S::from(2) + samples[1] / S::from(2)),
                None => None,
            }),
            other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 1 channels."))),
        }
    }
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.spec.channels }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono() }
}

impl<S, D> Decoder<S> for AdpcmDecoderWrap<D>
    where S: SampleType,
          D: adpcm::AdpcmDecoder {
    fn get_channels(&self) -> u16 { self.channels }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "mp3dec")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.get_channels() }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: Box<dyn Reader>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    spec: Spec,
    sample_decoder: fn(&mut dyn Reader) -> Result<S, AudioReadError>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, AudioError> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            other => return Err(AudioError::InvalidArguments(format!("`PcmDecoder` can't handle format_tag 0x{:x}", other))),
        }
        let wave_sample_type = spec.get_sample_type();
        Ok(Self {
            reader,
            data_offset,
            data_length,
            spec: spec.clone(),
            sample_decoder: Self::choose_sample_decoder(wave_sample_type)?,
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    fn decode_sample<T>(&mut self) -> Result<Option<S>, AudioReadError>
    where T: SampleType {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            Ok(Some(S::from(T::read_le(&mut self.reader)?)))
        }
    }

    // 这个函数用于给 choose_decoder 挑选
    fn decode_sample_to<T>(r: &mut dyn Reader) -> Result<S, AudioReadError>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }
    fn decode_samples_to<T>(r: &mut dyn Reader, num_samples_to_read: usize) -> Result<Vec<S>, AudioReadError>
    where T: SampleType {
        let mut ret = Vec::<S>::with_capacity(num_samples_to_read);
        for _ in 0..num_samples_to_read {
            ret.push(Self::decode_sample_to::<T>(r)?);
        }
        Ok(ret)
    }

    // 这个函数返回的 decoder 只负责读取和转换格式，不负责判断是否读到末尾
    fn choose_sample_decoder(wave_sample_type: WaveSampleType) -> Result<fn(&mut dyn Reader) -> Result<S, AudioReadError>, AudioError> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        match wave_sample_type {
            S8 =>  Ok(Self::decode_sample_to::<i8 >),
            S16 => Ok(Self::decode_sample_to::<i16>),
            S24 => Ok(Self::decode_sample_to::<i24>),
            S32 => Ok(Self::decode_sample_to::<i32>),
            S64 => Ok(Self::decode_sample_to::<i64>),
            U8 =>  Ok(Self::decode_sample_to::<u8 >),
            U16 => Ok(Self::decode_sample_to::<u16>),
            U24 => Ok(Self::decode_sample_to::<u24>),
            U32 => Ok(Self::decode_sample_to::<u32>),
            U64 => Ok(Self::decode_sample_to::<u64>),
            F32 => Ok(Self::decode_sample_to::<f32>),
            F64 => Ok(Self::decode_sample_to::<f64>),
            Unknown => Err(AudioError::InvalidArguments(format!("unknown sample type \"{:?}\"", wave_sample_type))),
        }
    }

    pub fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            let mut frame = Vec::<S>::with_capacity(self.spec.channels as usize);
            for _ in 0..self.spec.channels {
                frame.push((self.sample_decoder)(&mut self.reader)?);
            }
            Ok(Some(frame))
        }
    }

    pub fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.spec.channels {
                1 => {
                    let sample = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some((sample, sample)))
                },
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some((sample_l, sample_r)))
                },
                other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 2 channels."))),
            }
        }
    }

    pub fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.get_channels() {
                1 => {
                    Ok(Some((self.sample_decoder)(&mut self.reader)?))
                },
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some(sample_l / S::from(2) + sample_r / S::from(2)))
                },
                other => Err(AudioReadError::Unsupported(format!("Unsupported to merge {other} channels to 1 channels."))),
            }
        }
    }
}

#[derive(Debug)]
pub struct AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    channels: u16,
    reader: Box<dyn Reader>, // 数据读取器
    data_offset: u64,
    data_length: u64,
    decoder: D,
    samples: Vec<i16>,
}

impl<D> AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, fmt: &FmtChunk) -> Result<Self, AudioReadError> {
        Ok(Self {
            channels: fmt.channels,
            reader,
            data_offset,
            data_length,
            decoder: D::new(&fmt)?,
            samples: Vec::<i16>::new(),
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    fn read_byte(&mut self) -> Option<u8> {
        match self.is_end_of_data() {
            Ok(val) => {
                match val {
                    true => None,
                    false => {
                        match u8::read_le(&mut self.reader) {
                            Ok(byte) => Some(byte),
                            Err(_) => None,
                        }
                    },
                }
            },
            Err(_) => None,
        }
    }

    pub fn feed_until_output(&mut self, wanted_length: usize) -> Result<(), AudioReadError>{
        let end_of_data = self.data_offset + self.data_length;
        while self.samples.len() < wanted_length {
            let remains = end_of_data - self.reader.stream_position()?;
            if remains > 0 {
                let to_read = min(remains, 1024);
                let mut buf = Vec::<u8>::new();
                buf.resize(to_read as usize, 0);
                self.reader.read_exact(&mut buf)?;
                let mut iter = mem::replace(&mut buf, Vec::<u8>::new()).into_iter();
                self.decoder.decode(|| -> Option<u8> {iter.next()},|sample: i16| {self.samples.push(sample);})?;
            } else {
                self.decoder.flush(|sample: i16| {self.samples.push(sample);})?;
                break;
            }
        }
        Ok(())
    }

    pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
    where S: SampleType {
        match self.channels {
            1 => {
                self.feed_until_output(1)?;
                let mut iter = mem::replace(&mut self.samples, Vec::<i16>::new()).into_iter();
                let ret = iter.next();
                self.samples = iter.collect();
                if let Some(ret) = ret {
                    Ok(Some(S::from(ret)))
                } else {
                    Ok(None)
                }
            }
            2 => {
                self.feed_until_output(2)?;
                let mut iter = mem::replace(&mut self.samples, Vec::<i16>::new()).into_iter();
                let l = iter.next();
                let r = iter.next();
                self.samples = iter.collect();
                match (l, r) {
                    (Some(l), Some(r)) => Ok(Some(S::from(((l as i32 + r as i32) / 2) as i16))),
                    _ => Ok(None) //TODO
                }
            },
            other => Err(AudioReadError::Unsupported(format!("Unsupported channels {other}"))),
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where S: SampleType {
        match self.channels {
            1 => {
                self.feed_until_output(1)?;
                let mut iter = mem::replace(&mut self.samples, Vec::<i16>::new()).into_iter();
                let ret = iter.next();
                self.samples = iter.collect();
                if let Some(ret) = ret {
                    let ret = S::from(ret);
                    Ok(Some((ret, ret)))
                } else {
                    Ok(None)
                }
            }
            2 => {
                self.feed_until_output(2)?;
                let mut iter = mem::replace(&mut self.samples, Vec::<i16>::new()).into_iter();
                let l = iter.next();
                let r = iter.next();
                self.samples = iter.collect();
                match (l, r) {
                    (Some(l), Some(r)) => Ok(Some((S::from(l), S::from(r)))),
                    _ => Ok(None)
                }
            },
            other => Err(AudioReadError::Unsupported(format!("Unsupported channels {other}"))),
        }
    }

    pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
    where S: SampleType {
        match self.channels {
            1 => {
                match self.decode_mono::<S>()? {
                    Some(sample) => Ok(Some(vec![sample])),
                    None => Ok(None),
                }
            },
            2 => {
                match self.decode_stereo::<S>()? {
                    Some((l, r)) => Ok(Some(vec![l, r])),
                    None => Ok(None),
                }
            },
            other => Err(AudioReadError::Unsupported(format!("Unsupported channels {other}"))),
        }
    }
}


#[cfg(feature = "mp3dec")]
pub mod MP3 {
    use std::{io::{Read, SeekFrom}, fmt::Debug};
    use rmp3::{DecoderOwned, Frame};
    use crate::errors::AudioReadError;
    use crate::readwrite::Reader;
    use crate::sampleutils::{SampleType};

    pub struct Mp3Decoder {
        target_sample_format: u32,
        target_channels: u16,
        the_decoder: DecoderOwned<Vec<u8>>,
        cur_frame: Option<Mp3AudioData>,
        frame_index: u64,
    }

    impl Debug for Mp3Decoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Decoder")
                .field("target_sample_format", &self.target_sample_format)
                .field("target_channels", &self.target_channels)
                .field("the_decoder", &format_args!("DecoderOwned<Vec<u8>>"))
                .field("cur_frame", &self.cur_frame)
                .field("frame_index", &self.frame_index)
                .finish()
        }
    }

    #[derive(Clone)]
    pub struct Mp3AudioData {
        pub bitrate: u32,
        pub channels: u16,
        pub mpeg_layer: u8,
        pub sample_rate: u32,
        pub sample_count: usize,
        pub samples: Vec<i16>,
        pub buffer_index: usize,
    }

    impl Debug for Mp3AudioData{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3AudioData")
                .field("bitrate", &self.bitrate)
                .field("channels", &self.channels)
                .field("mpeg_layer", &self.mpeg_layer)
                .field("sample_rate", &self.sample_rate)
                .field("sample_count", &self.sample_count)
                .field("samples", &format_args!("[i16; {}]", self.samples.len()))
                .field("buffer_index", &self.buffer_index)
                .finish_non_exhaustive()
        }
    }

    impl Mp3Decoder {
        pub fn new(reader: Box<dyn Reader>, target_sample_format: u32, target_channels: u16, data_offset: u64, data_length: u64) -> Result<Self, AudioReadError> {
            let mut reader = reader;
            let mut mp3_raw_data = Vec::<u8>::new();
            mp3_raw_data.resize(data_length as usize, 0u8);
            reader.seek(SeekFrom::Start(data_offset))?;
            reader.read_exact(&mut mp3_raw_data)?;
            let the_decoder = rmp3::DecoderOwned::new(mp3_raw_data);
            let mut ret = Self {
                target_sample_format,
                target_channels,
                the_decoder,
                cur_frame: None,
                frame_index: 0,
            };
            ret.cur_frame = ret.get_next_frame();
            Ok(ret)
        }

        fn get_next_frame(&mut self) -> Option<Mp3AudioData> {
            while let Some(frame) = self.the_decoder.next() {
                if let Frame::Audio(audio) = frame {
                    self.frame_index += 1;
                    return Some(Mp3AudioData{
                        bitrate: audio.bitrate(),
                        channels: audio.channels(),
                        mpeg_layer: audio.mpeg_layer(),
                        sample_rate: audio.sample_rate(),
                        sample_count: audio.sample_count(),
                        samples: audio.samples().to_vec(),
                        buffer_index: 0,
                    });
                }
            }
            None
        }

        pub fn get_channels(&self) -> u16 {
            self.target_channels
        }

        pub fn get_sample_rate(&self) -> u32 {
            self.target_sample_format
        }

        pub fn get_cur_frame(&self) -> &Option<Mp3AudioData> {
            &self.cur_frame
        }

        pub fn decode_mono_raw(&mut self) -> Result<Option<i16>, AudioReadError> {
            match self.cur_frame {
                None => Ok(None),
                Some(ref mut frame) => {
                    match frame.channels {
                        1 => {
                            let sample = frame.samples[frame.buffer_index];
                            frame.buffer_index += 1;
                            if frame.buffer_index >= frame.sample_count {
                                self.cur_frame = self.get_next_frame();
                            }
                            Ok(Some(sample))
                        },
                        2 => {
                            let l = frame.samples[frame.buffer_index * 2];
                            let r = frame.samples[frame.buffer_index * 2 + 1];
                            frame.buffer_index += 1;
                            if frame.buffer_index >= frame.sample_count {
                                self.cur_frame = self.get_next_frame();
                            }
                            Ok(Some(((l as i32 +  r as i32) / 2i32) as i16))
                        },
                        other => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {other}."))),
                    }
                }
            }
        }

        pub fn decode_stereo_raw(&mut self) -> Result<Option<(i16, i16)>, AudioReadError> {
            match self.cur_frame {
                None => Ok(None),
                Some(ref mut frame) => {
                    match frame.channels {
                        1 => {
                            let sample = frame.samples[frame.buffer_index];
                            frame.buffer_index += 1;
                            if frame.buffer_index >= frame.sample_count {
                                self.cur_frame = self.get_next_frame();
                            }
                            Ok(Some((sample, sample)))
                        },
                        2 => {
                            let l = frame.samples[frame.buffer_index * 2];
                            let r = frame.samples[frame.buffer_index * 2 + 1];
                            frame.buffer_index += 1;
                            if frame.buffer_index >= frame.sample_count {
                                self.cur_frame = self.get_next_frame();
                            }
                            Ok(Some((l, r)))
                        },
                        other => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {other}."))),
                    }
                }
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where S: SampleType {
            match self.decode_mono_raw()? {
                None => Ok(None),
                Some(s) => {
                    Ok(Some(S::from(s)))
                },
            }
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where S: SampleType {
            match self.decode_stereo_raw()? {
                None => Ok(None),
                Some((l, r)) => Ok(Some((S::from(l), S::from(r)))),
            }
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where S: SampleType {
            let stereo = self.decode_stereo::<S>()?;
            match stereo {
                None => Ok(None),
                Some((l, r)) => {
                    match self.target_channels {
                        1 => Ok(Some(vec![S::from(l)])),
                        2 => Ok(Some(vec![S::from(l), S::from(r)])),
                        other => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {other}."))),
                    }
                },
            }
        }
    }
}

