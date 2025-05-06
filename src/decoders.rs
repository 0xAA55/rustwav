#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{cmp::min, fmt::Debug, io::SeekFrom, marker::PhantomData};

use crate::Reader;
use crate::adpcm;
use crate::get_rounded_up_fft_size;
use crate::wavcore::format_tags::*;
use crate::wavcore::{ExtensibleData, ExtensionData, FmtChunk, Spec, WaveSampleType};
use crate::xlaw::{PcmXLawDecoder, XLaw};
use crate::{AudioError, AudioReadError};
use crate::{SampleType, i24, u24};

#[cfg(feature = "mp3dec")]
use mp3::Mp3Decoder;

#[cfg(feature = "opus")]
use opus::OpusDecoder;

#[cfg(feature = "flac")]
use flac_dec::FlacDecoderWrap;

#[cfg(feature = "oggvorbis")]
use oggvorbis_dec::OggVorbisDecoderWrap;

/// ## Decodes audio into samples of the caller-provided format `S`.
pub trait Decoder<S>: Debug
where
    S: SampleType,
{
    /// Get num channels
    fn get_channels(&self) -> u16;

    /// Decode one audio frame. An audio frame is each channel has one sample. This method supports > 2 channels.
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError>;

    /// Seek to a specific audio frame. An audio frame is each channel has one sample.
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError>;

    /// Get current frame index.
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError>;

    /// Decode a mono sample, multiple channels will be mixed into one channel.
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(self.decode_frame()?.map(|samples| samples[0])),
            _ => Ok(self.decode_frame()?.map(|samples| S::average_arr(&samples))),
        }
    }

    /// Decode a stereo sample with left and right samples, if the audio has > 2 channels, this method fails.
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> {
        match self.get_channels() {
            1 => Ok(self.decode_frame()?.map(|samples| (samples[0], samples[0]))),
            2 => Ok(self.decode_frame()?.map(|samples| (samples[0], samples[1]))),
            o => Err(AudioReadError::Unsupported(format!(
                "Unsupported to merge {o} channels to 2 channels."
            ))),
        }
    }

    /// Decode multiple audio frames. This method supports > 2 channels.
    fn decode_frames(&mut self, num_frames: usize) -> Result<Vec<Vec<S>>, AudioReadError> {
        let mut frames = Vec::<Option<Vec<S>>>::with_capacity(num_frames);
        for _ in 0..num_frames {
            frames.push(self.decode_frame()?);
        }
        Ok(frames.into_iter().flatten().collect())
    }

    /// Decode multiple mono samples, multiple channels will be mixed into one channel.
    fn decode_monos(&mut self, num_monos: usize) -> Result<Vec<S>, AudioReadError> {
        let mut monos = Vec::<Option<S>>::with_capacity(num_monos);
        for _ in 0..num_monos {
            monos.push(self.decode_mono()?);
        }
        Ok(monos.into_iter().flatten().collect())
    }

    // Decode multiple stereo samples with left and right samples, if the audio has > 2 channels, this method fails.
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

