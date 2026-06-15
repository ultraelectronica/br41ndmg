# br41ndmg

[![Crates.io](https://img.shields.io/crates/v/br41ndmg)](https://crates.io/crates/br41ndmg)
[![docs.rs](https://img.shields.io/docsrs/br41ndmg)](https://docs.rs/br41ndmg)
[![CI](https://github.com/ultraelectronica/br41ndmg/actions/workflows/ci.yml/badge.svg)](https://github.com/ultraelectronica/br41ndmg/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A Rust audio resampling library with a polyphase sinc engine, offline and streaming APIs, WAV/FLAC input, and a stereo SSE2 fast path.

## Overview

`br41ndmg` resamples `f32` audio with a precomputed polyphase sinc filter bank. The library exposes both offline and streaming APIs, reads WAV and FLAC files, writes WAV, and uses the same filter math in chunked and full-buffer processing — so streaming output is bit-for-bit identical to the offline path.

## Current Features

- `f32` sample buffers for offline and streaming resampling
- `f32` and `f64` DSP helper APIs for sinc, window, and FIR kernel generation
- Offline resampling for mono and interleaved multichannel buffers
- Streaming resampling for interleaved audio
- Polyphase sinc filtering with precomputed fractional phases
- Configurable polyphase filter phase count, tap count, and window
- WAV input: 8/16/24/32-bit PCM and 32-bit float
- FLAC input: 4–32-bit samples (default `flac` feature, pure-Rust decoder)
- WAV output: 32-bit float
- SSE2 stereo fast path on `x86` and `x86_64`
- Streaming output is bit-exact with the offline path regardless of input chunking
- DSP validation tests for impulse, sine, sweep, DC, and alias-suppression regressions
- Real-audio integration tests driven by `test_subjects/` FLAC fixtures
- Criterion benchmarks for mono and stereo 44.1 kHz → 48 kHz conversion

## Quick Start

### Installation

```toml
[dependencies]
br41ndmg = "0.1"
```

The default features include FLAC support. To build a leaner, WAV-only crate:

```toml
[dependencies]
br41ndmg = { version = "0.1", default-features = false }
```

### Basic Usage

```rust
use br41ndmg::Resampler;

let input_rate = 44_100.0f32;
let output_rate = 48_000.0f32;
let resampler = Resampler::new(input_rate, output_rate)?;

let input_samples: Vec<f32> = /* your audio data */;
let output_samples = resampler.resample(&input_samples)?;
```

### Custom Filter Settings

```rust
use br41ndmg::{PolyphaseFilterParams, Resampler, Window};

let params = PolyphaseFilterParams {
    phases: 512,
    taps_per_phase: 95,
    window: Window::Blackman,
};
let resampler = Resampler::with_filter_params(44_100.0, 48_000.0, params)?;
```

### File I/O (WAV and FLAC)

```rust
use br41ndmg::io::{read_audio, write_wav};

// .wav or .flac — the decoder is selected from the extension
let input = read_audio("input.flac")?;
let output = input.resample_to(48_000)?;
write_wav("output.wav", &output)?;
```

### Real-Time Streaming

```rust
use br41ndmg::StreamingResampler;

let mut stream = StreamingResampler::new(44_100.0f32, 48_000.0f32, 2)?;
let input_frames = 256;
let input_chunk = vec![0.0f32; input_frames * 2];
let mut output = vec![0.0; stream.output_samples_for(input_frames)];

let written_frames = stream.process_into(&input_chunk, &mut output)?;
let ready = &output[..written_frames * stream.channels()];
```

### Command-Line Example

Resample a file, a file into a directory (auto-named `<stem>_<rate>Hz.wav`),
or a whole folder at once:

```bash
# single file -> explicit output path
cargo run --release --example resample_file -- input.flac output.wav 48000

# single file -> directory (auto-named)
cargo run --release --example resample_file -- input.flac out_dir/ 48000

# batch: every .wav/.flac in a folder -> out_dir/
cargo run --release --example resample_file -- test_subjects/ out_dir/ 48000
```

## Roadmap

- [x] Core math primitives (sinc, windows, FIR kernels)
- [x] Polyphase sinc implementation
- [x] File I/O integration (WAV + FLAC)
- [x] Real-time streaming support
- [x] SIMD optimization for stereo interleaved paths
- [x] `f32` DSP helper support
- [x] DSP quality validation suite expansion
- [x] Configurable polyphase filter parameters
- [x] Bit-exact streaming/offline equivalence
- [ ] Expanded performance baselines and profiling data

## Documentation

- [docs/USAGE.md](docs/USAGE.md) - End-user guide and recipes
- [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) - Scope and current capabilities
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Module map and current data flow
- [docs/DSP_NOTES.md](docs/DSP_NOTES.md) - Sinc and polyphase theory notes
- [docs/REALTIME.md](docs/REALTIME.md) - Real-time / callback guidance
- [docs/TEST_PLAN.md](docs/TEST_PLAN.md) - Testing strategy
- [docs/BENCHMARK_PLAN.md](docs/BENCHMARK_PLAN.md) - Benchmark coverage and next steps
- [docs/PERFORMANCE.md](docs/PERFORMANCE.md) - Current performance notes

API documentation is generated with `cargo doc --open`.

## Long-Term Targets

These are project goals, not hard guarantees from the current default filter settings.

| Metric | Target |
|--------|--------|
| Aliasing suppression | < -100 dB |
| Passband ripple | < 0.1 dB |
| Stopband attenuation | > 100 dB |

## License

MIT
