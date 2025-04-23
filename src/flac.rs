#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
// need `libflac-sys`

use std::{any::type_name, io::{self, ErrorKind}, fmt::{self, Debug, Display, Formatter}, slice, ffi::{CStr, c_void}, ptr, collections::BTreeMap};

use libflac_sys::*;

#[cfg(feature = "id3")]
use id3::{self, TagLike};

#[derive(Debug, Clone, Copy)]
pub enum FlacCompression {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
    Level6 = 6,
    Level7 = 7,
    Level8 = 8
}

pub trait FlacError {
    fn get_code(&self) -> u32;
    fn get_message(&self) -> &'static str;
    fn get_function(&self) -> &'static str;
    fn get_message_from_code(&self) -> &'static str;

    fn format(&self, f: &mut Formatter) -> fmt::Result {
        let code = self.get_code();
        let message = self.get_message();
        let function = self.get_function();
        write!(f, "Code: {code}, function: {function}, message: {message}")?;
        Ok(())
    }
}

macro_rules! impl_FlacError {
    ($error:ty) => {
        impl FlacError for $error {
            fn get_code(&self) -> u32 {self.code}
            fn get_message(&self) -> &'static str {self.message}
            fn get_function(&self) -> &'static str {self.function}
            fn get_message_from_code(&self) -> &'static str {
                Self::get_message_from_code(self.get_code())
            }
        }

        impl std::error::Error for $error {}

        impl Display for $error {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                <$error as FlacError>::format(self, f)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacEncoderError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamEncoderStateString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacEncoderError);

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderInitError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacEncoderInitError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamEncoderInitStatusString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacEncoderInitError);

impl From<FlacEncoderError> for FlacEncoderInitError {
    fn from(err: FlacEncoderError) -> Self {
        Self {
            code: err.code,
            message: err.message,
            function: err.function,
        }
    }
}

