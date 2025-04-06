use crate::{AudioWriteError};
use crate::{SampleType};

pub fn multiple_frames_to_tuples<S>(frames: &[Vec<S>]) -> Result<Vec<(S, S)>, AudioWriteError>
where S: SampleType {
    let mut tuples = Vec::<(S, S)>::with_capacity(frames.len());
    for frame in frames.iter() {
        match frame.len() {
            1 => tuples.push((frame[0], frame[0])),
            2 => tuples.push((frame[0], frame[1])),
            other => return Err(AudioWriteError::InvalidArguments(format!("Channel number is {other}, can't turn to 2 tuples."))),
        }
    }
    Ok(tuples)
}

pub fn stereos_to_dual_mono<S>(stereos: &[(S, S)]) -> (Vec<S>, Vec<S>)
where S: SampleType {
    let mut r = Vec::<S>::with_capacity(frames.len());
    for frame in frames.iter() {
        match frame.len() {
            1 => l.push(frame[0]),
            2 => {
                l.push(frame[0]);
                r.push(frame[1]);
                if l.len() != r.len() {
                    return Err(AudioWriteError::InvalidArguments(format!("Not all frames are the same channels.")));
                }
            },
            other => return Err(AudioWriteError::InvalidArguments(format!("Channel number is {other}, can't turn to 2 tuples."))),
        }
    }
    Ok((l, r))
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