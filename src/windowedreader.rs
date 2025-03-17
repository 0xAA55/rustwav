use std::error::Error;

use crate::audioreader::{AudioReader, Spec};
use crate::waveform::WaveForm;

pub struct WindowedAudioReader {
    reader: Box<dyn AudioReader>,
    spec: Spec,
    last_chunk: WaveForm,
    chunk_size: usize,
}

impl WindowedAudioReader {
    pub fn upgrade(reader: Box<dyn AudioReader>) -> Result<WindowedAudioReader, Box<dyn Error>> {
        let spec = reader.spec();
        Ok(Self {
            reader,
            spec,
            last_chunk: WaveForm::None,
            chunk_size: 0,
        })
    }
}

impl AudioReader for WindowedAudioReader {
    fn spec(&self) -> Spec {
        self.spec.clone()
    }

    fn set_chunk_size(&mut self, chunk_size: usize) {
        self.reader.set_chunk_size(chunk_size / 2);
        self.chunk_size = self.reader.get_chunk_size() * 2;
    }

    fn get_chunk_size(&self) -> usize {
        self.chunk_size
    }
}

impl Iterator for WindowedAudioReader {
    type Item = WaveForm;

    fn next(&mut self) -> Option<Self::Item> {
        if self.chunk_size == 0 {panic!("Must set chunk size before iterations.")}

        let new_chunk = self.reader.next();
        match new_chunk {
            None => { // 新块没有数据了
                match self.last_chunk {
                    // 旧的块都无了，则返回 None 结束迭代
                    WaveForm::None => None,

                    // 旧的块还有，延长旧的块为全块大小，将其返回，然后让旧的块变为无。
                    _ => {
                        let ret = self.last_chunk.resized(self.chunk_size);
                        self.last_chunk = WaveForm::None;
                        Some(ret)
                    },
                }
            },
            Some(chunk) => { // 新块有数据
                // 不论如何，延长到半块大小
                let chunk = chunk.resized(self.reader.get_chunk_size());
                match self.last_chunk {
                    // 没有旧块，但是有新块，说明是第一次迭代。此时应当再次迭代，才能组成一个完整的块。
                    WaveForm::None => {
                        self.last_chunk = chunk;
                        self.next()
                    },
                    // 有新块，有旧块
                    _ => {
                        let ret = self.last_chunk.extended(&chunk).unwrap();
                        self.last_chunk = chunk;
                        Some(ret)
                    },
                }
            },
        }
    }
}
