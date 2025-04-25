#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fmt::Debug, cmp::min, io::SeekFrom};

use crate::Reader;
use crate::{SampleType, i24, u24};
use crate::{AudioError, AudioReadError};
use crate::adpcm;
use crate::wavcore::{Spec, WaveSampleType, FmtChunk};
use crate::xlaw::{XLaw, PcmXLawDecoder};

#[cfg(feature = "mp3dec")]
use mp3::Mp3Decoder;

#[cfg(feature = "opus")]
use opus::OpusDecoder;

// Decodes audio into samples of the caller-provided format `S`.
pub trait Decoder<S>: Debug
    where S: SampleType {

    // These interfaces must be implemented
    fn get_channels(&self) -> u16;
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError>;
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError>;
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError>;

    // Optional interface
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(self.decode_frame()?.map(|samples| samples[0])),
            2 => Ok(self.decode_frame()?.map(|samples| S::average(samples[0], samples[1]))),
            o => Err(AudioReadError::Unsupported(format!("Unsupported to merge {o} channels to 1 channels."))),
        }
    }

    // Optional interface
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(self.decode_frame()?.map(|samples| (samples[0], samples[0]))),
            2 => Ok(self.decode_frame()?.map(|samples| (samples[0], samples[1]))),
            o => Err(AudioReadError::Unsupported(format!("Unsupported to merge {o} channels to 2 channels."))),
        }
    }

    // Optional interface
    fn decode_frames(&mut self, num_frames: usize) -> Result<Vec<Vec<S>>, AudioReadError> {
        let mut frames = Vec::<Option<Vec<S>>>::with_capacity(num_frames);
        for _ in 0..num_frames {
            frames.push(self.decode_frame()?);
        }
        Ok(frames.into_iter().flatten().collect())
    }

    // Optional interface
    fn decode_monos(&mut self, num_monos: usize) -> Result<Vec<S>, AudioReadError> {
        let mut monos = Vec::<Option<S>>::with_capacity(num_monos);
        for _ in 0..num_monos {
            monos.push(self.decode_mono()?);
        }
        Ok(monos.into_iter().flatten().collect())
    }

    // Optional interface
    fn decode_stereos(&mut self, num_stereos: usize) -> Result<Vec<(S, S)>, AudioReadError> {
        let mut stereos = Vec::<Option<(S, S)>>::with_capacity(num_stereos);
        for _ in 0..num_stereos {
            stereos.push(self.decode_stereo()?);
        }
        Ok(stereos.into_iter().flatten().collect())
    }
}

impl<S> Decoder<S> for PcmDecoder<S>
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.spec.channels }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { PcmDecoder::<S>::get_cur_frame_index(self) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono() }
}

