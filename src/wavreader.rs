#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, path::{Path, PathBuf}, io::{self, Read, Write, Seek, SeekFrom, BufReader}, sync::Arc, error::Error};

use crate::errors::{AudioReadError};
use crate::wavcore::*;
use crate::savagestr::SavageStringDecoder;

#[derive(Debug)]
pub enum WaveDataSource {
    Reader(Box<dyn Reader>),
    Filename(String),
    Unknown,
}

#[derive(Debug)]
pub struct WaveReader {
    riff_len: u64,
    spec: Spec,
    fmt_chunk: fmt_Chunk, // fmt 块，这个块一定会有
    fact_data: u32, // fact 块的参数
    data_offset: u64, // 音频数据的位置
    data_size: u64, // 音频数据的大小
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u64, // 总帧数
    data_chunk: WaveDataReader,
    bext_chunk: Option<BextChunk>,
    smpl_chunk: Option<SmplChunk>,
    inst_chunk: Option<InstChunk>,
    cue__chunk: Option<Cue_Chunk>,
    axml_chunk: Option<String>,
    ixml_chunk: Option<String>,
    list_chunk: Option<ListChunk>,
    acid_chunk: Option<AcidChunk>,
    trkn_chunk: Option<String>,
    junk_chunks: Vec<JunkChunk>,
    savage_decoder: SavageStringDecoder,
}

impl WaveReader {
    // 从文件路径打开
    pub fn open(file_source: &str) -> Result<Self, Box<dyn Error>> {
        Self::new(WaveDataSource::Filename(file_source.to_string()))
    }

    // 从读取器打开
    pub fn new(file_source: WaveDataSource) -> Result<Self, Box<dyn Error>> {
        let mut filesrc: Option<String> = None;
        let mut reader = match file_source {
            WaveDataSource::Reader(reader) => {
                reader
            },
            WaveDataSource::Filename(filename) => {
                filesrc = Some(filename.clone());
                Box::new(BufReader::new(File::open(&filename)?))
            },
            WaveDataSource::Unknown => return Err(AudioReadError::InvalidArguments(String::from("\"Unknown\" data source was given")).into()),
        };

        let savage_decoder = SavageStringDecoder::new();

        let mut riff_len = 0u64;
        let mut riff_end = 0u64;
        let mut isRF64 = false;
        let mut data_size = 0u64;

        // 先搞定最开始的头部，有 RIFF 头和 RF64 头，需要分开处理
        let chunk = ChunkHeader::read(&mut reader)?;
        match &chunk.flag {
            b"RIFF" => {
                riff_len = chunk.size as u64;
                riff_end = reader.stream_position()? + riff_len;
            },
            b"RF64" => {
                isRF64 = true;
                let _rf64_size = u32::read_le(&mut reader)?;
            },
            _ => return Err(AudioReadError::FormatError(String::from("Not a WAV file")).into()), // 根本不是 WAV
        }

        let start_of_riff = reader.stream_position()?;

        // 读完头部后，这里必须是 WAVE 否则不是音频文件。
        expect_flag(&mut reader, b"WAVE", AudioReadError::FormatError(String::from("not a WAVE file")).into())?;

        let mut fmt_chunk: Option<fmt_Chunk> = None;
        let mut data_offset = 0u64;
        let mut fact_data = 0;
        let mut bext_chunk: Option<BextChunk> = None;
        let mut smpl_chunk: Option<SmplChunk> = None;
        let mut inst_chunk: Option<InstChunk> = None;
        let mut cue__chunk: Option<Cue_Chunk> = None;
        let mut axml_chunk: Option<String> = None;
        let mut ixml_chunk: Option<String> = None;
        let mut list_chunk: Option<ListChunk> = None;
        let mut acid_chunk: Option<AcidChunk> = None;
        let mut trkn_chunk: Option<String> = None;
        let mut junk_chunks: Vec<JunkChunk>;

        junk_chunks = Vec::<JunkChunk>::new();

        // 循环处理 WAV 中的各种各样的小节
        while reader.stream_position()? < riff_end {
            let chunk = ChunkHeader::read(&mut reader)?;
            match &chunk.flag {
                b"JUNK" => {
                    let mut junk = Vec::<u8>::new();
                    junk.resize(chunk.size as usize, 0u8);
                    reader.read_exact(&mut junk)?;
                    junk_chunks.push(JunkChunk::from(junk));
                }
                b"fmt " => {
                    Self::verify_none(&fmt_chunk, &chunk.flag)?;
                    fmt_chunk = Some(fmt_Chunk::read(&mut reader, chunk.size)?);
                },
                b"fact" => {
                    fact_data = u32::read_le(&mut reader)?;
                },
                b"ds64" => {
                    if chunk.size < 28 {
                        return Err(AudioReadError::DataCorrupted(String::from("the size of \"ds64\" chunk is too small to contain enough data")).into())
                    }
                    riff_len = u64::read_le(&mut reader)?;
                    data_size = u64::read_le(&mut reader)?;
                    let _sample_count = u64::read_le(&mut reader)?;
                    // 后面就是 table 了，用来重新给每个 Chunk 提供 64 位的长度值（data 除外）
                    riff_end = start_of_riff + riff_len;
                }
                b"data" => {
                    if data_offset != 0 {
                        return Err(AudioReadError::FormatError(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(&chunk.flag))).into());
                    }
                    data_offset = chunk.chunk_start_pos;
                    if !isRF64 {
                        data_size = chunk.size as u64;
                    }
                    let chunk_end = ChunkHeader::align(chunk.chunk_start_pos + data_size);
                    reader.seek(SeekFrom::Start(chunk_end))?;
                    continue;
                },
                b"bext" => {
                    Self::verify_none(&bext_chunk, &chunk.flag)?;
                    bext_chunk = Some(BextChunk::read(&mut reader, &savage_decoder)?);
                },
                b"smpl" => {
                    Self::verify_none(&smpl_chunk, &chunk.flag)?;
                    smpl_chunk = Some(SmplChunk::read(&mut reader)?);
                },
                b"inst" | b"INST" => {
                    Self::verify_none(&inst_chunk, &chunk.flag)?;
                    inst_chunk = Some(InstChunk::read(&mut reader)?);
                },
                b"cue " => {
                    Self::verify_none(&cue__chunk, &chunk.flag)?;
                    cue__chunk = Some(Cue_Chunk::read(&mut reader)?);
                },
                b"axml" => {
                    Self::verify_none(&axml_chunk, &chunk.flag)?;
                    axml_chunk = Some(read_str(&mut reader, chunk.size as usize, &savage_decoder)?);
                },
                b"ixml" => {
                    Self::verify_none(&ixml_chunk, &chunk.flag)?;
                    ixml_chunk = Some(read_str(&mut reader, chunk.size as usize, &savage_decoder)?);
                },
                b"LIST" => {
                    Self::verify_none(&list_chunk, &chunk.flag)?;
                    list_chunk = Some(ListChunk::read(&mut reader, chunk.size as u64, &savage_decoder)?);
                }
                b"acid" => {
                    Self::verify_none(&acid_chunk, &chunk.flag)?;
                    acid_chunk = Some(AcidChunk::read(&mut reader)?);
                },
                b"Trkn" => {
                    Self::verify_none(&trkn_chunk, &chunk.flag)?;
                    trkn_chunk = Some(read_str(&mut reader, chunk.size as usize, &savage_decoder)?);
                }
                other => {
                    println!("Unknown chunk in RIFF or RF64 chunk: {}", savage_decoder.decode_flags(&other));
                },
            }
            // 跳到下一个块的开始位置
            chunk.seek_to_next_chunk(&mut reader)?;
        }

