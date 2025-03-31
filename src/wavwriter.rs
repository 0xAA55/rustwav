#![allow(non_snake_case)]
#![allow(dead_code)]

use std::fs::File;
use std::path::Path;
use std::io::BufWriter;
use std::error::Error;
use std::collections::HashMap;

pub use crate::wavcore::*;
use crate::wavreader::WaveReader;

// 你以为 WAV 文件只能在 4GB 以内吗？
#[derive(Debug)]
pub enum FileSizeOption{
    NeverLargerThan4GB,
    AllowLargerThan4GB,
    ForceUse4GBFormat,
}

// 你以为 WAV 就是用来存 PCM 的吗？
#[derive(Debug)]
pub enum DataFormat{
    PCM_Int,
    PCM_Float,
    Mp3,
    OggVorbis,
    Flac,
}

#[derive(Debug)]
pub struct WaveWriter {
    writer: Arc<Mutex<dyn Writer>>,
    spec: Spec,
    data_format: DataFormat,
    file_size_option: FileSizeOption,
    num_frames: u64,
    frame_size: u16,
    data_offset: u64,
    sample_type: WaveSampleType,
    sample_packer: SamplePacker,
    text_encoding: Box<dyn SavageStringCodec>,
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
    pub id3__chunk: Option<Id3::Tag>,
    pub junk_chunks: Vec<JunkChunk>,
}

impl WaveWriter {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter, Box<dyn Error>> {
        let file_reader = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(Arc::new(Mutex::new(file_reader)), spec, data_format, file_size_option)?;
        Ok(wave_writer)
    }

