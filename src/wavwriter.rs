#![allow(non_snake_case)]
#![allow(unused_imports)]

use std::{fs::File, io::{BufWriter, SeekFrom}, path::Path, collections::BTreeMap};

use crate::Writer;
use crate::WaveReader;
use crate::{AudioWriteError, AudioError};
use crate::SampleType;
use crate::wavcore;
use crate::wavcore::{DataFormat, AdpcmSubFormat, Spec, SampleFormat};
use crate::wavcore::ChunkWriter;
use crate::wavcore::FmtChunk;
use crate::wavcore::{SlntChunk, BextChunk, SmplChunk, InstChunk, PlstChunk, CueChunk, ListChunk, AcidChunk, JunkChunk, Id3};
use crate::wavcore::FullInfoCuePoint;
use crate::encoders::{Encoder, PcmEncoder, AdpcmEncoderWrap, PcmXLawEncoderWrap};
use crate::adpcm::{EncIMA, EncMS, EncYAMAHA};
use crate::readwrite::string_io::*;
use crate::savagestr::{StringCodecMaps, SavageStringCodecs};
use crate::hacks;
use crate::xlaw::XLaw;

#[cfg(feature = "mp3enc")]
use crate::encoders::mp3::Mp3Encoder;

#[cfg(feature = "opus")]
use crate::encoders::opus::OpusEncoder;

#[cfg(feature = "flac")]
use crate::encoders::flac_enc::FlacEncoderWrap;

/// ## These options are used to specify what type of WAV file you want to create.
#[derive(Debug)]
pub enum FileSizeOption{
    NeverLargerThan4GB,
    AllowLargerThan4GB,
    ForceUse4GBFormat,
}

/// # The `WaveWriter` is dedicated to creating a WAV file.
/// Usage:
/// * Choose one of the internal formats by specifying `DataFormat` and use the `WaveWriter` to create the WAV file.
/// * Use the methods, like `write_samples()`, `write_mono_channel()`, `write_monos()`, `write_stereos()`, etc, to write your PCM samples to the `WaveWriter`, it will encode.
/// * Call `finalize()` or just let the `WaveWriter` get out of the scope.
/// BAM. The WAV file was created successfully with the audio sound as you provided.
#[derive(Debug)]
pub struct WaveWriter<'a> {
    writer: Box<dyn Writer + 'a>,
    spec: Spec,
    data_format: DataFormat,
    file_size_option: FileSizeOption,
    fmt_chunk_offset: u64,
    fact_chunk_offset: u64,
    num_frames_written: u64,
    data_offset: u64,
    encoder: Encoder<'a>,
    text_encoding: StringCodecMaps,
    riff_chunk: Option<ChunkWriter<'a>>,
    data_chunk: Option<ChunkWriter<'a>>,
    pub fmt__chunk: FmtChunk,
    pub slnt_chunk: Vec<SlntChunk>,
    pub bext_chunk: Vec<BextChunk>,
    pub smpl_chunk: Vec<SmplChunk>,
    pub inst_chunk: Vec<InstChunk>,
    pub plst_chunk: Option<PlstChunk>,
    pub cue__chunk: Option<CueChunk>,
    pub axml_chunk: Vec<String>,
    pub ixml_chunk: Vec<String>,
    pub list_chunk: Vec<ListChunk>,
    pub acid_chunk: Vec<AcidChunk>,
    pub trkn_chunk: Vec<String>,
    pub id3__chunk: Option<Id3::Tag>,
    pub junk_chunks: Vec<JunkChunk>,
}

impl<'a> WaveWriter<'a> {
    pub fn create<P: AsRef<Path>>(filename: P, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter<'a>, AudioWriteError> {
        let file_writer = BufWriter::new(File::create(filename)?);
        let wave_writer = WaveWriter::from(Box::new(file_writer), spec, data_format, file_size_option)?;
        Ok(wave_writer)
    }

