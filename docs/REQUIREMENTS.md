# Requirements

## Objective

Build a production-quality polyphase sinc resampler in Rust that converts PCM audio between arbitrary sample rates with high fidelity, predictable latency, and measurable performance.

## Supported Formats

- **Audio types**: Mono and stereo PCM
- **Sample formats**: `f64` (primary), `f32` (future)
- **Input/Output**: Offline file processing first, real-time streaming later
- **Resampling ratios**: Arbitrary rational and non-rational sample-rate ratios

## Quality Targets

| Metric | Target |
|--------|--------|
| Aliasing suppression | Below -100 dB |
| Passband ripple | < 0.1 dB |
| Stopband attenuation | > 100 dB |
| Latency | Predictable, bounded by filter length |
| Output stability | Deterministic for identical inputs |

## Performance Targets

- Efficient polyphase filter implementation
- Minimal allocations in hot paths
- Throughput suitable for real-time audio processing
- Benchmark suite with measurable metrics

## Scope

### In Scope

- High-quality sinc-based polyphase resampling
- Arbitrary sample-rate conversion (e.g., 44.1kHz → 48kHz)
- Windowed FIR filter design (Hann, Hamming, Blackman, Kaiser)
- Offline batch processing
- Real-time streaming support (future)
- Comprehensive test suite (impulse, sine, sweep)
- Performance benchmarks

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

## Future Features

- SIMD optimization for performance
- `f32` support
- Real-time streaming with `cpal`
- Additional window functions
- Configurable filter parameters
