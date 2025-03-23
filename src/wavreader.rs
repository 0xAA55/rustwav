use std::{fs::File, path::Path, io::{self, Read, Write, Seek, SeekFrom, BufReader}, error::Error, collections::HashMap};

use tempfile::TempDir;

#[allow(unused_imports)]
pub use crate::errors::*;

#[allow(unused_imports)]
pub use crate::wavcore::*;

use crate::sampleutils::*;
use crate::filehasher::FileHasher;
use crate::audiocore::{SampleFormat, Spec};
use crate::audioreader::{AudioReader};

pub trait Reader: Read + Seek {}
impl<T> Reader for T
where T: Read + Seek {}

#[derive(Debug)]
pub enum FileSource {
    Reader(Reader),
    Filename(String),
}

#[derive(Debug)]
pub struct WaveReader {
    filesrc: FileSource,
    riff_len: u64,
    spec: Spec,
    fmt_chunk: fmt_Chunk, // fmt 块，这个块一定会有
    fact_data: u32, // fact 块的参数
    data_offset: u64, // 音频数据的位置
    data_size: u64, // 音频数据的大小
    data_hash: u64, // 音频数据哈希值
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u64, // 总帧数
    bwav_chunk: Option<BWAVChunk>,
    smpl_chunk: Option<SMPLChunk>,
    inst_chunk: Option<INSTChunk>,
    cue__chunk: Option<Cue_Chunk>,
    axml_chunk: Option<Vec<u8>>,
    ixml_chunk: Option<Vec<u8>>,
    list_chunk: Option<LISTChunk>,
    data_chunk: WaveDataReader,
}

impl WaveReader {
    // 从文件路径打开
    pub fn open(file_source: &str) -> Result<Self, Box<dyn Error>> {
        Self::new(FileSource::Filename(file_source.to_string()))
    }

