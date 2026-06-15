# Changelog

## [Unreleased]

## [0.1.0] - 2026-06-16

### Added
- Polyphase sinc resampling core with precomputed fractional phases (`Resampler`)
- Offline resampling for mono and interleaved multichannel buffers
- `StreamingResampler` for chunked, low-latency real-time processing with flush support
- Configurable polyphase filter parameters (`PolyphaseFilterParams`: phases, taps, window)
- `f64` and `f32` DSP helpers: sinc, windows (Hann, Hamming, Blackman, Kaiser), FIR kernels
- WAV I/O: 8/16/24/32-bit PCM and 32-bit float input, 32-bit float output
- FLAC input support behind the default `flac` feature (pure-Rust `claxon` decoder)
- Format-agnostic `read_audio` dispatcher (`.wav` / `.flac`)
- SSE2 stereo fast path on `x86` / `x86_64`
- Criterion benchmarks for mono and stereo 44.1 kHz → 48 kHz conversion
- DSP validation suite: impulse, sine, sweep, DC, alias-suppression, stereo separation
- Real-audio integration tests driven by `test_subjects/` FLAC fixtures

### Fixed
- `StreamingResampler` now derives output positions from the output-frame counter
  (`index / ratio`) instead of accumulating `next_position += step`. This makes
  streaming output bit-for-bit identical to the offline path on long signals,
  where accumulated floating-point drift previously could flip phase selection
  near fractional boundaries.