impl<S, D> Decoder<S> for AdpcmDecoderWrap<D>
    where S: SampleType,
          D: adpcm::AdpcmDecoder {
    fn get_channels(&self) -> u16 { self.channels }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(AdpcmDecoderWrap::<D>::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

impl<S> Decoder<S> for PcmXLawDecoderWrap
    where S: SampleType {
    fn get_channels(&self) -> u16 { self.channels }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(PcmXLawDecoderWrap::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "mp3dec")]
impl<S> Decoder<S> for Mp3Decoder
    where S: SampleType {
    fn get_channels(&self) -> u16 { Mp3Decoder::get_channels(self) }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(Mp3Decoder::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { Mp3Decoder::seek(self, seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "opus")]
impl<S> Decoder<S> for OpusDecoder
    where S: SampleType {
    fn get_channels(&self) -> u16 { OpusDecoder::get_channels(self) }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(OpusDecoder::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[derive(Debug)]
pub struct PcmDecoder<S>
where S: SampleType {
    reader: Box<dyn Reader>,
    data_offset: u64,
    data_length: u64,
    block_align: u16,
    total_frames: u64,
    spec: Spec,
    sample_decoder: fn(&mut dyn Reader) -> Result<S, AudioReadError>,
}

impl<S> PcmDecoder<S>
where S: SampleType {
    pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk) -> Result<Self, AudioError> {
        match fmt.format_tag {
            1 | 0xFFFE | 3 => (),
            o => return Err(AudioError::InvalidArguments(format!("`PcmDecoder` can't handle format_tag 0x{:x}", o))),
        }
        let wave_sample_type = spec.get_sample_type();
        Ok(Self {
            reader,
            data_offset,
            data_length,
            block_align: fmt.block_align,
            total_frames: data_length / fmt.block_align as u64,
            spec: *spec,
            sample_decoder: Self::choose_sample_decoder(wave_sample_type)?,
        })
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        Ok(self.reader.stream_position()? >= self.data_offset + self.data_length)
    }

    pub fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> {
        Ok((self.reader.stream_position()? - self.data_offset) / (self.block_align as u64))
    } 

    pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
        let frame_index = match seek_from{
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => {
                (self.get_cur_frame_index()? as i64 + cur) as u64
            },
            SeekFrom::End(end) => {
                (self.total_frames as i64 + end) as u64
            }
        };
        if frame_index > self.total_frames {
            self.reader.seek(SeekFrom::Start(self.data_offset + self.data_length))?;
            Ok(())
        } else {
            self.reader.seek(SeekFrom::Start(self.data_offset + frame_index * self.block_align as u64))?;
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

    // The decoder returned by this function has two exclusive responsibilities:
    // 1. Read raw bytes from the input stream.
    // 2. Convert them into samples of the target format `S`.
    // It does NOT handle end-of-stream detection — the caller must implement 
    // termination logic (e.g., check input.is_empty() or external duration tracking).
    #[allow(clippy::type_complexity)]
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
                o => Err(AudioReadError::Unsupported(format!("Unsupported to merge {o} channels to 2 channels."))),
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
                o => Err(AudioReadError::Unsupported(format!("Unsupported to merge {o} channels to 1 channels."))),
            }
        }
    }
}

#[derive(Debug)]
pub struct AdpcmDecoderWrap<D>
where D: adpcm::AdpcmDecoder {
    channels: u16,
    reader: Box<dyn Reader>,
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
        let decoder =  D::new(fmt)?;
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

    pub fn get_cur_frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn feed_until_output(&mut self, wanted_length: usize) -> Result<(), AudioReadError>{
        let end_of_data = self.data_offset + self.data_length;
        let mut sample_decoded = 0u64;
        while self.samples.len() < wanted_length {
            let cur_pos = self.reader.stream_position()?;
            if cur_pos < end_of_data {
                let remains = end_of_data - cur_pos;
                let to_read = min(remains, self.block_align as u64);
                let mut buf = vec![0u8; to_read as usize];
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
                // Force-decodes at least 1 sample to ensure data availability
                if self.samples.is_empty() {
                    self.feed_until_output(1)?;
                }

                // Empty after feeding indicates end-of-stream
                if self.samples.is_empty() {
                    Ok(None)
                } else {
                    // Internal status check
                    if self.frame_index < self.first_frame_of_samples {
                        panic!("Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}", self.frame_index, self.first_frame_of_samples);
                    } else if self.frame_index < self.frames_decoded {
                        let ret = self.samples[(self.frame_index - self.first_frame_of_samples) as usize];
                        self.frame_index += 1;
                        Ok(Some(S::from(ret)))
                    } else {
                        // Need to decode the next block
                        self.first_frame_of_samples += self.samples.len() as u64;
                        self.samples.clear();
                        self.decode_mono::<S>()
                    }
                }
            },
            2 => {
                let ret = self.decode_stereo::<S>()?;
                match ret {
                    None => Ok(None),
                    Some((l, r)) => {
                        Ok(Some(S::average(l, r)))
                    }
                }
            },
            o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
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
            },
            2 => {
                // Force-decodes at least 1 sample to ensure data availability
                if self.samples.is_empty() {
                    self.feed_until_output(2)?;
                }

                // Empty after feeding indicates end-of-stream
                if self.samples.is_empty() {
                    Ok(None)
                } else {
                    // Internal status check
                    if self.frame_index < self.first_frame_of_samples {
                        panic!("Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}", self.frame_index, self.first_frame_of_samples);
                    } else if self.frame_index < self.frames_decoded {
                        let index = ((self.frame_index - self.first_frame_of_samples) * 2) as usize;
                        self.frame_index += 1;
                        let l = self.samples[index];
                        let r = self.samples[index + 1];
                        Ok(Some((S::from(l), S::from(r))))
                    } else {
                        // Need to decode the next block
                        self.first_frame_of_samples += (self.samples.len() / 2) as u64;
                        self.samples.clear();
                        self.decode_stereo::<S>()
                    }
                }
            },
            o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
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
            o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
        }
    }
}

#[derive(Debug)]
pub struct PcmXLawDecoderWrap {
    reader: Box<dyn Reader>,
    channels: u16,
    data_offset: u64,
    data_length: u64,
    total_frames: u64,
    frame_index: u64,
    dec: PcmXLawDecoder,
}

impl PcmXLawDecoderWrap {
    pub fn new(reader: Box<dyn Reader>, which_law: XLaw, data_offset: u64, data_length: u64, fmt: &FmtChunk, total_samples: u64) -> Result<Self, AudioReadError> {
        match fmt.channels {
            1 => (),
            2 => (),
            o => return Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
        }
        Ok(Self {
            reader,
            channels: fmt.channels,
            data_offset,
            data_length,
            total_frames: total_samples / fmt.channels as u64,
            frame_index: 0,
            dec: PcmXLawDecoder::new(which_law),
        })
    }

    fn decode(&mut self) -> Result<i16, AudioReadError> {
        Ok(self.dec.decode(u8::read_le(&mut self.reader)?))
    }

    pub fn get_cur_frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn seek(&mut self, from: SeekFrom) -> Result<(), AudioReadError> {
        let mut frame_index = match from {
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => (self.frame_index as i64 + cur) as u64,
            SeekFrom::End(end) => (self.frame_index as i64 + end) as u64,
        };
        if frame_index > self.total_frames {
            frame_index = self.total_frames;
        }
        self.frame_index = frame_index;
        self.reader.seek(SeekFrom::Start(self.data_offset + self.frame_index * self.channels as u64))?;
        Ok(())
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
    }

    pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
    where S: SampleType {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.channels {
                1 => {
                    let s = S::from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some(s))
                },
                2 => {
                    let l = S::from(self.decode()?);
                    let r = S::from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some(S::average(l, r)))
                },
                o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
            }
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where S: SampleType {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.channels {
                1 => {
                    let s = S::from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some((s, s)))
                },
                2 => {
                    let l = S::from(self.decode()?);
                    let r = S::from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some((l, r)))
                },
                o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
            }
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
            o => Err(AudioReadError::Unsupported(format!("Unsupported channels {o}"))),
        }
    }
}

