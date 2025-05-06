## TODO:
* 将 FLAC 集成到 WaveReader 和 WaveWriter 里。
	* 注意实现 FLAC 的解码器，要做好 buffering 以便于用户使用迭代器一个个获取样本。
	* 注意看，FLAC 是不是首先调用你的 `metadata_callback()` 后，才开始读写样本。如果是的话，应当把 `metadata_callback()` 进行适当封装，在其提供 `FLAC__StreamMetadata_StreamInfo` 的时候，选取其中的有用信息传递给调用者。
* 检测 MP3 的采样率变化，应用 resampler。
* 实现 OggVorbis 编码到 WAV，以及解码。