#[cfg(feature = "flac")]
impl<S> Decoder<S> for FlacDecoderWrap<'_>
    where S: SampleType {
    fn get_channels(&self) -> u16 { FlacDecoderWrap::get_channels(self) }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(FlacDecoderWrap::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[cfg(feature = "oggvorbis")]
impl<S> Decoder<S> for OggVorbisDecoderWrap<'_>
    where S: SampleType {
    fn get_channels(&self) -> u16 { OggVorbisDecoderWrap::get_channels(self) }
    fn get_cur_frame_index(&mut self) -> Result<u64, AudioReadError> { Ok(OggVorbisDecoderWrap::get_cur_frame_index(self)) }
    fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> { self.seek(seek_from) }
    fn decode_frame(&mut self) -> Result<Option<Vec<S>>, AudioReadError> { self.decode_frame::<S>() }
    fn decode_stereo(&mut self) -> Result<Option<(S, S)>, AudioReadError> { self.decode_stereo::<S>() }
    fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> { self.decode_mono::<S>() }
}

#[derive(Debug)]
pub struct ExtensibleDecoder<S>
where
    S: SampleType,
{
    phantom: PhantomData<S>,
}

impl<S> ExtensibleDecoder<S>
where
    S: SampleType,
{
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        reader: Box<dyn Reader>,
        data_offset: u64,
        data_length: u64,
        spec: Spec,
        fmt: &FmtChunk,
    ) -> Result<Box<dyn Decoder<S>>, AudioError> {
        if fmt.format_tag != FORMAT_TAG_EXTENSIBLE {
            Err(AudioError::InvalidArguments(
                "The `format_tag` from `fmt ` chunk must be 0xFFFE for the extensible decoder."
                    .to_string(),
            ))
        } else {
            match &fmt.extension {
                None => {
                    eprintln!(
                        "No extension data was found in the `fmt ` chunk. The audio data is parsed as PCM."
                    );
                    Ok(Box::new(PcmDecoder::<S>::new(
                        reader,
                        data_offset,
                        data_length,
                        spec,
                        fmt,
                    )?))
                }
                Some(extension) => match &extension.data {
                    ExtensionData::Extensible(extensible) => {
                        if (extension.ext_len as usize) < ExtensibleData::sizeof() {
                            eprintln!(
                                "The size of the extension data found in the `fmt ` chunk is not big enough as the extensible data should be. The audio data is parsed as PCM."
                            );
                            Ok(Box::new(PcmDecoder::<S>::new(
                                reader,
                                data_offset,
                                data_length,
                                spec,
                                fmt,
                            )?))
                        } else {
                            let spec = Spec {
                                channels: spec.channels,
                                channel_mask: extensible.channel_mask,
                                sample_rate: spec.sample_rate,
                                bits_per_sample: spec.bits_per_sample,
                                sample_format: spec.sample_format,
                            };
                            use crate::wavcore::guids::*;
                            match extensible.sub_format {
                                GUID_PCM_FORMAT | GUID_IEEE_FLOAT_FORMAT => {
                                    Ok(Box::new(PcmDecoder::<S>::new(
                                        reader,
                                        data_offset,
                                        data_length,
                                        spec,
                                        fmt,
                                    )?))
                                }
                                o => Err(AudioError::Unimplemented(format!(
                                    "Unknown format of GUID {o} in the extensible data"
                                ))),
                            }
                        }
                    }
                    o => Err(AudioError::WrongExtensionData(format!(
                        "The extension data in the `fmt ` chunk must be `extensible`, got {:?}",
                        o
                    ))),
                },
            }
        }
    }
}

/// ## The `PcmDecoder<S>` to decode WAV PCM samples to your specific format
#[derive(Debug)]
pub struct PcmDecoder<S>
where
    S: SampleType,
{
    reader: Box<dyn Reader>,
    data_offset: u64,
    data_length: u64,
    block_align: u16,
    total_frames: u64,
    spec: Spec,
    sample_decoder: fn(&mut dyn Reader) -> Result<S, AudioReadError>,
}

impl<S> PcmDecoder<S>
where
    S: SampleType,
{
    pub fn new(
        reader: Box<dyn Reader>,
        data_offset: u64,
        data_length: u64,
        spec: Spec,
        fmt: &FmtChunk,
    ) -> Result<Self, AudioError> {
        let wave_sample_type = spec.get_sample_type();
        Ok(Self {
            reader,
            data_offset,
            data_length,
            block_align: fmt.block_align,
            total_frames: data_length / fmt.block_align as u64,
            spec,
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
        let frame_index = match seek_from {
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => (self.get_cur_frame_index()? as i64 + cur) as u64,
            SeekFrom::End(end) => (self.total_frames as i64 + end) as u64,
        };
        if frame_index > self.total_frames {
            self.reader
                .seek(SeekFrom::Start(self.data_offset + self.data_length))?;
            Ok(())
        } else {
            self.reader.seek(SeekFrom::Start(
                self.data_offset + frame_index * self.block_align as u64,
            ))?;
            Ok(())
        }
    }

    fn decode_sample<T>(&mut self) -> Result<Option<S>, AudioReadError>
    where
        T: SampleType,
    {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            Ok(Some(S::scale_from(T::read_le(&mut self.reader)?)))
        }
    }

    fn decode_sample_to<T>(r: &mut dyn Reader) -> Result<S, AudioReadError>
    where
        T: SampleType,
    {
        Ok(S::scale_from(T::read_le(r)?))
    }

    fn decode_samples_to<T>(
        r: &mut dyn Reader,
        num_samples_to_read: usize,
    ) -> Result<Vec<S>, AudioReadError>
    where
        T: SampleType,
    {
        let mut ret = Vec::<S>::with_capacity(num_samples_to_read);
        for _ in 0..num_samples_to_read {
            ret.push(Self::decode_sample_to::<T>(r)?);
        }
        Ok(ret)
    }

    /// ## The decoder returned by this function has two exclusive responsibilities:
    /// 1. Read raw bytes from the input stream.
    /// 2. Convert them into samples of the target format `S`.
    ///
    /// It does NOT handle end-of-stream detection — the caller must implement
    /// termination logic (e.g., check input.is_empty() or external duration tracking).
    #[allow(clippy::type_complexity)]
    fn choose_sample_decoder(
        wave_sample_type: WaveSampleType,
    ) -> Result<fn(&mut dyn Reader) -> Result<S, AudioReadError>, AudioError> {
        use WaveSampleType::{F32, F64, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, Unknown};
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
            Unknown => Err(AudioError::InvalidArguments(format!(
                "unknown sample type \"{:?}\"",
                wave_sample_type
            ))),
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
                }
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some((sample_l, sample_r)))
                }
                o => Err(AudioReadError::Unsupported(format!(
                    "Unsupported to merge {o} channels to 2 channels."
                ))),
            }
        }
    }

    pub fn decode_mono(&mut self) -> Result<Option<S>, AudioReadError> {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.get_channels() {
                1 => Ok(Some((self.sample_decoder)(&mut self.reader)?)),
                2 => {
                    let sample_l = (self.sample_decoder)(&mut self.reader)?;
                    let sample_r = (self.sample_decoder)(&mut self.reader)?;
                    Ok(Some(
                        sample_l / S::scale_from(2) + sample_r / S::scale_from(2),
                    ))
                }
                o => Err(AudioReadError::Unsupported(format!(
                    "Unsupported to merge {o} channels to 1 channels."
                ))),
            }
        }
    }
}

