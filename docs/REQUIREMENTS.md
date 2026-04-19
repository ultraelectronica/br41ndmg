# Requirements

## Objective

Build and validate a polyphase sinc resampler in Rust with predictable behavior and measurable performance.

## Current Support

- **Audio layout**: Mono and interleaved multichannel buffers, with stereo as the main optimized case
- **Sample buffers**: `f32`
- **DSP helpers**: `f64` and `f32` sinc/window/FIR generation
- **Resampling core**: Polyphase sinc interpolation with precomputed phases
- **Input/Output**: Offline file processing and real-time streaming
- **WAV I/O**: 8/16/24/32-bit PCM input, 32-bit float input, 32-bit float output
- **Resampling ratios**: Arbitrary rational and non-rational sample-rate ratios

## Quality Targets

| Metric | Target |
|--------|--------|
| Aliasing suppression | Below -100 dB |
| Passband ripple | < 0.1 dB |
| Stopband attenuation | > 100 dB |
| Latency | Predictable, bounded by filter length |
| Output stability | Deterministic for identical inputs |

## Current Performance Priorities

- Minimal allocations in hot paths
- Stereo fast path for interleaved buffers
- Throughput suitable for real-time audio processing
- Benchmark suite with measurable metrics

## Long-Term Performance Targets

- Efficient polyphase filter implementation
- Minimal allocations in hot paths
- Throughput suitable for real-time audio processing
- Benchmark suite with measurable metrics

## Scope

### In Scope

- Polyphase sinc resampling with stable offline and streaming APIs
- Arbitrary sample-rate conversion (e.g., 44.1kHz → 48kHz)
- Windowed FIR filter design helpers (Hann, Hamming, Blackman, Kaiser)
- Offline batch processing
- Real-time streaming support
- SIMD acceleration where the memory layout makes it safe and simple
- Performance benchmarks
- DSP validation tests for impulse, sine, sweep, DC, and alias suppression regressions

### Out of Scope (Initial Release)

- Pitch shifting
- Time stretching
- AI upscaling
- Compressed audio codecs (MP3, FLAC, etc.)
- Plugin hosting (VST, AU)
- GUI applications

## User Stories

1. **Audio Conversion**: As a user, I can convert a WAV file from 44.1 kHz to 48 kHz with preserved audio quality.
2. **Library Usage**: As a developer, I can use the resampler as a Rust library in my audio applications.
3. **Benchmarking**: As a performance tester, I can measure throughput, latency, and memory usage.
4. **Quality Validation**: As an audio engineer, I can inspect frequency response and verify aliasing suppression.
5. **Real-time Processing**: As an audio developer, I can integrate the resampler into a real-time audio pipeline.

## Testing Goals

- **Correctness**: Output length validation, boundary behavior, deterministic output
- **DSP Validation**: Impulse response, sine wave preservation, frequency sweep analysis
- **Regression**: Known input/output pairs for comparison

## Next Features

- Real-time streaming with `cpal`
- Additional window functions
- Expanded spectral-analysis regression tests
