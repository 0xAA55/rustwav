## TODO:
* 所有编解码器（包括 MP3、PCM ）要有 `new_fmt_chunk()`
* 所有编解码器的 `new_fmt_chunk()` 被调用。
	* 尤其是现有的 MP3 的编解码器。
* 实现 ADPCM-MS 的解码器。
* 解码器迭代器实现 `nth()`
* 解码器实现 `IntoIterator`
* 编码器实现 `std::iter::FromIterator`
* 宏 `define_copiable_buffer!()` 考虑其调试输出时不输出缓冲区大小外的数据。
* 实现 FLAC
* 实现 OGG
* 实现 AAC