impl From<FlacEncoderInitError> for FlacEncoderError {
    fn from(err: FlacEncoderInitError) -> Self {
        Self {
            code: err.code,
            message: err.message,
            function: err.function,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlacEncoderParams {
    pub verify_decoded: bool,
    pub compression: FlacCompression,
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u32,
    pub total_samples_estimate: u64,
}

impl FlacEncoderParams {
    pub fn new() -> Self {
        Self {
            verify_decoded: false,
            compression: FlacCompression::Level5,
            channels: 2,
            sample_rate: 44100,
            bits_per_sample: 16,
            total_samples_estimate: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct CueTrack {
    offset: u64,
    isrc: [i8; 13],
    type_: u32,
    pre_emphasis: u32,
}

impl CueTrack {
    pub fn new(offset: u64, isrc: [i8; 13]) -> Self {
        Self {
            offset,
            isrc,
            type_: 0,
            pre_emphasis: 0,
        }
    }
}

impl Debug for CueTrack {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("CueTrack")
            .field("offset", &self.offset)
            .field("isrc", &self.isrc)
            .field("type", &self.type_)
            .field("pre_emphasis", &self.pre_emphasis)
            .finish()
    }
}

impl Display for CueTrack {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("CueTrack")
            .field("offset", &self.offset)
            .field("isrc", &self.isrc)
            .field("type", &self.type_)
            .field("pre_emphasis", &self.pre_emphasis)
            .finish()
    }
}

pub const COMMENT_KEYS: [&str; 31] = [
    "ACTOR",
    "ALBUM",
    "ARTIST",
    "COMMENT",
    "COMPOSER",
    "CONTACT",
    "COPYRIGHT",
    "COVERART",
    "COVERARTMIME",
    "DATE",
    "DESCRIPTION",
    "DIRECTOR",
    "ENCODED_BY",
    "ENCODED_USING",
    "ENCODER",
    "ENCODER_OPTIONS",
    "GENRE",
    "ISRC",
    "LICENSE",
    "LOCATION",
    "ORGANIZATION",
    "PERFORMER",
    "PRODUCER",
    "REPLAYGAIN_ALBUM_GAIN",
    "REPLAYGAIN_ALBUM_PEAK",
    "REPLAYGAIN_TRACK_GAIN",
    "REPLAYGAIN_TRACK_PEAK",
    "TITLE",
    "TRACKNUMBER",
    "VERSION",
    "vendor"
];

#[derive(Clone)]
pub struct PictureData {
    pub picture: Vec<u8>,
    pub mime_type: String,
    pub description: String,
}

impl Debug for PictureData {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("PictureData")
            .field("picture", &format_args!("[u8; {}]", self.picture.len()))
            .field("mime_type", &self.mime_type)
            .field("description", &self.description)
            .finish()
    }
}

impl PictureData {
    pub fn new() -> Self {
        Self {
            picture: Vec::<u8>::new(),
            mime_type: "".to_owned(),
            description: "".to_owned(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.picture.is_empty()
    }
}

impl Default for PictureData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
#[repr(C)]
struct FlacMetadata {
    // https://xiph.org/flac/api/group__flac__metadata__object.html
    metadata: *mut FLAC__StreamMetadata,
}

#[derive(Debug)]
#[repr(C)]
struct FlacCueTrack {
    track: *mut FLAC__StreamMetadata_CueSheet_Track,
}

impl FlacCueTrack {
    pub fn new() -> Result<Self, FlacEncoderError> {
        let ret = Self {
            track: unsafe {FLAC__metadata_object_cuesheet_track_new()},
        };
        if ret.track.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_cuesheet_track_new"))
        } else {
            Ok(ret)
        }
    }

    pub fn get_ref_mut(&mut self) -> &mut FLAC__StreamMetadata_CueSheet_Track {
        unsafe {&mut *self.track}
    }

    pub fn get_ptr(&self) -> *const FLAC__StreamMetadata_CueSheet_Track{
        self.track as *const FLAC__StreamMetadata_CueSheet_Track
    }

    pub fn get_mut_ptr(&self) -> *mut FLAC__StreamMetadata_CueSheet_Track{
        self.track
    }
}

impl Default for FlacCueTrack {
    fn default() -> Self {
        Self {
            track: ptr::null_mut(),
        }
    }
}

impl Drop for FlacCueTrack {
    fn drop(&mut self) {
        if !self.track.is_null() {
            unsafe {FLAC__metadata_object_cuesheet_track_delete(self.track)};
            self.track = ptr::null_mut();
        }
    }
}

fn make_sz(s: &str) -> String {
    let mut s = s.to_owned();
    s.push_str("\0");
    s
}

impl FlacMetadata {
    pub fn new_vorbis_comment() -> Result<Self, FlacEncoderError> {
        let ret = Self {
            metadata: unsafe {FLAC__metadata_object_new(FLAC__METADATA_TYPE_VORBIS_COMMENT)},
        };
        if ret.metadata.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_new(FLAC__METADATA_TYPE_VORBIS_COMMENT)"))
        } else {
            Ok(ret)
        }
    }

    pub fn new_cue_sheet() -> Result<Self, FlacEncoderError> {
        let ret = Self {
            metadata: unsafe {FLAC__metadata_object_new(FLAC__METADATA_TYPE_CUESHEET)},
        };
        if ret.metadata.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_new(FLAC__METADATA_TYPE_CUESHEET)"))
        } else {
            Ok(ret)
        }
    }

    pub fn new_picture() -> Result<Self, FlacEncoderError> {
        let ret = Self {
            metadata: unsafe {FLAC__metadata_object_new(FLAC__METADATA_TYPE_PICTURE)},
        };
        if ret.metadata.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_new(FLAC__METADATA_TYPE_PICTURE)"))
        } else {
            Ok(ret)
        }
    }

    pub fn insert_comments(&self, key: &'static str, value: &str) -> Result<(), FlacEncoderError> {
        unsafe {
            // ATTENTION:
            // Any strings to be added to the entry must be NUL terminated.
            // Or you can see the `FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR` due to the failure to find the NUL terminator.
            let szkey = make_sz(key);
            let szvalue = make_sz(value);
            let mut entry = FLAC__StreamMetadata_VorbisComment_Entry{length: 0, entry: ptr::null_mut()};
            if FLAC__metadata_object_vorbiscomment_entry_from_name_value_pair (
                &mut entry as *mut FLAC__StreamMetadata_VorbisComment_Entry,
                szkey.as_ptr() as *mut i8,
                szvalue.as_ptr() as *mut i8
            ) == 0 {
                eprintln!("On set comment {key}: {value}: {:?}", FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_vorbiscomment_entry_from_name_value_pair"));
            }
            if FLAC__metadata_object_vorbiscomment_append_comment(self.metadata, entry, 0) == 0 {
                eprintln!("On set comment {key}: {value}: {:?}", FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_vorbiscomment_append_comment"));
            }
        }
        Ok(())
    }

    pub fn insert_cue_track(&mut self, track_no: u8, cue_track: &CueTrack) -> Result<(), FlacEncoderError> {
        unsafe {
            let mut track = FlacCueTrack::new()?;
            let track_data = track.get_ref_mut();
            track_data.offset = cue_track.offset;
            track_data.number = track_no;
            track_data.isrc = cue_track.isrc;
            track_data.set_type(cue_track.type_);
            track_data.set_pre_emphasis(cue_track.pre_emphasis);
            track_data.num_indices = 0;
            track_data.indices = ptr::null_mut();
            if FLAC__metadata_object_cuesheet_set_track(self.metadata, track_no as u32, track.get_mut_ptr(), 0) == 0 {
                eprintln!("Failed to create new cuesheet track for {track_no} {cue_track}:  {:?}", FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_cuesheet_set_track"));
            }
        }
        Ok(())
    }

    pub fn set_picture(&mut self, picture_binary: &mut [u8], description: &mut str, mime_type: &mut str) -> Result<(), FlacEncoderError> {
        let mut desc_sz = make_sz(description);
        let mut mime_sz = make_sz(mime_type);
        unsafe {
            if FLAC__metadata_object_picture_set_data(self.metadata, picture_binary.as_mut_ptr(), picture_binary.len() as u32, 0) == 0 {
                Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_picture_set_data"))
            } else if FLAC__metadata_object_picture_set_mime_type(self.metadata, desc_sz.as_mut_ptr() as *mut i8, 0) == 0 {
                Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_picture_set_mime_type"))
            } else if FLAC__metadata_object_picture_set_description(self.metadata, mime_sz.as_mut_ptr(), 0) == 0 {
                Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__metadata_object_picture_set_description"))
            } else {
                Ok(())
            }
        }
    }
}

impl Default for FlacMetadata {
    fn default() -> Self {
        Self {
            metadata: ptr::null_mut(),
        }
    }
}

impl Drop for FlacMetadata {
    fn drop(&mut self) {
        if !self.metadata.is_null() {
            unsafe {FLAC__metadata_object_delete(self.metadata)};
            self.metadata = ptr::null_mut();
        }
    }
}

pub struct FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    // https://xiph.org/flac/api/group__flac__stream__encoder.html
    encoder: *mut FLAC__StreamEncoder,
    metadata: Vec<FlacMetadata>,
    encoder_initialized: bool,
    params: FlacEncoderParams,
    on_write: Wr,
    on_seek: Sk,
    on_tell: Tl,
    comments: BTreeMap<&'static str, String>,
    cue_sheet: BTreeMap<u8, CueTrack>,
    picture_data: PictureData,
}

impl<Wr, Sk, Tl> FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    pub fn new(
        on_write: Wr,
        on_seek: Sk,
        on_tell: Tl,
        params: &FlacEncoderParams
    ) -> Result<Self, FlacEncoderError> {
        let ret = Self {
            encoder: unsafe {FLAC__stream_encoder_new()},
            metadata: Vec::<FlacMetadata>::new(),
            encoder_initialized: false,
            params: *params,
            on_write,
            on_seek,
            on_tell,
            comments: BTreeMap::new(),
            cue_sheet: BTreeMap::new(),
            picture_data: PictureData::new(),
        };
        if ret.encoder.is_null() {
            Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_MEMORY_ALLOCATION_ERROR, "FLAC__stream_encoder_new"))
        } else {
            Ok(ret)
        }
    }

    fn get_status_as_result(&self, function: &'static str) -> Result<(), FlacEncoderError> {
        let code = unsafe {FLAC__stream_encoder_get_state(self.encoder)};
        if code == 0 {
            Ok(())
        } else {
            Err(FlacEncoderError::new(code, function))
        }
    }

    fn get_status_as_error(&self, function: &'static str) -> Result<(), FlacEncoderError> {
        let code = unsafe {FLAC__stream_encoder_get_state(self.encoder)};
        Err(FlacEncoderError::new(code, function))
    }

    fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }

    fn insert_comments(&mut self, key: &'static str, value: &str) -> Result<(), FlacEncoderInitError> {
        if self.encoder_initialized {
            Err(FlacEncoderInitError::new(FLAC__STREAM_ENCODER_INIT_STATUS_ALREADY_INITIALIZED, "FlacEncoderUnmovable::insert_comments"))
        } else {
            if let Some(old_value) = self.comments.insert(key, value.to_owned()) {
                eprintln!("\"{key}\" is changed to \"{value}\" from \"{old_value}\"");
            }
            Ok(())
        }
    }

    fn insert_cue_track(&mut self, track_no: u8, cue_track: &CueTrack) -> Result<(), FlacEncoderInitError> {
        if self.encoder_initialized {
            Err(FlacEncoderInitError::new(FLAC__STREAM_ENCODER_INIT_STATUS_ALREADY_INITIALIZED, "FlacEncoderUnmovable::insert_cue_track"))
        } else {
            if let Some(old_track) = self.cue_sheet.insert(track_no, *cue_track) {
                eprintln!("Track index {old_track} is changed to {:?} from {:?}", cue_track, old_track);
            }
            Ok(())
        }
    }

    fn set_picture(&mut self, picture_binary: &[u8], description: &str, mime_type: &str) -> Result<(), FlacEncoderInitError> {
        if self.encoder_initialized {
            Err(FlacEncoderInitError::new(FLAC__STREAM_ENCODER_INIT_STATUS_ALREADY_INITIALIZED, "FlacEncoderUnmovable::set_picture"))
        } else {
            self.picture_data.picture = picture_binary.to_vec();
            self.picture_data.description = description.to_owned();
            self.picture_data.mime_type = mime_type.to_owned();
            Ok(())
        }
    }

    #[cfg(feature = "id3")]
    fn migrate_metadata_from_id3(&mut self, tag: &id3::Tag) -> Result<(), FlacEncoderInitError> {
        if let Some(artist) = tag.artist() {self.insert_comments("ARTIST", artist)?;}
        if let Some(album) = tag.album() {self.insert_comments("ALBUM", album)?;}
        if let Some(title) = tag.title() {self.insert_comments("TITLE", title)?;}
        if let Some(genre) = tag.genre() {self.insert_comments("GENRE", genre)?;}
        if let Some(picture) = tag.pictures().next() {
            self.set_picture(&picture.data, &picture.description, &picture.mime_type)?;
        }
        let comm_str = tag.comments().enumerate().map(|(i, comment)| -> String {
            let lang = &comment.lang;
            let desc = &comment.description;
            let text = &comment.text;
            format!("Comment {i}:\n\tlang: {lang}\n\tdesc: {desc}\n\ttext: {text}")
        }).collect::<Vec<String>>().join("\n");
        if !comm_str.is_empty() {self.insert_comments("COMMENT", &comm_str)?;}
        Ok(())
    }

    fn init(&mut self) -> Result<(), FlacEncoderError> {
        if self.encoder_initialized {
            return Err(FlacEncoderInitError::new(FLAC__STREAM_ENCODER_INIT_STATUS_ALREADY_INITIALIZED, "FlacEncoderUnmovable::init").into())
        }
        unsafe {
            if FLAC__stream_encoder_set_verify(self.encoder, if self.params.verify_decoded {1} else {0}) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_verify");
            }
            if FLAC__stream_encoder_set_compression_level(self.encoder, self.params.compression as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_compression_level");
            }
            if FLAC__stream_encoder_set_channels(self.encoder, self.params.channels as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_channels");
            }
            if FLAC__stream_encoder_set_bits_per_sample(self.encoder, self.params.bits_per_sample) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_bits_per_sample");
            }
            if FLAC__stream_encoder_set_sample_rate(self.encoder, self.params.sample_rate) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_sample_rate");
            }
            if self.params.total_samples_estimate > 0 && FLAC__stream_encoder_set_total_samples_estimate(self.encoder, self.params.total_samples_estimate) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_set_total_samples_estimate");
            }

            let set_metadata: Result<(), FlacEncoderError> = {
                if !self.comments.is_empty() {
                    let metadata = FlacMetadata::new_vorbis_comment()?;
                    for (key, value) in self.comments.iter() {
                        metadata.insert_comments(key, value)?;
                    }
                    self.metadata.push(metadata);
                }
                if !self.cue_sheet.is_empty() {
                    let mut metadata = FlacMetadata::new_cue_sheet()?;
                    for (track_no, cue_track) in self.cue_sheet.iter() {
                        metadata.insert_cue_track(*track_no, cue_track)?;
                    }
                    self.metadata.push(metadata);
                }
                if !self.picture_data.is_empty() {
                    let mut metadata = FlacMetadata::new_picture()?;
                    metadata.set_picture(&mut self.picture_data.picture, &mut self.picture_data.description, &mut self.picture_data.mime_type)?;
                    self.metadata.push(metadata);
                }
                if !self.metadata.is_empty() {
                    if FLAC__stream_encoder_set_metadata(self.encoder, self.metadata.as_mut_ptr() as *mut *mut FLAC__StreamMetadata, self.metadata.len() as u32) == 0 {
                        Err(FlacEncoderError::new(FLAC__STREAM_ENCODER_INIT_STATUS_ALREADY_INITIALIZED, "FLAC__stream_encoder_set_metadata"))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            };
            if let Err(e) = set_metadata {
                eprintln!("When setting the metadata: {:?}", e);
            }
            let ret = FLAC__stream_encoder_init_stream(self.encoder,
                Some(Self::write_callback),
                Some(Self::seek_callback),
                Some(Self::tell_callback),
                Some(Self::metadata_callback),
                self.as_mut_ptr() as *mut c_void,
            );
            if ret != 0 {
                return Err(FlacEncoderInitError::new(ret, "FLAC__stream_encoder_init_stream").into());
            } else {
                self.encoder_initialized = true;
            }
        }
        self.get_status_as_result("FlacEncoderUnmovable::Init()")
    }

    unsafe extern "C" fn write_callback(_encoder: *const FLAC__StreamEncoder, buffer: *const u8, bytes: usize, _samples: u32, _current_frame: u32, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_write)(slice::from_raw_parts(buffer, bytes)) {
            Ok(_) => FLAC__STREAM_ENCODER_WRITE_STATUS_OK,
            Err(e) => {
                eprintln!("On `write_callback()`: {:?}", e);
                FLAC__STREAM_ENCODER_WRITE_STATUS_FATAL_ERROR
            },
        }
    }

    unsafe extern "C" fn seek_callback(_encoder: *const FLAC__StreamEncoder, absolute_byte_offset: u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_seek)(absolute_byte_offset) {
            Ok(_) => FLAC__STREAM_ENCODER_SEEK_STATUS_OK,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_ENCODER_SEEK_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_ENCODER_SEEK_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn tell_callback(_encoder: *const FLAC__StreamEncoder, absolute_byte_offset: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_tell)() {
            Ok(offset) => {
                *absolute_byte_offset = offset;
                FLAC__STREAM_ENCODER_TELL_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_ENCODER_TELL_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_ENCODER_TELL_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn metadata_callback(_encoder: *const FLAC__StreamEncoder, metadata: *const FLAC__StreamMetadata, client_data: *mut c_void) {
        let _this = &mut *(client_data as *mut Self);
        let meta = *metadata;
        println!("{:?}", WrappedStreamMetadata(meta))
    }

    pub fn write_monos(&mut self, monos: &[i32]) -> Result<(), FlacEncoderError> {
        if monos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => unsafe {
                if FLAC__stream_encoder_process_interleaved(self.encoder, monos.as_ptr(), monos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            2 => self.write_stereos(&monos.iter().map(|mono| -> (i32, i32){(*mono, *mono)}).collect::<Vec<(i32, i32)>>()),
            o => self.write_frames(&monos.iter().map(|mono| -> Vec<i32> {(0..o).map(|_|{*mono}).collect()}).collect::<Vec<Vec<i32>>>()),
        }
    }

    pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), FlacEncoderError> {
        if stereos.is_empty() {return Ok(())}
        match self.params.channels {
            1 => self.write_monos(&stereos.iter().map(|(l, r): &(i32, i32)| -> i32 {((*l as i64 + *r as i64) / 2) as i32}).collect::<Vec<i32>>()),
            2 => unsafe {
                let samples: Vec<i32> = stereos.iter().flat_map(|(l, r): &(i32, i32)| -> [i32; 2] {[*l, *r]}).collect();
                if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr(), stereos.len() as u32) == 0 {
                    return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
                }
                Ok(())
            },
            o => panic!("Can't turn stereo audio into {o} channels audio."),
        }
    }

    pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), FlacEncoderError> {
        if frames.is_empty() {return Ok(())}
        let samples: Vec<i32> = frames.iter().flat_map(|frame: &Vec<i32>| -> Vec<i32> {
            if frame.len() != self.params.channels as usize {
                panic!("On FlacEncoderUnmovable::write_frames(): a frame size {} does not match the encoder channels.", frame.len())
            } else {frame.to_vec()}
        }).collect();
        unsafe {
            if FLAC__stream_encoder_process_interleaved(self.encoder, samples.as_ptr(), frames.len() as u32) == 0 {
                return self.get_status_as_error("FLAC__stream_encoder_process_interleaved");
            }
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), FlacEncoderError> {
        unsafe {
            if FLAC__stream_encoder_finish(self.encoder) != 0 {
                Ok(())
            } else {
                self.get_status_as_error("FLAC__stream_encoder_finish")
            }
        }
    }

    fn on_drop(&mut self) {
        unsafe {
            if let Err(e) = self.finish() {
                eprintln!("On FlacEncoderUnmovable::finish(): {:?}", e);
            }

            self.metadata.clear();
            FLAC__stream_encoder_delete(self.encoder);
        };
    }

    pub fn finalize(self) {}
}

impl<Wr, Sk, Tl> Debug for FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacEncoderUnmovable<{}, {}, {}>", type_name::<Wr>(), type_name::<Sk>(), type_name::<Tl>()))
            .field("encoder", &self.encoder)
            .field("params", &self.params)
            .field("on_write", &format_args!("0x{:x}", &self.on_write as *const Wr as usize))
            .field("on_seek", &format_args!("0x{:x}", &self.on_seek as *const Sk as usize))
            .field("on_tell", &format_args!("0x{:x}", &self.on_tell as *const Tl as usize))
            .field("comments", &self.comments)
            .field("cue_sheet", &self.cue_sheet)
            .field("picture", &format_args!("..."))
            .finish()
    }
}