        let fmt_chunk = match fmt_chunk {
            Some(fmt_chunk) => fmt_chunk,
            None => return Err(AudioReadError::DataCorrupted(String::from("the whole WAV file doesn't provide any \"data\" chunk")).into()),
        };

        let channel_mask = match fmt_chunk.extension {
            None => Spec::guess_channel_mask(fmt_chunk.channels)?,
            Some(extension) => extension.channel_mask,
        };

        let frame_size = fmt_chunk.block_align;
        let num_frames = data_size / frame_size as u64;
        let spec = Spec {
            channels: fmt_chunk.channels,
            channel_mask,
            sample_rate: fmt_chunk.sample_rate,
            bits_per_sample: fmt_chunk.bits_per_sample,
            sample_format: fmt_chunk.get_sample_format()?,
        };
        let new_data_source = match filesrc {
            Some(filename) => WaveDataSource::Filename(filename),
            None => WaveDataSource::Reader(reader),
        };
        let data_chunk = WaveDataReader::new(new_data_source, data_offset, data_size)?;
        Ok(Self {
            riff_len,
            spec,
            fmt_chunk,
            fact_data,
            data_offset,
            data_size,
            frame_size,
            num_frames,
            data_chunk,
            bext_chunk,
            smpl_chunk,
            inst_chunk,
            cue__chunk,
            axml_chunk,
            ixml_chunk,
            list_chunk,
            acid_chunk,
            trkn_chunk,
            junk_chunks,
            savage_decoder,
        })
    }

    pub fn spec(&self) -> &Spec{
        &self.spec
    }

    fn verify_none<T>(o: &Option<T>, flag: &[u8; 4]) -> Result<(), AudioReadError> {
        if o.is_some() {
            Err(AudioReadError::FormatError(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(flag))))
        } else {
            Ok(())
        }
    }

    // 创建迭代器。
    // 迭代器的作用是读取每个音频帧。
    // 但是嘞，这里有个问题： WaveReader 的创建方式有两种，一种是从 Read 创建，另一种是从文件创建。
    // 为了使迭代器的运行效率不至于太差，迭代器如果通过直接从 WaveReader 读取 body 的话，一旦迭代器太多，
    // 它就会疯狂 seek 然后读取，如果多个迭代器在多线程的情况下使用，绝对会乱套。
    // 因此，当 WaveReader 是从文件创建的，那可以给迭代器重新打开文件，让迭代器自己去 seek 和读取。
    // 而如果 WaveReader 是从 Read 创建的，那就创建临时文件，把 body 的内容转移到临时文件里，让迭代器使用。
    pub fn iter<S>(&mut self) -> Result<WaveIter<S>, Box<dyn Error>>
    where S: SampleType {
        Ok(WaveIter::<S>::new(BufReader::new(self.data_chunk.open()?), self.data_chunk.offset, self.spec.clone(), self.num_frames)?)
    }

    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&format!("riff_len   is {:?}\n", self.riff_len));
        ret.push_str(&format!("spec       is {:?}\n", self.spec));
        ret.push_str(&format!("fmt_chunk  is {:?}\n", self.fmt_chunk));
        ret.push_str(&format!("fact_data  is {:?}\n", self.fact_data));
        ret.push_str(&format!("data_offse is {:?}\n", self.data_offset));
        ret.push_str(&format!("data_size  is {:?}\n", self.data_size));
        ret.push_str(&format!("frame_size is {:?}\n", self.frame_size));
        ret.push_str(&format!("num_frames is {:?}\n", self.num_frames));
        ret.push_str(&format!("data_chunk is {:?}\n", self.data_chunk));
        ret.push_str(&format!("bext_chunk is {:?}\n", self.bext_chunk));
        ret.push_str(&format!("smpl_chunk is {:?}\n", self.smpl_chunk));
        ret.push_str(&format!("inst_chunk is {:?}\n", self.inst_chunk));
        ret.push_str(&format!("cue__chunk is {:?}\n", self.cue__chunk));
        ret.push_str(&format!("axml_chunk is {:?}\n", self.axml_chunk));
        ret.push_str(&format!("ixml_chunk is {:?}\n", self.ixml_chunk));
        ret.push_str(&format!("list_chunk is {:?}\n", self.list_chunk));
        ret.push_str(&format!("acid_chunk is {:?}\n", self.acid_chunk));
        ret.push_str(&format!("trkn_chunk is {:?}\n", self.trkn_chunk));
        ret.push_str(&format!("junk_chunks is {:?}\n", self.junk_chunks));
        ret
    }
}