/// ## The `AdpcmDecoderWrap` to decode ADPCM blocks to your specific format samples
#[derive(Debug)]
pub struct AdpcmDecoderWrap<D>
where
    D: adpcm::AdpcmDecoder,
{
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
where
    D: adpcm::AdpcmDecoder,
{
    pub fn new(
        reader: Box<dyn Reader>,
        data_offset: u64,
        data_length: u64,
        fmt: &FmtChunk,
        total_samples: u64,
    ) -> Result<Self, AudioReadError> {
        let decoder = D::new(fmt)?;
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
        if self.reader.stream_position()? >= end_of_data {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_cur_frame_index(&self) -> u64 {
        self.frame_index
    }

    pub fn feed_until_output(&mut self, wanted_length: usize) -> Result<(), AudioReadError> {
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
                self.decoder.decode(
                    || -> Option<u8> { iter.next() },
                    |sample: i16| {
                        sample_decoded += 1;
                        self.samples.push(sample)
                    },
                )?;
            } else {
                self.decoder.flush(|sample: i16| {
                    sample_decoded += 1;
                    self.samples.push(sample)
                })?;
                break;
            }
        }
        self.frames_decoded += sample_decoded / self.channels as u64;
        Ok(())
    }

    pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
        let frames_per_block = self.decoder.frames_per_block() as u64;
        let frame_index = match seek_from {
            SeekFrom::Start(fi) => fi,
            SeekFrom::Current(cur) => (self.frame_index as i64 + cur) as u64,
            SeekFrom::End(end) => (self.total_frames as i64 + end) as u64,
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
    where
        S: SampleType,
    {
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
                        panic!(
                            "Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}",
                            self.frame_index, self.first_frame_of_samples
                        );
                    } else if self.frame_index < self.frames_decoded {
                        let ret =
                            self.samples[(self.frame_index - self.first_frame_of_samples) as usize];
                        self.frame_index += 1;
                        Ok(Some(S::scale_from(ret)))
                    } else {
                        // Need to decode the next block
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
                    Some((l, r)) => Ok(Some(S::average(l, r))),
                }
            }
            o => Err(AudioReadError::Unsupported(format!(
                "Unsupported channels {o}"
            ))),
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where
        S: SampleType,
    {
        match self.channels {
            1 => {
                let ret = self.decode_mono::<S>()?;
                match ret {
                    None => Ok(None),
                    Some(ret) => Ok(Some((ret, ret))),
                }
            }
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
                        panic!(
                            "Unknown error occured when decoding the ADPCM data: the sample cache was updated while the previous cache is needed: FI = {}, FF = {}",
                            self.frame_index, self.first_frame_of_samples
                        );
                    } else if self.frame_index < self.frames_decoded {
                        let index = ((self.frame_index - self.first_frame_of_samples) * 2) as usize;
                        self.frame_index += 1;
                        let l = self.samples[index];
                        let r = self.samples[index + 1];
                        Ok(Some((S::scale_from(l), S::scale_from(r))))
                    } else {
                        // Need to decode the next block
                        self.first_frame_of_samples += (self.samples.len() / 2) as u64;
                        self.samples.clear();
                        self.decode_stereo::<S>()
                    }
                }
            }
            o => Err(AudioReadError::Unsupported(format!(
                "Unsupported channels {o}"
            ))),
        }
    }

    pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
    where
        S: SampleType,
    {
        match self.channels {
            1 => match self.decode_mono::<S>()? {
                Some(sample) => Ok(Some(vec![sample])),
                None => Ok(None),
            },
            2 => match self.decode_stereo::<S>()? {
                Some((l, r)) => Ok(Some(vec![l, r])),
                None => Ok(None),
            },
            o => Err(AudioReadError::Unsupported(format!(
                "Unsupported channels {o}"
            ))),
        }
    }
}

/// ## The `PcmXLawDecoderWrap` to decode aLaw or MuLaw PCM data to your specific format samples
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
    pub fn new(
        reader: Box<dyn Reader>,
        which_law: XLaw,
        data_offset: u64,
        data_length: u64,
        fmt: &FmtChunk,
        total_samples: u64,
    ) -> Result<Self, AudioReadError> {
        match fmt.channels {
            1 => (),
            2 => (),
            o => {
                return Err(AudioReadError::Unsupported(format!(
                    "Unsupported channels {o}"
                )));
            }
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
        self.reader.seek(SeekFrom::Start(
            self.data_offset + self.frame_index * self.channels as u64,
        ))?;
        Ok(())
    }

    fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
        let end_of_data = self.data_offset + self.data_length;
        if self.reader.stream_position()? >= end_of_data {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
    where
        S: SampleType,
    {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.channels {
                1 => {
                    let s = S::scale_from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some(s))
                }
                2 => {
                    let l = S::scale_from(self.decode()?);
                    let r = S::scale_from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some(S::average(l, r)))
                }
                o => Err(AudioReadError::Unsupported(format!(
                    "Unsupported channels {o}"
                ))),
            }
        }
    }

    pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
    where
        S: SampleType,
    {
        if self.is_end_of_data()? {
            Ok(None)
        } else {
            match self.channels {
                1 => {
                    let s = S::scale_from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some((s, s)))
                }
                2 => {
                    let l = S::scale_from(self.decode()?);
                    let r = S::scale_from(self.decode()?);
                    self.frame_index += 1;
                    Ok(Some((l, r)))
                }
                o => Err(AudioReadError::Unsupported(format!(
                    "Unsupported channels {o}"
                ))),
            }
        }
    }

    pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
    where
        S: SampleType,
    {
        match self.channels {
            1 => match self.decode_mono::<S>()? {
                Some(sample) => Ok(Some(vec![sample])),
                None => Ok(None),
            },
            2 => match self.decode_stereo::<S>()? {
                Some((l, r)) => Ok(Some(vec![l, r])),
                None => Ok(None),
            },
            o => Err(AudioReadError::Unsupported(format!(
                "Unsupported channels {o}"
            ))),
        }
    }
}

/// ## The MP3 decoder for `WaveReader`
#[cfg(feature = "mp3dec")]
pub mod mp3 {
    use std::{
        fmt::{self, Debug, Formatter},
        io::{Read, SeekFrom},
        mem,
    };

    use super::get_rounded_up_fft_size;
    use crate::AudioReadError;
    use crate::Reader;
    use crate::Resampler;
    use crate::SampleType;
    use crate::utils;
    use crate::wavcore::FmtChunk;

    use rmp3::{DecoderOwned, Frame};

    /// ## The `Mp3Decoder`, decodes the MP3 file encapsulated in the WAV file.
    pub struct Mp3Decoder {
        target_sample_rate: u32,
        target_channels: u16,
        the_decoder: DecoderOwned<Vec<u8>>,
        cur_frame: Option<Mp3AudioData>,
        sample_pos: u64,
        total_frames: u64,
        resampler: Resampler,
    }

    impl Debug for Mp3Decoder {
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