    pub fn from(writer: Arc<Mutex<dyn Writer>>, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter, Box<dyn Error>> {
        let sizeof_sample = spec.bits_per_sample / 8;
        let frame_size = sizeof_sample * spec.channels;
        let sample_type = spec.get_sample_type()?;
        let mut ret = Self{
            writer: writer.clone(),
            spec: *spec,
            data_format,
            file_size_option,
            num_frames: 0,
            frame_size,
            data_offset: 0,
            sample_type,
            sample_packer: SamplePacker::new(),
            text_encoding: Box::new(StringCodecMaps::new()),
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
            id3__chunk: None,
            junk_chunks: Vec::<JunkChunk>::new(),
        };
        ret.write_header()?;
        Ok(ret)
    }

    fn write_header(&mut self) -> Result<(), Box<dyn Error>> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"RIFF")?);

        // WAV 文件的 RIFF 块的开头是 WAVE 四个字符
        escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            writer.write_all(b"WAVE")?;
            Ok(())
        })?;

        // 如果说这个 WAV 文件是允许超过 4GB 的，那需要使用 RF64 格式，在 WAVE 后面留下一个 JUNK 块用来占坑。
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => (),
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                let mut cw = ChunkWriter::begin(self.writer.clone(), b"JUNK")?;
                escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                    writer.write_all(&[0u8; 28])?;
                    Ok(())
                })?;
                cw.end()?;
            },
        }

        // 准备写入 fmt 块
        if self.spec.channel_mask == 0 {
            self.spec.guess_channel_mask();
        }

        // 如果声道掩码不等于猜测的声道掩码，则说明需要 0xFFFE 的扩展格式
        let mut ext = self.spec.channel_mask != guess_channel_mask(self.spec.channels);

        let fmt__chunk = fmt__Chunk {
            format_tag: match data_format {
                PCM_Int => {
                    match ext {
                        false => 1,
                        true => 0xFFFE,
                    }
                },
                PCM_Float => 3,
                Mp3 => {
                    ext = false;
                },
                OggVorbis => {
                    ext = false;
                },
            },
            channels: self.spec.channels,
            sample_rate: self.spec.sample_rate,
            byte_rate: self.spec.sample_rate * self.frame_size as u32,
            block_align: self.frame_size,
            bits_per_sample: self.spec.bits_per_sample,
            extension: match ext {
                false => None,
                true => Some(fmt__Chunk_Extension {
                    ext_len: 22,
                    valid_bits_per_sample: self.spec.bits_per_sample,
                    channel_mask: self.spec.channel_mask,
                    sub_format: match self.spec.sample_format {
                        Int => GUID_PCM_FORMAT,
                        Float => GUID_IEEE_FLOAT_FORMAT,
                        other => return Err(AudioWriteError::InvalidArguments(format!("\"{}\" was given for specifying the sample format", other)).into()),
                    },
                }),
            },
        };

        fmt__chunk.write(self.writer.clone())?;

        self.data_chunk = Some(ChunkWriter::begin(self.writer.clone(), b"data")?);
        Ok(())
    }

    // 保存样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        if self.data_chunk.is_some() {
            escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                self.sample_packer.pack_frame::<S>(writer, frame, self.sample_type)
            })?;
            self.num_frames += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished(String::from("samples")).into())
        }
    }

    // 保存多个样本。样本的格式 S 由调用者定，而我们自己根据 Spec 转换为我们应当存储到 WAV 内部的样本格式。
    pub fn write_multiple_frames<S>(&mut self, frames: &[Vec<S>]) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        if self.data_chunk.is_some() {
            escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                self.sample_packer.pack_multiple_frames::<S>(writer, frames, self.sample_type)
            })?;
            self.num_frames += frames.len() as u64;
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
        self.inst_chunk = Some(*chunk);
    }
    pub fn set_cue__chunk(&mut self, chunk: &Cue_Chunk) {
        self.cue__chunk = Some(chunk.clone());
    }
    pub fn set_axml_chunk(&mut self, chunk: &String) {
        self.axml_chunk = Some(chunk.to_owned());
    }
    pub fn set_ixml_chunk(&mut self, chunk: &String) {
        self.ixml_chunk = Some(chunk.to_owned());
    }
    pub fn set_list_chunk(&mut self, chunk: &ListChunk) {
        self.list_chunk = Some(chunk.clone());
    }
    pub fn set_acid_chunk(&mut self, chunk: &AcidChunk) {
        self.acid_chunk = Some(chunk.clone());
    }
    pub fn set_trkn_chunk(&mut self, chunk: &String) {
        self.trkn_chunk = Some(chunk.to_owned());
    }
    pub fn add_junk_chunk(&mut self, chunk: &JunkChunk) {
        self.junk_chunks.push(chunk.clone());
    }

    // 从读取器那里迁移乐曲信息的元数据。但是不迁移 JUNK 块。
    pub fn migrate_metadata_from_reader(&mut self, reader: &WaveReader) {
        if reader.get_bext_chunk().is_some() {self.bext_chunk = reader.get_bext_chunk().clone();}
        if reader.get_smpl_chunk().is_some() {self.smpl_chunk = reader.get_smpl_chunk().clone();}
        if reader.get_inst_chunk().is_some() {self.inst_chunk = reader.get_inst_chunk().clone();}
        if reader.get_cue__chunk().is_some() {self.cue__chunk = reader.get_cue__chunk().clone();}
        if reader.get_axml_chunk().is_some() {self.axml_chunk = reader.get_axml_chunk().clone();}
        if reader.get_ixml_chunk().is_some() {self.ixml_chunk = reader.get_ixml_chunk().clone();}
        if reader.get_list_chunk().is_some() {self.list_chunk = reader.get_list_chunk().clone();}
        if reader.get_acid_chunk().is_some() {self.acid_chunk = reader.get_acid_chunk().clone();}
        if reader.get_trkn_chunk().is_some() {self.trkn_chunk = reader.get_trkn_chunk().clone();}
        if reader.get_id3__chunk().is_some() {self.id3__chunk = reader.get_id3__chunk().clone();}
    }

    pub fn finalize(&mut self) -> Result<(), Box<dyn Error>> {
        // 结束对 data 块的写入
        self.data_chunk = None;
        
        // 写入其它全部的结构体块
        if let Some(chunk) = &self.bext_chunk { chunk.write(self.writer.clone(), &*self.text_encoding)?; }
        if let Some(chunk) = &self.smpl_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.inst_chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.cue__chunk { chunk.write(self.writer.clone())?; }
        if let Some(chunk) = &self.list_chunk { chunk.write(self.writer.clone(), &*self.text_encoding)?; }
        if let Some(chunk) = &self.acid_chunk { chunk.write(self.writer.clone())?; }

        // 写入其它全部的字符串块
        let mut string_chunks_to_write = Vec::<([u8; 4], &String)>::new();
        if let Some(chunk) = &self.axml_chunk {
            string_chunks_to_write.push((*b"axml", chunk));
        }
        if let Some(chunk) = &self.ixml_chunk {
            string_chunks_to_write.push((*b"ixml", chunk));
        }
        if let Some(chunk) = &self.trkn_chunk {
            string_chunks_to_write.push((*b"Trkn", chunk));
        }
        for (flag, chunk) in string_chunks_to_write.iter() {
            let mut cw = ChunkWriter::begin(self.writer.clone(), flag)?;
            escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
                write_str(writer, chunk, &*self.text_encoding)?;
                Ok(())
            })?;
            cw.end()?;
        }

        // 写入所有的 JUNK 块
        for junk in self.junk_chunks.iter() {
            junk.write(self.writer.clone())?;
        }

        // 接下来是重点：判断文件大小是不是超过了 4GB，是的话，把文件头改为 RF64，然后在之前留坑的地方填入 RF64 的信息表
        self.riff_chunk = None;

        escorted_write(self.writer.clone(), |writer| -> Result<(), Box<dyn Error>> {
            let file_end_pos = writer.stream_position()?;
            let mut change_to_4gb_hreader = || -> Result<(), Box<dyn Error>> {
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
                Ok(())
            };
            match self.file_size_option {
                FileSizeOption::NeverLargerThan4GB => {
                    if file_end_pos > 0xFFFFFFFFu64 {
                        Err(AudioWriteError::NotPreparedFor4GBFile.into())
                    } else {
                        Ok(())
                    }
                },
                FileSizeOption::AllowLargerThan4GB => {
                    if file_end_pos > 0xFFFFFFFFu64 {
                        change_to_4gb_hreader()
                    } else {
                        Ok(())
                    }
                },
                FileSizeOption::ForceUse4GBFormat => {
                    change_to_4gb_hreader()
                },
            }
        })?;
        Ok(())
    }
}

