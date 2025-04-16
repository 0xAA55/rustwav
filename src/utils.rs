use crate::{AudioWriteError};
use crate::{SampleType};
use crate::{Resampler};

pub fn multiple_stereos_to_dual_mono<S>(stereos: &[(S, S)]) -> (Vec<S>, Vec<S>)
where S: SampleType {
    let l = stereos.iter().map(|(l, _r): &(S, S)| -> S {*l}).collect();
    let r = stereos.iter().map(|(_l, r): &(S, S)| -> S {*r}).collect();
    (l, r)
}

pub fn is_same_len<S>(data: &[Vec<S>]) -> Option<(bool, usize)> {
    if data.is_empty() {
        None
    } else {
        let lengths: Vec<usize> = data.iter().map(|item| item.len()).collect();
        let first = lengths[0];
        Some((lengths.iter().all(|&item| item == first), first))
    }
}

pub fn multiple_frames_to_tuples<S>(frames: &[Vec<S>]) -> Result<Vec<(S, S)>, AudioWriteError>
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

pub fn multiple_frames_to_dual_mono<S>(frames: &[Vec<S>]) -> Result<(Vec<S>, Vec<S>), AudioWriteError>
where S: SampleType {
    Ok(multiple_stereos_to_dual_mono(&multiple_frames_to_tuples(frames)?))
}

pub fn multiple_frames_to_multiple_monos<S>(frames: &[Vec<S>], channels: Option<u16>) -> Result<Vec<Vec<S>>, AudioWriteError>
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

pub fn multiple_monos_to_multiple_frames<S>(monos: &[Vec<S>]) -> Result<Vec<Vec<S>>, AudioWriteError>
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

pub fn multiple_monos_to_interleaved_samples<S>(monos: &[Vec<S>]) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    Ok(multiple_monos_to_multiple_frames(monos)?.into_iter().flatten().collect())
}

pub fn multiple_frames_to_interleaved_samples<S>(frames: &[Vec<S>], channels: Option<u16>) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    multiple_monos_to_interleaved_samples(&multiple_frames_to_multiple_monos(frames, channels)?)
}

pub fn multiple_stereos_to_interleaved_samples<S>(stereos: &[(S, S)]) -> Vec<S>
where S: SampleType {
    stereos.iter().flat_map(|(l, r): &(S, S)| -> [S; 2] {[*l, *r]}).collect()
}

pub fn interleaved_samples_to_multiple_monos<S>(samples: &[S], channels: u16) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    if channels == 0 {
        Err(AudioWriteError::InvalidArguments("Channels must not be zero".to_owned()))
    } else {
        Ok((0..channels).map(|channel| -> Vec<S> {samples.iter().skip(channel as usize).step_by(channels as usize).copied().collect()}).collect())
    }
}

pub fn dual_mono_to_multiple_stereos<S>(dual_monos: &(Vec<S>, Vec<S>)) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    let (l, r) = dual_monos;
    if l.len() != r.len() {
        Err(AudioWriteError::MultipleMonosAreNotSameSize)
    } else {
        Ok(l.into_iter().zip(r.into_iter()).map(|(l, r): (&S, &S)| -> (S, S) {(*l, *r)}).collect())
    }
}

pub fn interleaved_samples_to_stereos<S>(samples: &[S]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    if (samples.len() & 1) != 0 {
        Err(AudioWriteError::NotStereo)
    } else {
        Ok((0..(samples.len() / 2)).map(|position| -> (S, S) {(samples[position * 2], samples[position * 2 + 1])}).collect())
    }
}

// 样本类型缩放转换
// 根据样本的存储值范围大小的不同，进行缩放使适应目标样本类型。
#[inline(always)]
pub fn stereo_conv<S, D>(frame: (S, S)) -> (D, D)
where S: SampleType,
      D: SampleType {
    let (l, r) = frame;
    (D::from(l), D::from(r))
}

pub fn sample_conv<S, D>(frame: &[S]) -> Vec<D>
where S: SampleType,
      D: SampleType {

    frame.iter().map(|sample: &S| -> D {D::from(*sample)}).collect()
}

pub fn stereos_conv<S, D>(frame: &[(S, S)]) -> Vec<(D, D)>
where S: SampleType,
      D: SampleType {

    frame.iter().map(|stereo: &(S, S)| -> (D, D) {stereo_conv(*stereo)}).collect()
}

// 样本类型缩放转换批量版
pub fn sample_conv_batch<S, D>(frames: &[Vec<S>]) -> Vec<Vec<D>>
where S: SampleType,
      D: SampleType {

    frames.iter().map(|frame: &Vec<S>| -> Vec<D> {sample_conv(frame)}).collect()
}

pub fn do_resample_mono<S>(resampler: &mut Resampler, input: &[S], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<S>
where S: SampleType {
    const MAX_LENGTHEN_RATE: u32 = 4;
    let input = sample_conv::<S, f32>(input);
    let result = resampler.resample(&input, src_sample_rate, dst_sample_rate, MAX_LENGTHEN_RATE).unwrap();
    sample_conv::<f32, S>(&result)
}

pub fn do_resample_stereo<S>(resampler: &mut Resampler, input: &[(S, S)], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<(S, S)>
where S: SampleType {
    let block = multiple_stereos_to_dual_mono(input);
    let l = do_resample_mono(resampler, &block.0, src_sample_rate, dst_sample_rate);
    let r = do_resample_mono(resampler, &block.1, src_sample_rate, dst_sample_rate);
    dual_mono_to_multiple_stereos(&(l, r)).unwrap()
}

pub fn do_resample_frames<S>(resampler: &mut Resampler, input: &[Vec<S>], src_sample_rate: u32, dst_sample_rate: u32) -> Vec<Vec<S>>
where S: SampleType {
    let monos = multiple_frames_to_multiple_monos(input, None).unwrap();
    let monos = monos.into_iter().map(|mono|{do_resample_mono(resampler, &mono, src_sample_rate, dst_sample_rate)}).collect::<Vec<Vec<S>>>();
    multiple_monos_to_multiple_frames(&monos).unwrap()
}