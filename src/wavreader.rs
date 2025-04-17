#![allow(non_snake_case)]
#![allow(dead_code)]

use std::{fs::File, path::PathBuf, cmp::Ordering, io::{Read, Seek, SeekFrom, BufReader, BufWriter}};

use crate::wavcore;
use crate::readwrite;
use crate::AudioReadError;
use crate::Spec;
use crate::ChunkHeader;
use crate::{FmtChunk, ExtensionData};
use crate::{BextChunk, SmplChunk, InstChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::{Decoder, PcmDecoder, AdpcmDecoderWrap, AdpcmSubFormat};
use crate::{DecIMA, DecMS, DecYAMAHA};
use crate::{StringCodecMaps, SavageStringCodecs};
use crate::FileHasher;
use crate::SampleType;
use crate::{Reader, string_io::*};
use crate::CopiableBuffer;

#[cfg(feature = "mp3dec")]
use crate::Mp3Decoder;

#[cfg(feature = "opus")]
use crate::OpusDecoder;

#[derive(Debug)]
pub enum WaveDataSource {
    Reader(Box<dyn Reader>),
    Filename(String),
    Unknown,
}

#[derive(Debug)]
pub struct WaveReader {
    spec: Spec,
    fmt__chunk: FmtChunk, // fmt 块，这个块一定会有
    fact_data: u64, // 总样本数
    data_chunk: WaveDataReader,
    text_encoding: StringCodecMaps,
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

// 接收一个 result 结果，如果是 Ok 则返回 Some(数据)；如果是 Err 则打印错误信息并返回 None
pub fn optional<T, E>(result: Result<T, E>) -> Option<T>
where E: std::error::Error{
    match result {
        Ok(object) => Some(object),
        Err(err) => {
            eprintln!("Error occured while parsing \"{}\": {:?}", std::any::type_name::<T>(), err);
            None
        }
    }
}

impl WaveReader {
    // 从文件路径打开
    pub fn open(file_source: &str) -> Result<Self, AudioReadError> {
        Self::new(WaveDataSource::Filename(file_source.to_string()))
        // Self::new(WaveDataSource::Reader(Box::new(BufReader::new(File::open(file_source)?)))) // 测试临时文件
    }

    // 从读取器打开
    pub fn new(file_source: WaveDataSource) -> Result<Self, AudioReadError> {
        let mut filesrc: Option<String> = None;
        let mut reader = match file_source {
            WaveDataSource::Reader(reader) => {
                reader
            },
            WaveDataSource::Filename(filename) => {
                filesrc = Some(filename.clone());
                Box::new(BufReader::new(File::open(&filename)?))
            },
            WaveDataSource::Unknown => return Err(AudioReadError::InvalidArguments(String::from("\"Unknown\" data source was given"))),
        };

        let text_encoding = StringCodecMaps::new();

        let filestart = reader.stream_position()?;
        let filelen = {reader.seek(SeekFrom::End(0))?; let filelen = reader.stream_position()?; reader.seek(SeekFrom::Start(filestart))?; filelen};

        let mut riff_end = 0xFFFFFFFFu64; // 如果这个 WAV 文件是 RF64 的文件，此时给它临时设置一个很大的值，等到读取到 ds64 块时再更新这个值。
        let mut isRF64 = false;
        let mut data_size = 0u64;

        // 先搞定最开始的头部，有 RIFF 头和 RF64 头，需要分开处理
        let chunk = ChunkHeader::read(&mut reader)?;
        match &chunk.flag {
            b"RIFF" => {
                let riff_len = chunk.size as u64;
                riff_end = ChunkHeader::align(reader.stream_position()? + riff_len);
            },
            b"RF64" => {
                isRF64 = true;
            },
            _ => return Err(AudioReadError::FormatError(String::from("Not a WAV file"))), // 根本不是 WAV
        }

        let start_of_riff = reader.stream_position()?;

        // 读完头部后，这里必须是 WAVE 否则不是音频文件。
        expect_flag(&mut reader, b"WAVE")?;

        let mut fmt__chunk: Option<FmtChunk> = None;
        let mut data_offset = 0u64;
        let mut fact_data = 0u64;
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
        let mut last_flag: [u8; 4];
        let mut chunk = ChunkHeader::new();
        let mut manually_skipping = false;
        loop {
            let chunk_position = reader.stream_position()?;
            if ChunkHeader::align(chunk_position) == riff_end {
                // 正常退出
                break;
            } else if chunk_position + 4 >= riff_end {
                match riff_end.cmp(&filelen) {
                    Ordering::Greater => eprintln!("There end of the RIFF chunk exceeded the file size of {} bytes.", riff_end - filelen),
                    Ordering::Equal => eprintln!("There are some chunk sizes wrong, probably the \"{}\" chunk.", text_encoding.decode_flags(&chunk.flag)),
                    Ordering::Less => eprintln!("There are {} extra bytes at the end of the RIFF chunk.", filelen - riff_end),
                }
                break;
            }
            last_flag = chunk.flag;
            chunk = ChunkHeader::read(&mut reader)?;
            match &chunk.flag {
                b"JUNK" => {
                    let mut junk = vec![0u8; chunk.size as usize];
                    reader.read_exact(&mut junk)?;
                    junk_chunks.push(JunkChunk::from(junk));
                }
                b"fmt " => {
                    Self::verify_none(&fmt__chunk, &chunk.flag)?;
                    fmt__chunk = Some(FmtChunk::read(&mut reader, chunk.size)?);
                },
                b"fact" => {
                    let mut buf = vec![0u8; chunk.size as usize];
                    reader.read_exact(&mut buf)?;
                    fact_data = match buf.len() {
                        4 => u32::from_le_bytes(buf.into_iter().collect::<CopiableBuffer<u8, 4>>().into_array()) as u64,
                        8 => u64::from_le_bytes(buf.into_iter().collect::<CopiableBuffer<u8, 8>>().into_array()),
                        o => {
                            eprintln!("Bad fact chunk size: {o}");
                            0
                        }
                    };
                },
                b"ds64" => {
                    if chunk.size < 28 {
                        return Err(AudioReadError::DataCorrupted(String::from("the size of \"ds64\" chunk is too small to contain enough data")))
                    }
                    let riff_len = u64::read_le(&mut reader)?;
                    data_size = u64::read_le(&mut reader)?;
                    let _sample_count = u64::read_le(&mut reader)?;
                    // 后面就是 table 了，用来重新给每个 Chunk 提供 64 位的长度值（data 除外）
                    riff_end = ChunkHeader::align(start_of_riff + riff_len);
                }
                b"data" => {
                    if data_offset != 0 {
                        return Err(AudioReadError::DataCorrupted(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(&chunk.flag))));
                    }
                    data_offset = chunk.chunk_start_pos;
                    if !isRF64 {
                        data_size = chunk.size as u64;
                    }
                    let chunk_end = ChunkHeader::align(chunk.chunk_start_pos + data_size);
                    reader.seek(SeekFrom::Start(chunk_end))?;
                    manually_skipping = true;
                    continue;
                },
                b"bext" => {
                    Self::verify_none(&bext_chunk, &chunk.flag)?;
                    bext_chunk = optional(BextChunk::read(&mut reader, &text_encoding));
                },
                b"smpl" => {
                    Self::verify_none(&smpl_chunk, &chunk.flag)?;
                    smpl_chunk = optional(SmplChunk::read(&mut reader));
                },
                b"inst" | b"INST" => {
                    Self::verify_none(&inst_chunk, &chunk.flag)?;
                    inst_chunk = optional(InstChunk::read(&mut reader));
                },
                b"cue " => {
                    Self::verify_none(&cue__chunk, &chunk.flag)?;
                    cue__chunk = optional(CueChunk::read(&mut reader));
                },
                b"axml" => {
                    Self::verify_none(&axml_chunk, &chunk.flag)?;
                    axml_chunk = optional(read_str(&mut reader, chunk.size as usize, &text_encoding));
                },
                b"ixml" => {
                    Self::verify_none(&ixml_chunk, &chunk.flag)?;
                    ixml_chunk = optional(read_str(&mut reader, chunk.size as usize, &text_encoding));
                },
                b"LIST" => {
                    Self::verify_none(&list_chunk, &chunk.flag)?;
                    list_chunk = optional(ListChunk::read(&mut reader, chunk.size as u64, &text_encoding));
                }
                b"acid" => {
                    Self::verify_none(&acid_chunk, &chunk.flag)?;
                    acid_chunk = optional(AcidChunk::read(&mut reader));
                },
                b"Trkn" => {
                    Self::verify_none(&trkn_chunk, &chunk.flag)?;
                    trkn_chunk = optional(read_str(&mut reader, chunk.size as usize, &text_encoding));
                }
                b"id3 " => {
                    Self::verify_none(&id3__chunk, &chunk.flag)?;
                    id3__chunk = optional(Id3::id3_read(&mut reader, chunk.size as usize));
                },
                b"\0\0\0\0" => { // 空的 flag
                    return Err(AudioReadError::IncompleteFile(chunk_position));
                },
                // 曾经发现 BFDi 块，结果发现它是 BFD Player 生成的字符串块，里面大约是软件序列号之类的内容。
                // 所以此处就不记载 BFDi 块的信息了。
                other => {
                    println!("Skipped an unknown chunk in RIFF or RF64 chunk: '{}' [0x{:x}, 0x{:x}, 0x{:x}, 0x{:x}], Position: 0x{:x}, Size: 0x{:x}",
                             text_encoding.decode_flags(other),
                             other[0], other[1], other[2], other[3],
                             chunk_position, chunk.size);
                    println!("The previous chunk is '{}'", text_encoding.decode_flags(&last_flag))
                },
            }
            if !manually_skipping {
                // 跳到下一个块的开始位置
                chunk.seek_to_next_chunk(&mut reader)?;
            } else {
                manually_skipping = false;
            }
        }

        let fmt__chunk = match fmt__chunk {
            Some(fmt__chunk) => fmt__chunk,
            None => return Err(AudioReadError::DataCorrupted(String::from("the whole WAV file doesn't provide any \"data\" chunk"))),
        };

        let mut channel_mask: u32 = 0;
        if let Some(extension) = fmt__chunk.extension {
            channel_mask = match extension.data {
                ExtensionData::Extensible(extensible) => extensible.channel_mask,
                _ => wavcore::guess_channel_mask(fmt__chunk.channels)?,
            };
        }

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
            spec,
            fmt__chunk,
            fact_data,
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

    // 用于检测特定 Chunk 是否有被重复读取的情况，有就报错
    fn verify_none<T>(o: &Option<T>, flag: &[u8; 4]) -> Result<(), AudioReadError> {
        if o.is_some() {
            Err(AudioReadError::DataCorrupted(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(flag))))
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
    pub fn frame_iter<S>(&mut self) -> Result<FrameIter<S>, AudioReadError>
    where S: SampleType {
        FrameIter::<S>::new(self.data_chunk.open()?, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }
    pub fn stereo_iter<S>(&mut self) -> Result<StereoIter<S>, AudioReadError>
    where S: SampleType {
        StereoIter::<S>::new(self.data_chunk.open()?, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }
    pub fn mono_iter<S>(&mut self) -> Result<MonoIter<S>, AudioReadError>
    where S: SampleType {
        MonoIter::<S>::new(self.data_chunk.open()?, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }
}

fn create_decoder<S>(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Box<dyn Decoder<S>>, AudioReadError>
where S: SampleType {
    use AdpcmSubFormat::{Ima, Ms, Yamaha};
    const TAG_IMA: u16 = Ima as u16;
    const TAG_MS: u16 = Ms as u16;
    const TAG_YAMAHA: u16 = Yamaha as u16;
    match fmt.format_tag {
        1 | 0xFFFE | 3 => Ok(Box::new(PcmDecoder::<S>::new(reader, data_offset, data_length, spec, fmt)?)),
        TAG_IMA => Ok(Box::new(AdpcmDecoderWrap::<DecIMA>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        TAG_MS => Ok(Box::new(AdpcmDecoderWrap::<DecMS>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        TAG_YAMAHA => Ok(Box::new(AdpcmDecoderWrap::<DecYAMAHA>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        0x0055 => {
            if cfg!(feature = "mp3dec") {
                Ok(Box::new(Mp3Decoder::new(reader, data_offset, data_length, fmt, fact_data)?))
            } else {
                Err(AudioReadError::Unimplemented(String::from("not implemented for decoding MP3 audio data inside the WAV file")))
            }
        },
        0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => { // Ogg Vorbis 数据
            Err(AudioReadError::Unimplemented(String::from("not implemented for decoding ogg vorbis audio data inside the WAV file")))
        },
        0x704F => {
            if cfg!(feature = "opus") {
                Ok(Box::new(OpusDecoder::new(reader, data_offset, data_length, fmt, fact_data)?))
            } else {
                Err(AudioReadError::Unimplemented(String::from("not implemented for decoding opus audio data inside the WAV file")))
            }
        },
        0xF1AC => { // FLAC
            // #[cfg(not(feature = "flac"))]
            Err(AudioReadError::Unimplemented(String::from("not implemented for decoding FLAC audio data inside the WAV file")))
        },
        other => Err(AudioReadError::Unimplemented(format!("Not implemented for format_tag 0x{:x}", other))),
    }
}

pub fn expect_flag<T: Read>(r: &mut T, flag: &[u8; 4]) -> Result<(), AudioReadError> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    if &buf != flag {
        Err(AudioReadError::UnexpectedFlag(
            String::from_utf8_lossy(flag).to_string(),
            String::from_utf8_lossy(&buf).to_string())
        )
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
    fn new(file_source: WaveDataSource, data_offset: u64, data_size: u64) -> Result<Self, AudioReadError> {
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
                filepath = Some(tdir.path().join(format!("{:x}.tmp", datahash)));
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
            WaveDataSource::Unknown => return Err(AudioReadError::InvalidArguments(String::from("\"Unknown\" data source was given"))),
        };

        // 这个用来存储最原始的 Reader，如果最开始没有给 Reader 而是给了文件名，则存 None。
        let mut orig_reader: Option<Box<dyn Reader>> = None;

        // 把之前读到的东西都展开
        let mut reader = reader.unwrap();
        let filepath = filepath.unwrap();

        // 没有原始文件名，只有一个 Reader，那就从 Reader 那里把 WAV 文件肚子里的 data chunk 复制到一个临时文件里。
        if !have_source_file {

            // 根据临时文件名创建临时文件
            let mut file = BufWriter::new(File::create(&filepath)?);
            reader.seek(SeekFrom::Start(offset))?;

            readwrite::copy(&mut reader, &mut file, data_size)?;

            // 这个时候，我们再把原始提供下来的 reader 收集起来存到结构体里
            orig_reader = Some(reader);

            #[cfg(debug_assertions)]
            println!("Using tempfile to store \"data\" chunk: {}", filepath.to_string_lossy());
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

    fn open(&self) -> Result<Box<dyn Reader>, AudioReadError> {
        let mut file = BufReader::new(File::open(&self.filepath)?);
        file.seek(SeekFrom::Start(self.offset))?;
        Ok(Box::new(file))
    }
}

#[derive(Debug)]
pub struct FrameIter<S>
where S: SampleType {
    data_offset: u64, // 音频数据在文件中的位置
    data_length: u64, // 音频数据的总大小
    spec: Spec,
    fact_data: u64, // 音频总样本数量
    decoder: Box<dyn Decoder<S>>, // 解码器
}

impl<S> FrameIter<S>
where S: SampleType {
    fn new(mut reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }
}

impl<S> Iterator for FrameIter<S>
where S: SampleType {
    type Item = Vec<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_frame().unwrap()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

#[derive(Debug)]
pub struct StereoIter<S>
where S: SampleType {
    data_offset: u64, // 音频数据在文件中的位置
    data_length: u64, // 音频数据的总大小
    spec: Spec,
    fact_data: u64, // 音频总样本数量
    decoder: Box<dyn Decoder<S>>, // 解码器
}

impl<S> StereoIter<S>
where S: SampleType {
    fn new(mut reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }
}

impl<S> Iterator for StereoIter<S>
where S: SampleType {
    type Item = (S, S);

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_stereo().unwrap()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

#[derive(Debug)]
pub struct MonoIter<S>
where S: SampleType {
    data_offset: u64, // 音频数据在文件中的位置
    data_length: u64, // 音频数据的总大小
    spec: Spec,
    fact_data: u64, // 音频总样本数量
    decoder: Box<dyn Decoder<S>>, // 解码器
}

impl<S> MonoIter<S>
where S: SampleType {
    fn new(mut reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }
}

impl<S> Iterator for MonoIter<S>
where S: SampleType {
    type Item = S;

    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_mono().unwrap()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}