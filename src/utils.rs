use crate::{AudioWriteError};
use crate::{SampleType};

pub fn multiple_frames_to_tuples<S>(frames: &[Vec<S>]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    let mut tuples = Vec::<(S, S)>::with_capacity(frames.len());
    for frame in frames.iter() {
        match frame.len() {
            1 => tuples.push((frame[0], frame[0])),
            2 => tuples.push((frame[0], frame[1])),
            _ => return Err(AudioWriteError::FrameChannelsNotSame),
        }
    }
    Ok(tuples)
}

pub fn stereos_to_dual_mono<S>(stereos: &[(S, S)]) -> (Vec<S>, Vec<S>)
where S: SampleType {
    let mut l = Vec::<S>::with_capacity(stereos.len());
    let mut r = Vec::<S>::with_capacity(stereos.len());
    for stereo in stereos.iter() {
        l.push(stereo.0);
        r.push(stereo.1);
    }
    (l, r)
}

pub fn is_same_len<S>(data: &[Vec<S>]) -> Option<(bool, usize)> {
    if data.len() == 0 {
        None
    } else {
        let lengths = data.iter().map(|item| item.len()).collect::<Vec<usize>>();
        let first = lengths[0];
        for i in lengths.iter() {
            if *i != first {
                return Some((false, 0));
            }
        }
        Some((true, first))
    }
}

pub fn multiple_frames_to_dual_mono<S>(frames: &[Vec<S>]) -> Result<(Vec<S>, Vec<S>), AudioWriteError>
where S: SampleType {
    Ok(stereos_to_dual_mono(&multiple_frames_to_tuples(frames)?))
}

pub fn multiple_frames_to_multiple_monos<S>(frames: &[Vec<S>], channels: Option<u16>) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    let mut ret = Vec::<Vec<S>>::new();
    match is_same_len(frames) {
        None => Ok(ret),
        Some((equal, length)) => {
            match equal {
                false => Err(AudioWriteError::FrameChannelsNotSame),
                true => {
                    ret.resize(length, {let mut mono = Vec::<S>::new(); mono.resize(frames.len(), S::new()); mono});
                    for (position, frame) in frames.into_iter().enumerate() {
                        if let Some(channels) = channels {
                            if channels as usize != frame.len() {
                                return Err(AudioWriteError::WrongChannels(format!("The channels is {channels} but the frames include a {} channel frame.", frame.len())));
                            }
                        }
                        for (channel, sample) in frame.into_iter().enumerate() {
                            ret[channel][position] = *sample;
                        }
                    }
                    Ok(ret)
                }
            }
        }
    }
}

pub fn multiple_monos_to_interleaved_samples<S>(monos: &[Vec<S>]) -> Result<Vec<S>, AudioWriteError>
where S: SampleType {
    let mut ret = Vec::<S>::new();
    match is_same_len(monos) {
        None => Ok(ret),
        Some((equal, length)) => {
            match equal {
                false => Err(AudioWriteError::MultipleMonosAreNotSameSize),
                true => {
                    ret.resize(length * monos.len(), S::new());
                    let mut write_position = 0usize;
                    for position in 0..length {
                        for channel in 0..monos.len() {
                            ret[write_position] = monos[channel][position];
                            write_position += 1;
                        }
                    }
                    Ok(ret)
                }
            }
        }
    }
}

pub fn multiple_stereos_to_interleaved_samples<S>(stereos: &[(S, S)]) -> Vec<S>
where S: SampleType {
    let mut ret = Vec::<S>::with_capacity(stereos.len() * 2);
    for (l, r) in stereos.into_iter() {
        ret.push(*l);
        ret.push(*r);
    }
    ret
}

pub fn interleaved_samples_to_multiple_monos<S>(samples: &[S], channels: u16) -> Result<Vec<Vec<S>>, AudioWriteError>
where S: SampleType {
    if channels == 0 {
        return Err(AudioWriteError::InvalidArguments("Channels must not be zero".to_owned()));
    }
    let channels = channels as usize;
    let mut ret = Vec::<Vec<S>>::new();
    ret.resize(channels, Vec::<S>::new());
    for (index, sample) in samples.into_iter().enumerate() {
        let channel = index % channels;
        ret[channel].push(*sample);
    }
    match is_same_len(&ret) {
        None => Ok(ret),
        Some((equal, _)) => {
            match equal {
                false => Err(AudioWriteError::MultipleMonosAreNotSameSize),
                true => Ok(ret)
            }
        }
    }
}

pub fn interleaved_samples_to_stereos<S>(samples: &[S]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    if (samples.len() & 1) != 0 {
        return Err(AudioWriteError::NotStereo);
    }
    let stereo_len = samples.len() / 2;
    let mut ret = Vec::<(S, S)>::with_capacity(stereo_len);
    for i in 0..stereo_len {
        ret.push((samples[i * 2], samples[i * 2 + 1]));
    }
    Ok(ret)
}

// 样本类型缩放转换
// 根据样本的存储值范围大小的不同，进行缩放使适应目标样本类型。
pub fn sample_conv<S, D>(frame: &[S]) -> Vec<D>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<D>::with_capacity(frame.len());
    for f in frame.iter() {
        ret.push(D::from(*f));
    }
    ret
}

pub fn stereo_conv<S, D>(frame: &[(S, S)]) -> Vec<(D, D)>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<(D, D)>::with_capacity(frame.len());
    for f in frame.into_iter() {
        let (l, r) = *f;
        let (l, r) = (D::from(l), D::from(r));
        ret.push((l, r));
    }
    ret
}

// 样本类型缩放转换批量版
pub fn sample_conv_batch<S, D>(frames: &[Vec<S>]) -> Vec<Vec<D>>
where S: SampleType,
      D: SampleType {

    let mut ret = Vec::<Vec<D>>::with_capacity(frames.len());
    for f in frames.iter() {
        ret.push(sample_conv(f));
    }
    ret
}
