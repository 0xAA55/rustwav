## TODO:
* 将 FLAC 集成到 WaveReader 和 WaveWriter 里。
	* 注意实现 FLAC 的解码器，要做好 buffering 以便于用户使用迭代器一个个获取样本。
* 检测 MP3 的采样率变化，应用 resampler。
* 实现 OGG Vorbis 编码到 WAV，以及解码。