impl<Wr, Sk, Tl> Drop for FlacEncoderUnmovable<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn drop(&mut self) {
        self.on_drop();
    }
}

pub struct FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    encoder: Box<FlacEncoderUnmovable<Wr, Sk, Tl>>,
}

impl<Wr, Sk, Tl> FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    pub fn new(
        on_write: Wr,
        on_seek: Sk,
        on_tell: Tl,
        params: &FlacEncoderParams
    ) -> Result<Self, FlacEncoderError> {
        Ok(Self {
            encoder: Box::new(FlacEncoderUnmovable::new(on_write, on_seek, on_tell, params)?)
        })
    }

    pub fn insert_comments(&mut self, key: &'static str, value: &str) -> Result<(), FlacEncoderInitError> {
        self.encoder.insert_comments(key, value)
    }

    pub fn insert_cue_track(&mut self, track_no: u8, cue_track: &CueTrack) -> Result<(), FlacEncoderInitError> {
        self.encoder.insert_cue_track(track_no, cue_track)
    }

    pub fn set_picture(&mut self, picture_binary: &[u8], description: &str, mime_type: &str) -> Result<(), FlacEncoderInitError> {
        self.encoder.set_picture(picture_binary, description, mime_type)
    }

    #[cfg(feature = "id3")]
    pub fn migrate_metadata_from_id3(&mut self, tag: &id3::Tag) -> Result<(), FlacEncoderInitError> {
        self.encoder.migrate_metadata_from_id3(tag)
    }

    fn ensure_initialized(&mut self) -> Result<(), FlacEncoderInitError> {
        if !self.encoder.encoder_initialized {
            self.encoder.init()?
        }
        Ok(())
    }

    pub fn write_monos(&mut self, monos: &[i32]) -> Result<(), FlacEncoderError> {
        self.ensure_initialized()?;
        self.encoder.write_monos(monos)
    }

    pub fn write_stereos(&mut self, stereos: &[(i32, i32)]) -> Result<(), FlacEncoderError> {
        self.ensure_initialized()?;
        self.encoder.write_stereos(stereos)
    }

    pub fn write_frames(&mut self, frames: &[Vec<i32>]) -> Result<(), FlacEncoderError> {
        self.ensure_initialized()?;
        self.encoder.write_frames(frames)
    }

    pub fn finish(&mut self) -> Result<(), FlacEncoderError> {
        self.ensure_initialized()?;
        self.encoder.finish()
    }

    pub fn finalize(self) {}
}

impl<Wr, Sk, Tl> Debug for FlacEncoder<Wr, Sk, Tl>
where
    Wr: FnMut(&[u8]) -> Result<(), io::Error>,
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error> {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacEncoder<{}, {}, {}>", type_name::<Wr>(), type_name::<Sk>(), type_name::<Tl>()))
            .field("encoder", &self.encoder)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlacDecoderError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacDecoderError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamDecoderStateString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacDecoderError);

