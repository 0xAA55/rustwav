use std::error::Error;

use crate::errors::*;
use crate::audiocore::{Spec};
use crate::sampleutils::SampleConv;

// 音频文件写入器的接口，写入格式随意，会自动转换。
// 实际存储的格式按 Spec 的来。自己初始化自己的 Spec。
// 调用 write<T> 的时候，实际的音频数据格式会根据 Spec 自动转换为存储所需的音频格式。
// 且每次调用的时候，都会自动调用 check_channels<T> 来判断写入的声道数对不对，如果不对就会返回错误值。
// 完成所有的音频样本的写入后，一定要调用一次 finalize()，否则头文件里面的信息不会被更新。
pub trait AudioWriter {
    fn spec(&self) -> &Spec;
    fn write<T>(&mut self, frame: &Vec<T>) -> Result<(), Box<dyn Error>> while T: SampleConv;
    fn finalize(&mut self) -> Result<(), Box<dyn Error>>;

    fn check_channels<T>(&self, frame: &Vec<T>) -> Result<(), Box<dyn Error>> while T: SampleConv;{
        if frame.len() != self.spec().channels as usize {
            Err(AudioWriteError::ChannelCountNotMatch.into())
        } else {
            Ok(())
        }
    }
}
