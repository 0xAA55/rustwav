use std::{fs::File, {path::Path}, io::{self, BufReader, Write}, error::Error, collections::HashMap};

use tempfile::tempfile;

#[allow(unused_imports)]
pub use crate::errors::*;

#[allow(unused_imports)]
pub use crate::wavcore::*;

use crate::sampleutils::*;
use crate::structread::{Reader, StructRead};
use crate::audiocore::{SampleFormat, Spec};
use crate::audioreader::{AudioReader, AudioIter};

pub struct WaveReader {
    filepath: Option<Box<Path>>,
    reader: StructRead,
    spec: Spec,
    fmt_chunk: fmt_Chunk, // fmt 块，这个块一定会有
    fact_data: u32, // fact 块的参数
    data_offset: u64, // 音频数据的位置
    data_size: u64, // 音频数据的大小
    frame_size: u16, // 每一帧音频的字节数
    num_frames: u64, // 总帧数
    bwav_chunk: Option<BWAVChunk>,
    smpl_chunk: Option<SMPLChunk>,
    inst_chunk: Option<INSTChunk>,
    cue__chunk: Option<Cue_Chunk>,
    axml_chunk: Option<Vec<u8>>,
    ixml_chunk: Option<Vec<u8>>,
    list_chunk: Option<LISTChunk>,
}

impl WaveReader {
    pub fn new(&mut reader: Reader) -> Result<WaveReader, Box<dyn Error>> {
        let mut reader = StructRead::new(reader);

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
                let _rf64_size = reader.read_le_u32()?;
            },
            _ => return Err(AudioReadError::FormatError.into()), // 根本不是 WAV
        }

        let start_of_riff = reader.stream_position()?;

        // 读完头部后，这里必须是 WAVE 否则不是音频文件。
        reader.expect_flag(b"WAVE", AudioReadError::FormatError.into())?;

        // 如果是 RF64 头，此处有 ds64 节
        let chunk = Chunk::read(&mut reader)?;
        if isRF64 {
            if &chunk.flag != b"ds64" || chunk.size < 28 {
                return Err(AudioReadError::DataCorrupted.into());
            }
            riff_len = reader.read_le_u64()?;
            data_size = reader.read_le_u64()?;
            riff_end = start_of_riff + riff_len;
            chunk.seek_to_next_chunk(&mut reader)?;
        }

        let mut fmt_chunk: Option<fmt_Chunk> = None;
        let mut data_offset = 0u64;
        let mut channel_mask = 0;
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
                    fact_data = reader.read_le_u32()?;
                },
                b"data" => {
                    data_offset = chunk.chunk_start_pos;
                    if !isRF64 {
                        data_size = chunk.size as u64;
                    }
                    let chunk_end = Chunk::align(chunk.chunk_start_pos + data_size);
                    reader.seek_to(chunk_end)?;
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
                    let data = Vec::<u8>::new();
                    data.resize(chunk.size as usize, 0);
                    reader.read_exact(&mut data)?;
                    axml_chunk = Some(data);
                },
                b"ixml" => {
                    let data = Vec::<u8>::new();
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

        let frame_size = fmt_chunk.block_align;
        let num_frames = data_size / frame_size as u64;
        let fmt_chunk = fmt_chunk;
        Ok(Self {
            filepath: None,
            reader,
            spec: Spec {
                channels: fmt_chunk.channels,
                channel_mask,
                sample_rate: fmt_chunk.sample_rate,
                bits_per_sample: fmt_chunk.bits_per_sample,
                sample_format: fmt_chunk.get_sample_format()?,
            },
            fmt_chunk,
            fact_data,
            data_offset,
            data_size,
            frame_size,
            num_frames,
            bwav_chunk,
            smpl_chunk,
            inst_chunk,
            cue__chunk,
            axml_chunk,
            ixml_chunk,
            list_chunk,
        })
    }
}