#[derive(Debug, Clone, Copy)]
pub struct FlacDecoderInitError {
    code: u32,
    message: &'static str,
    function: &'static str,
}

impl FlacDecoderInitError {
    pub fn new(code: u32, function: &'static str) -> Self {
        Self {
            code,
            message: Self::get_message_from_code(code),
            function,
        }
    }

    pub fn get_message_from_code(code: u32) -> &'static str {
        unsafe {
            CStr::from_ptr(*FLAC__StreamDecoderInitStatusString.as_ptr().add(code as usize)).to_str().unwrap()
        }
    }
}

impl_FlacError!(FlacDecoderInitError);

#[derive(Debug, Clone, Copy)]
pub enum ReadStatus {
    GoOn,
    Eof,
    Abort,
}

impl Display for ReadStatus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::GoOn => write!(f, "go_on"),
            Self::Eof => write!(f, "eof"),
            Self::Abort => write!(f, "abort"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DecoderError {
    LostSync,
    BadHeader,
    FrameCrcMismatch,
    UnparseableStream,
    BadMetadata,
    OutOfBounds,
}

impl Display for DecoderError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::LostSync => write!(f, "FLAC: An error in the stream caused the decoder to lose synchronization."),
            Self::BadHeader => write!(f, "FLAC: The decoder encountered a corrupted frame header."),
            Self::FrameCrcMismatch => write!(f, "FLAC: The frame's data did not match the CRC in the footer."),
            Self::UnparseableStream => write!(f, "FLAC: The decoder encountered reserved fields in use in the stream."),
            Self::BadMetadata => write!(f, "FLAC: The decoder encountered a corrupted metadata block."),
            Self::OutOfBounds => write!(f, "FLAC: The decoder encountered a otherwise valid frame in which the decoded samples exceeded the range offered by the stated bit depth."),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AudioForm {
    FrameArray,
    ChannelArray,
}

#[derive(Debug, Clone, Copy)]
pub struct SamplesInfo {
    pub samples: u32,
    pub channels: u32,
    pub sample_rate: u32,
    pub bits_per_sample: u32,
    pub audio_form: AudioForm,
}

pub struct FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>, // monos, sample_rate
    Er: FnMut(DecoderError) {
    // https://xiph.org/flac/api/group__flac__stream__decoder.html
    decoder: *mut FLAC__StreamDecoder,
    on_read: Rd,
    on_seek: Sk,
    on_tell: Tl,
    on_length: Ln,
    on_eof: Ef,
    on_write: Wr,
    on_error: Er,
    md5_checking: bool,
    pub scale_to_i32_range: bool,
    pub desired_audio_form: AudioForm,
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    pub fn new(
        on_read: Rd,
        on_seek: Sk,
        on_tell: Tl,
        on_length: Ln,
        on_eof: Ef,
        on_write: Wr,
        on_error: Er,
        md5_checking: bool,
        scale_to_i32_range: bool,
        desired_audio_form: AudioForm,
    ) -> Result<Self, FlacDecoderError> {
        let ret = Self {
            decoder: unsafe {FLAC__stream_decoder_new()},
            on_read,
            on_seek,
            on_tell,
            on_length,
            on_eof,
            on_write,
            on_error,
            md5_checking,
            scale_to_i32_range,
            desired_audio_form,
        };

        if ret.decoder.is_null() {
            Err(FlacDecoderError::new(FLAC__STREAM_DECODER_MEMORY_ALLOCATION_ERROR, "FLAC__stream_decoder_new"))
        } else {
            Ok(ret)
        }
    }

    fn get_status_as_result(&self, function: &'static str) -> Result<(), FlacDecoderError> {
        let code = unsafe {FLAC__stream_decoder_get_state(self.decoder)};
        if code == 0 {
            Ok(())
        } else {
            Err(FlacDecoderError::new(code, function))
        }
    }

    fn get_status_as_error(&self, function: &'static str) -> Result<(), FlacDecoderError> {
        let code = unsafe {FLAC__stream_decoder_get_state(self.decoder)};
        Err(FlacDecoderError::new(code, function))
    }

    fn as_ptr(&self) -> *const Self {
        self as *const Self
    }

    fn as_mut_ptr(&mut self) -> *mut Self {
        self as *mut Self
    }

    fn init(&mut self) -> Result<(), FlacDecoderError> {
        unsafe {
            if FLAC__stream_decoder_set_md5_checking(self.decoder, self.md5_checking as i32) == 0 {
                return self.get_status_as_error("FLAC__stream_decoder_set_md5_checking");
            }
            let ret = FLAC__stream_decoder_init_stream(
                self.decoder,
                Some(Self::read_callback),
                Some(Self::seek_callback),
                Some(Self::tell_callback),
                Some(Self::length_callback),
                Some(Self::eof_callback),
                Some(Self::write_callback),
                Some(Self::metadata_callback),
                Some(Self::error_callback),
                self.as_mut_ptr() as *mut c_void,
            );
            if ret != 0 {
                return Err(FlacDecoderError {
                    code: ret,
                    message: FlacDecoderInitError::get_message_from_code(ret),
                    function: "FLAC__stream_decoder_init_stream",
                });
            }
        }
        self.get_status_as_result("FlacDecoderUnmovable::Init()")
    }

    unsafe extern "C" fn read_callback(_decoder: *const FLAC__StreamDecoder, buffer: *mut u8, bytes: *mut usize, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        if *bytes == 0 {
            FLAC__STREAM_DECODER_READ_STATUS_ABORT
        } else {
            let buf = slice::from_raw_parts_mut(buffer, *bytes);
            let (bytes_read, status) = (this.on_read)(buf);
            let ret = match status{
                ReadStatus::GoOn => FLAC__STREAM_DECODER_READ_STATUS_CONTINUE,
                ReadStatus::Eof => FLAC__STREAM_DECODER_READ_STATUS_END_OF_STREAM,
                ReadStatus::Abort => FLAC__STREAM_DECODER_READ_STATUS_ABORT,
            };

            *bytes = bytes_read;
            ret
        }
    }

    unsafe extern "C" fn seek_callback(_decoder: *const FLAC__StreamDecoder, absolute_byte_offset: u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_seek)(absolute_byte_offset) {
            Ok(_) => FLAC__STREAM_DECODER_SEEK_STATUS_OK,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_SEEK_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_SEEK_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn tell_callback(_decoder: *const FLAC__StreamDecoder, absolute_byte_offset: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_tell)() {
            Ok(offset) => {
                *absolute_byte_offset = offset;
                FLAC__STREAM_DECODER_TELL_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_TELL_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_TELL_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn length_callback(_decoder: *const FLAC__StreamDecoder, stream_length: *mut u64, client_data: *mut c_void) -> u32 {
        let this = &mut *(client_data as *mut Self);
        match (this.on_length)() {
            Ok(length) => {
                *stream_length = length;
                FLAC__STREAM_DECODER_LENGTH_STATUS_OK
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotSeekable => FLAC__STREAM_DECODER_LENGTH_STATUS_UNSUPPORTED,
                    _ => FLAC__STREAM_DECODER_LENGTH_STATUS_ERROR,
                }
            },
        }
    }

    unsafe extern "C" fn eof_callback(_decoder: *const FLAC__StreamDecoder, client_data: *mut c_void) -> i32 {
        let this = &mut *(client_data as *mut Self);
        if (this.on_eof)() {1} else {0}
    }

    unsafe extern "C" fn write_callback(_decoder: *const FLAC__StreamDecoder, frame: *const FLAC__Frame, buffer: *const *const i32, client_data: *mut c_void) -> u32 {
        // Scales signed PCM samples to full i32 dynamic range.
        // - `bits`: Valid bits in `sample` (1-32).
        // - Example: 8-bit samples [-128, 127] → [i32::MIN, i32::MAX]
        fn scale_to_i32(sample: i32, bits: u32) -> i32 {
            assert!(bits <= 32);
            if bits == 32 {
                sample
            } else {
                fn scale_to_unsigned(sample: i32, bits: u32) -> u32 {
                    let mask = (1u32 << bits) - 1;
                    let mid_number = 1u32 << (bits - 1);
                    ((sample as u32).wrapping_add(mid_number) & mask) << (32 - bits)
                }
                let mut lower_fill = scale_to_unsigned(sample, bits);
                let mut result = (sample as u32) << (32 - bits);
                while lower_fill > 0 {
                    lower_fill >>= bits;
                    result |= lower_fill;
                }
                result as i32
            }
        }

        let this = &mut *(client_data as *mut Self);
        let frame = *frame;
        let samples = frame.header.blocksize;
        let channels = frame.header.channels;
        let sample_rate = frame.header.sample_rate;
        let bits_per_sample = frame.header.bits_per_sample;

        let mut samples_info = SamplesInfo {
            samples,
            channels,
            sample_rate,
            bits_per_sample,
            audio_form: this.desired_audio_form,
        };

        let mut ret: Vec<Vec<i32>>;
        match this.desired_audio_form {
            AudioForm::FrameArray => {
                // Each `frame` contains one sample for each channel
                ret = vec![Vec::<i32>::new(); samples as usize];
                for s in 0..samples {
                    for c in 0..channels {
                        let channel = *buffer.add(c as usize);
                        ret[s as usize].push(*channel.add(s as usize));
                    }
                }
            },
            AudioForm::ChannelArray => {
                // Each `channel` contains all samples for the channel
                ret = vec![Vec::<i32>::new(); channels as usize];
                for c in 0..channels {
                    ret[c as usize] = slice::from_raw_parts(*buffer.add(c as usize), samples as usize).to_vec();
                }
            }
        }

        // Whatever it was, now it's just a two-dimensional array
        if this.scale_to_i32_range {
            for x in ret.iter_mut() {
                for y in x.iter_mut() {
                    *y = scale_to_i32(*y, bits_per_sample);
                }
            }
            samples_info.bits_per_sample = 32;
        }

        match (this.on_write)(&ret, &samples_info) {
            Ok(_) => FLAC__STREAM_DECODER_WRITE_STATUS_CONTINUE,
            Err(e) => {
                eprintln!("On `write_callback()`: {:?}", e);
                FLAC__STREAM_DECODER_WRITE_STATUS_ABORT
            },
        }
    }

    unsafe extern "C" fn metadata_callback(_decoder: *const FLAC__StreamDecoder, metadata: *const FLAC__StreamMetadata, client_data: *mut c_void) {
        let _this = &mut *(client_data as *mut Self);
        let meta = *metadata;
        println!("{:?}", WrappedStreamMetadata(meta))
    }

    unsafe extern "C" fn error_callback(_decoder: *const FLAC__StreamDecoder, status: u32, client_data: *mut c_void) {
        let this = &mut *(client_data as *mut Self);
        (this.on_error)(match status {
            FLAC__STREAM_DECODER_ERROR_STATUS_LOST_SYNC => DecoderError::LostSync,
            FLAC__STREAM_DECODER_ERROR_STATUS_BAD_HEADER => DecoderError::BadHeader,
            FLAC__STREAM_DECODER_ERROR_STATUS_FRAME_CRC_MISMATCH => DecoderError::FrameCrcMismatch,
            FLAC__STREAM_DECODER_ERROR_STATUS_UNPARSEABLE_STREAM => DecoderError::UnparseableStream,
            FLAC__STREAM_DECODER_ERROR_STATUS_BAD_METADATA => DecoderError::BadMetadata,
            o => panic!("Unknown value of `FLAC__StreamDecoderErrorStatus`: {o}"),
        });
    }

    pub fn decode(&mut self) -> Result<bool, FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_process_single(self.decoder) != 0} {
            Ok(true)
        } else {
            match self.get_status_as_result("FLAC__stream_decoder_process_single") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }

    pub fn decode_all(&mut self) -> Result<bool, FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_process_until_end_of_stream(self.decoder) != 0} {
            Ok(true)
        } else {
            match self.get_status_as_result("FLAC__stream_decoder_process_until_end_of_stream") {
                Ok(_) => Ok(false),
                Err(e) => Err(e),
            }
        }
    }

    pub fn finish(&mut self) -> Result<(), FlacDecoderError> {
        if unsafe {FLAC__stream_decoder_finish(self.decoder) != 0} {
            Ok(())
        } else {
            self.get_status_as_result("FLAC__stream_decoder_finish")
        }
    }


    fn on_drop(&mut self) {
        unsafe {
            if let Err(e) =  self.finish() {
                eprintln!("On FlacDecoderUnmovable::finish(): {:?}", e);
            }

            // Must delete `self.decoder` even `self.finish()` fails.
            FLAC__stream_decoder_delete(self.decoder);
        };
    }

    pub fn finalize(self) {}
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Debug for FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacDecoderUnmovable<{}, {}, {}, {}, {}, {}, {}>",
                type_name::<Rd>(),
                type_name::<Sk>(),
                type_name::<Tl>(),
                type_name::<Ln>(),
                type_name::<Ef>(),
                type_name::<Wr>(),
                type_name::<Er>()))
            .field("decoder", &self.decoder)
            .field("on_read", &format_args!("0x{:x}", &self.on_read as *const Rd as usize))
            .field("on_seek", &format_args!("0x{:x}", &self.on_seek as *const Sk as usize))
            .field("on_tell", &format_args!("0x{:x}", &self.on_tell as *const Tl as usize))
            .field("on_length", &format_args!("0x{:x}", &self.on_length as *const Ln as usize))
            .field("on_eof", &format_args!("0x{:x}", &self.on_eof as *const Ef as usize))
            .field("on_write", &format_args!("0x{:x}", &self.on_write as *const Wr as usize))
            .field("on_error", &format_args!("0x{:x}", &self.on_error as *const Er as usize))
            .field("md5_checking", &self.md5_checking)
            .finish()
    }
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Drop for FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn drop(&mut self) {
        self.on_drop();
    }
}