// 莽夫式 PathBuf 转换为字符串
fn savage_path_buf_to_string(filepath: &Path) -> String {
    match filepath.to_str() {
        Some(pathstr) => pathstr.to_string(),
        None => format!("{:?}", filepath), // 要是不能转换成 UTF-8 字符串，那就爱怎么样怎么样吧
    }
}

#[derive(Debug)]
pub struct WaveDataReader {
    reader: Option<Box<dyn Reader>>,
    tempfile: Arc<File>,
    filepath: PathBuf,
    offset: u64,
}

impl WaveDataReader {
    // 从原始 WAV 肚子里抠出所有的 data 数据，然后找个临时文件位置存储。
    // 能得知临时文件的文件夹。
    fn new(file_source: WaveDataSource, data_offset: u64, data_size: u64) -> Result<Self, Box<dyn Error>> {
        let reader: Option<Box<dyn Reader>>;
        let tempfile = Arc::new(tempfile::tempfile()?);
        let filepath: Option<PathBuf>;
        let mut offset: u64 = 0;
        let mut have_source_file = false;
        match file_source {
            WaveDataSource::Reader(r) => {
                reader = Some(r);
                filepath = None;
            },
            WaveDataSource::Filename(path) => {
                let path = PathBuf::from(path);
                reader = Some(Box::new(BufReader::new(File::open(&path)?)));
                filepath = Some(path);
                offset = data_offset;
                have_source_file = true;
            },
            WaveDataSource::Unknown => return Err(AudioReadError::InvalidArguments(String::from("\"Unknown\" data source was given")).into()),
        };

        // 这个用来存储最原始的 Reader，如果最开始没有给 Reader 而是给了文件名，则存 None。
        let mut orig_reader: Option<Box<dyn Reader>> = None;

        // 把之前读到的东西都展开
        let mut reader = reader.unwrap();
        let filepath = filepath.unwrap();

        // 没有原始文件名，只有一个 Reader，那就从 Reader 那里把 WAV 文件肚子里的 data chunk 复制到一个临时文件里。
        if ! have_source_file {
            // 分段复制文件到临时文件里
            const BUFFER_SIZE: u64 = 81920;
            let mut buf = vec![0u8; BUFFER_SIZE as usize];

            let mut file = tempfile.clone();
            reader.seek(SeekFrom::Start(offset))?;

            // 按 BUFFER_SIZE 不断复制
            let mut to_move = data_size;
            while to_move >= BUFFER_SIZE {
                reader.read_exact(&mut buf)?;
                file.write_all(&buf)?;
                to_move -= BUFFER_SIZE;
            }
            // 复制最后剩下的
            if to_move != 0 {
                buf.resize(to_move as usize, 0);
                reader.read_exact(&mut buf)?;
                file.write_all(&buf)?;
            }

            // 这个时候，我们再把原始提供下来的 reader 收集起来存到结构体里
            orig_reader = Some(reader);

            #[cfg(debug_assertions)]
            println!("Using tempfile to store \"data\" chunk");
        }

        Ok(Self {
            reader: orig_reader,
            tempfile,
            filepath: filepath.into(),
            offset
        })
    }

