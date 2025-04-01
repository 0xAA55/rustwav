#![allow(dead_code)]
#![allow(non_snake_case)]

use std::{fs::File, path::{Path, PathBuf}, io::{self, Read, Write, Seek, SeekFrom, BufReader, BufWriter}, error::Error};

use crate::errors::{AudioError, AudioReadError};
use crate::wavcore::{Spec};
use crate::wavcore::{ChunkHeader};
use crate::wavcore::{FmtChunk, BextChunk, SmplChunk, InstChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::wavcore::{guess_channel_mask};
use crate::decoders::{Decoder, PcmDecoder};
use crate::savagestr::{StringCodecMaps, SavageStringCodecs};
use crate::filehasher::FileHasher;
use crate::sampleutils::{SampleType};
use crate::readwrite::{Reader, StringIO::*};

#[cfg(feature = "mp3")]
use crate::decoders::MP3::Mp3Decoder;

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
    fmt__chunk: FmtChunk, // fmt 块，这个块一定会有
    fact_data: Option<u32>, // fact 块的参数
    data_offset: u64, // 音频数据的位置
    data_size: u64, // 音频数据的大小
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u64, // 总帧数
    data_chunk: WaveDataReader,
    text_encoding: Box<dyn SavageStringCodecs>,
    bext_chunk: Option<BextChunk>,
    smpl_chunk: Option<SmplChunk>,
    inst_chunk: Option<InstChunk>,
    cue__chunk: Option<CueChunk>,
    axml_chunk: Option<String>,
    ixml_chunk: Option<String>,
    list_chunk: Option<ListChunk>,
    acid_chunk: Option<AcidChunk>,
    trkn_chunk: Option<String>,
    id3__chunk: Option<Id3::Tag>,
    junk_chunks: Vec<JunkChunk>,
}