    pub fn from(writer: Box<dyn Writer + 'a>, spec: &Spec, data_format: DataFormat, file_size_option: FileSizeOption) -> Result<WaveWriter<'a>, AudioWriteError> {
        let mut ret = Self{
            writer,
            spec: *spec,
            data_format,
            file_size_option,
            fmt_chunk_offset: 0,
            fact_chunk_offset: 0,
            num_frames_written: 0,
            data_offset: 0,
            encoder: Encoder::default(),
            text_encoding: StringCodecMaps::new(),
            fmt__chunk: FmtChunk::new(),
            riff_chunk: None,
            data_chunk: None,
            slnt_chunk: Vec::<SlntChunk>::new(),
            bext_chunk: Vec::<BextChunk>::new(),
            smpl_chunk: Vec::<SmplChunk>::new(),
            inst_chunk: Vec::<InstChunk>::new(),
            plst_chunk: None,
            cue__chunk: None,
            axml_chunk: Vec::<String>::new(),
            ixml_chunk: Vec::<String>::new(),
            list_chunk: Vec::<ListChunk>::new(),
            acid_chunk: Vec::<AcidChunk>::new(),
            trkn_chunk: Vec::<String>::new(),
            id3__chunk: None,
            junk_chunks: Vec::<JunkChunk>::new(),
        };
        ret.create_encoder()?;
        ret.write_header()?;
        Ok(ret)
    }

    fn create_encoder(&mut self) -> Result<(), AudioWriteError> {
        use DataFormat::{Unspecified, Pcm, Adpcm, PcmALaw, PcmMuLaw, Mp3, Opus, Flac};
        let spec = self.spec;
        self.encoder = match self.data_format {
            Pcm => {
                spec.verify_for_pcm()?;
                Encoder::new(PcmEncoder::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec)?)
            },
            Adpcm(sub_format) => {
                use AdpcmSubFormat::{Ima, Ms, Yamaha};
                match sub_format {
                    Ima => Encoder::new(AdpcmEncoderWrap::<EncIMA>::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec)?),
                    Ms => Encoder::new(AdpcmEncoderWrap::<EncMS>::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec)?),
                    Yamaha => Encoder::new(AdpcmEncoderWrap::<EncYAMAHA>::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec)?),
                }
            },
            PcmALaw => Encoder::new(PcmXLawEncoderWrap::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec, XLaw::ALaw)),
            PcmMuLaw => Encoder::new(PcmXLawEncoderWrap::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec, XLaw::MuLaw)),
            #[cfg(feature = "mp3enc")]
            Mp3(ref mp3_options) => Encoder::new(Mp3Encoder::<f32>::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec, mp3_options)?),
            #[cfg(feature = "opus")]
            Opus(ref opus_options) => Encoder::new(OpusEncoder::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), spec, opus_options)?),
            #[cfg(feature = "flac")]
            Flac(ref flac_options) => Encoder::new(FlacEncoderWrap::new(hacks::force_borrow_mut!(*self.writer, dyn Writer), flac_options)?),
            Unspecified => return Err(AudioWriteError::InvalidArguments(format!("`data_format` is {}.", self.data_format))),
            #[allow(unreachable_patterns)]
            other => return Err(AudioWriteError::InvalidArguments(format!("`data_format` is {other} which is a disabled feature."))),
        };
        Ok(())
    }

    fn write_header(&mut self) -> Result<(), AudioWriteError> {
        use SampleFormat::{Int, UInt, Float};

        self.riff_chunk = Some(ChunkWriter::begin(hacks::force_borrow_mut!(*self.writer, dyn Writer), b"RIFF")?);

        // The first 4 bytes of the `RIFF` or `RF64` chunk must be `WAVE`. Then follows each chunk.
        self.writer.write_all(b"WAVE")?;

        // If the WAV file may exceed 4GB in size, the RF64 format must be used. 
        // This requires reserving a JUNK chunk after the WAVE header as a placeholder for the ds64 metadata.
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => (),
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                let cw = ChunkWriter::begin(&mut self.writer, b"JUNK")?;
                cw.writer.write_all(&[0u8; 28])?;
            },
        }

        // Uses the encoder's `new_fmt_chunk()` to generate the fmt chunk data.
        self.fmt__chunk = self.encoder.new_fmt_chunk()?;

        let mut cw = ChunkWriter::begin(&mut self.writer, b"fmt ")?;
        self.fmt_chunk_offset = cw.writer.stream_position()?;
        self.fmt__chunk.write(&mut cw.writer)?;
        cw.end();

        // Reserves space here for the fact chunk, to be updated later.
        let mut cw = ChunkWriter::begin(&mut self.writer, b"fact")?;
        self.fact_chunk_offset = cw.writer.stream_position()?;
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                0u32.write_le(&mut cw.writer)?;
            },
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                0u64.write_le(&mut cw.writer)?;
            },
        }
        cw.end();

        self.data_chunk = Some(ChunkWriter::begin(hacks::force_borrow_mut!(*self.writer, dyn Writer), b"data")?);
        self.data_offset = self.data_chunk.as_ref().unwrap().get_chunk_start_pos();

        self.encoder.begin_encoding()?;

        Ok(())
    }

    /// Stores audio samples. The generic parameter `S` represents the user-provided input format.
    /// The encoder converts samples to the internal target format before encoding them into the WAV file.
    pub fn write_samples<S>(&mut self, samples: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_samples(samples)?;
            self.num_frames_written += (samples.len() / self.spec.channels as usize) as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Saves a single mono sample. Avoid frequent calls due to inefficiency.
    pub fn write_sample<S>(&mut self, mono: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_sample(mono)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Batch-saves mono samples.
    pub fn write_mono_channel<S>(&mut self, monos: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_mono_channel(monos)?;
            self.num_frames_written += monos.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Batch-saves multiple mono channels.
    pub fn write_monos<S>(&mut self, monos: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_monos(monos)?;
            self.num_frames_written += monos[0].len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Saves a single stereo sample (left + right). Avoid frequent calls due to inefficiency.
    pub fn write_stereo<S>(&mut self, stereo: (S, S)) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_stereo(stereo)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Batch-saves stereo samples.
    pub fn write_stereos<S>(&mut self, stereos: &[(S, S)]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            if self.spec.channels != 2 {
                return Err(AudioWriteError::WrongChannels(format!("Can't write stereo audio to {} channels audio file.", self.spec.channels)));
            }
            self.encoder.write_stereos(stereos)?;
            self.num_frames_written += stereos.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Saves two mono samples (as one stereo frame). Avoid frequent calls due to inefficiency.
    pub fn write_dual_mono<S>(&mut self, mono1: S, mono2: S) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_dual_mono(mono1, mono2)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Batch-saves pairs of mono samples (as stereo audio).
    pub fn write_dual_monos<S>(&mut self, mono1: &[S], mono2: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_dual_monos(mono1, mono2)?;
            self.num_frames_written += mono1.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Saves one audio frame. Avoid frequent calls due to inefficiency. Supports multi-channel layouts.
    pub fn write_frame<S>(&mut self, frame: &[S]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_frame(frame)?;
            self.num_frames_written += 1;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    /// Batch-saves audio frames. Supports multi-channel layouts.
    pub fn write_frames<S>(&mut self, frames: &[Vec<S>]) -> Result<(), AudioWriteError>
    where S: SampleType {
        if self.data_chunk.is_some() {
            self.encoder.write_frames(frames, self.spec.channels)?;
            self.num_frames_written += frames.len() as u64;
            Ok(())
        } else {
            Err(AudioWriteError::AlreadyFinished("The `data` chunk was sealed, and no longer accepts new samples to be encoded.".to_owned()))
        }
    }

    pub fn spec(&self) -> Spec{
        self.spec
    }
    pub fn get_data_format(&self) -> DataFormat {
        self.data_format
    }
    pub fn get_num_frames_written(&self) -> u64 {
        self.num_frames_written
    }
    /// * See `WaveReader`
    pub fn add_slnt_chunk(&mut self, chunk: &SlntChunk) {
        self.slnt_chunk.push(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_bext_chunk(&mut self, chunk: &BextChunk) {
        self.bext_chunk.push(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_smpl_chunk(&mut self, chunk: &SmplChunk) {
        self.smpl_chunk.push(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_inst_chunk(&mut self, chunk: &InstChunk) {
        self.inst_chunk.push(*chunk);
    }
    /// * See `WaveReader`
    pub fn set_plst_chunk(&mut self, chunk: &PlstChunk) {
        self.plst_chunk = Some(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn set_cue__chunk(&mut self, chunk: &CueChunk) {
        self.cue__chunk = Some(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_axml_chunk(&mut self, chunk: &String) {
        self.axml_chunk.push(chunk.to_owned());
    }
    /// * See `WaveReader`
    pub fn add_ixml_chunk(&mut self, chunk: &String) {
        self.ixml_chunk.push(chunk.to_owned());
    }
    /// * See `WaveReader`
    pub fn add_list_chunk(&mut self, chunk: &ListChunk) {
        self.list_chunk.push(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_acid_chunk(&mut self, chunk: &AcidChunk) {
        self.acid_chunk.push(chunk.clone());
    }
    /// * See `WaveReader`
    pub fn add_trkn_chunk(&mut self, chunk: &String) {
        self.trkn_chunk.push(chunk.to_owned());
    }
    /// * See `WaveReader`
    pub fn add_junk_chunk(&mut self, chunk: &JunkChunk) {
        self.junk_chunks.push(chunk.clone());
    }

    /// Transfers audio metadata (e.g., track info) from the reader.
    pub fn inherit_metadata_from_reader(&mut self, reader: &WaveReader, include_junk_chunks: bool) {
        if !reader.get_slnt_chunk().is_empty() {self.slnt_chunk.extend(reader.get_slnt_chunk().clone());}
        if !reader.get_bext_chunk().is_empty() {self.bext_chunk.extend(reader.get_bext_chunk().clone());}
        if !reader.get_smpl_chunk().is_empty() {self.smpl_chunk.extend(reader.get_smpl_chunk().clone());}
        if !reader.get_inst_chunk().is_empty() {self.inst_chunk.extend(reader.get_inst_chunk().clone());}
        if reader.get_plst_chunk().is_some() {self.plst_chunk = reader.get_plst_chunk().clone();}
        if reader.get_cue__chunk().is_some() {self.cue__chunk = reader.get_cue__chunk().clone();}
        if !reader.get_axml_chunk().is_empty() {self.axml_chunk.extend(reader.get_axml_chunk().clone());}
        if !reader.get_ixml_chunk().is_empty() {self.ixml_chunk.extend(reader.get_ixml_chunk().clone());}
        if !reader.get_list_chunk().is_empty() {self.list_chunk.extend(reader.get_list_chunk().clone());}
        if !reader.get_acid_chunk().is_empty() {self.acid_chunk.extend(reader.get_acid_chunk().clone());}
        if !reader.get_trkn_chunk().is_empty() {self.trkn_chunk.extend(reader.get_trkn_chunk().clone());}
        if reader.get_id3__chunk().is_some() {self.id3__chunk = reader.get_id3__chunk().clone();}
        if include_junk_chunks {
            self.junk_chunks.extend(reader.get_junk_chunks().clone());
        }
    }

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

    fn on_drop(&mut self) -> Result<(), AudioWriteError> {
        // Finalizes writing to the data chunk and updates relevant parameters in the `fmt` chunk.
        self.encoder.finish()?;

        // Finalizes writing to the data chunk and records its size.
        let mut data_size = 0u64;
        if let Some(data_chunk) = &self.data_chunk {
            data_size = self.writer.stream_position()? - data_chunk.get_chunk_start_pos();
            self.data_chunk = None;
        }

        let end_of_data = self.writer.stream_position()?;

        // Updates `fmt` chunk fields (e.g., byte_rate, extension data) and rewrites the header.
        self.writer.seek(SeekFrom::Start(self.fmt_chunk_offset))?;
        self.encoder.update_fmt_chunk(&mut self.fmt__chunk)?;
        self.fmt__chunk.write(&mut self.writer)?;

        // Updates `fact` chunk data, the total number of samples written to the `data` chunk.
        self.writer.seek(SeekFrom::Start(self.fact_chunk_offset))?;
        let fact_data = self.num_frames_written * self.spec.channels as u64;
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                (fact_data.clamp(0, 0xFFFFFFFF) as u32).write_le(&mut self.writer)?;
            },
            FileSizeOption::AllowLargerThan4GB | FileSizeOption::ForceUse4GBFormat => {
                fact_data.write_le(&mut self.writer)?;
            },
        }

        // Get back to the end of the data chunk, and then write all remaining chunks (metadata, auxiliary data) to the file.
        self.writer.seek(SeekFrom::Start(end_of_data))?;
        self.bext_chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer, &self.text_encoding).unwrap();});
        self.smpl_chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer).unwrap();});
        self.inst_chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer).unwrap();});
        self.cue__chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer).unwrap();});
        self.list_chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer, &self.text_encoding).unwrap();});
        self.acid_chunk.iter().for_each(|chunk|{chunk.write(&mut self.writer).unwrap();});
        if let Some(chunk) = &self.id3__chunk {
            let mut cw = ChunkWriter::begin(&mut self.writer, b"id3 ")?;
            Id3::id3_write(chunk, &mut cw.writer)?;
        }

        // Writes all remaining string-based chunks to the file.
        let mut string_chunks_to_write = Vec::<([u8; 4], &String)>::new();
        self.axml_chunk.iter().for_each(|chunk|{string_chunks_to_write.push((*b"axml", chunk))});
        self.ixml_chunk.iter().for_each(|chunk|{string_chunks_to_write.push((*b"ixml", chunk))});
        self.trkn_chunk.iter().for_each(|chunk|{string_chunks_to_write.push((*b"Trkn", chunk))});
        for (flag, chunk) in string_chunks_to_write.iter() {
            let mut cw = ChunkWriter::begin(&mut self.writer, flag)?;
            write_str(&mut cw.writer, chunk, &self.text_encoding)?;
        }

        // Writes all JUNK chunks to the file.
        self.junk_chunks.iter().for_each(|chunk|{chunk.write(&mut self.writer).unwrap();});

        self.riff_chunk = None;

        // Critical large-file handling workflow:
        // ---------------------------------------------------------------------
        // 1. RF64 Header Conversion:
        //    - If total file size exceeds 4GB (u32::MAX): 
        //      a. Overwrite the initial 'RIFF' header with 'RF64'
        //      b. Write the ds64 chunk immediately after, containing:
        //         - riff_size: u64
        //         - data_size: u64 (actual data chunk length)
        //         - table: Vec<(u32, u64)> (maps original chunk IDs to 64-bit sizes)
        //
        // 2. Backfill Pre-Reserved Regions:
        //    - Replace the JUNK chunk placeholder (reserved via `write_junk()`) 
        //      with the ds64 chunk's binary data.
        //    - Update all chunk size fields marked with 0xFFFFFFFF during encoding 
        //      using the ds64 table entries.
        //
        // 3. Error Handling:
        //    - Fails if RF64 is required but no JUNK placeholder was pre-reserved.
        //    - Callers must invoke `prepare_rf64_placeholder()` before writing chunks
        //      that may exceed 4GB.
        let file_end_pos = self.writer.stream_position()?;
        let mut change_to_4gb_hreader = || -> Result<(), AudioWriteError> {
            self.writer.seek(SeekFrom::Start(0))?;
            self.writer.write_all(b"RF64")?;
            0xFFFFFFFFu32.write_le(&mut self.writer)?;
            self.writer.write_all(b"WAVE")?;
            self.writer.write_all(b"ds64")?;
            28u32.write_le(&mut self.writer)?; // Length of the `ds64` chunk
            let riff_size = file_end_pos - 8;
            let sample_count = fact_data;
            riff_size.write_le(&mut self.writer)?;
            data_size.write_le(&mut self.writer)?;
            sample_count.write_le(&mut self.writer)?;
            0u32.write_le(&mut self.writer)?; // table length
            Ok(())
        };
        match self.file_size_option {
            FileSizeOption::NeverLargerThan4GB => {
                if file_end_pos > 0xFFFFFFFFu64 {
                    Err(AudioWriteError::NotPreparedFor4GBFile)?;
                }
            },
            FileSizeOption::AllowLargerThan4GB => {
                if file_end_pos > 0xFFFFFFFFu64 {
                    change_to_4gb_hreader()?;
                }
            },
            FileSizeOption::ForceUse4GBFormat => {
                change_to_4gb_hreader()?;
            },
        }
        self.writer.flush()?;
        Ok(())
    }

    pub fn finalize(self) {}
}

impl Drop for WaveWriter<'_> {
    fn drop(&mut self) {
        self.on_drop().unwrap()
    }
}