    fn open(&self) -> Result<File, io::Error> {
        let mut file = File::open(&self.filepath)?;
        file.seek(SeekFrom::Start(self.offset))?;
        Ok(file)
    }
}

#[derive(Debug)]
pub struct WaveIter<S>
where S: SampleType {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64, // 数据的位置
    spec: Spec,
    frame_pos: u64, // 当前帧位置
    num_frames: u64, // 最大帧数量
    unpacker: fn(&mut BufReader<File>) -> Result<S, io::Error>,
    frame_size: u16,
}

impl<S> WaveIter<S>
where S: SampleType {
    fn new(reader: BufReader<File>, data_offset: u64, spec: Spec, num_frames: u64) -> Result<Self, Box<dyn Error>> {
        use WaveSampleType::{Unknown, S8, S16, S24, S32, S64, U8, U16, U24, U32, U64, F32, F64};
        let sample_type = get_sample_type(spec.bits_per_sample, spec.sample_format)?;
        let mut ret = Self {
            reader,
            data_offset,
            spec,
            frame_pos: 0,
            num_frames,
            unpacker: match sample_type {
                Unknown => return Err(AudioError::UnknownSampleType.into()),
                S8 =>  Self::unpack_to::<i8 >,
                S16 => Self::unpack_to::<i16>,
                S24 => Self::unpack_to::<i24>,
                S32 => Self::unpack_to::<i32>,
                S64 => Self::unpack_to::<i64>,
                U8 =>  Self::unpack_to::<u8 >,
                U16 => Self::unpack_to::<u16>,
                U24 => Self::unpack_to::<u24>,
                U32 => Self::unpack_to::<u32>,
                U64 => Self::unpack_to::<u64>,
                F32 => Self::unpack_to::<f32>,
                F64 => Self::unpack_to::<f64>,
            },
            frame_size: match sample_type {
                Unknown => 0,
                S8 =>  1,
                S16 => 2,
                S24 => 3,
                S32 => 4,
                S64 => 8,
                U8 =>  1,
                U16 => 2,
                U24 => 3,
                U32 => 4,
                U64 => 8,
                F32 => 4,
                F64 => 8,
            } * spec.channels,
        };
        ret.reader.seek(SeekFrom::Start(data_offset))?;
        Ok(ret)
    }

    fn seek_to_sample(&mut self, sample_pos: u64) -> Result<u64, io::Error> {
        self.reader.seek(SeekFrom::Start(self.data_offset + sample_pos * self.frame_size as u64))
    }

    fn unpack(&mut self) -> Result<S, io::Error> {
        (self.unpacker)(&mut self.reader)
    }

    fn unpack_to<T>(r: &mut BufReader<File>) -> Result<S, io::Error>
    where T: SampleType {
        Ok(S::from(T::read_le(r)?))
    }
}

impl<S> Iterator for WaveIter<S>
where S: SampleType {
    type Item = Vec<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_pos >= self.num_frames {return None;}

        let mut ret = Vec::<S>::with_capacity(self.spec.channels as usize);
        for _ in 0..self.spec.channels {
            match self.unpack() {
                Ok(sample) => ret.push(sample),
                Err(_) => return None,
            }
        }
        self.frame_pos += 1;
        Some(ret)
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.frame_pos += n as u64;
        match self.seek_to_sample(self.frame_pos) {
            Ok(_) => (),
            Err(_) => return None,
        }
        self.next()
    }
}
