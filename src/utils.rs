#![allow(dead_code)]

use std::{any::TypeId, borrow::Cow, slice};

use crate::AudioWriteError;
use crate::SampleType;
use crate::Resampler;

/// * Turns a stereo audio into two individual mono waveforms.
pub fn stereos_to_dual_monos<S>(stereos: &[(S, S)]) -> (Vec<S>, Vec<S>)
where S: SampleType {
    let l = stereos.iter().map(|(l, _r): &(S, S)| -> S {*l}).collect();
    let r = stereos.iter().map(|(_l, r): &(S, S)| -> S {*r}).collect();
    (l, r)
}

/// * Check every element in the data has the same length. The length will be returned.
pub fn is_same_len<S>(data: &[Vec<S>]) -> Option<(bool, usize)> {
    if data.is_empty() {
        None
    } else {
        let lengths: Vec<usize> = data.iter().map(|item| item.len()).collect();
        let first = lengths[0];
        Some((lengths.iter().all(|&item| item == first), first))
    }
}

/// * Convert audio frames into stereo audio. Mono audio will be converted to stereo by duplicating samples. Only support 1 or 2 channels of audio.
pub fn frames_to_stereos<S>(frames: &[Vec<S>]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    match is_same_len(frames) {
        None => Ok(Vec::<(S, S)>::new()),
        Some((equal, channels)) => {
            match equal {
                false => Err(AudioWriteError::FrameChannelsNotSame),
                true => {
                    match channels {
                        1 => Ok(frames.iter().map(|frame: &Vec<S>| -> (S, S) {(frame[0], frame[0])}).collect()),
                        2 => Ok(frames.iter().map(|frame: &Vec<S>| -> (S, S) {(frame[0], frame[1])}).collect()),
                        o => Err(AudioWriteError::WrongChannels(format!("{o}"))),
                    }
                }
            }
        }
    }
}

/// * Convert audio frames into two individual mono waveforms. Only support two-channel audio.
pub fn frames_to_dual_mono<S>(frames: &[Vec<S>]) -> Result<(Vec<S>, Vec<S>), AudioWriteError>
where S: SampleType {
    Ok(stereos_to_dual_monos(&frames_to_stereos(frames)?))
}

/// * Convert audio frames into every individual mono waveform. Support any channels.
/// * The param `channels` is optional, if you provide it, the conversion will be a little faster than if you just give it a `None.`
pub fn frames_to_monos<S>(frames: &[Vec<S>], channels: Option<u16>) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    match is_same_len(frames) {
        None => Ok(Vec::<Vec<S>>::new()),
        Some((equal, length)) => {
            match equal {
                false => Err(AudioWriteError::FrameChannelsNotSame),
                true => {
                    if let Some(channels) = channels {
                        if channels as usize != length {
                            return Err(AudioWriteError::WrongChannels(format!("The channels is {channels} but the frames are {length} channels")));
                        }
                    }
                    Ok((0..length).map(|channel| -> Vec<S> {frames.iter().map(|frame: &Vec<S>| -> S {frame[channel]}).collect()}).collect())
                }
            }
        }
    }
}

/// * Convert every individual mono waveform into an audio frame array. Support any channels.
pub fn monos_to_frames<S>(monos: &[Vec<S>]) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    match is_same_len(monos) {
        None => Ok(Vec::<Vec<S>>::new()),
        Some((equal, length)) => {
            match equal {
                false => Err(AudioWriteError::MultipleMonosAreNotSameSize),
                true => {
                    Ok((0..length).map(|position: usize| -> Vec<S> {monos.iter().map(|channel: &Vec<S>| -> S {channel[position]}).collect()}).collect())
                }
            }
        }
    }
}

/// * Convert every individual mono waveform into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn monos_to_interleaved_samples<S>(monos: &[Vec<S>]) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    Ok(monos_to_frames(monos)?.into_iter().flatten().collect())
}

/// * Convert audio frames into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn frames_to_interleaved_samples<S>(frames: &[Vec<S>], channels: Option<u16>) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    monos_to_interleaved_samples(&frames_to_monos(frames, channels)?)
}
/// * Convert stereo audio into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn stereos_to_interleaved_samples<S>(stereos: &[(S, S)]) -> Vec<S>
where S: SampleType {
    stereos.iter().flat_map(|(l, r): &(S, S)| -> [S; 2] {[*l, *r]}).collect()
}

/// * Convert interleaved samples into individual mono waveforms by the specified channels.
pub fn interleaved_samples_to_monos<S>(samples: &[S], channels: u16) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    if channels == 0 {
        Err(AudioWriteError::InvalidArguments("Channels must not be zero".to_owned()))
    } else {
        Ok((0..channels).map(|channel| -> Vec<S> {samples.iter().skip(channel as usize).step_by(channels as usize).copied().collect()}).collect())
    }
}

/// * Convert two individual mono waveforms into a stereo audio form.
pub fn dual_monos_to_stereos<S>(dual_monos: &(Vec<S>, Vec<S>)) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    let (l, r) = dual_monos;
    if l.len() != r.len() {
        Err(AudioWriteError::MultipleMonosAreNotSameSize)
    } else {
        Ok(l.iter().zip(r).map(|(l, r): (&S, &S)| -> (S, S) {(*l, *r)}).collect())
    }
}

/// * Convert interleaved samples into a stereo audio form. The interleaved samples are treated as a two-channel audio.
pub fn interleaved_samples_to_stereos<S>(samples: &[S]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    if (samples.len() & 1) != 0 {
        Err(AudioWriteError::NotStereo)
    } else {
        Ok((0..(samples.len() / 2)).map(|position| -> (S, S) {(samples[position * 2], samples[position * 2 + 1])}).collect())
    }
}