// 用文件来读取的方式，自动套上 BufReader 来提升读取效率
impl WaveReader {
    // 从文件打开一个 WaveReader，因为有文件名，所以可以记录文件名
    pub fn open(filename: &Path) -> Result<WaveReader, Box<dyn Error>> {
        let file = File::open(filename)?;
        let buf_reader = BufReader::new(file);
        let mut ret = WaveReader::new(buf_reader)?;
        ret.filepath = Some(filename.into());
        Ok(ret)
    }

    // 创建迭代器。
    // 迭代器的作用是读取每个音频帧。
    // 但是嘞，这里有个问题： WaveReader 的创建方式有两种，一种是从 Read 创建，另一种是从文件创建。
    // 为了使迭代器的运行效率不至于太差，迭代器如果通过直接从 WaveReader 读取 body 的话，一旦迭代器太多，
    // 它就会疯狂 seek 然后读取，如果多个迭代器在多线程的情况下使用，绝对会乱套。
    // 因此，当 WaveReader 是从文件创建的，那可以给迭代器重新打开文件，让迭代器自己去 seek 和读取。
    // 而如果 WaveReader 是从 Read 创建的，那就创建临时文件，把 body 的内容转移到临时文件里，让迭代器使用。
    pub fn CreateIter<R: SampleConv>(&mut self) -> Result<WaveIter<R>, Box<dyn Error>> {
        let mut data_offset: u64 = 0;

        // 打开文件，不论是打开原有的 WAV 还是生成一个会自动删除的临时文件
        let mut file = match &self.filepath {
            Some(filepath) => {
                data_offset = self.data_offset;
                File::open(filepath)?
            },
            None => {
                tempfile()?
            },
        };

        // 把 data 段的数据全部填充到临时文件里（如果不能打开原有 WAV 文件）
        // 不使用 BufWriter，因为它会把我的 File 抢走。
        // 自制缓冲区用于拷贝，每次 8 KB。
        match &self.filepath {
            Some(_) => (),
            None => {
                // 按指定大小分块拷贝数据
                const buffer_size: usize = 8192;
                let mut bytes_to_transfer = self.data_size as u64;
                let mut buf = vec![0u8; buffer_size];

                // 按块拷贝数据
                self.reader.seek_to(self.data_offset);
                while bytes_to_transfer >= buffer_size as u64 {
                    self.reader.read_exact(&mut buf)?;
                    file.write_all(&buf)?;
                    bytes_to_transfer -= buffer_size as u64;
                }
                if bytes_to_transfer > 0 {
                    buf.resize(bytes_to_transfer as usize, 0);
                    self.reader.read_exact(&mut buf)?;
                    file.write_all(&buf)?;
                }
                self.reader.seek_to(self.data_offset);
            }
        }

        let mut reader = StructRead::new(Box::new(BufReader::new(file)));
        let spec = self.spec.clone();
        let frame_pos = 0;
        let max_frames = self.num_frames;
        reader.seek_to(data_offset)?;

        Ok(WaveIter::<R> {
            reader,
            data_offset,
            spec,
            frame_pos,
            max_frames,
            unpacker: SampleReader::<R>::new(&reader, spec.sample_format),
        })
    }
}

struct WaveIter<'a, R: SampleConv> {
    reader: StructRead, // 数据读取器
    data_offset: u64, // 数据的位置
    spec: Spec,
    frame_pos: u64, // 当前帧位置
    max_frames: u64, // 最大帧数量
    unpacker: SampleReader<'a, R>,
}

struct SampleReader<'a, C> where C: SampleConv {
    reader: &'a StructRead,
    get_sample: fn(&mut StructRead) -> Result<C, io::Error>,
}

