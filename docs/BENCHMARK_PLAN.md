# Benchmark Plan

## Overview

Performance measurement strategy for the current linear-interpolation resampler and its stereo SIMD fast path.

## Benchmark Infrastructure

- **Framework**: criterion.rs
- **Environment**: Controlled CPU, no background load
- **Measurement**: Multiple iterations, outlier removal, confidence intervals

## Current Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| Throughput | Samples processed per second | Track mono vs stereo trends |
| Latency | Processing time per sample | Track relative changes |
| Memory | Peak allocation per resampler | No extra hot-path channel copies |
| SIMD benefit | Stereo speedup vs scalar fallback | Positive on supported CPUs |

## Current Benchmark Datasets

The checked-in benchmark generates deterministic synthetic audio in memory:

- **Mono signal**: 44_100 frames of mixed sine and cosine content
- **Stereo signal**: 44_100 interleaved frames with per-channel phase offsets

### Real-World Audio (Future)

- CD-quality WAV (44.1kHz, stereo, 3 minutes)
- High-resolution audio (96kHz, 1 minute)
- Speech sample (16kHz, 30 seconds)

## Current Benchmarks

The current `benches/resampler_bench.rs` file covers:

```rust
- mono `44_100 -> 48_000` via `Resampler::resample()`
- stereo `44_100 -> 48_000` via `Resampler::resample_interleaved(..., 2)`
```

## Next Benchmarks

- 48 kHz -> 44.1 kHz downsampling
- 1:1 passthrough overhead
- Streaming chunk sizes for mono vs stereo
- Direct scalar-vs-SIMD comparison on supported x86 targets

## Memory Checks

Track allocations in hot path:

```rust
#[global_allocator]
static ALLOC: tracy_client::AllocProfiler = tracy_client::AllocProfiler;
```

**Measure**:
- Allocations per `resample()` call
- Peak memory for FilterBank
- Stack vs heap usage

## Comparison (Future)

Compare against reference implementations:

| Library | Language | Algorithm |
|---------|----------|-----------|
| libsamplerate | C | Secret Rabbit Code |
| SoX | C | Various |
| Rust sinc | Rust | Current implementation |

**Comparison metrics**:
- Throughput ratio
- Quality metrics (SNR, THD)
- Memory usage

## Performance Targets

### Near-Term Targets
- No per-channel deinterleave/reinterleave work in interleaved resampling
- Stereo SIMD path measurably faster than the scalar fallback on supported CPUs
- Stable Criterion baselines for mono and stereo 44.1 kHz -> 48 kHz cases

### Longer-Term Targets
- Polyphase benchmarks once that implementation exists
- Latency and quality measurements tied to the future filterbank

## Regression Testing

Establish baseline performance on reference hardware:

```
benchmarks/
├── baseline/
│   ├── arm64_macos.csv
│   ├── x86_64_linux.csv
│   └── x86_64_windows.csv
```

Alert on regressions:
- > 10% throughput decrease
- > 5% memory increase
- Any regression in quality metrics

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Compile benchmark without running
cargo bench --bench resampler_bench --no-run

# Run current benchmark group
cargo bench --bench resampler_bench

# Generate flamegraph
cargo bench --bench resampler_bench -- --profile-time=5

# Compare with baseline
cargo bench -- --save-baseline current
```

## Profiling Tools

- **criterion**: Throughput measurement and regression comparison
- **perf**: Linux CPU profiling (`perf record -g`)
- **flamegraph**: Visual call graphs
- **tracy**: Memory allocation tracking
- **valgrind//cachegrind**: Cache performance analysis
