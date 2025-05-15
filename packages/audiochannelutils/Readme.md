# Audio channel utilities

This crate is for converting and resampling audio wave form into variaties of formats.

## Reference

```rust
/// * Turns a stereo audio into two individual mono waveforms.
pub fn stereos_to_dual_monos<S>(stereos: &[(S, S)]) -> (Vec<S>, Vec<S>) where S: SampleType;

/// * Check every element in the data has the same length. The length will be returned.
pub fn is_same_len<S>(data: &[Vec<S>]) -> Option<(bool, usize)> where S: SampleType;

/// * Convert audio frames into stereo audio. Mono audio will be converted to stereo by duplicating samples. Only support 1 or 2 channels of audio.
pub fn frames_to_stereos<S>(channel_mask: u32, frames: &[Vec<S>]) -> Result<Vec<(S, S)>, AudioConvError> where S: SampleType;

/// * Convert audio frames into two individual mono waveforms. Only support two-channel audio.
pub fn frames_to_dual_mono<S>(channel_mask: u32, frames: &[Vec<S>]) -> Result<(Vec<S>, Vec<S>), AudioConvError> where S: SampleType;

/// * Convert audio frames into every individual mono waveform. Support any channels.
/// * The param `channels` is optional, if you provide it, the conversion will be a little faster than if you just give it a `None.`
pub fn frames_to_monos<S>(frames: &[Vec<S>]) -> Result<Vec<Vec<S>>, AudioConvError> where S: SampleType;

/// * Convert every individual mono waveform into an audio frame array. Support any channels.
pub fn monos_to_frames<S>(monos: &[Vec<S>]) -> Result<Vec<Vec<S>>, AudioConvError> where S: SampleType;

/// * Convert every individual mono waveform into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn monos_to_interleaved_samples<S>(monos: &[Vec<S>]) -> Result<Vec<S>, AudioConvError> where S: SampleType;

/// * Convert audio frames into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn frames_to_interleaved_samples<S>(frames: &[Vec<S>]) -> Result<Vec<S>, AudioConvError> where S: SampleType;

/// * Convert the interleaved samples of audio interleaved by channels into audio frames.
pub fn interleaved_samples_to_frames<S>(samples: &[S], channels: u16) -> Result<Vec<Vec<S>>, AudioConvError> where S: SampleType;

/// * Convert stereo audio into the interleaved samples of audio interleaved by channels. The WAV file stores PCM samples in this form.
pub fn stereos_to_interleaved_samples<S>(stereos: &[(S, S)]) -> Vec<S> where S: SampleType;

/// * Convert interleaved samples into individual mono waveforms by the specified channels.
pub fn interleaved_samples_to_monos<S>(samples: &[S], channels: u16) -> Result<Vec<Vec<S>>, AudioConvError> where S: SampleType;

/// * Convert two individual mono waveforms into a stereo audio form.
pub fn dual_monos_to_stereos<S>(dual_monos: &(Vec<S>, Vec<S>)) -> Result<Vec<(S, S)>, AudioConvError> where S: SampleType;

/// * Convert interleaved samples into a stereo audio form. The interleaved samples are treated as a two-channel audio.
pub fn interleaved_samples_to_stereos<S>(samples: &[S]) -> Result<Vec<(S, S)>, AudioConvError> where S: SampleType;

/// * Convert two individual mono waveforms into one mono waveform. Stereo to mono conversion.
pub fn dual_monos_to_monos<S>(dual_monos: &(Vec<S>, Vec<S>)) -> Result<Vec<S>, AudioConvError> where S: SampleType;

/// * Convert a mono waveform to two individual mono waveforms by duplication. Mono to stereo conversion.
pub fn monos_to_dual_monos<S>(monos: &[S]) -> (Vec<S>, Vec<S>) where S: SampleType;

/// * Convert stereo audio to a mono waveform. Stereo to mono conversion.
pub fn stereos_to_mono_channel<S>(stereos: &[(S, S)]) -> Vec<S> where S: SampleType;

/// * Convert mono waveform to a stereo audio form by duplication. Mono to stereo conversion.
pub fn monos_to_stereos<S>(monos: &[S]) -> Vec<(S, S)> where S: SampleType;

/// * Convert one stereo sample to another format by scaling, see `sample_conv()`.
pub fn stereo_conv<S, D>(frame: (S, S)) -> (D, D) where S: SampleType, D: SampleType;

/// * Convert samples to another format by scaling. e.g. `u8` to `i16` conversion is to scale `[0, 255]` into `[-32768, +32767]`
/// * Upscaling is lossless. Beware, the precision of `f32` is roughly the same as `i24`. Convert `i32` to `f32` is lossy.
/// * `i32` to `f64` is lossless but `f64` for audio processing consumes lots of memory.
pub fn sample_conv<S, D>(frame: &[S]) -> Cow<'_, [D]> where S: SampleType, D: SampleType;

/// * Convert multiple stereo samples to another format by scaling, see `sample_conv()`.
pub fn stereos_conv<S, D>(stereos: &[(S, S)]) -> Cow<'_, [(D, D)]> where S: SampleType, D: SampleType;

/// * Convert 2D audio e.g. Audio frames or multiple mono waveforms, to another format by scaling, see `sample_conv()`.
pub fn sample_conv_batch<S, D>(frames: &[Vec<S>]) -> Cow<'_, [Vec<D>]> where S: SampleType, D: SampleType;

/// * Use the `Resampler` to resample a mono waveform from original sample rate to a specific sample rate.
pub fn do_resample_mono<S>(
    resampler: &Resampler,
    input: &[S],
    src_sample_rate: u32,
    dst_sample_rate: u32,
) -> Vec<S>
where
	S: SampleType;

/// * Use the `Resampler` to resample a stereo audio from the original sample rate to a specific sample rate.
pub fn do_resample_stereo<S>(
    resampler: &Resampler,
    input: &[(S, S)],
    src_sample_rate: u32,
    dst_sample_rate: u32,
) -> Vec<(S, S)>
where
    S: SampleType;

/// * Use the `Resampler` to resample audio frames from the original sample rate to a specific sample rate.
pub fn do_resample_frames<S>(
    resampler: &Resampler,
    input: &[Vec<S>],
    src_sample_rate: u32,
    dst_sample_rate: u32,
) -> Vec<Vec<S>>
where
    S: SampleType;
```

## Errors

```rust
pub enum AudioConvError {
    /// * The parameters are invalid
    InvalidArguments(String),

    /// * When the input audio is an array of audio frames, each frame should have the same channels, otherwise, this error occurs.
    FrameChannelsNotSame,

    /// * When the input audio is an array of individual waveforms, each waveform should have the same length.
    ChannelsNotInSameSize,

    /// * When the input audio is the interleaved sample array, the number of samples must be divisible by the number of channels
    TruncatedSamples,
}
```