    /// ## The `Mp3AudioData` for a MP3 frame.
    /// **NOTE:** Some people like to concat MP3 files savagely just like concat binary files, and the MP3 format actually supports this kind of operation.
    /// If the concat MP3 files have different sample rates, this will cause the sample rate to change while you are just normally parsing and decoding the MP3 file.
    /// This can be done by using a resampler here but I personally don't want to support this variable sample rate audio file.
    /// The `resampler` crate is here, ready to use, if you want to support variable sample rate audio files, create a pull request from your repo.
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

    impl Debug for Mp3AudioData {
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
        pub fn new(
            reader: Box<dyn Reader>,
            data_offset: u64,
            data_length: u64,
            fmt: &FmtChunk,
            total_samples: u64,
        ) -> Result<Self, AudioReadError> {
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
                resampler: Resampler::new(get_rounded_up_fft_size(fmt.sample_rate)),
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
            let process_size = self.resampler.get_process_size(
                self.resampler.get_fft_size(),
                src_sample_rate,
                self.target_sample_rate,
            );
            let mut monos = utils::interleaved_samples_to_monos(samples, channels).unwrap();
            for mono in monos.iter_mut() {
                let mut iter = mem::take(mono).into_iter();
                loop {
                    let block: Vec<i16> = iter.by_ref().take(process_size).collect();
                    if block.is_empty() {
                        break;
                    }
                    mono.extend(&utils::do_resample_mono(
                        &self.resampler,
                        &block,
                        src_sample_rate,
                        self.target_sample_rate,
                    ));
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

                    let mut ret = Mp3AudioData {
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
                            ret.samples = mem::take(&mut ret.samples)
                                .into_iter()
                                .flat_map(|s| -> Vec<i16> { vec![s; t as usize] })
                                .collect();
                            ret.channels = self.target_channels;
                        }
                        (t, 1) => {
                            let mut iter = mem::take(&mut ret.samples).into_iter();
                            loop {
                                let frame: Vec<i32> =
                                    iter.by_ref().take(t as usize).map(|s| s as i32).collect();
                                if frame.is_empty() {
                                    break;
                                }
                                ret.samples
                                    .push((frame.iter().sum::<i32>() / frame.len() as i32) as i16);
                            }
                            ret.channels = self.target_channels;
                        }
                        (s, t) => {
                            if s != t {
                                eprintln!("Can't change {s} channels to {t} channels.");
                            }
                        }
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
            let frame_index = match seek_from {
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(cur) => (self.get_cur_frame_index() as i64 + cur) as u64,
                SeekFrom::End(end) => (self.total_frames as i64 + end) as u64,
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
                    return Ok(());
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
                Some(ref mut frame) => match frame.channels {
                    1 => {
                        let sample = frame.samples[frame.buffer_index];
                        frame.buffer_index += 1;
                        if frame.buffer_index >= frame.sample_count {
                            self.cur_frame = self.get_next_frame();
                        }
                        Ok(Some(sample))
                    }
                    2 => {
                        let l = frame.samples[frame.buffer_index * 2];
                        let r = frame.samples[frame.buffer_index * 2 + 1];
                        frame.buffer_index += 1;
                        if frame.buffer_index >= frame.sample_count {
                            self.cur_frame = self.get_next_frame();
                        }
                        Ok(Some(((l as i32 + r as i32) / 2i32) as i16))
                    }
                    o => Err(AudioReadError::DataCorrupted(format!(
                        "Unknown channel count {o}."
                    ))),
                },
            }
        }

        pub fn decode_stereo_raw(&mut self) -> Result<Option<(i16, i16)>, AudioReadError> {
            match self.cur_frame {
                None => Ok(None),
                Some(ref mut frame) => match frame.channels {
                    1 => {
                        let sample = frame.samples[frame.buffer_index];
                        frame.buffer_index += 1;
                        if frame.buffer_index >= frame.sample_count {
                            self.cur_frame = self.get_next_frame();
                        }
                        Ok(Some((sample, sample)))
                    }
                    2 => {
                        let l = frame.samples[frame.buffer_index * 2];
                        let r = frame.samples[frame.buffer_index * 2 + 1];
                        frame.buffer_index += 1;
                        if frame.buffer_index >= frame.sample_count {
                            self.cur_frame = self.get_next_frame();
                        }
                        Ok(Some((l, r)))
                    }
                    o => Err(AudioReadError::DataCorrupted(format!(
                        "Unknown channel count {o}."
                    ))),
                },
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where
            S: SampleType,
        {
            match self.decode_mono_raw()? {
                None => Ok(None),
                Some(s) => Ok(Some(S::scale_from(s))),
            }
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where
            S: SampleType,
        {
            match self.decode_stereo_raw()? {
                None => Ok(None),
                Some((l, r)) => Ok(Some((S::scale_from(l), S::scale_from(r)))),
            }
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where
            S: SampleType,
        {
            let stereo = self.decode_stereo::<S>()?;
            match stereo {
                None => Ok(None),
                Some((l, r)) => match self.target_channels {
                    1 => Ok(Some(vec![S::scale_from(l)])),
                    2 => Ok(Some(vec![S::scale_from(l), S::scale_from(r)])),
                    o => Err(AudioReadError::DataCorrupted(format!(
                        "Unknown channel count {o}."
                    ))),
                },
            }
        }
    }
}

/// ## The Opus decoder for `WaveReader`
#[cfg(feature = "opus")]
pub mod opus {
    use std::{
        fmt::{self, Debug, Formatter},
        io::SeekFrom,
    };

    use crate::AudioReadError;
    use crate::Reader;
    use crate::SampleType;
    use crate::wavcore::FmtChunk;

    use opus::{Channels, Decoder};

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
    }

    impl OpusDecoder {
        pub fn new(
            mut reader: Box<dyn Reader>,
            data_offset: u64,
            data_length: u64,
            fmt: &FmtChunk,
            total_samples: u64,
        ) -> Result<Self, AudioReadError> {
            let channels = fmt.channels;
            let sample_rate = fmt.sample_rate;
            let opus_channels = match channels {
                1 => Channels::Mono,
                2 => Channels::Stereo,
                o => {
                    return Err(AudioReadError::InvalidArguments(format!(
                        "Bad channels: {o} for the opus decoder."
                    )));
                }
            };
            let decoder = Decoder::new(sample_rate, opus_channels)?;
            reader.seek(SeekFrom::Start(data_offset))?;
            Ok(Self {
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
            if self.reader.stream_position()? >= self.data_offset + self.data_length {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        fn clear_decoded_samples_buffer(&mut self) {
            self.decoded_samples.clear();
            self.decoded_samples_index = 0;
        }

        fn get_samples_per_block(&self) -> usize {
            self.block_align
        }

        fn decode_block(&mut self) -> Result<(), AudioReadError> {
            if self.is_end_of_data()? {
                self.clear_decoded_samples_buffer();
                return Ok(());
            }

            // Prepare the buffers
            let mut buf = vec![0u8; self.block_align];
            let samples_to_get = self.get_samples_per_block();
            self.reader.read_exact(&mut buf)?;
            self.decoded_samples = vec![0.0; samples_to_get];

            // Reset the sample index
            self.decoded_samples_index = 0;

            // Perform the decode call
            let frames =
                self.decoder
                    .decode_float(&buf, &mut self.decoded_samples, /*fec*/ false)?;

            // Check out the result
            let samples = frames * self.channels as usize;
            if samples != samples_to_get {
                Err(AudioReadError::IncompleteData(format!(
                    "Expected {samples_to_get} samples will be decoded, got {samples} samples."
                )))
            } else {
                Ok(())
            }
        }

        pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
            let frame_index = match seek_from {
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(cur) => (self.frame_index as i64 + cur) as u64,
                SeekFrom::End(end) => (self.total_frames as i64 + end) as u64,
            };
            self.frame_index = frame_index;
            let block_align = self.block_align as u64;
            let block_index = frame_index / block_align;
            let seek_to = self.data_offset + block_index * block_align;
            self.reader.seek(SeekFrom::Start(seek_to))?;
            if seek_to < self.data_offset + self.data_length {
                self.decode_block()?;
                self.decoded_samples_index = ((frame_index * self.channels as u64)
                    - block_index * self.get_samples_per_block() as u64)
                    as usize;
            } else {
                self.clear_decoded_samples_buffer();
            }
            Ok(())
        }

        fn decode_sample<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where
            S: SampleType,
        {
            if self.decoded_samples_index >= self.decoded_samples.len() {
                self.decode_block()?;
            }
            if self.decoded_samples.is_empty() {
                Ok(None)
            } else {
                let ret = S::scale_from(self.decoded_samples[self.decoded_samples_index]);
                self.decoded_samples_index += 1;
                Ok(Some(ret))
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where
            S: SampleType,
        {
            let frame: Result<Vec<Option<S>>, AudioReadError> = (0..self.channels)
                .map(|_| self.decode_sample::<S>())
                .collect();
            match frame {
                Ok(frame) => {
                    let frame: Vec<S> = frame.into_iter().flatten().collect();
                    if frame.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(S::average_arr(&frame)))
                    }
                }
                Err(e) => Err(e),
            }
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where
            S: SampleType,
        {
            match self.channels {
                1 => {
                    if let Some(s) = self.decode_sample::<S>()? {
                        self.frame_index += 1;
                        Ok(Some((s, s)))
                    } else {
                        Ok(None)
                    }
                }
                2 => {
                    let l = self.decode_sample::<S>()?;
                    let r = self.decode_sample::<S>()?;
                    if l.is_some() || r.is_some() {
                        self.frame_index += 1;
                        Ok(Some((l.unwrap(), r.unwrap())))
                    } else {
                        Ok(None)
                    }
                }
                o => Err(AudioReadError::DataCorrupted(format!(
                    "Can't convert {o} channel audio to stereo channel audio"
                ))),
            }
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where
            S: SampleType,
        {
            let frame: Result<Vec<Option<S>>, AudioReadError> = (0..self.channels)
                .map(|_| self.decode_sample::<S>())
                .collect();
            match frame {
                Ok(frame) => {
                    let frame: Vec<S> = frame.into_iter().flatten().collect();
                    if frame.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(frame))
                    }
                }
                Err(e) => Err(e),
            }
        }
    }

    impl Debug for OpusDecoder {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            f.debug_struct("OpusDecoder")
                .field("reader", &self.reader)
                .field("decoder", &self.decoder)
                .field("channels", &self.channels)
                .field("sample_rate", &self.sample_rate)
                .field("data_offset", &self.data_offset)
                .field("data_length", &self.data_length)
                .field("total_frames", &self.total_frames)
                .field("block_align", &self.block_align)
                .field(
                    "decoded_samples",
                    &format_args!("[f32; {}]", self.decoded_samples.len()),
                )
                .field("decoded_samples_index", &self.decoded_samples_index)
                .field("frame_index", &self.frame_index)
                .finish()
        }
    }
}

/// ## The FLAC decoder for `WaveReader`
#[cfg(feature = "flac")]
pub mod flac_dec {
    use std::{
        cmp::Ordering,
        collections::BTreeMap,
        fmt::{self, Debug, Formatter},
        io::{self, Read, Seek, SeekFrom},
        ptr,
    };

    use super::get_rounded_up_fft_size;
    use crate::AudioReadError;
    use crate::Reader;
    use crate::SampleType;
    use crate::readwrite::ReadBridge;
    use crate::utils::{do_resample_frames, sample_conv, sample_conv_batch};
    use crate::wavcore::{FmtChunk, ListChunk, ListInfo, get_listinfo_flacmeta};
    use flac::{
        FlacAudioForm, FlacDecoderUnmovable, FlacInternalDecoderError, FlacReadStatus, SamplesInfo,
    };
    use resampler::Resampler;

    pub struct FlacDecoderWrap<'a> {
        reader: Box<dyn Reader>,
        decoder: Box<FlacDecoderUnmovable<'a, ReadBridge<'a>>>,
        resampler: Resampler,
        channels: u16,
        sample_rate: u32,
        data_offset: u64,
        data_length: u64,
        decoded_frames: Vec<Vec<i32>>,
        decoded_frames_index: usize,
        frame_index: u64,
        total_frames: u64,
        self_ptr: Box<*mut FlacDecoderWrap<'a>>,
    }

    impl FlacDecoderWrap<'_> {
        pub fn new(
            reader: Box<dyn Reader>,
            data_offset: u64,
            data_length: u64,
            fmt: &FmtChunk,
            total_samples: u64,
        ) -> Result<Self, AudioReadError> {
            // `self_ptr`: A boxed raw pointer points to the `FlacDecoderWrap`, before calling `decoder.decode()`, must set the pointer inside the box to `self`
            let mut self_ptr: Box<*mut Self> = Box::new(ptr::null_mut());
            let self_ptr_ptr = (&mut *self_ptr) as *mut *mut Self;
            let reader_ptr = Box::into_raw(reader); // On the fly reader
            let decoder = Box::new(FlacDecoderUnmovable::new(
                ReadBridge::new(unsafe { &mut *reader_ptr }),
                // on_read
                Box::new(
                    move |reader: &mut ReadBridge, buffer: &mut [u8]| -> (usize, FlacReadStatus) {
                        let to_read = buffer.len();
                        match reader.read(buffer) {
                            Ok(size) => match size.cmp(&to_read) {
                                Ordering::Equal => (size, FlacReadStatus::GoOn),
                                Ordering::Less => (size, FlacReadStatus::Eof),
                                Ordering::Greater => panic!(
                                    "`reader.read()` returns a size greater than the desired size."
                                ),
                            },
                            Err(e) => {
                                eprintln!("on_read(): {:?}", e);
                                (0, FlacReadStatus::Abort)
                            }
                        }
                    },
                ),
                // on_seek
                Box::new(
                    move |reader: &mut ReadBridge, position: u64| -> Result<(), io::Error> {
                        reader.seek(SeekFrom::Start(data_offset + position))?;
                        Ok(())
                    },
                ),
                // on_tell
                Box::new(move |reader: &mut ReadBridge| -> Result<u64, io::Error> {
                    Ok(reader.stream_position()? - data_offset)
                }),
                // on_length
                Box::new(move |_reader: &mut ReadBridge| -> Result<u64, io::Error> {
                    Ok(data_length)
                }),
                // on_eof
                Box::new(move |reader: &mut ReadBridge| -> bool {
                    reader.stream_position().unwrap() >= data_offset + data_length
                }),
                // on_write
                Box::new(
                    move |frames: &[Vec<i32>],
                          sample_info: &SamplesInfo|
                          -> Result<(), io::Error> {
                        // Before `on_write()` was called, make sure `self_ptr` was updated to the `self` pointer of `FlacDecoderWrap`
                        let this = unsafe { &mut *(*self_ptr_ptr).cast::<Self>() };
                        this.decoded_frames_index = 0;
                        if sample_info.sample_rate != this.sample_rate {
                            this.decoded_frames.clear();
                            let process_size = this.resampler.get_process_size(
                                this.resampler.get_fft_size(),
                                sample_info.sample_rate,
                                this.sample_rate,
                            );
                            let mut iter = frames.iter();
                            loop {
                                let block: Vec<Vec<i32>> =
                                    iter.by_ref().take(process_size).cloned().collect();
                                if block.is_empty() {
                                    break;
                                }
                                this.decoded_frames.extend(
                                    sample_conv_batch(&do_resample_frames(
                                        &this.resampler,
                                        &block,
                                        sample_info.sample_rate,
                                        this.sample_rate,
                                    ))
                                    .to_vec(),
                                );
                            }
                            this.decoded_frames.shrink_to_fit();
                        } else {
                            this.decoded_frames = frames.to_vec();
                        }

                        Ok(())
                    },
                ),
                // on_error
                Box::new(move |error: FlacInternalDecoderError| {
                    eprintln!("on_error({error})");
                }),
                true, // md5_checking
                true, // scale_to_i32_range
                FlacAudioForm::FrameArray,
            )?);
            let mut ret = Self {
                reader: unsafe { Box::from_raw(reader_ptr) },
                decoder,
                resampler: Resampler::new(get_rounded_up_fft_size(fmt.sample_rate)),
                channels: fmt.channels,
                sample_rate: fmt.sample_rate,
                data_offset,
                data_length,
                decoded_frames: Vec::<Vec<i32>>::new(),
                decoded_frames_index: 0,
                frame_index: 0,
                total_frames: total_samples / fmt.channels as u64,
                self_ptr,
            };
            *ret.self_ptr = &mut ret as *mut Self;
            ret.decoder.initialize()?;
            Ok(ret)
        }

        fn is_end_of_data(&mut self) -> Result<bool, AudioReadError> {
            Ok(self.decoder.eof())
        }

        fn clear_decoded_frames(&mut self) {
            self.decoded_frames.clear();
            self.decoded_frames_index = 0;
        }

        fn decode_block(&mut self) -> Result<(), AudioReadError> {
            if self.is_end_of_data()? {
                self.clear_decoded_frames();
                Ok(())
            } else {
                // When to decode, the FLAC decoder will call our callback functions, then our closures will be called.
                // These closures captured the address of the boxed `self_ptr`, and will use the pointer to find `self`
                *self.self_ptr = self as *mut Self;
                self.decoder.decode()?;
                Ok(())
            }
        }

        pub fn get_metadata_as_list(&self) -> Result<ListChunk, AudioReadError> {
            let comments = self.decoder.get_comments();
            let mut listinfo = ListChunk::Info(BTreeMap::<String, String>::new());

            for (list_key, flac_key) in get_listinfo_flacmeta().iter() {
                if let Some(data) = comments.get(flac_key.to_owned()) {
                    listinfo.set(list_key, data)?;
                }
            }

            Ok(listinfo)
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

        pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
            let frame_index = match seek_from {
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(cur) => (self.frame_index as i64 + cur) as u64,
                SeekFrom::End(end) => (self.total_frames as i64 + end) as u64,
            };
            self.clear_decoded_frames();
            self.frame_index = frame_index;
            self.decoder.seek(frame_index)?;

            Ok(())
        }

        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where
            S: SampleType,
        {
            if self.is_end_of_data()? {
                Ok(None)
            } else if self.decoded_frames_index < self.decoded_frames.len() {
                let ret = sample_conv(&self.decoded_frames[self.decoded_frames_index]);
                self.decoded_frames_index += 1;
                self.frame_index += 1;
                Ok(Some(ret.to_vec()))
            } else {
                self.decode_block()?;
                self.decoded_frames_index = 0;
                self.decode_frame::<S>()
            }
        }

        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where
            S: SampleType,
        {
            if let Some(frame) = self.decode_frame::<S>()? {
                match frame.len() {
                    1 => Ok(Some((frame[0], frame[0]))),
                    2 => Ok(Some((frame[0], frame[1]))),
                    o => Err(AudioReadError::Unsupported(format!(
                        "Unsupported to merge {o} channels to 2 channels."
                    ))),
                }
            } else {
                Ok(None)
            }
        }

        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where
            S: SampleType,
        {
            if let Some(frame) = self.decode_frame::<S>()? {
                Ok(Some(S::average_arr(&frame)))
            } else {
                Ok(None)
            }
        }
    }

    impl Debug for FlacDecoderWrap<'_> {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            f.debug_struct("FlacDecoderWrap")
                .field("reader", &self.reader)
                .field("decoder", &self.decoder)
                .field("resampler", &self.resampler)
                .field("channels", &self.channels)
                .field("sample_rate", &self.sample_rate)
                .field("data_offset", &self.data_offset)
                .field("data_length", &self.data_length)
                .field(
                    "decoded_frames",
                    &format_args!("[i32; {}]", self.decoded_frames.len()),
                )
                .field("decoded_frames_index", &self.decoded_frames_index)
                .field("frame_index", &self.frame_index)
                .field("total_frames", &self.total_frames)
                .field("self_ptr", &self.self_ptr)
                .finish()
        }
    }
}

/// ## The OggVorbis decoder for `WaveReader`
#[cfg(feature = "oggvorbis")]
pub mod oggvorbis_dec {
    use std::{
        fmt::{self, Debug, Formatter},
        io::SeekFrom,
    };

    use crate::AudioReadError;
    use crate::SampleType;
    use crate::hacks;
    use crate::wavcore::{FmtChunk, ExtensionData};
    use crate::{Reader, SharedReader};
    use vorbis_rs::VorbisDecoder;

    /// ## The OggVorbis decoder for `WaveReader`
    pub struct OggVorbisDecoderWrap<'a> {
        /// The shared reader for the decoder to use
        reader: SharedReader<'a>,

        /// Because the reader of the shared reader was borrowed from somewhere, let the `reader_holder` lend the reader for it.
        reader_holder: Box<dyn Reader>,

        /// Let the decoder use the shared reader. Because the decoder did't need the reader to implement the `Seek` trait, we can control where to let it decode from.
        decoder: VorbisDecoder<SharedReader<'a>>,

        /// The data offset of the OggVorbis data in the WAV file.
        data_offset: u64,

        /// The size of the data
        data_length: u64,

        /// How many audio frames in total
        total_frames: u64,

        /// Channels, it seems that OggVorbis supports up to 8 channels
        channels: u16,

        /// Sample rate
        sample_rate: u32,

        /// The decoded samples as waveform arrays. If the decoder hits the end of the file, this field is set to `None`
        decoded_samples: Option<Vec<Vec<f32>>>,

        /// Current frame index
        cur_frame_index: u64,

        /// Current block frame index. The start index of the decoded samples.
        cur_block_frame_index: u64,

        /// Ogg Vorbis header for `OggVorbisMode::HaveIndependentHeader`
        ogg_vorbis_header: Vec<u8>,
    }

    impl OggVorbisDecoderWrap<'_> {
        pub fn new(
            reader: Box<dyn Reader>,
            data_offset: u64,
            data_length: u64,
            fmt: &FmtChunk,
            total_samples: u64,
        ) -> Result<Self, AudioReadError> {
            use crate::wavcore::format_tags::*;
            let mut reader_holder = reader;
            let reader = SharedReader::new(hacks::force_borrow_mut!(*reader_holder, dyn Reader));
            let decoder = VorbisDecoder::new(reader.clone())?;
            let channels = decoder.channels().get() as u16;
            let sample_rate = decoder.sampling_frequency().get();
            let mut ret = Self {
                reader,
                reader_holder,
                decoder,
                data_offset,
                data_length,
                total_frames: total_samples / fmt.channels as u64,
                channels,
                sample_rate,
                decoded_samples: None,
                cur_frame_index: 0,
                cur_block_frame_index: 0,
                ogg_vorbis_header: if let Some(extension) = &fmt.extension {
                    match &extension.data {
                        ExtensionData::OggVorbis(_) => {
                            if [
                                FORMAT_TAG_OGG_VORBIS1,
                                FORMAT_TAG_OGG_VORBIS1P,
                                FORMAT_TAG_OGG_VORBIS3,
                                FORMAT_TAG_OGG_VORBIS3P,
                            ].contains(&fmt.format_tag) {
                                Vec::new()
                            } else {
                                return Err(AudioReadError::FormatError(format!("For `format_tag` is `FORMAT_TAG_OGG_VORBIS2` or `FORMAT_TAG_OGG_VORBIS2P`, the `fmt ` chunk must provide the Ogg Vorbis header data.")));
                            }
                        }
                        ExtensionData::OggVorbisWithHeader(data) => {
                            if [
                                FORMAT_TAG_OGG_VORBIS2,
                                FORMAT_TAG_OGG_VORBIS2P,
                            ].contains(&fmt.format_tag) {
                                data.header.clone()
                            } else {
                                return Err(AudioReadError::FormatError(format!("The extension data of the `fmt ` chunk provides the Ogg Vorbis header data, but the `format_tag` value indicates that there shouldn't need to be any Ogg Vorbis header data in the `fmt ` chunk.")));
                            }
                        }
                        o => return Err(AudioReadError::FormatError(format!("The extension data type is not for Ogg Vorbis, it is {:?}", o))),
                    }
                } else {
                    Vec::new()
                }
            };
            assert_eq!(fmt.channels, ret.channels);
            assert_eq!(fmt.sample_rate, ret.sample_rate);
            std::hint::black_box(&mut ret);
            ret.decode()?;
            Ok(ret)
        }

        fn cur_block_frames(&self) -> usize {
            match self.decoded_samples {
                None => 0,
                Some(ref samples) => samples[0].len(),
            }
        }

        fn decode(&mut self) -> Result<(), AudioReadError> {
            self.cur_block_frame_index += self.cur_block_frames() as u64;
            self.cur_frame_index = self.cur_block_frame_index;
            self.decoded_samples = self.decoder.decode_audio_block()?.map(|samples| {
                samples
                    .samples()
                    .iter()
                    .map(|frame| frame.to_vec())
                    .collect()
            });
            Ok(())
        }

        /// Get how many channels in the OggVorbis audio data
        pub fn get_channels(&self) -> u16 {
            self.channels
        }

        /// Get the current decoding audio frame index. The audio frame is an array for all channels' one sample.
        pub fn get_cur_frame_index(&self) -> u64 {
            self.cur_frame_index
        }

        /// Seek to the block that contains the specific frame index of the audio frame.
        pub fn seek(&mut self, seek_from: SeekFrom) -> Result<(), AudioReadError> {
            let frame_index = match seek_from {
                SeekFrom::Start(fi) => fi,
                SeekFrom::Current(ci) => (self.cur_frame_index as i64 + ci) as u64,
                SeekFrom::End(ei) => (self.cur_frame_index as i64 + ei) as u64,
            };
            if frame_index < self.cur_block_frame_index {
                self.reader_holder.seek(SeekFrom::Start(self.data_offset))?;
                self.cur_block_frame_index = 0;
            }
            self.cur_frame_index = frame_index;
            while self.cur_block_frame_index + (self.cur_block_frames() as u64)
                < self.cur_frame_index
            {
                self.decode()?;
                if self.decoded_samples.is_none() {
                    return Ok(());
                }
            }
            Ok(())
        }

        /// Decode as audio frames. The audio frame is an array for all channels' one sample.
        pub fn decode_frame<S>(&mut self) -> Result<Option<Vec<S>>, AudioReadError>
        where
            S: SampleType,
        {
            match self.decoded_samples {
                None => Ok(None),
                Some(ref samples) => {
                    let cache_frame_index =
                        (self.cur_frame_index - self.cur_block_frame_index) as usize;
                    if cache_frame_index < samples[0].len() {
                        let ret: Vec<S> = (0..self.channels)
                            .map(|channel| {
                                S::scale_from(samples[channel as usize][cache_frame_index])
                            })
                            .collect();
                        self.cur_frame_index += 1;
                        Ok(Some(ret))
                    } else {
                        self.decode()?;
                        self.decode_frame()
                    }
                }
            }
        }

        /// Decode as stereo audio
        pub fn decode_stereo<S>(&mut self) -> Result<Option<(S, S)>, AudioReadError>
        where
            S: SampleType,
        {
            match self.decode_frame()? {
                None => Ok(None),
                Some(frame) => match frame.len() {
                    1 => Ok(Some((frame[0], frame[0]))),
                    2 => Ok(Some((frame[0], frame[1]))),
                    o => Err(AudioReadError::Unsupported(format!(
                        "Could not convert {o} channel audio to stereo audio."
                    ))),
                },
            }
        }

        /// Decode as mono audio. All channel samples will be mixed into one.
        pub fn decode_mono<S>(&mut self) -> Result<Option<S>, AudioReadError>
        where
            S: SampleType,
        {
            match self.decode_frame()? {
                None => Ok(None),
                Some(frame) => Ok(Some(S::average_arr(&frame))),
            }
        }
    }

    impl Debug for OggVorbisDecoderWrap<'_> {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            f.debug_struct("OggVorbisDecoderWrap")
                .field("reader", &self.reader)
                .field("reader_holder", &self.reader_holder)
                .field("decoder", &format_args!("VorbisDecoder<Reader>"))
                .field("data_offset", &self.data_offset)
                .field("data_length", &self.data_length)
                .field("total_frames", &self.total_frames)
                .field("channels", &self.channels)
                .field("sample_rate", &self.sample_rate)
                .field(
                    "decoded_samples",
                    &match self.decoded_samples {
                        None => "None".to_string(),
                        Some(_) => format!("Some([f32; {}])", self.cur_block_frames()),
                    },
                )
                .field("cur_frame_index", &self.cur_frame_index)
                .field("cur_block_frame_index", &self.cur_block_frame_index)
                .finish()
        }
    }
}
