#![allow(non_snake_case)]
#![allow(dead_code)]

use std::fs::File;
use std::path::Path;
use std::io::{SeekFrom, BufWriter};
use std::sync::{Arc, Mutex};
use std::error::Error;

use crate::errors::{AudioWriteError};
pub use crate::wavcore::*;

#[derive(Debug)]
pub struct WaveWriter {
    writer: Arc<Mutex<dyn Writer>>,
    spec: Spec,
    allow_larger_than_4gb: bool,
    num_frames: u64,
    frame_size: u16,
    data_offset: u64,
    sample_type: WaveSampleType,
    sample_packer: WaveSamplePacker,
    riff_chunk: Option<ChunkWriter>,
    data_chunk: Option<ChunkWriter>,
    pub bext_chunk: Option<BextChunk>,
    pub smpl_chunk: Option<SmplChunk>,
    pub inst_chunk: Option<InstChunk>,
    pub cue__chunk: Option<Cue_Chunk>,
    pub axml_chunk: Option<String>,
    pub ixml_chunk: Option<String>,
    pub list_chunk: Option<ListChunk>,
    pub acid_chunk: Option<AcidChunk>,
    pub trkn_chunk: Option<String>,
    pub junk_chunks: Vec<Vec<u8>>,
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, allow_larger_than_4gb: bool) -> Result<WaveWriter, Box<dyn Error>> {
        let file_reader = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(Arc::new(Mutex::new(file_reader)), spec, allow_larger_than_4gb)?;
        Ok(wave_writer)
    }

    pub fn from(writer: Arc<Mutex<dyn Writer>>, spec: &Spec, allow_larger_than_4gb: bool) -> Result<WaveWriter, Box<dyn Error>> {
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let sample_type = spec.get_sample_type()?;
        let mut ret = Self{
            writer: writer.clone(),
            spec: spec.clone(),
            allow_larger_than_4gb,
            num_frames: 0,
            frame_size,
            data_offset: 0,
            sample_type,
            sample_packer: WaveSamplePacker::new(sample_type),
            riff_chunk: None,
            data_chunk: None,
            bext_chunk: None,
            smpl_chunk: None,
            inst_chunk: None,
            cue__chunk: None,
            axml_chunk: None,
            ixml_chunk: None,
            list_chunk: None,
            acid_chunk: None,
            trkn_chunk: None,
            junk_chunks: Vec::<Vec<u8>>::new(),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"RIFF")?);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            writer.write_all(b"WAVE")?;
            Ok(())
        })?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        if self.allow_larger_than_4gb {
            let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                writer.write_all(&[0u8; 28])?;
                Ok(())
            })?;
            cw.end()?;
        }

        // 准备写入 fmt 块
        // 如果格式类型是 0xFFFE 则需要单独对待
        let mut ext = match (self.spec.bits_per_sample, self.spec.sample_format) {
            (24, Int) | (32, Int) => true,
            _ => false
        };
        // 如果有针对声道的特殊要求，则需要扩展数据
        ext |= match self.spec.channels {
            1 => {
                if self.spec.channel_mask != SpeakerPosition::FrontCenter as u32 {
                    true
                } else {
                    false
                }
            },
            2 => {
                if self.spec.channel_mask != SpeakerPosition::FrontLeft as u32 | SpeakerPosition::FrontRight as u32 {
                    true
                } else {
                    false
                }
            },
            _ => true, // 否则就需要额外的数据了
        };

        let fmt_chunk = fmt_Chunk {
            format_tag: match ext {
                true => 0xFFFE,
                false => {
                    match (self.spec.bits_per_sample, self.spec.sample_format) {
                        (8, UInt) => 1,
                        (16, Int) => 1,
                        (32, Float) => 3,
                        (64, Float) => 3,
                        _ => return Err(AudioWriteError::UnsupportedFormat(format!("Don't know how to specify format tag")).into()),
                    }
                }
            },
            channels: self.spec.channels,
            sample_rate: self.spec.sample_rate,
            byte_rate: self.spec.sample_rate * self.frame_size as u32,
            block_align: self.frame_size,
            bits_per_sample: self.spec.bits_per_sample,
            extension: match ext {
                false => None,
                true => Some(fmt_Chunk_Extension {
                    ext_len: 22,
                    bits_per_sample: self.spec.bits_per_sample,
                    channel_mask: self.spec.channel_mask,
                    sub_format: match self.spec.sample_format {
                        Int => GUID_PCM_FORMAT,
                        Float => GUID_IEEE_FLOAT_FORMAT,
                        other => return Err(AudioWriteError::InvalidArguments(format!("\"{}\" was given for specifying the sample format", other)).into()),
                    },
                }),
            },
        };

        fmt_chunk.write(self.writer.clone())?;

        self.data_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"data")?);
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_sample<S>(&mut self, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        if self.data_chunk.is_some() {
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                self.sample_packer.pack_sample::<S>(writer, frame);
            })?;
            self.num_frames += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")).into())
        }
    }

    pub fn spec(&self) -> &Spec{
        &self.spec
    }
    pub fn get_num_frames(&self) -> u64 {
        self.num_frames
    }
    pub fn get_frame_size(&self) -> u16 {
        self.frame_size
    }
    pub fn set_bext_chunk(&mut self, chunk: &BextChunk) {
        self.bext_chunk = Some(chunk.clone());
    }
    pub fn set_smpl_chunk(&mut self, chunk: &SmplChunk) {
        self.smpl_chunk = Some(chunk.clone());
    }
    pub fn set_inst_chunk(&mut self, chunk: &InstChunk) {
        self.inst_chunk = Some(chunk.clone());
    }
    pub fn set_cue__chunk(&mut self, chunk: &Cue_Chunk) {
        self.cue__chunk = Some(chunk.clone());
    }
    pub fn set_axml_chunk(&mut self, chunk: &String) {
        self.axml_chunk = Some(chunk.clone());
    }
    pub fn set_ixml_chunk(&mut self, chunk: &String) {
        self.ixml_chunk = Some(chunk.clone());
    }
    pub fn set_list_chunk(&mut self, chunk: &ListChunk) {
        self.list_chunk = Some(chunk.clone());
    }
    pub fn set_acid_chunk(&mut self, chunk: &AcidChunk) {
        self.acid_chunk = Some(chunk.clone());
    }
    pub fn set_trkn_chunk(&mut self, chunk: &String) {
        self.trkn_chunk = Some(chunk.clone());
    }
    pub fn add_junk_chunk(&mut self, chunk: &Vec<u8>) {
        self.junk_chunks.push(chunk.clone());
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // 结束对 data 块的写入
        self.data_chunk = None;

        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.smpl_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.inst_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.cue__chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.list_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.acid_chunk { chunk.write(self.writer.clone())?; }

        // 写入其它全部的字符串块
        let mut string_chunks_to_write = Vec::<([u8; 4], &String)>::new();
        if let Some(chunk) = &self.axml_chunk {
            string_chunks_to_write.push((*b"axml", &chunk));
        }
        if let Some(chunk) = &self.ixml_chunk {
            string_chunks_to_write.push((*b"ixml", &chunk));
        }
        if let Some(chunk) = &self.trkn_chunk {
            string_chunks_to_write.push((*b"Trkn", &chunk));
        }
        for (flag, chunk) in string_chunks_to_write.iter() {
            let mut cw = ChunkWriter::begin(self.writer.clone(), flag)?;
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                write_str(writer, &chunk)?;
                Ok(())
            })?;
            cw.end()?;
        }

        // 写入所有的 JUNK 块
        for junk in self.junk_chunks.iter() {
            let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
            use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                writer.write_all(&junk)?;
                Ok(())
            })?;
            cw.end()?;
        }

        // 接下来是重点：判断文件大小是不是超过了 4GB，是的话，把文件头改为 RF64，然后在之前留坑的地方填入 RF64 的信息表
        self.riff_chunk = None;

        use_writer(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            let file_end_pos = writer.stream_position()?;
            if file_end_pos > 0xFFFFFFFFu64 {
                writer.seek(SeekFrom::Start(0))?;
                writer.write_all(b"RF64")?;
                0xFFFFFFFFu32.write_le(writer)?;
                writer.write_all(b"WAVE")?;
                writer.write_all(b"ds64")?;
                28u32.write_le(writer)?; // ds64 段的长度
                let riff_size = file_end_pos - 8;
                let data_size = self.num_frames * self.frame_size as u64;
                let sample_count = self.num_frames / self.spec.channels as u64;
                riff_size.write_le(writer)?;
                data_size.write_le(writer)?;
                sample_count.write_le(writer)?;
                0u32.write_le(writer)?; // table length
            }
            Ok(())
        })?;
        Ok(())
    }
}

