# br41ndmg

A Rust audio resampling library with offline WAV I/O, streaming support, and a stereo SIMD fast path for the current linear-interpolation engine.

## Overview

`br41ndmg` currently resamples `f32` audio with linear interpolation. The library exposes both offline and streaming APIs, reads and writes WAV files, and keeps the hot stereo interleaved path allocation-free aside from the returned output buffer.

The longer-term goal is still a polyphase sinc resampler, but that filterbank is not implemented yet.

## Current Features

- `f32` sample buffers for offline and streaming resampling
- `f32` and `f64` DSP helper APIs for sinc, window, and FIR kernel generation
- Offline resampling for mono and interleaved multichannel buffers
- Streaming resampling for interleaved audio
- WAV input: 8/16/24/32-bit PCM and 32-bit float
- WAV output: 32-bit float
- SSE2 stereo fast path on `x86` and `x86_64`
- Criterion benchmarks for mono and stereo 44.1 kHz -> 48 kHz conversion

## Quick Start

### Installation

```toml
[dependencies]
br41ndmg = "0.1"
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

### WAV File I/O

```rust
use br41ndmg::io::{read_wav, write_wav};

let input = read_wav("input.wav")?;
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

## Roadmap

- [x] Core math primitives (sinc, windows, FIR kernels)
- [x] Naive linear resampler prototype
- [x] File I/O integration (WAV)
- [x] Real-time streaming support
- [x] SIMD optimization for stereo interleaved paths
- [x] `f32` DSP helper support
- [ ] Polyphase sinc implementation
- [ ] DSP quality validation suite expansion

## Documentation

- [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) - Scope and current capabilities
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - Module map and current data flow
- [docs/DSP_NOTES.md](docs/DSP_NOTES.md) - Sinc and polyphase theory notes
- [docs/TEST_PLAN.md](docs/TEST_PLAN.md) - Testing strategy
- [docs/BENCHMARK_PLAN.md](docs/BENCHMARK_PLAN.md) - Benchmark coverage and next steps
- [docs/PERFORMANCE.md](docs/PERFORMANCE.md) - Current performance notes

## Long-Term Targets

These are project goals for the future polyphase implementation, not current guarantees.

| Metric | Target |
|--------|--------|
| Aliasing suppression | < -100 dB |
| Passband ripple | < 0.1 dB |
| Stopband attenuation | > 100 dB |

## License

MIT
