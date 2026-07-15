# Changelog

## [Unreleased]

## [0.2.0] - 2026-07-16

### Added
- Interactive terminal file browser for the CLI: navigate directories, mark
  `.wav`/`.flac` files, set a target rate and output directory, and watch a
  per-file progress bar. Large files are resampled on a background thread so the
  UI stays responsive. Run `br41ndmg` with no arguments, a directory, or
  `-i`/`--interactive` to open it.

### Changed
- The command-line tool moved into its own package, `br41ndmg-cli` (the binary
  is still named `br41ndmg`). Install it with
  `cargo install br41ndmg-cli`. The library package no longer ships a binary.
- The three-argument form `br41ndmg <input> <output> <rate>` is unchanged and
  remains the non-interactive path.

## [0.1.1] - 2026-06-22

### Added
- `br41ndmg` command-line binary, installable via `cargo install br41ndmg`.
  Resamples a single file or a whole folder of `.wav`/`.flac` inputs to a
  target sample rate (promoted from the former `resample_file` example).

### Changed
- Republished so the crates.io `repository` link resolves to
  `ultraelectronica/br41ndmg` (the manifest was already correct in 0.1.0,
  but the published `0.1.0` carried a stale `anomalyco` URL).

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