pub struct FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    decoder: Box<FlacDecoderUnmovable<Rd, Sk, Tl, Ln, Ef, Wr, Er>>,
}

impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    pub fn new(
        on_read: Rd,
        on_seek: Sk,
        on_tell: Tl,
        on_length: Ln,
        on_eof: Ef,
        on_write: Wr,
        on_error: Er,
        md5_checking: bool,
        scale_to_i32_range: bool,
        desired_audio_form: AudioForm,
    ) -> Result<Self, FlacDecoderError> {
        let mut ret = Self {
            decoder: Box::new(FlacDecoderUnmovable::<Rd, Sk, Tl, Ln, Ef, Wr, Er>::new(
                on_read,
                on_seek,
                on_tell,
                on_length,
                on_eof,
                on_write,
                on_error,
                md5_checking,
                scale_to_i32_range,
                desired_audio_form,
            )?),
        };
        ret.decoder.init()?;
        Ok(ret)
    }

    pub fn decode(&mut self) -> Result<bool, FlacDecoderError> {
        self.decoder.decode()
    }

    pub fn decode_all(&mut self) -> Result<bool, FlacDecoderError> {
        self.decoder.decode_all()
    }

    pub fn finish(&mut self) -> Result<(), FlacDecoderError> {
        self.decoder.finish()
    }

    pub fn finalize(self) {}
}