    // 从读取器打开
    pub fn new(file_source: FileSource) -> Result<Self, Box<dyn Error>> {
        let filepath = Option::<String>::None;
        let mut reader = match file_source {
            FileSource::Reader(reader) => {
                filepath = None;
                reader
            },
            FileSource::Filename(filename) => {
                filepath = Some(filename);
                BufReader::new(File::open(filename)?)
            },
        };
        let mut riff_len = 0u64;
        let mut riff_end = 0u64;
        let mut isRF64 = false;
        let mut data_size = 0u64;

        // 先搞定最开始的头部，有 RIFF 头和 RF64 头，需要分开处理
        let chunk = Chunk::read(&mut reader)?;
        match &chunk.flag {
            b"RIFF" => {
                riff_len = chunk.size as u64;
                riff_end = reader.stream_position()? + riff_len;
            },
            b"RF64" => {
                isRF64 = true;
                let _rf64_size = u32::read_le(&mut reader)?;
            },
            _ => return Err(AudioReadError::FormatError.into()), // 根本不是 WAV
        }

        let start_of_riff = reader.stream_position()?;

        // 读完头部后，这里必须是 WAVE 否则不是音频文件。
        expect_flag(&mut reader, b"WAVE", AudioReadError::FormatError.into())?;

        // 如果是 RF64 头，此处有 ds64 节
        let chunk = Chunk::read(&mut reader)?;
        if isRF64 {
            if &chunk.flag != b"ds64" || chunk.size < 28 {
                return Err(AudioReadError::DataCorrupted.into());
            }
            riff_len = u64::read_le(&mut reader)?;
            data_size = u64::read_le(&mut reader)?;
            riff_end = start_of_riff + riff_len;
            chunk.seek_to_next_chunk(&mut reader)?;
        }

        let mut fmt_chunk: Option<fmt_Chunk> = None;
        let mut data_offset = 0u64;
        let mut fact_data = 0;
        let mut bwav_chunk: Option<BWAVChunk> = None;
        let mut smpl_chunk: Option<SMPLChunk> = None;
        let mut inst_chunk: Option<INSTChunk> = None;
        let mut cue__chunk: Option<Cue_Chunk> = None;
        let mut axml_chunk: Option<Vec<u8>> = None;
        let mut ixml_chunk: Option<Vec<u8>> = None;
        let mut list_chunk: Option<LISTChunk> = None;

        // 循环处理 WAV 中的各种各样的小节
        while reader.stream_position()? < riff_end {
            let chunk = Chunk::read(&mut reader)?;
            match &chunk.flag {
                // 注意这里会自动跳过 JUNK 节，因此没办法处理 JUNK 节里面的数据
                b"fmt " => {
                    fmt_chunk = Some(fmt_Chunk::read(&mut reader, chunk.size)?);
                },
                b"fact" => {
                    fact_data = u32::read_le(&mut reader)?;
                },
                b"data" => {
                    data_offset = chunk.chunk_start_pos;
                    if !isRF64 {
                        data_size = chunk.size as u64;
                    }
                    let chunk_end = Chunk::align(chunk.chunk_start_pos + data_size);
                    reader.seek(SeekFrom::Start(chunk_end))?;
                    continue;
                },
                b"bext" => {
                    bwav_chunk = Some(BWAVChunk::read(&mut reader)?);
                },
                b"smpl" => {
                    smpl_chunk = Some(SMPLChunk::read(&mut reader)?);
                },
                b"inst" | b"INST" => {
                    inst_chunk = Some(INSTChunk::read(&mut reader)?);
                },
                b"cue " => {
                    cue__chunk = Some(Cue_Chunk::read(&mut reader)?);
                },
                b"axml" => {
                    let mut data = Vec::<u8>::new();
                    data.resize(chunk.size as usize, 0);
                    reader.read_exact(&mut data)?;
                    axml_chunk = Some(data);
                },
                b"ixml" => {
                    let mut data = Vec::<u8>::new();
                    data.resize(chunk.size as usize, 0);
                    reader.read_exact(&mut data)?;
                    ixml_chunk = Some(data);
                },
                b"LIST" => {
                    list_chunk = Some(LISTChunk::read(&mut reader)?);
                }
                other => {
                    println!("Unknown chunk in RIFF or RF64 chunk: {:?}", other);
                },
            }
            // 跳到下一个块的开始位置
            chunk.seek_to_next_chunk(&mut reader)?;
        }

        let fmt_chunk = match fmt_chunk {
            Some(fmt_chunk) => fmt_chunk,
            None => return Err(AudioReadError::DataCorrupted.into()),
        };

        let channel_mask = match fmt_chunk.extension {
            None => Spec::guess_channel_mask(fmt_chunk.channels)?,
            Some(extension) => extension.channel_mask,
        };

        let mut hasher = FileHasher::new();
        let data_hash = hasher.hash(&mut reader, data_offset, data_size)?;

        let frame_size = fmt_chunk.block_align;
        let num_frames = data_size / frame_size as u64;
        let spec = Spec {
            channels: fmt_chunk.channels,
            channel_mask,
            sample_rate: fmt_chunk.sample_rate,
            bits_per_sample: fmt_chunk.bits_per_sample,
            sample_format: fmt_chunk.get_sample_format()?,
        };
        let data_chunk = WaveDataReader::new(file_source, data_offset, data_size, data_hash)?;
        Ok(Self {
            filesrc: file_source,
            riff_len,
            spec,
            fmt_chunk,
            fact_data,
            data_offset,
            data_size,
            data_hash,
            frame_size,
            num_frames,
            bwav_chunk,
            smpl_chunk,
            inst_chunk,
            cue__chunk,
            axml_chunk,
            ixml_chunk,
            list_chunk,
            data_chunk,
        })
    }