// S：用户给我们的格式
// T：我们要写入到 WAV 中的格式
fn write_sample_to<S, T>(writer: &mut dyn Writer, frame: &Vec<S>) -> Result<(), Box<dyn Error>>
where S: SampleType + SampleFrom,
      T: SampleType {
    for sample in frame.iter() {
        <S as SampleFrom>::to::<T>(sample).write_le(writer)?;
    }
    Ok(())
}

trait SamplePackTo<T>
where T: SampleType {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo__i8;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_i16;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_i24;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_i32;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_i64;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo__u8;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_u16;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_u24;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_u32;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_u64;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_f32;
#[derive(Debug)]#[allow(non_camel_case_types)] struct SamplePackTo_f64;

// TODO
// 已经很明确 SamplePackTo__i8 是要把任意的输入格式转换为 i8，别的类型同理。
// 但是任意的输入格式是一个泛型。

impl<T> SamplePackTo<T> for SamplePackTo__i8 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<i8, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_i16 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<i16, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_i24 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<i24, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_i32 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<i32, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_i64 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<i64, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo__u8 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<u8, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_u16 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<u16, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_u24 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<u24, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_u32 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<u32, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_u64 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<u64, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_f32 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<f32, T>(writer, frame)
    }
}

impl<T> SamplePackTo<T> for SamplePackTo_f64 {
    fn pack_from(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        write_sample_to::<f64, T>(writer, frame)
    }
}

