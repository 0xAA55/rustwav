#![allow(non_snake_case)]
#![allow(dead_code)]

use std::{fs::File, path::PathBuf, cmp::Ordering, mem, io::{Read, Seek, SeekFrom, BufReader, BufWriter}, collections::{BTreeSet, BTreeMap}};

use crate::Reader;
use crate::SampleType;
use crate::{AudioReadError, AudioError};
use crate::readwrite::{self, string_io::*};
use crate::wavcore;
use crate::wavcore::Spec;
use crate::wavcore::ChunkHeader;
use crate::wavcore::{FmtChunk, ExtensionData};
use crate::wavcore::{SlntChunk, BextChunk, SmplChunk, InstChunk, PlstChunk, TrknChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::wavcore::FullInfoCuePoint;
use crate::decoders::{Decoder, PcmDecoder, AdpcmDecoderWrap, PcmXLawDecoderWrap};
use crate::adpcm::{DecIMA, DecMS, DecYAMAHA};
use crate::savagestr::{StringCodecMaps, SavageStringCodecs};
use crate::filehasher::FileHasher;
use crate::copiablebuf::CopiableBuffer;
use crate::xlaw::XLaw;

#[cfg(feature = "mp3dec")]
use crate::decoders::mp3::Mp3Decoder;

#[cfg(feature = "opus")]
use crate::decoders::opus::OpusDecoder;

#[cfg(feature = "flac")]
use crate::decoders::flac_dec::FlacDecoderWrap;

/// ## The data source for the `WaveReader`, currently we have a file reader or a file path.
#[derive(Debug)]
pub enum WaveDataSource {
    Reader(Box<dyn Reader>),
    Filename(String),
    Unknown,
}

/// # The `WaveReader` is dedicated to reading a WAV file and provides you with samples as you want.
/// Usage:
/// * Open a WAV file
/// * Get the iterator
/// * The iterator excretes the PCM samples with the format you specified.
#[derive(Debug)]
pub struct WaveReader {
    spec: Spec,
    fmt__chunk: FmtChunk, // fmt chunk must exists
    fact_data: u64, // Total samples in the data chunk
    data_chunk: WaveDataReader,
    text_encoding: StringCodecMaps,
    slnt_chunk: Option<SlntChunk>,
    bext_chunk: Option<BextChunk>,
    smpl_chunk: Option<SmplChunk>,
    inst_chunk: Option<InstChunk>,
    plst_chunk: Option<PlstChunk>,
    trkn_chunk: Option<TrknChunk>,
    cue__chunk: Option<CueChunk>,
    axml_chunk: Option<String>,
    ixml_chunk: Option<String>,
    list_chunk: BTreeSet<ListChunk>,
    acid_chunk: Option<AcidChunk>,
    id3__chunk: Option<Id3::Tag>,
    junk_chunks: BTreeSet<JunkChunk>,
}

/// Accepts a result, if it is `Ok`, return a `Some`; otherwise print the error message and return `None`
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
    /// * Open the WAV file from a file path. No temporary files will be created.
    pub fn open(file_source: &str) -> Result<Self, AudioReadError> {
        Self::new(WaveDataSource::Filename(file_source.to_string()))
        // Self::new(WaveDataSource::Reader(Box::new(BufReader::new(File::open(file_source)?)))) // Test for the temp file
    }

    /// * Open the WAV file from a `WaveDataSource`, if the `WaveDataSource` is `Reader`, the `WaveReader` will create an auto-delete temporary file for the `data` chunk.
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

        let mut riff_end = 0xFFFFFFFFu64;
        let mut isRF64 = false;
        let mut ds64_read = false;
        let mut data_size = 0u64;

        // The whole file should be a `RIFF` chunk or a `RF64` chunk.
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

        // This flag must be `WAVE`, after the flag, there are chunks to read and parse.
        expect_flag(&mut reader, b"WAVE")?;

        let mut fmt__chunk: Option<FmtChunk> = None;
        let mut data_offset = 0u64;
        let mut fact_data = 0u64;
        let mut slnt_chunk: Option<SlntChunk> = None;
        let mut bext_chunk: Option<BextChunk> = None;
        let mut smpl_chunk: Option<SmplChunk> = None;
        let mut inst_chunk: Option<InstChunk> = None;
        let mut plst_chunk: Option<PlstChunk> = None;
        let mut trkn_chunk: Option<TrknChunk> = None;
        let mut cue__chunk: Option<CueChunk> = None;
        let mut axml_chunk: Option<String> = None;
        let mut ixml_chunk: Option<String> = None;
        let mut list_chunk = BTreeSet::<ListChunk>::new();
        let mut acid_chunk: Option<AcidChunk> = None;
        let mut id3__chunk: Option<Id3::Tag> = None;
        let mut junk_chunks = BTreeSet::<JunkChunk>::new();

        // Read each chunks from the WAV file
        let mut last_flag: [u8; 4];
        let mut chunk = ChunkHeader::new();
        let mut manually_skipping = false;
        loop { // Loop through the chunks inside the RIFF chunk or RF64 chunk.
            let chunk_position = reader.stream_position()?;
            if ChunkHeader::align(chunk_position) == riff_end {
                // Normally hit the end of the WAV file.
                break;
            } else if chunk_position + 4 >= riff_end {
                // Hit the end but not good.
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
                    junk_chunks.insert(JunkChunk::from(junk));
                }
                b"fmt " => {
                    Self::no_duplication(&fmt__chunk, &chunk.flag)?;
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
                    if ds64_read {
                        return Err(AudioReadError::DataCorrupted(String::from("Duplicated \"ds64\" chunk appears")))
                    }
                    if chunk.size < 28 {
                        return Err(AudioReadError::DataCorrupted(String::from("the size of \"ds64\" chunk is too small to contain enough data")))
                    }
                    let riff_len = u64::read_le(&mut reader)?;
                    data_size = u64::read_le(&mut reader)?;
                    let _sample_count = u64::read_le(&mut reader)?;
                    // After these fields, there are tables for each chunk's size in 64 bits. Normally it's not needed to read this, except for huge > 4GB JUNK chunks.
                    riff_end = ChunkHeader::align(start_of_riff + riff_len);
                    ds64_read = true;
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
                b"slnt" => {
                    Self::verify_none(&slnt_chunk, &chunk.flag)?;
                    slnt_chunk = optional(SlntChunk::read(&mut reader));
                }
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
                b"plst" => {
                    Self::verify_none(&plst_chunk, &chunk.flag)?;
                    plst_chunk = optional(PlstChunk::read(&mut reader));
                }
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
                    list_chunk.append(&mut optional(ListChunk::read(&mut reader, chunk.size as u64, &text_encoding)).into_iter().collect::<BTreeSet<ListChunk>>());
                }
                b"acid" => {
                    Self::verify_none(&acid_chunk, &chunk.flag)?;
                    acid_chunk = optional(AcidChunk::read(&mut reader));
                },
                b"Trkn" => {
                    Self::verify_none(&trkn_chunk, &chunk.flag)?;
                    trkn_chunk = optional(TrknChunk::read(&mut reader));
                }
                b"id3 " => {
                    Self::verify_none(&id3__chunk, &chunk.flag)?;
                    id3__chunk = optional(Id3::id3_read(&mut reader, chunk.size as usize));
                },
                b"\0\0\0\0" => { // empty flag
                    return Err(AudioReadError::IncompleteFile(chunk_position));
                },
                // I used to find a BFDi chunk, after searching the internet, the chunk is dedicated to the BFD Player,
                // Its content seems like a serial number string for the software, So no need to parse it.
                other => {
                    println!("Skipped an unknown chunk in RIFF or RF64 chunk: '{}' [0x{:x}, 0x{:x}, 0x{:x}, 0x{:x}], Position: 0x{:x}, Size: 0x{:x}",
                             text_encoding.decode_flags(other),
                             other[0], other[1], other[2], other[3],
                             chunk_position, chunk.size);
                    println!("The previous chunk is '{}'", text_encoding.decode_flags(&last_flag))
                },
            }
            if !manually_skipping {
                chunk.seek_to_next_chunk(&mut reader)?;
            } else {
                manually_skipping = false;
            }
        }

        if isRF64 && !ds64_read {
            return Err(AudioReadError::DataCorrupted(String::from("the WAV file is a RF64 file but doesn't provide the \"ds64\" chunk")));
        }

        let fmt__chunk = match fmt__chunk {
            Some(fmt__chunk) => fmt__chunk,
            None => return Err(AudioReadError::DataCorrupted(String::from("the whole WAV file doesn't provide the \"data\" chunk"))),
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
            slnt_chunk,
            bext_chunk,
            smpl_chunk,
            inst_chunk,
            plst_chunk,
            trkn_chunk,
            cue__chunk,
            axml_chunk,
            ixml_chunk,
            list_chunk,
            acid_chunk,
            id3__chunk,
            junk_chunks,
        })
    }

    /// Provice spec information
    pub fn spec(&self) -> Spec{
        self.spec
    }

    /// * The `fact` data is the number of the total samples in the `data` chunk.
    pub fn get_fact_data(&self) -> u64 {self.fact_data}

    /// * The `fmt ` chunk is to specify the detailed audio file format.
    pub fn get_fmt__chunk(&self) -> &FmtChunk { &self.fmt__chunk }

    /// * The `slnt` chunk indicates how long to stay silent.
    pub fn get_slnt_chunk(&self) -> &Option<SlntChunk> { &self.slnt_chunk }

    /// * The `bext` chunk has some `description`, `version`, `time_ref` pieces of information, etc.
    pub fn get_bext_chunk(&self) -> &Option<BextChunk> { &self.bext_chunk }

    /// * The `smpl` chunk has some pieces of information about MIDI notes, pitch, etc.
    pub fn get_smpl_chunk(&self) -> &Option<SmplChunk> { &self.smpl_chunk }

    /// * The `inst` chunk has some `base_note`, `gain`, `velocity`, etc.
    pub fn get_inst_chunk(&self) -> &Option<InstChunk> { &self.inst_chunk }

    /// * The `plst` chunk is the playlist, it has a list that each element have `cue_point`, `num_samples`, `repeats`.
    pub fn get_plst_chunk(&self) -> &Option<PlstChunk> { &self.plst_chunk }

    /// * The `trkn` chunk, by the name.
    pub fn get_trkn_chunk(&self) -> &Option<TrknChunk> { &self.trkn_chunk }

    /// * The `cue ` chunk is with the `plst` chunk, it has a list that each element have `cue_point_id`, `position`, `chunk_start`, etc.
    pub fn get_cue__chunk(&self) -> &Option<CueChunk> { &self.cue__chunk }

    /// * The `axml` chunk. I personally don't know what it is, by the name it looks like some kind of `audio XML`. It's a pure string chunk.
    pub fn get_axml_chunk(&self) -> &Option<String> { &self.axml_chunk }

    /// * The `ixml` chunk. I personally don't know what it is, by the name it looks like some kind of `info XML`. It's a pure string chunk.
    pub fn get_ixml_chunk(&self) -> &Option<String> { &self.ixml_chunk }

    /// * The `list` chunk, it has 2 subtypes, one is `INFO`, and another is `adtl`.
    /// * The `INFO` subtype is the metadata that contains `artist`, `album`, `title`, etc. It lacks `albumartist` info.
    /// * The `adtl` subtype is with the `cue ` chunk, it's a list including the `label`, `note`, `text`, `file` for the playlist.
    pub fn get_list_chunk(&self) -> &BTreeSet<ListChunk> { &self.list_chunk }

    /// * The `acid` chunk, contains some pieces of information about the rhythm, e.g. `num_beats`, `tempo`, etc.
    pub fn get_acid_chunk(&self) -> &Option<AcidChunk> { &self.acid_chunk }

    /// * Another metadata chunk for the audio file. This covers more metadata than the `LIST INFO` chunk.
    pub fn get_id3__chunk(&self) -> &Option<Id3::Tag> { &self.id3__chunk }

    /// * The `JUNK` chunk, sometimes it's used for placeholder, sometimes it contains some random data for some random music software to show off.
    pub fn get_junk_chunks(&self) -> &BTreeSet<JunkChunk> { &self.junk_chunks }

    /// * If your audio file has `plst`, `cue `, and `LIST adtl` chunks, then BAM you can call this function for full playlist info.
    /// * Returns `Err` if some of these chunks are absent.
    pub fn create_full_info_cue_data(&self) -> Result<BTreeMap<u32, FullInfoCuePoint>, AudioError> {
        if self.list_chunk.is_empty() {
            return Err(AudioError::NoSuchData("You don't have a `LIST` chunk.".to_owned()));
        }
        for list_chunk in self.list_chunk.iter() {
            if let ListChunk::Adtl(adtl) = list_chunk {
                return if let Some(ref cue__chunk) = self.cue__chunk {
                    wavcore::create_full_info_cue_data(cue__chunk, adtl, &self.plst_chunk)
                } else {
                    Err(AudioError::NoSuchData("You don't have a `cue ` chunk.".to_owned()))
                };
            } else {
                eprintln!("The data type of the `LIST` chunk is `INFO`, not `adtl`: {:?}", list_chunk);
            }
        }
        Err(AudioError::NoSuchData(format!("The data type of your `LIST` chunk is `INFO`, not `adtl`: {:?}", self.list_chunk)))
    }

    /// * To verify if a chunk had not read. Some chunks should not be duplicated.
    fn no_duplication<T>(o: &Option<T>, flag: &[u8; 4]) -> Result<(), AudioReadError> {
        if o.is_some() {
            Err(AudioReadError::DataCorrupted(format!("Duplicated chunk '{}' in the WAV file", String::from_utf8_lossy(flag))))
        } else {
            Ok(())
        }
    }

    /// ## Create an iterator for iterating through each audio frame, excretes multi-channel audio frames.
    /// * Every audio frame is an array that includes one sample for every channel.
    /// * This iterator supports multi-channel audio files e.g. 5.1 stereo or 7.1 stereo audio files.
    /// * Since each audio frame is a `Vec` , it's expensive in memory and slow.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple frames.
    pub fn frame_iter<S>(&mut self) -> Result<FrameIter<S>, AudioReadError>
    where S: SampleType {
        FrameIter::<S>::new(&self.data_chunk, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }

    /// ## Create an iterator for iterating through each audio frame, excretes mono-channel samples.
    /// * This iterator is dedicated to mono audio, it combines every channel into one channel and excretes every single sample as an audio frame.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
    pub fn mono_iter<S>(&mut self) -> Result<MonoIter<S>, AudioReadError>
    where S: SampleType {
        MonoIter::<S>::new(&self.data_chunk, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }

    /// ## Create an iterator for iterating through each audio frame, excretes two-channel stereo frames.
    /// * This iterator is dedicated to two-channel stereo audio, if the source audio is mono, it duplicates the sample to excrete stereo frames for you. If the source audio is multi-channel audio, this iterator can't be created.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
    pub fn stereo_iter<S>(&mut self) -> Result<StereoIter<S>, AudioReadError>
    where S: SampleType {
        StereoIter::<S>::new(&self.data_chunk, self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }

    /// ## Create an iterator for iterating through each audio frame and consumes the `WaveReader`, excretes multi-channel audio frames.
    /// * Every audio frame is an array that includes one sample for every channel.
    /// * This iterator supports multi-channel audio files e.g. 5.1 stereo or 7.1 stereo audio files.
    /// * Since each audio frame is a `Vec` , it's expensive in memory and slow.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple frames.
    pub fn frame_intoiter<S>(mut self) -> Result<FrameIntoIter<S>, AudioReadError>
    where S: SampleType {
        FrameIntoIter::<S>::new(mem::take(&mut self.data_chunk), self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }

    /// ## Create an iterator for iterating through each audio frame and consumes the `WaveReader`, excretes mono-channel samples.
    /// * This iterator is dedicated to mono audio, it combines every channel into one channel and excretes every single sample as an audio frame.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
    pub fn mono_intoiter<S>(mut self) -> Result<MonoIntoIter<S>, AudioReadError>
    where S: SampleType {
        MonoIntoIter::<S>::new(mem::take(&mut self.data_chunk), self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }

    /// ## Create an iterator for iterating through each audio frame and consumes the `WaveReader`, excretes two-channel stereo frames.
    /// * This iterator is dedicated to two-channel stereo audio, if the source audio is mono, it duplicates the sample to excrete stereo frames for you. If the source audio is multi-channel audio, this iterator can't be created.
    /// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
    pub fn stereo_intoiter<S>(mut self) -> Result<StereoIntoIter<S>, AudioReadError>
    where S: SampleType {
        StereoIntoIter::<S>::new(mem::take(&mut self.data_chunk), self.data_chunk.offset, self.data_chunk.length, &self.spec, &self.fmt__chunk, self.fact_data)
    }
}

/// * The `IntoIterator` is **only for** two-channel stereo `f32` samples.
impl IntoIterator for WaveReader {
    type Item = (f32, f32);
    type IntoIter = StereoIntoIter<f32>;

    fn into_iter(self) -> Self::IntoIter {
        self.stereo_intoiter::<f32>().unwrap()
    }
}

fn expect_flag<T: Read>(r: &mut T, flag: &[u8; 4]) -> Result<(), AudioReadError> {
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

/// ## Create the decoder for each specific `format_tag` in the `fmt` chunk.
fn create_decoder<S>(reader: Box<dyn Reader>, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Box<dyn Decoder<S>>, AudioReadError>
where S: SampleType {
    use wavcore::AdpcmSubFormat::{Ms, Ima, Yamaha};
    const TAG_MS: u16 = Ms as u16;
    const TAG_IMA: u16 = Ima as u16;
    const TAG_YAMAHA: u16 = Yamaha as u16;
    match fmt.format_tag {
        1 | 0xFFFE | 3 => Ok(Box::new(PcmDecoder::<S>::new(reader, data_offset, data_length, spec, fmt)?)),
        6 => Ok(Box::new(PcmXLawDecoderWrap::new(reader, XLaw::ALaw, data_offset, data_length, fmt, fact_data)?)),
        7 => Ok(Box::new(PcmXLawDecoderWrap::new(reader, XLaw::MuLaw, data_offset, data_length, fmt, fact_data)?)),
        TAG_MS => Ok(Box::new(AdpcmDecoderWrap::<DecMS>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        TAG_IMA | 0x0069 => Ok(Box::new(AdpcmDecoderWrap::<DecIMA>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        TAG_YAMAHA => Ok(Box::new(AdpcmDecoderWrap::<DecYAMAHA>::new(reader, data_offset, data_length, fmt, fact_data)?)),
        0x0055 => {
            #[cfg(feature = "mp3dec")]
            {Ok(Box::new(Mp3Decoder::new(reader, data_offset, data_length, fmt, fact_data)?))}
            #[cfg(not(feature = "mp3dec"))]
            {Err(AudioReadError::Unimplemented(String::from("not implemented for decoding MP3 audio data inside the WAV file")))}
        },
        0x674f | 0x6750 | 0x6751 | 0x676f | 0x6770 | 0x6771 => { // Ogg Vorbis 数据
            Err(AudioReadError::Unimplemented(String::from("not implemented for decoding ogg vorbis audio data inside the WAV file")))
        },
        0x704F => {
            #[cfg(feature = "opus")]
            {Ok(Box::new(OpusDecoder::new(reader, data_offset, data_length, fmt, fact_data)?))}
            #[cfg(not(feature = "opus"))]
            {Err(AudioReadError::Unimplemented(String::from("not implemented for decoding opus audio data inside the WAV file")))}
        },
        0xF1AC => { // FLAC
            #[cfg(feature = "flac")]
            {Ok(Box::new(FlacDecoderWrap::new(reader, data_offset, data_length, fmt, fact_data)?))}
            #[cfg(not(feature = "flac"))]
            Err(AudioReadError::Unimplemented(String::from("not implemented for decoding FLAC audio data inside the WAV file")))
        },
        other => Err(AudioReadError::Unimplemented(format!("Not implemented for format_tag 0x{:x}", other))),
    }
}

/// * The `WaveDataReader` provides the way to access the audio data, by a `Reader` or a file.
/// * This is for creating the iterators. Each iterator uses this to read bytes, seek an offset, and convert data into samples.
/// * By using this, every individual iterator can have its iterating position.
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
                // Only a reader, no filenames, it's time to cut open its belly, seek for the `data` chunk,
                // and copy the data into a temporary file for the iterator.
                datahash = hasher.hash(&mut r, data_offset, data_size)?;
                reader = Some(r);
                let tdir = tempfile::TempDir::new()?;
                filepath = Some(tdir.path().join(format!("{:x}.tmp", datahash)));
                tempdir = Some(tdir);
                offset = 0;
                have_source_file = false;
            },
            WaveDataSource::Filename(path) => {
                // There is a filename. Just open the file for the iterator to read.
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

        // This is to store the original `Reader`, if there's no original `Reader`, this value is `None`.
        let mut orig_reader: Option<Box<dyn Reader>> = None;

        let mut reader = reader.unwrap();
        let filepath = filepath.unwrap();

        // Create the temporary file for the iterator if there's no source file name.
        if !have_source_file {

            // The temporary file name is the hash of the `data` chunk.
            let mut file = BufWriter::new(File::create(&filepath)?);
            reader.seek(SeekFrom::Start(offset))?;
            readwrite::copy(&mut reader, &mut file, data_size)?;
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

    // Open the source file or the temporary file, and seek the `data` chunk inner data offset.
    fn open(&self) -> Result<Box<dyn Reader>, AudioReadError> {
        let mut file = BufReader::new(File::open(&self.filepath)?);
        file.seek(SeekFrom::Start(self.offset))?;
        Ok(Box::new(file))
    }
}

/// For `mem::take()`, used by the `IntoIter` iterators.
impl Default for WaveDataReader {
    fn default() -> Self {
        Self {
            reader: None,
            tempdir: None,
            filepath: PathBuf::new(),
            offset: 0,
            length: 0,
            datahash: 0,
        }
    }
}

/// ## The audio frame iterator was created from the `WaveReader` to decode the audio frames.
/// * Every audio frame is an array that includes one sample for every channel.
/// * This iterator supports multi-channel audio files e.g. 5.1 stereo or 7.1 stereo audio files.
/// * Since each audio frame is a `Vec` , it's expensive in memory and slow.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple frames.
#[derive(Debug)]
pub struct FrameIter<'a, S>
where S: SampleType {
    /// * The borrowed data reader from the `WaveReader`
    data_reader: &'a WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<'a, S> FrameIter<'a, S>
where S: SampleType {
    fn new(data_reader: &'a WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_frames(&mut self, num_frames: usize) -> Result<Vec<Vec<S>>, AudioReadError> {
        self.decoder.decode_frames(num_frames)
    }
}

impl<S> Iterator for FrameIter<'_, S>
where S: SampleType {
    type Item = Vec<S>;

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_frame().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

/// ## The audio frame iterator was created from the `WaveReader` to decode the mono audio.
/// * This iterator is dedicated to mono audio, it combines every channel into one channel and excretes every single sample as an audio frame.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
#[derive(Debug)]
pub struct MonoIter<'a, S>
where S: SampleType {
    /// * The borrowed data reader from the `WaveReader`
    data_reader: &'a WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<'a, S> MonoIter<'a, S>
where S: SampleType {
    fn new(data_reader: &'a WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_monos(&mut self, num_monos: usize) -> Result<Vec<S>, AudioReadError> {
        self.decoder.decode_monos(num_monos)
    }
}

impl<S> Iterator for MonoIter<'_, S>
where S: SampleType {
    type Item = S;

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_mono().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}


/// ## The audio frame iterator was created from the `WaveReader` to decode the stereo audio.
/// * This iterator is dedicated to two-channel stereo audio, if the source audio is mono, it duplicates the sample to excrete stereo frames for you. If the source audio is multi-channel audio, this iterator can't be created.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
#[derive(Debug)]
pub struct StereoIter<'a, S>
where S: SampleType {
    /// * The borrowed data reader from the `WaveReader`
    data_reader: &'a WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<'a, S> StereoIter<'a, S>
where S: SampleType {
    fn new(data_reader: &'a WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_stereos(&mut self, num_stereos: usize) -> Result<Vec<(S, S)>, AudioReadError> {
        self.decoder.decode_stereos(num_stereos)
    }
}

impl<S> Iterator for StereoIter<'_, S>
where S: SampleType {
    type Item = (S, S);

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_stereo().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

/// ## The audio frame iterator was created from the `WaveReader` to decode the audio frames.
/// * Every audio frame is an array that includes one sample for every channel.
/// * This iterator supports multi-channel audio files e.g. 5.1 stereo or 7.1 stereo audio files.
/// * Since each audio frame is a `Vec` , it's expensive in memory and slow.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple frames.
/// * After the iterator was created, the `WaveReader` was consumed and couldn't be used anymore.
#[derive(Debug)]
pub struct FrameIntoIter<S>
where S: SampleType {
    /// * The owned data reader from the `WaveReader`
    data_reader: WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<S> FrameIntoIter<S>
where S: SampleType {
    fn new(data_reader: WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_frames(&mut self, num_frames: usize) -> Result<Vec<Vec<S>>, AudioReadError> {
        self.decoder.decode_frames(num_frames)
    }
}

impl<S> Iterator for FrameIntoIter<S>
where S: SampleType {
    type Item = Vec<S>;

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_frame().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

/// ## The audio frame iterator was created from the `WaveReader` to decode the mono audio.
/// * This iterator is dedicated to mono audio, it combines every channel into one channel and excretes every single sample as an audio frame.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
/// * After the iterator was created, the `WaveReader` was consumed and couldn't be used anymore.
#[derive(Debug)]
pub struct MonoIntoIter<S>
where S: SampleType {
    /// * The owned data reader from the `WaveReader`
    data_reader: WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<S> MonoIntoIter<S>
where S: SampleType {
    fn new(data_reader: WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_monos(&mut self, num_monos: usize) -> Result<Vec<S>, AudioReadError> {
        self.decoder.decode_monos(num_monos)
    }
}

impl<S> Iterator for MonoIntoIter<S>
where S: SampleType {
    type Item = S;

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_mono().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

/// ## The audio frame iterator was created from the `WaveReader` to decode the stereo audio.
/// * This iterator is dedicated to two-channel stereo audio, if the source audio is mono, it duplicates the sample to excrete stereo frames for you. If the source audio is multi-channel audio, this iterator can't be created.
/// * Besides it's an iterator, the struct itself provides `decode_frames()` for batch decode multiple samples.
/// * After the iterator was created, the `WaveReader` was consumed and couldn't be used anymore.
#[derive(Debug)]
pub struct StereoIntoIter<S>
where S: SampleType {
    /// * The owned data reader from the `WaveReader`
    data_reader: WaveDataReader,

    /// * The position of the audio data in the audio file, normally it is inside the WAV file `data` chunk, but there's an exception for a temporarily created data-only file.
    data_offset: u64,

    /// * The length of the audio data. After the audio data, it's often followed by the metadata of the audio file, so `EOF` doesn't indicate the end of the audio data.
    data_length: u64,

    /// * The spec for the audio data
    spec: Spec,

    /// * The total samples in the `data` chunk. If the WAV file doesn't have a `fact` chunk, this field is zero.
    fact_data: u64,

    /// * The decoder dedicated for the format of the audio data, excretes the `<S>` format of the PCM samples for you.
    decoder: Box<dyn Decoder<S>>,
}

impl<S> StereoIntoIter<S>
where S: SampleType {
    fn new(data_reader: WaveDataReader, data_offset: u64, data_length: u64, spec: &Spec, fmt: &FmtChunk, fact_data: u64) -> Result<Self, AudioReadError> {
        let mut reader = data_reader.open()?;
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self {
            data_reader,
            data_offset,
            data_length,
            spec: *spec,
            fact_data,
            decoder: create_decoder::<S>(reader, data_offset, data_length, spec, fmt, fact_data)?,
        })
    }

    /// * Batch decodes multiple frames. For some types of audio formats, this method is faster than decoding every frame one by one.
    pub fn decode_stereos(&mut self, num_stereos: usize) -> Result<Vec<(S, S)>, AudioReadError> {
        self.decoder.decode_stereos(num_stereos)
    }
}

impl<S> Iterator for StereoIntoIter<S>
where S: SampleType {
    type Item = (S, S);

    /// * This method is for decoding each audio frame.
    fn next(&mut self) -> Option<Self::Item> {
        self.decoder.decode_stereo().unwrap()
    }

    /// * This method is for seeking.
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.decoder.seek(SeekFrom::Current(n as i64)).unwrap();
        self.next()
    }
}