    // 创建迭代器。
    // 迭代器的作用是读取每个音频帧。
    // 但是嘞，这里有个问题： WaveReader 的创建方式有两种，一种是从 Read 创建，另一种是从文件创建。
    // 为了使迭代器的运行效率不至于太差，迭代器如果通过直接从 WaveReader 读取 body 的话，一旦迭代器太多，
    // 它就会疯狂 seek 然后读取，如果多个迭代器在多线程的情况下使用，绝对会乱套。
    // 因此，当 WaveReader 是从文件创建的，那可以给迭代器重新打开文件，让迭代器自己去 seek 和读取。
    // 而如果 WaveReader 是从 Read 创建的，那就创建临时文件，把 body 的内容转移到临时文件里，让迭代器使用。
    pub fn CreateIter<S>(&mut self) -> Result<WaveIter<S>, Box<dyn Error>>
    where S: SampleConv {

        // 打开文件，不论是打开原有的 WAV 还是生成一个会自动删除的临时文件
        let reader = BufReader::new(self.data_chunk.open()?);

        let spec = self.spec.clone();
        let max_frames = self.num_frames;

        Ok(WaveIter::<S>::new(reader, self.data_chunk.offset, spec, max_frames)?)
    }
}

#[derive(Debug)]
pub struct WaveDataReader {
    temp_dir: Option<TempDir>,
    filepath: Box<Path>,
    offset: u64,
}