/// * Convert two individual mono waveforms into one mono waveform. Stereo to mono conversion.
pub fn dual_monos_to_monos<S>(dual_monos: &(Vec<S>, Vec<S>)) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    let (l, r) = dual_monos;
    if l.len() != r.len() {
        Err(AudioWriteError::MultipleMonosAreNotSameSize)
    } else {
        Ok(l.iter().zip(r).map(|(l, r): (&S, &S)| -> S {S::average(*l, *r)}).collect())
    }
}

/// * Convert a mono waveform to two individual mono waveforms by duplication. Mono to stereo conversion.
pub fn monos_to_dual_monos<S>(monos: &[S]) -> (Vec<S>, Vec<S>)
where S: SampleType {
    (monos.to_vec(), monos.to_vec())
}

/// * Convert stereo audio to a mono waveform. Stereo to mono conversion.
pub fn stereos_to_monos<S>(stereos: &[(S, S)]) -> Vec<S>
where S: SampleType {
    stereos.iter().map(|(l, r): &(S, S)| -> S {S::average(*l, *r)}).collect()
}

/// * Convert mono waveform to a stereo audio form by duplication. Mono to stereo conversion.
pub fn monos_to_stereos<S>(monos: &[S]) -> Vec<(S, S)>
where S: SampleType {
    monos.iter().map(|s|{(*s, *s)}).collect()
}

/// * Convert one stereo sample to another format by scaling, see `sample_conv()`.
#[inline(always)]
pub fn stereo_conv<S, D>(frame: (S, S)) -> (D, D)
where S: SampleType,
      D: SampleType {
    let (l, r) = frame;
    (D::scale_from(l), D::scale_from(r))
}

/// * Convert samples to another format by scaling. e.g. `u8` to `i16` conversion is to scale `[0, 255]` into `[-32768, +32767]`
/// * Upscaling is lossless. Beware, the precision of `f32` is roughly the same as `i24`. Convert `i32` to `f32` is lossy.
/// * `i32` to `f64` is lossless but `f64` for audio processing consumes lots of memory.
pub fn sample_conv<'a, S, D>(frame: &'a [S]) -> Cow<'a, [D]>
where S: SampleType,
      D: SampleType {

    if TypeId::of::<S>() == TypeId::of::<D>() {
        Cow::Borrowed(unsafe{slice::from_raw_parts(frame.as_ptr() as *const D, frame.len())})
    } else {
        Cow::Owned(frame.iter().map(|sample: &S| -> D {D::scale_from(*sample)}).collect())
    }
}

/// * Convert multiple stereo samples to another format by scaling, see `sample_conv()`.
pub fn stereos_conv<'a, S, D>(stereos: &'a [(S, S)]) -> Cow<'a, [(D, D)]>
where S: SampleType,
      D: SampleType {

    if TypeId::of::<S>() == TypeId::of::<D>() {
        Cow::Borrowed(unsafe{slice::from_raw_parts(stereos.as_ptr() as *const (D, D), stereos.len())})
    } else {
        Cow::Owned(stereos.iter().map(|stereo: &(S, S)| -> (D, D) {stereo_conv(*stereo)}).collect())
    }
}

/// * Convert 2D audio e.g. Audio frames or multiple mono waveforms, to another format by scaling, see `sample_conv()`.
pub fn sample_conv_batch<'a, S, D>(frames: &[Vec<S>]) ->  Cow<'a, [Vec<D>]>
where S: SampleType,
      D: SampleType {

    if TypeId::of::<S>() == TypeId::of::<D>() {
        Cow::Borrowed(unsafe{slice::from_raw_parts(frames.as_ptr() as *const Vec<D>, frames.len())})
    } else {
        Cow::Owned(frames.iter().map(|frames: &Vec<S>| -> Vec<D> {sample_conv(frames).to_vec()}).collect())
    }
}

/// * Use the `Resampler` to resample a mono waveform from original sample rate to a specific sample rate.
pub fn do_resample_mono<S>(resampler: &Resampler, input: &[S], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<S>
where S: SampleType {
    let input = sample_conv::<S, f32>(input);
    let result = resampler.resample(&input, src_sample_rate, dst_sample_rate).unwrap();
    sample_conv::<f32, S>(&result).to_vec()
}

/// * Use the `Resampler` to resample a stereo audio from the original sample rate to a specific sample rate.
pub fn do_resample_stereo<S>(resampler: &Resampler, input: &[(S, S)], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<(S, S)>
where S: SampleType {
    let block = stereos_to_dual_monos(input);
    let l = do_resample_mono(resampler, &block.0, src_sample_rate, dst_sample_rate);
    let r = do_resample_mono(resampler, &block.1, src_sample_rate, dst_sample_rate);
    dual_monos_to_stereos(&(l, r)).unwrap()
}

/// * Use the `Resampler` to resample audio frames from the original sample rate to a specific sample rate.
pub fn do_resample_frames<S>(resampler: &Resampler, input: &[Vec<S>], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<Vec<S>>
where S: SampleType {
    let monos = frames_to_monos(input, None).unwrap();
    let monos: Vec<Vec<S>> = monos.into_iter().map(|mono|{do_resample_mono(resampler, &mono, src_sample_rate, dst_sample_rate)}).collect();
    monos_to_frames(&monos).unwrap()
}