impl Drop for WaveWriter {
    fn drop(&mut self) {
        self.finalize().unwrap();
    }
}

struct SamplePackerFrom<S>
where S: SampleType {
    packers: HashMap<WaveSampleType, fn(&mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>>,
    packer: fn(&mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>,
    last_sample: WaveSampleType,
}

// 调试信息部分，音频样本打包器的调试信息会霸屏，对于没用过的打包器让它少说点。
#[derive(Debug)]
#[allow(non_camel_case_types)]
struct HashMap_for_packers;
impl<S> std::fmt::Debug for SamplePackerFrom<S>
where S: SampleType {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.last_sample {
            WaveSampleType::Unknown => {
                fmt.debug_struct(&format!("SamplePackerFrom<{}>", std::any::type_name::<S>()))
                    .finish_non_exhaustive()
                },
            _ => {
                fmt.debug_struct(&format!("SamplePackerFrom<{}>", std::any::type_name::<S>()))
                    .field("packers", &HashMap_for_packers{})
                    .field("packer", &self.packer)
                    .field("last_sample", &self.last_sample)
                    .finish()
            },
        }
    }
}

fn dummy_fn<S>(_writer: &mut dyn Writer, _frame: &[S]) -> Result<(), Box<dyn Error>>
where S: SampleType {
    panic!("`dummy_fn()` was called.");
}

impl<S> SamplePackerFrom<S>
where S: SampleType {
    pub fn new() -> Self {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        let mut packers = HashMap::<WaveSampleType, fn(&mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>>::new();
        packers.insert(Unknown, dummy_fn::<S>);
        packers.insert(S8 , Self::write_sample_to::<i8 >);
        packers.insert(S16, Self::write_sample_to::<i16>);
        packers.insert(S24, Self::write_sample_to::<i24>);
        packers.insert(S32, Self::write_sample_to::<i32>);
        packers.insert(S64, Self::write_sample_to::<i64>);
        packers.insert(U8,  Self::write_sample_to::<u8 >);
        packers.insert(U16, Self::write_sample_to::<u16>);
        packers.insert(U24, Self::write_sample_to::<u24>);
        packers.insert(U32, Self::write_sample_to::<u32>);
        packers.insert(U64, Self::write_sample_to::<u64>);
        packers.insert(F32, Self::write_sample_to::<f32>);
        packers.insert(F64, Self::write_sample_to::<f64>);
        Self {
            packers,
            packer: dummy_fn::<S>,
            last_sample: Unknown,
        }
    }

    // S：别人给我们的格式
    // T：我们要写入到 WAV 中的格式
    fn write_sample_to<T>(writer: &mut dyn Writer, frame: &[S]) -> Result<(), Box<dyn Error>>
    where T: SampleType {
        for sample in frame.iter() {
            T::from(*sample).write_le(writer)?;
        }
        Ok(())
    }

    pub fn switch_to_target_sample(&mut self, target_sample: WaveSampleType) -> Result<(), Box<dyn Error>> {
        if self.last_sample != target_sample {
            let pack_fn = self.packers.get(&target_sample);
            match pack_fn {
                None => return Err(AudioWriteError::WrongSampleFormat(format!("{:?}", target_sample)).into()),
                Some(pack_fn) => {
                    self.packer = *pack_fn;
                },
            }
            self.last_sample = target_sample;
        }
        Ok(())
    }
}

impl<S> SamplePackerFrom<S>
where S: SampleType {
    fn pack_frame(&mut self, writer: &mut dyn Writer, frame: &[S], target_sample: WaveSampleType) -> Result<(), Box<dyn Error>> {
        self.switch_to_target_sample(target_sample)?;
        (self.packer)(writer, frame)
    }

    fn pack_multiple_frames(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>], target_sample: WaveSampleType) -> Result<(), Box<dyn Error>> {
        self.switch_to_target_sample(target_sample)?;
        for frame in frames.iter() {
            (self.packer)(writer, frame)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct SamplePacker {
    packer_from__i8: SamplePackerFrom< i8>,
    packer_from_i16: SamplePackerFrom<i16>,
    packer_from_i24: SamplePackerFrom<i24>,
    packer_from_i32: SamplePackerFrom<i32>,
    packer_from_i64: SamplePackerFrom<i64>,
    packer_from__u8: SamplePackerFrom< u8>,
    packer_from_u16: SamplePackerFrom<u16>,
    packer_from_u24: SamplePackerFrom<u24>,
    packer_from_u32: SamplePackerFrom<u32>,
    packer_from_u64: SamplePackerFrom<u64>,
    packer_from_f32: SamplePackerFrom<f32>,
    packer_from_f64: SamplePackerFrom<f64>,
}

impl SamplePacker {
    pub fn new() -> Self {
        Self {
            packer_from__i8: SamplePackerFrom::< i8>::new(),
            packer_from_i16: SamplePackerFrom::<i16>::new(),
            packer_from_i24: SamplePackerFrom::<i24>::new(),
            packer_from_i32: SamplePackerFrom::<i32>::new(),
            packer_from_i64: SamplePackerFrom::<i64>::new(),
            packer_from__u8: SamplePackerFrom::< u8>::new(),
            packer_from_u16: SamplePackerFrom::<u16>::new(),
            packer_from_u24: SamplePackerFrom::<u24>::new(),
            packer_from_u32: SamplePackerFrom::<u32>::new(),
            packer_from_u64: SamplePackerFrom::<u64>::new(),
            packer_from_f32: SamplePackerFrom::<f32>::new(),
            packer_from_f64: SamplePackerFrom::<f64>::new(),
        }
    }

    // 注意 frame_type_conv 和 frames_type_conv 这两个函数
    // 其实在实际调用的时候，S 和 D 是完全一样的类型。
    // 只是编译器不知道。
    // 所以要忽悠一下编译器。

    fn frame_type_conv<S, D>(frame: &[S]) -> Vec<D>
    where S: SampleType,
          D: SampleType {

        assert_eq!(std::any::TypeId::of::<S>(), std::any::TypeId::of::<D>());
        let mut ret = Vec::<D>::with_capacity(frame.len());
        for f in frame.iter() {
            ret.push(D::from(*f));
        }
        ret
    }

    fn frames_type_conv<S, D>(frames: &[Vec<S>]) -> Vec<Vec<D>>
    where S: SampleType,
          D: SampleType {

        assert_eq!(std::any::TypeId::of::<S>(), std::any::TypeId::of::<D>());
        let mut ret = Vec::<Vec<D>>::with_capacity(frames.len());
        for f in frames.iter() {
            ret.push(Self::frame_type_conv::<S, D>(f));
        }
        ret
    }

    pub fn pack_frame<S>(&mut self, writer: &mut dyn Writer, frame: &[S], target_sample: WaveSampleType) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.packer_from__i8.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "i16" => self.packer_from_i16.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "i24" => self.packer_from_i24.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "i32" => self.packer_from_i32.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "i64" => self.packer_from_i64.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "u8"  => self.packer_from__u8.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "u16" => self.packer_from_u16.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "u24" => self.packer_from_u24.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "u32" => self.packer_from_u32.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "u64" => self.packer_from_u64.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "f32" => self.packer_from_f32.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            "f64" => self.packer_from_f64.pack_frame(writer, &Self::frame_type_conv(frame), target_sample),
            other => Err(AudioWriteError::WrongSampleFormat(other.to_owned()).into()),
        }
    }

    pub fn pack_multiple_frames<S>(&mut self, writer: &mut dyn Writer, frames: &[Vec<S>], target_sample: WaveSampleType) -> Result<(), Box<dyn Error>>
    where S: SampleType {
        match std::any::type_name::<S>() {
            "i8"  => self.packer_from__i8.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "i16" => self.packer_from_i16.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "i24" => self.packer_from_i24.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "i32" => self.packer_from_i32.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "i64" => self.packer_from_i64.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "u8"  => self.packer_from__u8.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "u16" => self.packer_from_u16.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "u24" => self.packer_from_u24.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "u32" => self.packer_from_u32.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "u64" => self.packer_from_u64.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "f32" => self.packer_from_f32.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            "f64" => self.packer_from_f64.pack_multiple_frames(writer, &Self::frames_type_conv(frames), target_sample),
            other => Err(AudioWriteError::WrongSampleFormat(other.to_owned()).into()),
        }
    }
}


