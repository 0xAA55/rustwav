use std::{fs::File, {path::Path}, io::{self, Read, BufReader, Seek}, error::Error, collections::HashMap};

use crate::structread::StructRead;
use crate::sampleutils::SampleUtils;
use crate::audiocore::{SampleFormat, Spec, Frame};
use crate::audioreader::{AudioReader, AudioReadError};

pub struct WaveReader<R> {
    reader: StructRead<R>,
    first_sample_offset: u64,
    spec: Spec,
    channel_mask: u32, // 通道标记，整逗比音效多音道那一块儿的玩意儿
    fact_data: u32, // fact 块的参数
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

#[derive(Clone, Copy, Debug)]
struct Chunk {
    flag: [u8; 4], // 实际存储在文件里的
    size: u32, // 实际存储在文件里的
    chunk_start_pos: u64, // Chunk 内容在文件中的位置，不包含 Chunk 头
}

impl Chunk {
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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

    fn next_chunk_pos<R>(&self, reader: &mut StructRead<R>) -> u64 where R: Read + Seek {
        Self::align(self.chunk_start_pos + self.size as u64)
    }

    fn seek_to_next_chunk<R>(&self, reader: &mut StructRead<R>) -> Result<u64, io::Error> where R: Read + Seek {
        reader.seek_to(self.next_chunk_pos())
    }
}

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
struct Chunk_fmt {
    format_tag: u16,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

#[derive(Clone, Copy, PartialEq)]
struct GUID (pub u32, pub u16, pub u16, pub [u8; 8]);

const guid_pcm_format: GUID = GUID(0x00000001, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);
const guid_ieee_float_format: GUID = GUID(0x00000003, 0x0000, 0x0010, [0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71]);

impl GUID {
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
        Ok( Self (
            reader.read_le_u32()?,
            reader.read_le_u16()?,
            reader.read_le_u16()?,
            [
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
                reader.read_le_u8()?,
            ]
        ))
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, Box<dyn Error>> where R: Read + Seek {
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
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
    info: Option<HashMap<String, String>>,
    adtl: Option<AdtlChunk>,
}

impl LISTChunk {
    fn read<R>(reader: &mut StructRead<R>) -> Result<Self, io::Error> where R: Read + Seek {
        let mut info: Option<HashMap<String, String>> = None;
        let mut adtl: Option<AdtlChunk> = None;
        let sub_chunk = Chunk::read(reader)?;
        let end_of_chunk = sub_chunk.next_chunk_pos();
        match &sub_chunk.flag {
            b"info" | b"INFO" => {
                // INFO 节其实是很多键值对，用来标注歌曲信息。在它的字节范围的限制下，读取所有的键值对。
                let mut dict = HashMap::<String, String>::new();
                while reader.stream_position()? < end_of_chunk {
                    let key_chunk = Chunk::read(reader)?; // 每个键其实就是一个 Chunk，它的大小值就是字符串大小值。
                    let value_str = reader.read_string(key_chunk.size)?;
                    dict.insert(key_chunk.flag.to_string(), value_str);
                    key_chunk.seek_to_next_chunk(&mut reader)?;
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
        sub_chunk.seek_to_next_chunk(&mut reader)?;
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

impl<R> WaveReader<R> where R: Read + Seek {
    pub fn new(reader: R) -> Result<WaveReader<R>, Box<dyn Error>> {
        use SampleFormat::{Int, Float, Unknown};
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

        let mut fmt: Option<Chunk_fmt> = None;
        let mut first_sample_offset = 0u64;
        let mut sample_format = Unknown;
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
                    fmt = Some(Chunk_fmt{
                        format_tag: reader.read_le_u16()?,
                        channels: reader.read_le_u16()?,
                        sample_rate: reader.read_le_u32()?,
                        byte_rate: reader.read_le_u32()?,
                        block_align: reader.read_le_u16()?,
                        bits_per_sample: reader.read_le_u16()?,
                    });
                    let fmt = fmt.unwrap();
                    match fmt.format_tag {
                        1 => {
                            sample_format = match fmt.bits_per_sample {
                                8  | 16 => Int,
                                _ => return Err(AudioReadError::DataCorrupted.into()),
                            }
                        },
                        0xFFFE => {
                            if chunk.size < 40 {
                                sample_format = Int;
                            } else {
                                let _ext_len = reader.read_le_u16()?;
                                let _bits_per_sample = reader.read_le_u16()?;
                                channel_mask = reader.read_le_u32()?;
                                let sub_format = GUID::read(&mut reader)?;
                                match sub_format {
                                    guid_pcm_format => {
                                        sample_format = match fmt.bits_per_sample {
                                            24 | 32 => Int,
                                            _ => return Err(AudioReadError::DataCorrupted.into()),
                                        }
                                    }
                                    guid_ieee_float_format => {
                                        match fmt.bits_per_sample {
                                            32 | 64 => sample_format = Float,
                                            _ => return Err(AudioReadError::DataCorrupted.into()),
                                        }
                                    }
                                    _ => return Err(AudioReadError::Unimplemented.into()),
                                }
                            }
                        },
                        3 => {
                            sample_format = match fmt.bits_per_sample {
                                32 | 64 => Float,
                                _ => return Err(AudioReadError::Unimplemented.into()),
                            }
                        },
                        0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => {
                            // Ogg Vorbis
                            return Err(AudioReadError::Unimplemented.into())
                        },
                        _ => return Err(AudioReadError::Unimplemented.into()),
                    }
                },
                b"fact" => {
                    fact_data = reader.read_le_u32()?;
                },
                b"data" => {
                    first_sample_offset = chunk.chunk_start_pos;
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
                    data.resize(chunk.size, 0);
                    reader.read_exact(&mut data)?;
                    axml_chunk = Some(data);
                },
                b"ixml" => {
                    let data = Vec::<u8>::new();
                    data.resize(chunk.size, 0);
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

        let fmt = match fmt {
            Some(fmt) => fmt,
            None => return Err(AudioReadError::DataCorrupted.into()),
        };

        let frame_size = fmt.block_align;
        let num_frames = data_size / frame_size as u64;
        Ok(Self {
            reader,
            first_sample_offset,
            spec: Spec {
                channels: fmt.channels,
                sample_rate: fmt.sample_rate,
                bits_per_sample: fmt.bits_per_sample,
                sample_format,
            },
            channel_mask,
            fact_data,
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
impl WaveReader<BufReader<File>> {
    pub fn open<P: AsRef<Path>>(filename: P) -> Result<WaveReader<BufReader<File>>, Box<dyn Error>> {
        let file = File::open(filename)?;
        let buf_reader = BufReader::new(file);
        WaveReader::new(buf_reader)
    }
}

impl<R> AudioReader for WaveReader<R> where R: Read + Seek {
    fn spec(&self) -> Spec {
        self.spec.clone()
    }

    fn iter<T>(&mut self) -> Iter<T> where Self: Sized;
}

impl Iter<T>: Iterator {}