#[derive(Debug, Clone)]
struct WaveSamplePacker<T>
    where T: SampleType {
    packer: Box<dyn SamplePackTo<T>>,
}

impl<T> WaveSamplePacker<T> {
    fn new(to_type: WaveSampleType) -> Self {
        use WaveSampleType::{S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        Self {
            packer: match to_type {
                S8  => Box::new(SamplePackTo__i8::<i8 >{}),
                S16 => Box::new(SamplePackTo_i16::<i16>{}),
                S24 => Box::new(SamplePackTo_i24::<i24>{}),
                S32 => Box::new(SamplePackTo_i32::<i32>{}),
                S64 => Box::new(SamplePackTo_i64::<i64>{}),
                U8  => Box::new(SamplePackTo__u8::<u8 >{}),
                U16 => Box::new(SamplePackTo_u16::<u16>{}),
                U24 => Box::new(SamplePackTo_u24::<u24>{}),
                U32 => Box::new(SamplePackTo_u32::<u32>{}),
                U64 => Box::new(SamplePackTo_u64::<u64>{}),
                F32 => Box::new(SamplePackTo_f32::<f32>{}),
                F64 => Box::new(SamplePackTo_f64::<f64>{}),
                other => panic!("Don't know how to convert to \"{:?}\"", to_type),
            },
        }
    }

    fn pack_sample(&self, writer: &mut dyn Writer, frame: &Vec<T>) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        self.packer.pack_sample::<T>(writer, frame)
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        self.finalize().unwrap();
    }
}
