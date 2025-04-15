#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fmt::Debug, cmp::min, io::SeekFrom};

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
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError>;

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
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono() }
}

impl<S, D> Decoder<S> for AdpcmDecoderWrap<D>
    where S: SampleType,
          D: adpcm::AdpcmDecoder {
    fn get_channels(&self) -> u16 { self.channels }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "mp3dec")]
impl<S> Decoder<S> for MP3::Mp3Decoder
    where S: SampleType {
    fn get_channels(&self) -> u16 { MP3::Mp3Decoder::get_channels(self) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
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
    block_align: u16,
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
            block_align: fmt.block_align,
            spec: spec.clone(),
            sample_decoder: Self::choose_sample_decoder(wave_sample_type)?,
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    pub fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> {
        Ok((self.reader.stream_position()? - self.data_offset) / (self.block_align as u64))
    } 

    pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
        let total_frames = self.data_length / self.block_align as u64;
        let frame_index = match seek_from{
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => {
                (self.get_cur_frame_index()? as i64 + cur) as u64
            },
            SeekFrom::End(end) => {
                (total_frames as i64 + end) as u64
            }
        };
        if frame_index > total_frames {
            self.reader.seek(SeekFrom::Start(self.data_offset + self.data_length))?;
            Ok(())
        } else {
            self.reader.seek(SeekFrom::Start(frame_index * self.block_align as u64))?;
            Ok(())
        }
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
    block_align: u16,
    frame_index: u64,
    frames_decoded: u64,
    total_frames: u64,
    decoder: D,
    samples: Vec<i16>,
    first_frame_of_samples: u64,
}

impl<D> AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, fmt: &FmtChunk, total_samples: u64) -> Result<Self, AudioReadError> {
        let decoder =  D::new(&fmt)?;
        let total_frames = if total_samples == 0 {
            let frames_per_block = decoder.frames_per_block() as u64;
            let total_blocks = data_length / fmt.block_align as u64;
            total_blocks * frames_per_block
        } else {
            total_samples / fmt.channels as u64
        };
        Ok(Self {
            channels: fmt.channels,
            reader,
            data_offset,
            data_length,
            block_align: fmt.block_align,
            frame_index: 0,
            frames_decoded: 0,
            total_frames,
            decoder,
            samples: Vec::<i16>::new(),
            first_frame_of_samples: 0,
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    pub fn feed_until_output(&mut self, wanted_length: usize) -> Result<(), AudioReadError>{
        let end_of_data = self.data_offset + self.data_length;
        let mut sample_decoded = 0u64;
        while self.samples.len() < wanted_length {
            let cur_pos = self.reader.stream_position()?;
            if cur_pos < end_of_data {
                let remains = end_of_data - cur_pos;
                let to_read = min(remains, self.block_align as u64);
                let mut buf = Vec::<u8>::new();
                buf.resize(to_read as usize, 0);
                self.reader.read_exact(&mut buf)?;
                let mut iter = buf.into_iter();
                self.decoder.decode(|| -> Option<u8> {iter.next()},|sample: i16| {sample_decoded += 1; self.samples.push(sample)})?;
            } else {
                self.decoder.flush(|sample: i16| {sample_decoded += 1; self.samples.push(sample)})?;
                break;
            }
        }
        self.frames_decoded += sample_decoded / self.channels as u64;
        Ok(())
    }

    pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
        let frames_per_block = self.decoder.frames_per_block() as u64;
        let frame_index = match seek_from{
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => {
                (self.frame_index as i64 + cur) as u64
            },
            SeekFrom::End(end) => {
                (self.total_frames as i64 + end) as u64
            }
        };
        let block_index = frame_index / frames_per_block;
        self.samples.clear();
        self.decoder.reset_states();
        if frame_index >= self.total_frames {
            let end_of_data = self.data_offset + self.data_length;
            self.reader.seek(SeekFrom::Start(end_of_data))?;
            self.first_frame_of_samples = self.total_frames;
            self.frames_decoded = self.total_frames;
            self.frame_index = frame_index;
            Ok(())
        } else {
            let block_pos = self.data_offset + block_index * self.block_align as u64;
            self.reader.seek(SeekFrom::Start(block_pos))?;
            self.first_frame_of_samples = block_index * frames_per_block;
            self.frames_decoded = self.first_frame_of_samples;
            self.frame_index = frame_index;
            Ok(())
        }
    }

    pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
    where S: SampleType {
        match self.channels {
            1 => {
                // 确保解码出至少一个样本来
                if self.samples.len() == 0 {
                    self.feed_until_output(1)?;
                }
                // 确保不了，说明到头了。
                if self.samples.len() == 0 {
                    Ok(None)
                } else {
                    // 内部状态检查
                    if self.frame_index < self.first_frame_of_samples {
                        panic!("Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}", self.frame_index, self.first_frame_of_samples);
                    } else if self.frame_index < self.frames_decoded {
                        let ret = self.samples[(self.frame_index - self.first_frame_of_samples) as usize];
                        self.frame_index += 1;
                        Ok(Some(S::from(ret)))
                    } else {
                        // 需要继续解码下一个块
                        self.first_frame_of_samples += self.samples.len() as u64;
                        self.samples.clear();
                        self.decode_mono::<S>()
                    }
                }
            }
            2 => {
                let ret = self.decode_stereo::<S>()?;
                match ret {
                    None => Ok(None),
                    Some((l, r)) => {
                        Ok(Some(S::average(l, r)))
                    }
                }
            },
            other => Err(AudioReadError::Unsupported(format!("Unsupported channels {other}"))),
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where S: SampleType {
        match self.channels {
            1 => {
                let ret = self.decode_mono::<S>()?;
                match ret {
                    None => Ok(None),
                    Some(ret) => Ok(Some((ret, ret)))
                }
            }
            2 => {
                // 确保解码出至少两个样本来
                if self.samples.len() == 0 {
                    self.feed_until_output(2)?;
                }
                // 确保不了，说明到头了。
                if self.samples.len() == 0 {
                    Ok(None)
                } else {
                    // 内部状态检查
                    if self.frame_index < self.first_frame_of_samples {
                        panic!("Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}", self.frame_index, self.first_frame_of_samples);
                    } else if self.frame_index < self.frames_decoded {
                        let index = ((self.frame_index - self.first_frame_of_samples) * 2) as usize;
                        self.frame_index += 1;
                        let l = self.samples[index];
                        let r = self.samples[index + 1];
                        Ok(Some((S::from(l), S::from(r))))
                    } else {
                        // 需要继续解码下一个块
                        self.first_frame_of_samples += (self.samples.len() / 2) as u64;
                        self.samples.clear();
                        self.decode_stereo::<S>()
                    }
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
    use crate::{AudioReadError};
    use crate::Reader;
    use crate::SampleType;

    pub struct Mp3Decoder {
        target_sample_format: u32,
        target_channels: u16,
        the_decoder: DecoderOwned<Vec<u8>>,
        cur_frame: Option<Mp3AudioData>,
        sample_pos: u64,
        total_frames: u64,
    }

    impl Debug for Mp3Decoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Decoder")
                .field("target_sample_format", &self.target_sample_format)
                .field("target_channels", &self.target_channels)
                .field("the_decoder", &format_args!("DecoderOwned<Vec<u8>>"))
                .field("cur_frame", &self.cur_frame)
                .field("sample_pos", &self.sample_pos)
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
        pub fn new(reader: Box<dyn Reader>, target_sample_format: u32, target_channels: u16, data_offset: u64, data_length: u64, total_samples: u64) -> Result<Self, AudioReadError> {
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
                sample_pos: 0,
                total_frames: total_samples,
            };
            ret.cur_frame = ret.get_next_frame();
            if let Some(ref mp3frame) = ret.cur_frame {
                ret.total_frames /= mp3frame.channels as u64;
            }
            Ok(ret)
        }

        fn reset(&mut self) {
            self.the_decoder.set_position(0);
            self.cur_frame = self.get_next_frame();
            self.sample_pos = 0;
        }

        fn get_next_frame(&mut self) -> Option<Mp3AudioData> {
            while let Some(frame) = self.the_decoder.next() {
                if let Frame::Audio(audio) = frame {
                    if let Some(cur_frame) = &self.cur_frame {
                        self.sample_pos += cur_frame.sample_count as u64;
                    }
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

        pub fn get_cur_frame_index(&self) -> u64 {
            if let Some(frame) = &self.cur_frame {
                self.sample_pos + (frame.buffer_index as u64)
            } else {
                0u64
            }
        }

        pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
            let frame_index = match seek_from{
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(cur) => {
                    (self.get_cur_frame_index() as i64 + cur) as u64
                },
                SeekFrom::End(end) => {
                    (self.total_frames as i64 + end) as u64
                }
            };
            if self.sample_pos > frame_index {
                self.reset();
            }
            loop {
                if let Some(cur_frame) = &self.cur_frame {
                    if self.sample_pos + (cur_frame.sample_count as u64) > frame_index {
                        break;
                    } else {
                        self.cur_frame = self.get_next_frame();
                    }
                } else {
                    return Ok(())
                }
            }
            for _ in 0..(frame_index - self.sample_pos) {
                let _ = self.decode_stereo_raw()?;
            }
            Ok(())
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