impl WaveDataReader {
    // 从原始 WAV 肚子里抠出所有的 data 数据，然后找个临时文件位置存储。
    // 能得知临时文件的文件夹。
    fn new(file_source: FileSource, data_offset: u64, data_size: u64, data_hash: u64) -> Result<Self, io::Error> {
        let mut temp_dir: Option<TempDir> = None;
        let (reader, filepath, offset) = match file_source {

            // 有读取器的情况，生成临时文件存储它的 data
            FileSource::Reader(reader) => {
                temp_dir = match TempDir::new() {
                    Ok(temp_dir) => Some(temp_dir),
                    Err(err) => return Err(err),
                };
                (reader, temp_dir.unwrap().path().join(format!("{:x}.tmp", data_hash)), 0u64)
            },

            // 没有临时文件的情况，直接读取这个文件本身
            FileSource::Filename(filepath) => {
                let filepath = Path::new(&filepath);
                (BufReader::new(File::open(filepath)?), filepath, data_offset)
            },
        };

        if let Some(_) = temp_dir {
            const BUFFER_SIZE: u64 = 81920;
            let mut buf = vec![0u8; BUFFER_SIZE as usize];

            let mut file = File::create(filepath)?;
            reader.seek(SeekFrom::Start(data_offset))?;

            let mut to_move = data_size;
            while to_move >= BUFFER_SIZE {
                reader.read_exact(&mut buf)?;
                file.write_all(&buf)?;
                to_move -= BUFFER_SIZE;
            }
            if to_move != 0 {
                buf.resize(to_move as usize, 0);
                reader.read_exact(&mut buf)?;
                file.write_all(&buf)?;
            }
        }

        Ok(Self {
            temp_dir,
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

struct WaveIter<S>
where S: SampleConv {
    reader: BufReader<File>, // 数据读取器
    data_offset: u64, // 数据的位置
    spec: Spec,
    frame_pos: u64, // 当前帧位置
    max_frames: u64, // 最大帧数量
    unpacker: SampleReader<S>,
}

impl<S> WaveIter<S>
where S: SampleConv {
    fn new(reader: BufReader<File>, data_offset: u64, spec: Spec, max_frames: u64) -> Result<Self, AudioError> {
        let ret = Self {
            reader,
            data_offset,
            spec,
            frame_pos: 0,
            max_frames,
            unpacker: SampleReader::<S>::new(reader, get_sample_type(spec.bits_per_sample, spec.sample_format)?)?,
        };
        Ok(ret)
    }
}

struct SampleReader<S>
where S: SampleConv {
    reader: BufReader<File>,
    get_sample_func: fn(&mut BufReader<File>) -> Result<S, io::Error>,
}

impl<S> SampleReader<S>
where S: SampleConv {
    fn new(reader: BufReader<File>, sample_type: WaveSampleType) -> Result<Self, AudioError> {
        use WaveSampleType::{U8, S16, S24, S32, F32, F64};
        Ok(Self {
            reader,
            get_sample_func: {
                match sample_type {
                    U8 =>  Self::get_sample_var::<u8 >,
                    S16 => Self::get_sample_var::<i16>,
                    S24 => Self::get_sample_var::<i24>,
                    S32 => Self::get_sample_var::<i32>,
                    F32 => Self::get_sample_var::<f32>,
                    F64 => Self::get_sample_var::<f64>,
                }
            },
        })
    }

    fn get_sample(&self) -> Result<S, io::Error> {
        (self.get_sample_func)(&mut self.reader)
    }

    fn get_sample_var<T>(r: &mut BufReader<File>) -> Result<S, io::Error>
    where T: SampleConv {
        Ok(S::from(T::read_le(r)?))
    }
}

impl<S> Iterator for WaveIter<S>
where S: SampleConv {
    type Item = Vec<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_pos >= self.max_frames {return None;}
        self.frame_pos += 1;

        let mut ret = Vec::<S>::with_capacity(self.spec.channels as usize);
        for _ in 0..self.spec.channels {
            match self.unpacker.get_sample() {
                Ok(sample) => ret.push(sample),
                Err(_) => return None,
            }
        }
        Some(ret)
    }
}

impl AudioReader for WaveReader where Self: Sized {
    fn spec(&self) -> &Spec{
        return &self.spec;
    }

    fn iter<T>(&mut self) -> Result<WaveIter<T>, Box<dyn Error>>
    where Self: Sized,
          T: SampleConv {
        Ok(self.CreateIter::<T>()?)
    }
}

#[derive(Clone, Copy, Debug)]
struct Chunk {
    flag: [u8; 4], // 实际存储在文件里的
    size: u32, // 实际存储在文件里的
    chunk_start_pos: u64, // Chunk 内容在文件中的位置，不包含 Chunk 头
}

impl Chunk {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        // 读取 WAV 中的每个块
        // 注意 WAV 中会有 JUNK 块，目前的做法就是跳过所有的 JUNK 块。
        // 在 AVI 里面，JUNK 块里面会包含重要信息，但是 WAV 我就管它丫的了。
        let mut flag = [0u8; 4];
        let mut size : u32;
        loop {
            reader.read_exact(&mut flag)?;
            size = u32::read_le(reader)?;
            if &flag == b"JUNK" {
                reader.seek(SeekFrom::Current(size.into()))?;
            } else {
                break;
            }
        }
        Ok(Self {
            flag,
            size,
            chunk_start_pos: reader.stream_position()?
        })
    }

    fn align(addr: u64) -> u64 {
        addr + (addr & 1)
    }

    fn next_chunk_pos(&self) -> u64 {
        Self::align(self.chunk_start_pos + self.size as u64)
    }

    fn seek_to_next_chunk<R>(&self, reader: &mut R) -> Result<u64, io::Error>
    where R: Reader {
        reader.seek(SeekFrom::Start(self.next_chunk_pos()))
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
struct fmt_Chunk {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
    extension: Option<fmt_Chunk_Extension>,
}

#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
struct fmt_Chunk_Extension {
    ext_len: u16,
    bits_per_sample: u16,
    channel_mask: u32,
    sub_format: GUID,
}

impl fmt_Chunk {
    fn read<R>(reader: &mut R, chunk_size: u32) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let mut ret = fmt_Chunk{
            format_tag: u16::read_le(reader)?,
            channels: u16::read_le(reader)?,
            sample_rate: u32::read_le(reader)?,
            byte_rate: u32::read_le(reader)?,
            block_align: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            extension: None,
        };
        match ret.format_tag {
            0xFFFE => {
                if chunk_size >= 40 {
                    ret.extension = Some(fmt_Chunk_Extension::read(reader)?);
                }
            },
            0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => {
                // Ogg Vorbis 数据
                return Err(AudioError::Unimplemented.into());
            },
        }
        Ok(ret)
    }

    fn get_sample_format(&self) -> Result<SampleFormat, AudioError> {
        use SampleFormat::{Int, UInt, Float};
        match (self.format_tag, self.bits_per_sample) {
            (1, 8) => Ok(UInt),
            (1, 16) => Ok(Int),
            (0xFFFE, 24) => Ok(Int),
            (0xFFFE, 32) => {
                match self.extension {
                    Some(extension) => {
                        match extension.sub_format {
                            guid_pcm_format => Ok(Int),
                            guid_ieee_float_format => Ok(Float),
                            _ => Err(AudioError::Unimplemented),
                        }
                    },
                    None => Ok(Int),
                }
            },
            (3, 32) => Ok(Float),
            (3, 46) => Ok(Float),
            _ => Err(AudioError::Unimplemented),
        }
    }
}

impl fmt_Chunk_Extension {
    fn read<R>(reader: &mut R) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        Ok(Self{
            ext_len: u16::read_le(reader)?,
            bits_per_sample: u16::read_le(reader)?,
            channel_mask: u32::read_le(reader)?,
            sub_format: GUID::read(&mut reader)?,
        })
    }
}


#[derive(Debug, Clone)]
struct BWAVChunk {
    description: String,
    originator: String,
    originatorRef: String,
    originationDate: String,
    originationTime: String,
    timeRef: u64,
    version: u16,
    umid: [u8; 64],
    reserved: [u8; 190],
    coding_history: [u8; 1],
}

impl BWAVChunk {
    fn read<R>(reader: &mut R) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let description = read_str(reader, 256)?;
        let originator = read_str(reader, 32)?;
        let originatorRef = read_str(reader, 32)?;
        let originationDate = read_str(reader, 10)?;
        let originationTime = read_str(reader, 8)?;
        let timeRef = u64::read_le(reader)?;
        let version = u16::read_le(reader)?;
        let mut umid = [0u8; 64];
        let mut reserved = [0u8; 190];
        let mut coding_history = [0u8; 1];
        reader.read_exact(&mut umid)?;
        reader.read_exact(&mut reserved)?;
        reader.read_exact(&mut coding_history)?;
        Ok(Self {
            description,
            originator,
            originatorRef,
            originationDate,
            originationTime,
            timeRef,
            version,
            umid,
            reserved,
            coding_history,
        })
    }
}

#[derive(Debug, Clone)]
struct SMPLChunk {
    manufacturer: u32,
    product: u32,
    samplePeriod: u32,
    midiUnityNote: u32,
    midiPitchFraction: u32,
    smpteFormat: u32,
    smpteOffset: u32,
    numSampleLoops: u32,
    samplerData: u32,
    loops: Vec<SMPLSampleLoop>,
}

#[derive(Debug, Clone, Copy)]
struct SMPLSampleLoop {
    identifier: u32,
    type_: u32,
    start: u32,
    end: u32,
    fraction: u32,
    playCount: u32,
}

impl SMPLChunk {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        let mut ret = Self{
            manufacturer: u32::read_le(reader)?,
            product: u32::read_le(reader)?,
            samplePeriod: u32::read_le(reader)?,
            midiUnityNote: u32::read_le(reader)?,
            midiPitchFraction: u32::read_le(reader)?,
            smpteFormat: u32::read_le(reader)?,
            smpteOffset: u32::read_le(reader)?,
            numSampleLoops: u32::read_le(reader)?,
            samplerData: u32::read_le(reader)?,
            loops: Vec::<SMPLSampleLoop>::new(),
        };
        for _ in 0..ret.numSampleLoops {
            ret.loops.push(SMPLSampleLoop::read(reader)?);
        }
        Ok(ret)
    }
}

