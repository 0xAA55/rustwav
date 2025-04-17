@cd /d %~dp0
cargo run --release pcm-alaw %1 output.wav output2.wav
@pause