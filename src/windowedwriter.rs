
use std::error::Error;
use crate::audiowriter::{AudioWriter, WriterError};
use crate::waveform::WaveForm;

pub struct WindowedAudioComposer {
    writer: Box<dyn AudioWriter>,
    window_size: usize,
    window: WaveForm,
}

impl AudioWriter for WindowedAudioComposer {
    fn upgrade(writer: Box<dyn AudioWriter>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            writer,
            window_size: 0,
            window: WaveForm::None,
        })
    }

    fn set_window_size(&mut self, window_size: usize) {
        self.window_size = window_size;
    }

    fn write(&mut self, channels_data: WaveForm) -> Result<(), Box<dyn Error>> {
        // 策略：
        // 输入的音频被设计为窗口长度的两倍。
        // 平时存储上一个输入的音频的后半部分，音频来了以后，合并前半部分，写入文件。
        // 然后再存储新音频的后半部分。
        // 最后要调用一次 `finalize()`，把存储的最后一段音频的后半部分写入。
        if channels_data.len().unwrap() < self.window_size {
            return Err(WriterError::WaveFormLengthError.into());
        }
        // 按窗口大小分割音频
        let (first, second) = channels_data.split(self.window_size);
        if let WaveForm::None = self.window {
            // 第一次写入，直接写入前半段，存住后半段
            self.writer.write(first)?;
        } else {
            // 写入叠加窗口
            self.writer.write(first.add_to(&self.window)?)?;
        }
        self.window = second;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.writer.write(self.window.clone())?;
        self.writer.finalize()
    }
}
