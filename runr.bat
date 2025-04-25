@cd /d %~dp0
cargo run --release flac %1 output.wav output2.wav
@pause