impl<C> SampleReader<'_, C> where C: SampleConv {
    fn new(reader: &StructRead, sample_format: SampleFormat) -> Self {
        Self {
            reader,
            get_sample: {
                match sample_format {
                    U8 =>  Self::_get_sample::<u8>,
                    S16 => Self::_get_sample::<i16>,
                    S24 => Self::_get_sample::<i24>,
                    S32 => Self::_get_sample::<i32>,
                    F32 => Self::_get_sample::<f32>,
                    F64 => Self::_get_sample::<f64>,
                    _ => panic!("Unknown sample_format {:?}", sample_format),
                }
            },
        }
    }

    fn get_sample(&self) -> Result<C, io::Error> {
        (self.get_sample)(&mut self.reader.get_underlying_reader())
    }

    fn _get_sample<T: SampleConv>(r: &mut StructRead) -> Result<C, io::Error> {
        C::from(T::read_le(&mut r)?)
    }
}

impl<T> Iterator for WaveIter<'_, T> where T: SampleConv {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.frame_pos >= self.max_frames {return None;}
        self.frame_pos += 1;

        let mut ret = Vec::<T>::with_capacity(self.spec.channels as usize);
        for i in 0..self.spec.channels {
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

    fn iter<T>(&mut self) -> Result<Box<dyn AudioIter<T>>, Box<dyn Error>> where Self: Sized, T: SampleConv {
        Box::new(self.CreateIter::<T>()?);
    }
}

#[derive(Clone, Copy, Debug)]
struct Chunk {
    flag: [u8; 4], // 实际存储在文件里的
    size: u32, // 实际存储在文件里的
    chunk_start_pos: u64, // Chunk 内容在文件中的位置，不包含 Chunk 头
}

