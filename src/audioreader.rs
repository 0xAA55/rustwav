use std::error::Error;

use crate::audiocore::{Spec};
use crate::sampleutils::SampleType;

// 音频文件读取器的接口，提供迭代器用于获取每个 Frame 的音频数据。
// 当音频的声道数超出你的认知的时候，请阅读 audiocore 里面的 Spec::which_channel_which_speaker()，它会返回一个数组告诉你哪个声道对应哪个扬声器。
// 要求从文件里读取信息后填写自己的 Spec。
// 迭代器的泛型参数 T 是表示使用者想要得到什么样的格式的音频数据，使用 sampleutils 里面的工具可以把原始的数据转换为用户想要的。
// 迭代器返回的 Vec<T> 是一个 Frame，里面是每个声道的电平值。
pub trait AudioReader {
    fn spec(&self) -> &Spec;

    fn iter<T>(&mut self) -> Result<Iterator<Item = Vec<T>>>, Box<dyn Error>>
    where Self: Sized,
          T: SampleType;
}

pub trait AudioIterator<T>: Iterator {

}
