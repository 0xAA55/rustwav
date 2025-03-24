use std::{fs::File, {path::Path}, io::{self, Write, Seek, SeekFrom, BufWriter}, error::Error};

#[allow(unused_imports)]
pub use crate::errors::*;

#[allow(unused_imports)]
pub use crate::wavcore::*;

#[allow(unused_imports)]
pub use crate::audiocore::*;

use crate::sampleutils::*;
use crate::audiowriter::{AudioWriter};

trait Writer: Write + Seek {}
impl<T> Writer for T where T: Write + Seek{}

pub struct WaveWriter<W: Writer> {
    writer: W,
    spec: Spec,
    num_frames: u32,
    frame_size: u16,
    riff_offset: u64,
    data_offset: u64,
    sample_offset: u64,
    packer: Packer<f64>,
}

impl<W> WaveWriter<W>
where W: Writer {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec) -> Result<WaveWriter<BufWriter<File>>, Box<dyn Error>> {
        WaveWriter::new(BufWriter::new(File::create(filename)?), spec)
    }
 
    pub fn new(writer: &mut Writer, spec: &Spec) -> Result<WaveWriter<W>, Box<dyn Error>> {
        use SampleFormat::{Int, UInt, Float};
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        writer.write_all(b"RIFF")?;
        let riff_offset = writer.stream_position()?;
        0u32.write_le(writer)?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        // 如果格式类型是 0xFFFE 则需要单独对待
        let mut ext = match (spec.bits_per_sample, spec.sample_format) {
            (24, Int) | (32, Int) => true,
            _ => false
        };
        // 如果有针对声道的特殊要求，则需要扩展数据
        ext |= match spec.channels {
            1 => {
                if spec.channel_mask != SpeakerPosition::FrontCenter as u32 {
                    true
                } else {
                    false
                }
            },
            2 => {
                if spec.channel_mask != SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32 {
                    true
                } else {
                    false
                }
            },
            _ => true, // 否则就需要额外的数据了
        };
        // fmt 块的大小
        (match ext {
            true => 40,
            false => 16,
        } as u32).write_le(writer)?;
        (match ext {
            true => 0xFFFE,
            false => {
                match (spec.bits_per_sample, spec.sample_format) {
                    (8, UInt) => 1,
                    (16, Int) => 1,
                    (32, Float) => 3,
                    (64, Float) => 3,
                    _ => return Err(AudioWriteError::UnsupportedFormat.into()),
                }
            }
        } as u16).write_le(writer)?;
        (spec.channels as u16).write_le(writer)?;
        (spec.sample_rate as u32).write_le(writer)?;
        (spec.sample_rate * frame_size as u32).write_le(writer)?;
        (frame_size as u16).write_le(writer)?;
        (spec.bits_per_sample as u16).write_le(writer)?;
        if ext == true {
            22u16.write_le(writer)?; // 额外数据大小
            (spec.bits_per_sample as u16).write_le(writer)?;
            (spec.channel_mask as u32).write_le(writer)?; // 声道掩码

            // 写入具体格式的 GUID
            match spec.sample_format {
                Int => guid_pcm_format.write(writer)?,
                Float => guid_ieee_float_format.write(writer)?,
                _ => return Err(AudioWriteError::InvalidArguments.into()),
            }
        };
        writer.write_all(b"data")?;
        let data_offset = writer.stream_position()?;
        0u32.write_le(writer)?;
        let sample_offset = writer.stream_position()?;
        let spec = spec.clone();
        Ok(Self{
            writer,
            spec,
            num_frames: 0,
            frame_size,
            riff_offset,
            data_offset,
            sample_offset,
            packer: Packer::<f64>::new(&spec)?,
        })
    }

    fn update_header(&mut self) -> Result<(), Box<dyn Error>>
    {
        const header_size: u32 = 44;
        let all_sample_size = self.num_frames * self.frame_size as u32;
        self.writer.seek(SeekFrom::Start(self.riff_offset))?;
        (header_size + all_sample_size - 8).write_le(&mut self.writer)?;
        self.writer.seek(SeekFrom::Start(self.data_offset))?;
        all_sample_size.write_le(&mut self.writer)?;
        Ok(())
    }
}

struct Packer<S: SampleType> {
    save_sample_func: fn(&mut dyn Writer, &Vec<S>) -> Result<(), io::Error>,
}

impl<S> Packer<S>
where S: SampleType {

    // 根据自己的音频格式，挑选合适的函数指针来写入正确的样本类型。
    pub fn new(writer_spec: &Spec) -> Result<Self, AudioWriteError> {
        Ok(Self {
            save_sample_func: match (writer_spec.bits_per_sample, writer_spec.sample_format){
                (8, UInt) => Self::save_u8,
                (16, Int) => Self::save_i16,
                (24, Int) => Self::save_i24,
                (32, Int) => Self::save_i32,
                (32, Float) => Self::save_f32,
                (64, Float) => Self::save_f64,
                _ => return Err(AudioWriteError::InvalidArguments),
            }
        })
    }

    pub fn save_sample(&self, writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        (self.save_sample_func)(writer, frame)
    }

    fn save_u8(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_u8().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i16(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i16().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i24(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i24().write_le(writer)?;
        }
        Ok(())
    }

    fn save_i32(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_i32().write_le(writer)?;
        }
        Ok(())
    }

    fn save_f32(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f32().write_le(writer)?;
        }
        Ok(())
    }

    fn save_f64(writer: &mut Writer, frame: &Vec<S>) -> Result<(), io::Error> {
        for sample in frame.iter() {
            sample.to_f64().write_le(writer)?;
        }
        Ok(())
    }
}

impl<W> AudioWriter for WaveWriter<W>
where W: Writer {
    fn spec(&self) -> &Spec {
        &self.spec
    }

    fn write(&mut self, frame: &Vec<f64>) -> Result<(), Box<dyn Error>> {
        self.check_channels(frame)?;
        self.packer.save_sample(&mut self.writer, frame)?;
        self.num_frames += 1;
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), Box<dyn Error>>
    {
        self.update_header()
    }
}