impl Chunk {
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        // 读取 WAV 中的每个块
        // 注意 WAV 中会有 JUNK 块，目前的做法就是跳过所有的 JUNK 块。
        // 在 AVI 里面，JUNK 块里面会包含重要信息，但是 WAV 我就管它丫的了。
        let mut flag = [0u8; 4];
        let mut size : u32;
        loop {
            reader.read_exact(&mut flag)?;
            size = reader.read_le_u32()?;
            if &flag == b"JUNK" {
                reader.skip(size.into())?;
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

    fn seek_to_next_chunk(&self, reader: &mut StructRead) -> Result<u64, io::Error> {
        reader.seek_to(self.next_chunk_pos())
    }
}

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
struct fmt_Chunk_Extension {
    ext_len: u16,
    bits_per_sample: u16,
    channel_mask: u32,
    sub_format: GUID,
}

impl fmt_Chunk {
    fn read(reader: &mut StructRead, chunk_size: u32) -> Result<Self, Box<dyn Error>> {
        let mut ret = fmt_Chunk{
            format_tag: reader.read_le_u16()?,
            channels: reader.read_le_u16()?,
            sample_rate: reader.read_le_u32()?,
            byte_rate: reader.read_le_u32()?,
            block_align: reader.read_le_u16()?,
            bits_per_sample: reader.read_le_u16()?,
            None,
        };
        match ret.format_tag {
            0xFFFE => {
                if chunk_size >= 40 {
                    ret.extension = Some(fmt_Chunk_Extension::read(reader));
                }
            },
            0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => {
                // Ogg Vorbis 数据
                return Err(AudioError::Unimplemented);
            },
        }
        ret
    }

    fn get_sample_format(&self) -> Result<SampleFormat, AudioError> {
        use SampleFormat::{Int, UInt, Float};
        match (self.format_tag, self.bits_per_sample) {
            (1, 8) => UInt,
            (1, 16) => Int,
            (0xFFFE, 24) => Int,
            (0xFFFE, 32) => {
                match self.extension {
                    Some(extension) => {
                        match extension.sub_format {
                            guid_pcm_format => Int,
                            guid_ieee_float_format => Float,
                            _ => Err(AudioError::Unimplemented),
                        }
                    },
                    None => Int,
                }
            },
            (3, 32) => Float,
            (3, 46) => Float,
            _ => Err(AudioError::Unimplemented),
        }
    }
}

impl fmt_Chunk_Extension {
    fn read(reader: &mut StructRead) -> Result<Self, Box<dyn Error>> {
        Ok(Self{
            ext_len: reader.read_le_u16()?,
            bits_per_sample: reader.read_le_u16()?,
            channel_mask: reader.read_le_u32()?,
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
    fn read(reader: &mut StructRead) -> Result<Self, Box<dyn Error>> {
        let description = reader.read_string(256)?;
        let originator = reader.read_string(32)?;
        let originatorRef = reader.read_string(32)?;
        let originationDate = reader.read_string(10)?;
        let originationTime = reader.read_string(8)?;
        let timeRef = reader.read_le_u64()?;
        let version = reader.read_le_u16()?;
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
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        let mut ret = Self{
            manufacturer: reader.read_le_u32()?,
            product: reader.read_le_u32()?,
            samplePeriod: reader.read_le_u32()?,
            midiUnityNote: reader.read_le_u32()?,
            midiPitchFraction: reader.read_le_u32()?,
            smpteFormat: reader.read_le_u32()?,
            smpteOffset: reader.read_le_u32()?,
            numSampleLoops: reader.read_le_u32()?,
            samplerData: reader.read_le_u32()?,
            loops: Vec::<SMPLSampleLoop>::new(),
        };
        for _ in 0..ret.numSampleLoops {
            ret.loops.push(SMPLSampleLoop::read(reader)?);
        }
        Ok(ret)
    }
}

impl SMPLSampleLoop {
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        Ok(Self{
            identifier: reader.read_le_u32()?,
            type_: reader.read_le_u32()?,
            start: reader.read_le_u32()?,
            end: reader.read_le_u32()?,
            fraction: reader.read_le_u32()?,
            playCount: reader.read_le_u32()?,
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
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        Ok(Self{
            baseNote: reader.read_le_u8()?,
            detune: reader.read_le_u8()?,
            gain: reader.read_le_u8()?,
            lowNote: reader.read_le_u8()?,
            highNote: reader.read_le_u8()?,
            lowVelocity: reader.read_le_u8()?,
            highVelocity: reader.read_le_u8()?,
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
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        let mut ret = Cue_Chunk {
            num_cues: reader.read_le_u32()?,
            cues: Vec::<Cue>::new(),
        };
        for _ in 0..ret.num_cues {
            ret.cues.push(Cue::read(reader)?);
        }
        Ok(ret)
    }
}

impl Cue {
    fn read(reader: &mut StructRead) -> Result<Self, io::Error> {
        Ok(Self{
            identifier: reader.read_le_u32()?,
            order: reader.read_le_u32()?,
            chunkID: reader.read_le_u32()?,
            chunkStart: reader.read_le_u32()?,
            blockStart: reader.read_le_u32()?,
            offset: reader.read_le_u32()?,
        })
    }
}

#[derive(Debug, Clone)]
struct LISTChunk { // https://www.recordingblogs.com/wiki/list-chunk-of-a-wave-file
    info: Option<HashMap<[u8; 4], String>>,
    adtl: Option<AdtlChunk>,
}

impl LISTChunk {
    fn read(reader: &mut StructRead) -> Result<Self, Box<dyn Error>> {
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
                    let value_str = reader.read_string(key_chunk.size as usize)?;
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
                            cue_point_id: reader.read_le_u32()?,
                            data: reader.read_zstring()?,
                        });
                    },
                    b"ltxt" => {
                        ltxt = Some(LTXTChunk{
                            cue_point_id: reader.read_le_u32()?,
                            sample_length: reader.read_le_u32()?,
                            purpose_id: reader.read_string(4)?,
                            country: reader.read_le_u16()?,
                            language: reader.read_le_u16()?,
                            dialect: reader.read_le_u16()?,
                            code_page: reader.read_le_u16()?,
                            data: reader.read_zstring()?,
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