impl SMPLSampleLoop {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        Ok(Self{
            identifier: u32::read_le(reader)?,
            type_: u32::read_le(reader)?,
            start: u32::read_le(reader)?,
            end: u32::read_le(reader)?,
            fraction: u32::read_le(reader)?,
            playCount: u32::read_le(reader)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct INSTChunk {
    baseNote: u8,
    detune: u8,
    gain: u8,
    lowNote: u8,
    highNote: u8,
    lowVelocity: u8,
    highVelocity: u8,
}

impl INSTChunk {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        Ok(Self{
            baseNote: u8::read_le(reader)?,
            detune: u8::read_le(reader)?,
            gain: u8::read_le(reader)?,
            lowNote: u8::read_le(reader)?,
            highNote: u8::read_le(reader)?,
            lowVelocity: u8::read_le(reader)?,
            highVelocity: u8::read_le(reader)?,
        })
    }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
struct Cue_Chunk {
    num_cues: u32,
    cues: Vec<Cue>,
}

#[derive(Debug, Clone, Copy)]
struct Cue {
    identifier: u32,
    order: u32,
    chunkID: u32,
    chunkStart: u32,
    blockStart: u32,
    offset: u32,
}

impl Cue_Chunk {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        let mut ret = Cue_Chunk {
            num_cues: u32::read_le(reader)?,
            cues: Vec::<Cue>::new(),
        };
        for _ in 0..ret.num_cues {
            ret.cues.push(Cue::read(reader)?);
        }
        Ok(ret)
    }
}

impl Cue {
    fn read<R>(reader: &mut R) -> Result<Self, io::Error>
    where R: Reader {
        Ok(Self{
            identifier: u32::read_le(reader)?,
            order: u32::read_le(reader)?,
            chunkID: u32::read_le(reader)?,
            chunkStart: u32::read_le(reader)?,
            blockStart: u32::read_le(reader)?,
            offset: u32::read_le(reader)?,
        })
    }
}

#[derive(Debug, Clone)]
struct LISTChunk { // https://www.recordingblogs.com/wiki/list-chunk-of-a-wave-file
    info: Option<HashMap<[u8; 4], String>>,
    adtl: Option<AdtlChunk>,
}

impl LISTChunk {
    fn read<R>(reader: &mut R) -> Result<Self, Box<dyn Error>>
    where R: Reader {
        let mut info: Option<HashMap<[u8; 4], String>> = None;
        let mut adtl: Option<AdtlChunk> = None;
        let sub_chunk = Chunk::read(reader)?;
        let end_of_chunk = sub_chunk.next_chunk_pos();
        match &sub_chunk.flag {
            b"info" | b"INFO" => {
                // INFO 节其实是很多键值对，用来标注歌曲信息。在它的字节范围的限制下，读取所有的键值对。
                let mut dict = HashMap::<[u8; 4], String>::new();
                while reader.stream_position()? < end_of_chunk {
                    let key_chunk = Chunk::read(reader)?; // 每个键其实就是一个 Chunk，它的大小值就是字符串大小值。
                    let value_str = read_str(reader, key_chunk.size as usize)?;
                    dict.insert(key_chunk.flag, value_str);
                    key_chunk.seek_to_next_chunk(reader)?;
                }
                info = Some(dict);
            },
            b"adtl" => {
                let sub_chunk = Chunk::read(reader)?;
                let mut labl: Option<LABLChunk> = None;
                let mut ltxt: Option<LTXTChunk> = None;
                match &sub_chunk.flag {
                    b"labl" | b"note" => {
                        labl = Some(LABLChunk{
                            cue_point_id: u32::read_le(reader)?,
                            data: read_sz(reader)?,
                        });
                    },
                    b"ltxt" => {
                        ltxt = Some(LTXTChunk{
                            cue_point_id: u32::read_le(reader)?,
                            sample_length: u32::read_le(reader)?,
                            purpose_id: read_str(reader, 4)?,
                            country: u16::read_le(reader)?,
                            language: u16::read_le(reader)?,
                            dialect: u16::read_le(reader)?,
                            code_page: u16::read_le(reader)?,
                            data: read_sz(reader)?,
                        });
                    },
                    other => {
                        println!("Unknown chunk in adtl chunk: {:?}", other);
                    },
                }
                adtl = Some(AdtlChunk{labl, ltxt});
            },
            other => {
                println!("Unknown chunk in LIST chunk: {:?}", other);
            },
        }
        sub_chunk.seek_to_next_chunk(reader)?;
        Ok(Self{
            info,
            adtl,
        })
    }
}

#[derive(Debug, Clone)]
struct AdtlChunk {
    labl: Option<LABLChunk>,
    ltxt: Option<LTXTChunk>,
}

#[derive(Debug, Clone)]
struct LABLChunk {
    cue_point_id: u32,
    data: String,
}

#[derive(Debug, Clone)]
struct LTXTChunk {
    cue_point_id: u32,
    sample_length: u32,
    purpose_id: String,
    country: u16,
    language: u16,
    dialect: u16,
    code_page: u16,
    data: String,
}