#[cfg(feature = "mp3dec")]
pub mod mp3 {
    const FFT_SIZE: usize = 65536;
    use std::{io::{Read, SeekFrom}, fmt::{self, Debug, Formatter}, mem};

    use crate::{AudioReadError};
    use crate::Reader;
    use crate::SampleType;
    use crate::Resampler;
    use crate::wavcore::FmtChunk;
    use crate::utils;

    use rmp3::{DecoderOwned, Frame};

    pub struct Mp3Decoder {
        target_sample_rate: u32,
        target_channels: u16,
        the_decoder: DecoderOwned<Vec<u8>>,
        cur_frame: Option<Mp3AudioData>,
        sample_pos: u64,
        total_frames: u64,
        resampler: Resampler,
    }

    impl Debug for Mp3Decoder{
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            fmt.debug_struct("Mp3Decoder")
                .field("target_sample_rate", &self.target_sample_rate)
                .field("target_channels", &self.target_channels)
                .field("the_decoder", &format_args!("DecoderOwned<Vec<u8>>"))
                .field("cur_frame", &self.cur_frame)
                .field("sample_pos", &self.sample_pos)
                .field("total_frames", &self.total_frames)
                .field("resampler", &self.resampler)
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
        fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
            fmt.debug_struct("Mp3AudioData")
                .field("bitrate", &self.bitrate)
                .field("channels", &self.channels)
                .field("mpeg_layer", &self.mpeg_layer)
                .field("sample_rate", &self.sample_rate)
                .field("sample_count", &self.sample_count)
                .field("samples", &format_args!("[i16; {}]", self.samples.len()))
                .field("buffer_index", &self.buffer_index)
                .finish()
        }
    }

    impl Mp3Decoder {
        pub fn new(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, fmt: &FmtChunk, total_samples: u64) -> Result<Self, AudioReadError> {
            let mut reader = reader;
            let mut mp3_raw_data = vec![0u8; data_length as usize];
            reader.seek(SeekFrom::Start(data_offset))?;
            reader.read_exact(&mut mp3_raw_data)?;
            let the_decoder = rmp3::DecoderOwned::new(mp3_raw_data);
            let mut ret = Self {
                target_sample_rate: fmt.sample_rate,
                target_channels: fmt.channels,
                the_decoder,
                cur_frame: None,
                sample_pos: 0,
                total_frames: total_samples,
                resampler: Resampler::new(FFT_SIZE),
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

        fn do_resample(&self, samples: &[i16], channels: u16, src_sample_rate: u32) -> Vec<i16> {
            let process_size = self.resampler.get_process_size(FFT_SIZE, src_sample_rate, self.target_sample_rate);
            let mut monos = utils::interleaved_samples_to_monos(samples, channels).unwrap();
            for mono in monos.iter_mut() {
                let mut iter = mem::take(mono).into_iter();
                loop {
                    let block: Vec<i16> = iter.by_ref().take(process_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    mono.extend(&utils::do_resample_mono(&self.resampler, &block, src_sample_rate, self.target_sample_rate));
                }
            }
            utils::monos_to_interleaved_samples(&monos).unwrap()
        }

        fn get_next_frame(&mut self) -> Option<Mp3AudioData> {
            while let Some(frame) = self.the_decoder.next() {
                if let Frame::Audio(audio) = frame {
                    if let Some(cur_frame) = &self.cur_frame {
                        self.sample_pos += cur_frame.sample_count as u64;
                    }

                    let mut ret = Mp3AudioData{
                        bitrate: audio.bitrate(),
                        channels: audio.channels(),
                        mpeg_layer: audio.mpeg_layer(),
                        sample_rate: audio.sample_rate(),
                        sample_count: audio.sample_count(),
                        samples: audio.samples().to_vec(),
                        buffer_index: 0,
                    };

                    // First, convert the source channels to the target channels
                    match (ret.channels, self.target_channels) {
                        (1, t) => {
                            ret.samples = mem::take(&mut ret.samples).into_iter().flat_map(|s| -> Vec<i16> {vec![s; t as usize]}).collect();
                            ret.channels = self.target_channels;
                        },
                        (t, 1) => {
                            let mut iter = mem::take(&mut ret.samples).into_iter();
                            loop {
                                let frame: Vec<i32> = iter.by_ref().take(t as usize).map(|s|{s as i32}).collect();
                                if frame.is_empty() {
                                    break;
                                }
                                ret.samples.push((frame.iter().sum::<i32>() / frame.len() as i32) as i16);
                            }
                            ret.channels = self.target_channels;
                        },
                        (s, t) => {
                            if s != t {
                                eprintln!("Can't change {s} channels to {t} channels.");
                            }
                        },
                    }

                    // Second, change the sample rate to match the target sample rate
                    ret.samples = self.do_resample(&ret.samples, ret.channels, ret.sample_rate);
                    ret.sample_rate = self.target_sample_rate;
                    ret.sample_count = ret.samples.len() / ret.channels as usize;

                    return Some(ret);
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
            self.target_sample_rate
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
                        o => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {o}."))),
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
                        o => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {o}."))),
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
                        o => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {o}."))),
                    }
                },
            }
        }
    }
}

#[cfg(feature = "opus")]
pub mod opus {
    use std::{io::SeekFrom, cmp::Ordering};

    use crate::Reader;
    use crate::AudioReadError;
    use crate::SampleType;
    use crate::wavcore::FmtChunk;

    use opus::{Decoder, Channels};

    #[derive(Debug)]
    pub struct OpusDecoder {
        reader: Box<dyn Reader>,
        decoder: Decoder,
        channels: u16,
        sample_rate: u32,
        data_offset: u64,
        data_length: u64,
        total_frames: u64,
        block_align: usize,
        decoded_samples: Vec<f32>,
        decoded_samples_index: usize,
        frame_index: u64,
        block_frame_counts: Vec<u16>,
    }

    impl OpusDecoder {
        pub fn new(mut reader: Box<dyn Reader>, data_offset: u64, data_length: u64, fmt: &FmtChunk, total_samples: u64) -> Result<Self, AudioReadError> {
            let channels = fmt.channels;
            let sample_rate = fmt.sample_rate;
            let opus_channels = match channels {
                1 => Channels::Mono,
                2 => Channels::Stereo,
                o => return Err(AudioReadError::InvalidArguments(format!("Bad channels: {o} for the opus decoder."))),
            };
            let decoder = Decoder::new(sample_rate, opus_channels)?;
            reader.seek(SeekFrom::Start(data_offset))?;
            Ok(Self{
                reader,
                decoder,
                channels,
                sample_rate,
                data_offset,
                data_length,
                total_frames: total_samples / channels as u64,
                block_align: fmt.block_align as usize,
                decoded_samples: Vec::<f32>::new(),
                decoded_samples_index: 0,
                frame_index: 0,
                block_frame_counts: Vec::<u16>::new(),
            })
        }

        pub fn get_channels(&self) -> u16 {
            self.channels
        }

        pub fn get_sample_rate(&self) -> u32 {
            self.sample_rate
        }

        pub fn get_cur_frame_index(&self) -> u64 {
            self.frame_index
        }

        fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
            let end_of_data = self.data_offset + self.data_length;
            if self.reader.stream_position()? >= end_of_data { Ok(true) } else { Ok(false) }
        }

        fn get_num_samples_in_ms(&self, ms_val: f32) -> usize {
            (self.sample_rate as f32 * ms_val / 1000.0) as usize * self.channels as usize
        }

        fn decode_block(&mut self) -> Result<(), AudioReadError> {
            if self.is_end_of_data()? {
                self.decoded_samples = vec![0.0; 0];
                self.decoded_samples_index = 0;
                return Ok(());
            }
            let block_index = ((self.reader.stream_position()? - self.data_offset) / self.block_align as u64) as usize;
            let mut buf = vec![0u8; self.block_align];
            self.reader.read_exact(&mut buf)?;
            self.decoded_samples = vec![0.0; self.get_num_samples_in_ms(60.0) * self.channels as usize];
            let samples = self.decoder.decode_float(&buf, &mut self.decoded_samples, false)? * self.channels as usize;
            self.decoded_samples.truncate(samples);
            self.decoded_samples_index = 0;
            let cur_frames = samples as u16 / self.channels;
            match block_index.cmp(&self.block_frame_counts.len()) {
                Ordering::Equal => self.block_frame_counts.push(cur_frames),
                Ordering::Less => self.block_frame_counts[block_index] = cur_frames,
                Ordering::Greater => {
                    self.block_frame_counts.resize(block_index + 1, 0);
                    self.block_frame_counts[block_index] = cur_frames;
                },
            }
            Ok(())
        }

        pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
            let frame_index = match seek_from{
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(cur) => {
                    (self.frame_index as i64 + cur) as u64
                },
                SeekFrom::End(end) => {
                    (self.total_frames as i64 + end) as u64
                }
            };
            let mut block_frames = 0u64;
            let mut bi = 0usize;
            loop {
                self.reader.seek(SeekFrom::Start(self.data_offset + (bi * self.block_align) as u64))?;
                if self.is_end_of_data()? {
                    self.frame_index = frame_index;
                    self.decoded_samples.clear();
                    self.decoded_samples_index = 0;
                    break;
                }
                if bi >= self.block_frame_counts.len() {
                    self.decode_block()?;
                }
                let cur_block_samples = self.block_frame_counts[bi] as u64;
                bi += 1;
                if block_frames <= frame_index && block_frames + cur_block_samples > frame_index {
                    self.frame_index = frame_index;
                    self.decoded_samples_index = (frame_index - block_frames) as usize * self.channels as usize;
                    break;
                }
                block_frames += cur_block_samples;
            }
            Ok(())
        }

        fn decode_sample(&mut self) -> Result<Option<f32>, AudioReadError> {
            if self.decoded_samples_index >= self.decoded_samples.len() {
                self.decode_block()?;
            }
            if self.decoded_samples.is_empty() {
                Ok(None)
            } else {
                let ret = self.decoded_samples[self.decoded_samples_index];
                self.decoded_samples_index += 1;
                Ok(Some(ret))
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where S: SampleType {
            match self.channels {
                1 => {
                    let s = self.decode_sample()?;
                    if let Some(s) = s {self.frame_index += 1; Ok(Some(S::from(s)))} else {Ok(None)}
                },
                2 => {
                    let l = self.decode_sample()?;
                    let r = self.decode_sample()?;
                    let l = if let Some(l) = l {S::from(l)} else {return Ok(None);};
                    let r = if let Some(r) = r {S::from(r)} else {return Ok(None);};
                    self.frame_index += 1;
                    Ok(Some(S::average(l, r)))
                },
                o => Err(AudioReadError::DataCorrupted(format!("Bad channels: {o} for the opus decoder."))),
            }
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where S: SampleType {
            match self.channels {
                1 => {
                    let s = self.decode_sample()?;
                    if let Some(s) = s {self.frame_index += 1; let s = S::from(s); Ok(Some((s, s)))} else {Ok(None)}
                }
                2 => {
                    let l = self.decode_sample()?;
                    let r = self.decode_sample()?;
                    let l = if let Some(l) = l {S::from(l)} else {return Ok(None);};
                    let r = if let Some(r) = r {S::from(r)} else {return Ok(None);};
                    self.frame_index += 1;
                    Ok(Some((l, r)))
                },
                o => Err(AudioReadError::DataCorrupted(format!("Bad channels: {o} for the opus decoder."))),
            }
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where S: SampleType {
            let stereo = self.decode_stereo::<S>()?;
            match stereo {
                None => Ok(None),
                Some((l, r)) => {
                    match self.channels {
                        1 => Ok(Some(vec![S::from(l)])),
                        2 => Ok(Some(vec![S::from(l), S::from(r)])),
                        o => Err(AudioReadError::DataCorrupted(format!("Unknown channel count {o}."))),
                    }
                },
            }
        }
    }
}

#[cfg(feature = "flac")]
pub mod flac {
    
}