impl WaveReader {
    // 从文件路径打开
    pub fn open(file_source: &str) -> Result<Self, Box<dyn Error>> {
        Self::new(WaveDataSource::Filename(file_source.to_string()))
        // Self::new(WaveDataSource::Reader(Box::new(BufReader::new(File::open(file_source)?)))) // 测试临时文件
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

        let text_encoding: Box<dyn SavageStringCodecs> = Box::new(StringCodecMaps::new());

        let mut riff_len = 0u64;
        let mut riff_end = 0xFFFFFFFFu64; // 如果这个 WAV 文件是 RF64 的文件，此时给它临时设置一个很大的值，等到读取到 ds64 块时再更新这个值。
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
            },
            _ => return Err(AudioReadError::FormatError(String::from("Not a WAV file")).into()), // 根本不是 WAV
        }

        let start_of_riff = reader.stream_position()?;

        // 读完头部后，这里必须是 WAVE 否则不是音频文件。
        expect_flag(&mut reader, b"WAVE")?;

        let mut fmt__chunk: Option<FmtChunk> = None;
        let mut data_offset = 0u64;
        let mut fact_data: Option<u32> = None;
        let mut bext_chunk: Option<BextChunk> = None;
        let mut smpl_chunk: Option<SmplChunk> = None;
        let mut inst_chunk: Option<InstChunk> = None;
        let mut cue__chunk: Option<CueChunk> = None;
        let mut axml_chunk: Option<String> = None;
        let mut ixml_chunk: Option<String> = None;
        let mut list_chunk: Option<ListChunk> = None;
        let mut acid_chunk: Option<AcidChunk> = None;
        let mut trkn_chunk: Option<String> = None;
        let mut id3__chunk: Option<Id3::Tag> = None;
        let mut junk_chunks: Vec<JunkChunk>;

        junk_chunks = Vec::<JunkChunk>::new();

        // 循环处理 WAV 中的各种各样的小节
        while reader.stream_position()? < riff_end {
            let chunk = ChunkHeader::read(&mut reader)?;
            match &chunk.flag {
                b"JUNK" => {
                    let mut junk = vec![0; chunk.size as usize];
                    reader.read_exact(&mut junk)?;
                    junk_chunks.push(JunkChunk::from(junk));
                }
                b"fmt " => {
                    Self::verify_none(&fmt__chunk, &chunk.flag)?;
                    fmt__chunk = Some(FmtChunk::read(&mut reader, chunk.size)?);
                },
                b"fact" => {
                    fact_data = Some(u32::read_le(&mut reader)?);
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
                    bext_chunk = Some(BextChunk::read(&mut reader, &*text_encoding)?);
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
                    cue__chunk = Some(CueChunk::read(&mut reader)?);
                },
                b"axml" => {
                    Self::verify_none(&axml_chunk, &chunk.flag)?;
                    axml_chunk = Some(read_str(&mut reader, chunk.size as usize, &*text_encoding)?);
                },
                b"ixml" => {
                    Self::verify_none(&ixml_chunk, &chunk.flag)?;
                    ixml_chunk = Some(read_str(&mut reader, chunk.size as usize, &*text_encoding)?);
                },
                b"LIST" => {
                    Self::verify_none(&list_chunk, &chunk.flag)?;
                    list_chunk = Some(ListChunk::read(&mut reader, chunk.size as u64, &*text_encoding)?);
                }
                b"acid" => {
                    Self::verify_none(&acid_chunk, &chunk.flag)?;
                    acid_chunk = Some(AcidChunk::read(&mut reader)?);
                },
                b"Trkn" => {
                    Self::verify_none(&trkn_chunk, &chunk.flag)?;
                    trkn_chunk = Some(read_str(&mut reader, chunk.size as usize, &*text_encoding)?);
                }
                b"id3 " => {
                    Self::verify_none(&id3__chunk, &chunk.flag)?;
                    id3__chunk = Some(Id3::read(&mut reader, chunk.size as usize)?);
                },
                b"\0\0\0\0" => { // 空的 flag
                    return Err(AudioReadError::IncompleteFile.into());
                },
                // 曾经发现 BFDi 块，结果发现它是 BFD Player 生成的字符串块，里面大约是软件序列号之类的内容。
                // 所以此处就不记载 BFDi 块的信息了。
                other => {
                    println!("Unknown chunk in RIFF or RF64 chunk: {}", text_encoding.decode_flags(other));
                },
            }
            // 跳到下一个块的开始位置
            chunk.seek_to_next_chunk(&mut reader)?;
        }

        let fmt__chunk = match fmt__chunk {
            Some(fmt__chunk) => fmt__chunk,
            None => return Err(AudioReadError::DataCorrupted(String::from("the whole WAV file doesn't provide any \"data\" chunk")).into()),
        };

        let channel_mask = match fmt__chunk.extension {
            None => guess_channel_mask(fmt__chunk.channels)?,
            Some(extension) => extension.channel_mask,
        };

        let frame_size = fmt__chunk.block_align;
        let num_frames = data_size / frame_size as u64;
        let spec = Spec {
            channels: fmt__chunk.channels,
            channel_mask,
            sample_rate: fmt__chunk.sample_rate,
            bits_per_sample: fmt__chunk.bits_per_sample,
            sample_format: fmt__chunk.get_sample_format(),
        };
        let new_data_source = match filesrc {
            Some(filename) => WaveDataSource::Filename(filename),
            None => WaveDataSource::Reader(reader),
        };
        let data_chunk = WaveDataReader::new(new_data_source, data_offset, data_size)?;
        Ok(Self {
            riff_len,
            spec,
            fmt__chunk,
            fact_data,
            data_offset,
            data_size,
            frame_size,
            num_frames,
            data_chunk,
            text_encoding,
            bext_chunk,
            smpl_chunk,
            inst_chunk,
            cue__chunk,
            axml_chunk,
            ixml_chunk,
            list_chunk,
            acid_chunk,
            trkn_chunk,
            id3__chunk,
            junk_chunks,
        })
    }

    // 提供音频参数信息
    pub fn spec(&self) -> &Spec{
        &self.spec
    }

    // 提供乐曲信息元数据
    pub fn get_fmt__chunk(&self) -> &FmtChunk { &self.fmt__chunk }
    pub fn get_bext_chunk(&self) -> &Option<BextChunk> { &self.bext_chunk }
    pub fn get_smpl_chunk(&self) -> &Option<SmplChunk> { &self.smpl_chunk }
    pub fn get_inst_chunk(&self) -> &Option<InstChunk> { &self.inst_chunk }
    pub fn get_cue__chunk(&self) -> &Option<CueChunk> { &self.cue__chunk }
    pub fn get_axml_chunk(&self) -> &Option<String> { &self.axml_chunk }
    pub fn get_ixml_chunk(&self) -> &Option<String> { &self.ixml_chunk }
    pub fn get_list_chunk(&self) -> &Option<ListChunk> { &self.list_chunk }
    pub fn get_acid_chunk(&self) -> &Option<AcidChunk> { &self.acid_chunk }
    pub fn get_trkn_chunk(&self) -> &Option<String> { &self.trkn_chunk }
    pub fn get_id3__chunk(&self) -> &Option<Id3::Tag> { &self.id3__chunk }
    pub fn get_junk_chunks(&self) -> &Vec<JunkChunk> { &self.junk_chunks }

    // 创建迭代器。
    // 迭代器的作用是读取每个音频帧。
    // 但是嘞，这里有个问题： WaveReader 的创建方式有两种，一种是从 Read 创建，另一种是从文件创建。
    // 为了使迭代器的运行效率不至于太差，迭代器如果通过直接从 WaveReader 读取 body 的话，一旦迭代器太多，
    // 它就会疯狂 seek 然后读取，如果多个迭代器在多线程的情况下使用，绝对会乱套。
    // 因此，当 WaveReader 是从文件创建的，那可以给迭代器重新打开文件，让迭代器自己去 seek 和读取。
    // 而如果 WaveReader 是从 Read 创建的，那就创建临时文件，把 body 的内容转移到临时文件里，让迭代器使用。
    pub fn iter<S>(&mut self) -> Result<WaveIter<S>, Box<dyn Error>>
    where S: SampleType {
        WaveIter::<S>::new(BufReader::new(self.data_chunk.open()?), self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, match self.fact_data {
            None => 0,
            Some(fact) => fact,
        })
    }

    // 用于检测特定 Chunk 是否有被重复读取的情况，有就报错
    fn verify_none<T>(o: &Option<T>, flag: &[u8; 4]) -> Result<(), AudioReadError> {
        if o.is_some() {
            Err(AudioReadError::FormatError(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(flag))))
        } else {
            Ok(())
        }
    }
}