impl<Rd, Sk, Tl, Ln, Ef, Wr, Er> Debug for FlacDecoder<Rd, Sk, Tl, Ln, Ef, Wr, Er>
where
    Rd: FnMut(&mut [u8]) -> (usize, ReadStatus),
    Sk: FnMut(u64) -> Result<(), io::Error>,
    Tl: FnMut() -> Result<u64, io::Error>,
    Ln: FnMut() -> Result<u64, io::Error>,
    Ef: FnMut() -> bool,
    Wr: FnMut(&[Vec<i32>], &SamplesInfo) -> Result<(), io::Error>,
    Er: FnMut(DecoderError) {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct(&format!("FlacDecoder<{}, {}, {}, {}, {}, {}, {}>",
                type_name::<Rd>(),
                type_name::<Sk>(),
                type_name::<Tl>(),
                type_name::<Ln>(),
                type_name::<Ef>(),
                type_name::<Wr>(),
                type_name::<Er>()))
            .field("decoder", &self.decoder)
            .finish()
    }
}


#[derive(Clone, Copy)]
struct WrappedStreamInfo(FLAC__StreamMetadata_StreamInfo);

impl Debug for WrappedStreamInfo {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_StreamInfo")
            .field("min_blocksize", &self.0.min_blocksize)
            .field("max_blocksize", &self.0.max_blocksize)
            .field("min_framesize", &self.0.min_framesize)
            .field("max_framesize", &self.0.max_framesize)
            .field("sample_rate", &self.0.sample_rate)
            .field("channels", &self.0.channels)
            .field("bits_per_sample", &self.0.bits_per_sample)
            .field("total_samples", &self.0.total_samples)
            .field("md5sum", &format_args!("{}", self.0.md5sum.iter().map(|x|{format!("{:02x}", x)}).collect::<Vec<String>>().join("")))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedPadding(FLAC__StreamMetadata_Padding);
impl Debug for WrappedPadding {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_Padding")
            .field("dummy", &self.0.dummy)
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedApplication(FLAC__StreamMetadata_Application, u32);
impl WrappedApplication {
    pub fn get_header(&self) -> String {
        String::from_utf8_lossy(&self.0.id).to_string()
    }
    pub fn get_data(&self) -> Vec<u8> {
        let n = self.1 - 4;
        unsafe {slice::from_raw_parts(self.0.data, n as usize)}.to_vec()
    }
}

impl Debug for WrappedApplication {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_Application")
            .field("id", &self.get_header())
            .field("data", &String::from_utf8_lossy(&self.get_data()))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedSeekPoint(FLAC__StreamMetadata_SeekPoint);
impl Debug for WrappedSeekPoint {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_SeekPoint")
            .field("sample_number", &self.0.sample_number)
            .field("stream_offset", &self.0.stream_offset)
            .field("frame_samples", &self.0.frame_samples)
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedSeekTable(FLAC__StreamMetadata_SeekTable);
impl Debug for WrappedSeekTable {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let points: Vec<WrappedSeekPoint> = unsafe {slice::from_raw_parts(self.0.points, self.0.num_points as usize).iter().map(|p|{WrappedSeekPoint(*p)}).collect()};
        fmt.debug_struct("FLAC__StreamMetadata_SeekTable")
            .field("num_points", &self.0.num_points)
            .field("points", &format_args!("{:?}", points))
            .finish()
    }
}

fn entry_to_string(entry: &FLAC__StreamMetadata_VorbisComment_Entry) -> String {
    unsafe {String::from_utf8_lossy(slice::from_raw_parts(entry.entry, entry.length as usize)).to_string()}
}

#[derive(Clone, Copy)]
struct WrappedVorbisComment(FLAC__StreamMetadata_VorbisComment);
impl Debug for WrappedVorbisComment {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_VorbisComment")
            .field("vendor_string", &entry_to_string(&self.0.vendor_string))
            .field("num_comments", &self.0.num_comments)
            .field("comments", &format_args!("[{}]", (0..self.0.num_comments).map(|i|unsafe{entry_to_string(&*self.0.comments.add(i as usize))}).collect::<Vec<String>>().join(", ")))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedCueSheet(FLAC__StreamMetadata_CueSheet);
impl Debug for WrappedCueSheet {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_CueSheet")
            .field("media_catalog_number", &String::from_utf8_lossy(&self.0.media_catalog_number.into_iter().map(|c|{c as u8}).collect::<Vec<u8>>()))
            .field("lead_in", &self.0.lead_in)
            .field("is_cd", &self.0.is_cd)
            .field("num_tracks", &self.0.num_tracks)
            .field("tracks", &format_args!("[{}]", (0..self.0.num_tracks).map(|i|format!("{:?}", unsafe{*self.0.tracks.add(i as usize)})).collect::<Vec<String>>().join(", ")))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedCueSheetTrack(FLAC__StreamMetadata_CueSheet_Track);
impl Debug for WrappedCueSheetTrack {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_CueSheet_Track")
            .field("offset", &self.0.offset)
            .field("number", &self.0.number)
            .field("isrc", &self.0.isrc)
            .field("type", &self.0.type_())
            .field("pre_emphasis", &self.0.pre_emphasis())
            .field("num_indices", &self.0.num_indices)
            .field("indices", &format_args!("[{}]", (0..self.0.num_indices).map(|i|format!("{:?}", unsafe{*self.0.indices.add(i as usize)})).collect::<Vec<String>>().join(", ")))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedCueSheetIndex(FLAC__StreamMetadata_CueSheet_Index);
impl Debug for WrappedCueSheetIndex {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_CueSheet_Index")
            .field("offset", &self.0.offset)
            .field("number", &self.0.number)
            .finish()
    }
}

fn picture_type_to_str(pictype: u32) -> &'static str {
    match pictype {
        FLAC__STREAM_METADATA_PICTURE_TYPE_FILE_ICON_STANDARD => "32x32 pixels 'file icon' (PNG only)",
        FLAC__STREAM_METADATA_PICTURE_TYPE_FILE_ICON => "Other file icon",
        FLAC__STREAM_METADATA_PICTURE_TYPE_FRONT_COVER => "Cover (front)",
        FLAC__STREAM_METADATA_PICTURE_TYPE_BACK_COVER => "Cover (back)",
        FLAC__STREAM_METADATA_PICTURE_TYPE_LEAFLET_PAGE => "Leaflet page",
        FLAC__STREAM_METADATA_PICTURE_TYPE_MEDIA => "Media (e.g. label side of CD)",
        FLAC__STREAM_METADATA_PICTURE_TYPE_LEAD_ARTIST => "Lead artist/lead performer/soloist",
        FLAC__STREAM_METADATA_PICTURE_TYPE_ARTIST => "Artist/performer",
        FLAC__STREAM_METADATA_PICTURE_TYPE_CONDUCTOR => "Conductor",
        FLAC__STREAM_METADATA_PICTURE_TYPE_BAND => "Band/Orchestra",
        FLAC__STREAM_METADATA_PICTURE_TYPE_COMPOSER => "Composer",
        FLAC__STREAM_METADATA_PICTURE_TYPE_LYRICIST => "Lyricist/text writer",
        FLAC__STREAM_METADATA_PICTURE_TYPE_RECORDING_LOCATION => "Recording Location",
        FLAC__STREAM_METADATA_PICTURE_TYPE_DURING_RECORDING => "During recording",
        FLAC__STREAM_METADATA_PICTURE_TYPE_DURING_PERFORMANCE => "During performance",
        FLAC__STREAM_METADATA_PICTURE_TYPE_VIDEO_SCREEN_CAPTURE => "Movie/video screen capture",
        FLAC__STREAM_METADATA_PICTURE_TYPE_FISH => "A bright coloured fish",
        FLAC__STREAM_METADATA_PICTURE_TYPE_ILLUSTRATION => "Illustration",
        FLAC__STREAM_METADATA_PICTURE_TYPE_BAND_LOGOTYPE => "Band/artist logotype",
        FLAC__STREAM_METADATA_PICTURE_TYPE_PUBLISHER_LOGOTYPE => "Publisher/Studio logotype",
        _ => "Other",
    }
}

#[derive(Clone, Copy)]
struct WrappedPicture(FLAC__StreamMetadata_Picture);
impl Debug for WrappedPicture {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_Picture")
            .field("type_", &picture_type_to_str(self.0.type_))
            .field("mime_type", &unsafe{CStr::from_ptr(self.0.mime_type).to_str()})
            .field("description", &unsafe{CStr::from_ptr(self.0.description as *const i8).to_str()})
            .field("width", &self.0.width)
            .field("height", &self.0.height)
            .field("depth", &self.0.depth)
            .field("colors", &self.0.colors)
            .field("data_length", &self.0.data_length)
            .field("data", &format_args!("[u8; {}]", self.0.data_length))
            .finish()
    }
}

#[derive(Clone, Copy)]
struct WrappedUnknown(FLAC__StreamMetadata_Unknown);
impl Debug for WrappedUnknown {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata_Unknown")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
struct WrappedStreamMetadata(FLAC__StreamMetadata);

impl Debug for WrappedStreamMetadata {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.debug_struct("FLAC__StreamMetadata")
            .field("type_", &self.0.type_)
            .field("is_last", &self.0.is_last)
            .field("length", &self.0.length)
            .field("data", &match self.0.type_ {
                FLAC__METADATA_TYPE_STREAMINFO => format!("{:?}", unsafe{WrappedStreamInfo(self.0.data.stream_info)}),
                FLAC__METADATA_TYPE_PADDING => format!("{:?}", unsafe{WrappedPadding(self.0.data.padding)}),
                FLAC__METADATA_TYPE_APPLICATION => format!("{:?}", unsafe{WrappedApplication(self.0.data.application, self.0.length)}),
                FLAC__METADATA_TYPE_SEEKTABLE => format!("{:?}", unsafe{WrappedSeekTable(self.0.data.seek_table)}),
                FLAC__METADATA_TYPE_VORBIS_COMMENT => format!("{:?}", unsafe{WrappedVorbisComment(self.0.data.vorbis_comment)}),
                FLAC__METADATA_TYPE_CUESHEET => format!("{:?}", unsafe{WrappedCueSheet(self.0.data.cue_sheet)}),
                FLAC__METADATA_TYPE_PICTURE => format!("{:?}", unsafe{WrappedPicture(self.0.data.picture)}),
                FLAC__METADATA_TYPE_UNDEFINED => format!("{:?}", unsafe{WrappedUnknown(self.0.data.unknown)}),
                o => format!("Unknown metadata type {o}"),
            })
            .finish()
    }
}
