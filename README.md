# br41ndmg

A high-performance audio resampling library written in Rust, implementing polyphase sinc resampling for high-fidelity sample-rate conversion.

## Overview

br41ndmg provides production-quality sample-rate conversion for audio applications. It uses windowed sinc interpolation with polyphase filter decomposition for efficient arbitrary-ratio resampling while maintaining excellent frequency response characteristics.

### Key Features

- **High-quality resampling**: Windowed sinc-based FIR filtering
- **Arbitrary ratios**: Convert between any sample rates (e.g., 44.1kHz → 48kHz)
- **Configurable filters**: Multiple window functions (Hann, Hamming, Blackman, Kaiser)
- **Benchmarked**: Comprehensive performance testing with criterion
- **Testable**: DSP-validated with impulse, sine, and sweep tests

## Quick Start

### Installation

```toml
[dependencies]
br41ndmg = "0.1"
```

### Basic Usage

```rust
use br41ndmg::Resampler;

let input_rate = 44100.0;
let output_rate = 48000.0;
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

### Real-time Streaming

```rust
use br41ndmg::StreamingResampler;

let mut stream = StreamingResampler::new(44_100.0, 48_000.0, 2)?;
let input_frames = 256;
let input_chunk = vec![0.0f32; input_frames * 2];
let mut output = vec![0.0; stream.output_samples_for(input_frames)];

let written_frames = stream.process_into(&input_chunk, &mut output)?;
let ready = &output[..written_frames * stream.channels()];
```

### Examples

```bash
cargo run --example resample_file -- input.wav output.wav 48000
cargo run --example tone_resample -- tone_resampled.wav
```

## Roadmap

- [x] Core math primitives (sinc, windows, FIR kernels)
- [x] Naive resampler prototype
- [x] Polyphase sinc implementation
- [x] File I/O integration (WAV)
- [x] Real-time streaming support
- [ ] SIMD optimization
- [ ] f32 support

## Documentation

- [docs/REQUIREMENTS.md](docs/REQUIREMENTS.md) - Project requirements and scope
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System design and data flow
- [docs/DSP_NOTES.md](docs/DSP_NOTES.md) - DSP theory and algorithms
- [docs/TEST_PLAN.md](docs/TEST_PLAN.md) - Testing strategy
- [docs/BENCHMARK_PLAN.md](docs/BENCHMARK_PLAN.md) - Performance benchmarks

## Supported WAV Formats

- Input: 8/16/24/32-bit PCM WAV and 32-bit float WAV
- Output: 32-bit float WAV

## Quality Targets

| Metric | Target |
|--------|--------|
| Aliasing suppression | < -100 dB |
| Passband ripple | < 0.1 dB |
| Stopband attenuation | > 100 dB |

## License

MIT