// 莽夫式 PathBuf 转换为字符串
fn savage_path_buf_to_string(filepath: &Path) -> String {
    match filepath.to_str() {
        Some(pathstr) => pathstr.to_string(),
        None => format!("{:?}", filepath), // 要是不能转换成 UTF-8 字符串，那就爱怎么样怎么样吧
    }
}

pub fn expect_flag<T: Read>(r: &mut T, flag: &[u8; 4]) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    if &buf != flag {
        Err(AudioReadError::UnexpectedFlag(String::from_utf8_lossy(flag).to_string(), String::from_utf8_lossy(&buf).to_string()).into())
    } else {
        Ok(())
    }
}

#[derive(Debug)]
struct WaveDataReader {
    reader: Option<Box<dyn Reader>>,
    tempdir: Option<tempfile::TempDir>,
    filepath: PathBuf,
    offset: u64,
    length: u64,
    datahash: u64,
}

impl WaveDataReader {
    // 从原始 WAV 肚子里抠出所有的 data 数据，然后找个临时文件位置存储。
    // 能得知临时文件的文件夹。
    fn new(file_source: WaveDataSource, data_offset: u64, data_size: u64) -> Result<Self, Box<dyn Error>> {
        let reader: Option<Box<dyn Reader>>;
        let filepath: Option<PathBuf>;
        let tempdir: Option<tempfile::TempDir>;
        let mut hasher = FileHasher::new();
        let datahash: u64;
        let offset: u64;
        let have_source_file: bool;
        match file_source {
            WaveDataSource::Reader(mut r) => {
                // 只有读取器，没有源文件名
                datahash = hasher.hash(&mut r, data_offset, data_size)?;
                reader = Some(r);
                let tdir = tempfile::TempDir::new()?;
                filepath = Some(tdir.path().join(&format!("{:x}.tmp", datahash)));
                tempdir = Some(tdir);
                offset = 0;
                have_source_file = false;
            },
            WaveDataSource::Filename(path) => {
                // 有源文件名，因此打开文件
                let path = PathBuf::from(path);
                let mut r = Box::new(BufReader::new(File::open(&path)?));
                datahash = hasher.hash(&mut r, data_offset, data_size)?;
                reader = Some(r);
                tempdir = None;
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

            // 根据临时文件名创建临时文件
            let mut file = BufWriter::new(File::create(&filepath)?);
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
            println!("Using tempfile to store \"data\" chunk: {}", filepath);
        }

        Ok(Self {
            reader: orig_reader,
            tempdir,
            filepath,
            offset,
            length: data_size,
            datahash,
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
    data_offset: u64, // 音频数据在文件中的位置
    data_length: u64, // 音频数据的总大小
    spec: Spec,
    fact: u32, // fact 数据，部分解码器需要
    frame_pos: u64, // 当前帧位置
    decoder: Box<dyn Decoder<S>>, // 解码器
}

impl<S> WaveIter<S>
where S: SampleType {
    fn new(mut reader: BufReader<File>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact: u32) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_offset,
            data_length,
            spec: spec.clone(),
            fact,
            frame_pos: 0,
            decoder: match fmt.format_tag {
                1 | 0xFFFE | 3 => Box::new(PcmDecoder::<S>::new(reader, data_offset, data_length, spec, fmt)?),
                0x0055 => {
                    #[cfg(not(feature = "mp3"))]
                    return Err(AudioError::Unimplemented(String::from("not implemented for decoding MP3 audio data inside the WAV file")).into());
                    #[cfg(feature = "mp3")]
                    {Box::new(Mp3Decoder::new(reader, data_offset, data_length, fmt)?)}
                },
                0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => { // Ogg Vorbis 数据
                    return Err(AudioError::Unimplemented(String::from("not implemented for decoding ogg vorbis audio data inside the WAV file")).into());
                },
                0xF1AC => { // FLAG
                    return Err(AudioError::Unimplemented(String::from("not implemented for decoding FLAC audio data inside the WAV file")).into());
                },
                other => return Err(AudioReadError::Unimplemented(format!("0x{:x}", other)).into()),
            },
        })
    }
}

impl<S> Iterator for WaveIter<S>
where S: SampleType {
    type Item = Vec<S>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = match self.decoder.decode() {
            Ok(sample) => Some(sample),
            Err(_) => None,
        };
        self.frame_pos += 1;
        ret
    }
}
