# Benchmark Plan

## Overview

Performance measurement strategy for quantifying resampler throughput, latency, and resource usage.

## Benchmark Infrastructure

- **Framework**: criterion.rs
- **Environment**: Controlled CPU, no background load
- **Measurement**: Multiple iterations, outlier removal, confidence intervals

## Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| Throughput | Samples processed per second | > 10M samples/sec |
| Latency | Processing time per sample | < 100 ns/sample |
| Memory | Peak allocation per resampler | < 1 MB |
| Scaling | Performance vs filter length | Linear |

## Test Datasets

### Synthetic Signals

For consistent, reproducible measurements:

**Impulse** (1000 samples):
- Single impulse at center
- Tests filter application

**Sine wave** (48000 samples, 1 second at 48kHz):
- 1 kHz pure tone
- Realistic audio length

**Noise** (48000 samples):
- White noise
- Tests worst-case interpolation

**Silence** (48000 samples):
- All zeros
- Tests overhead without computation

### Real-World Audio (Future)

- CD-quality WAV (44.1kHz, stereo, 3 minutes)
- High-resolution audio (96kHz, 1 minute)
- Speech sample (16kHz, 30 seconds)

## Benchmarks

### 1. Throughput Benchmarks

```rust
fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("resample_44100_to_48000", |b| {
        let resampler = Resampler::new(44100.0, 48000.0);
        let input = generate_sine(1000.0, 44100.0, 1.0);
        b.iter(|| resampler.resample(&input));
    });
}
```

**Benchmark cases**:
- `resample_44100_to_48000` - Common non-integer ratio
- `resample_48000_to_44100` - Inverse ratio
- `resample_48000_to_96000` - 2x upsample
- `resample_96000_to_48000` - 2x downsample
- `resample_passthrough` - 1:1 ratio

### 2. Scaling Benchmarks

Test performance vs algorithmic parameters:

```rust
fn bench_filter_length(c: &mut Criterion) {
    let input = generate_sine(1000.0, 48000.0, 1.0);
    let mut group = BenchmarkGroup::new("filter_length");
    for taps in [16, 32, 64, 128, 256] {
        group.bench_with_input(taps, |b, &t| {
            let resampler = Resampler::with_taps(48000.0, 96000.0, taps);
            b.iter(|| resampler.resample(&input));
        });
    }
    group.finish();
}
```

**Vary**:
- Filter length (taps)
- Number of phases
- Input buffer size

### 3. Memory Benchmarks

Track allocations in hot path:

```rust
#[global_allocator]
static ALLOC: tracy_client::AllocProfiler = tracy_client::AllocProfiler;
```

**Measure**:
- Allocations per `resample()` call
- Peak memory for FilterBank
- Stack vs heap usage

### 4. Comparison (Future)

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

### Minimum Targets
- 1M samples/second sustained
- < 1μs setup time per conversion
- Zero allocations in `resample()` path

### Target Targets
- 10M samples/second sustained
- < 100ns per output sample
- < 100KB memory per resampler

### Stretch Goals
- 50M samples/second with SIMD
- < 50ns per output sample
- < 10KB memory per resampler

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

# Run specific benchmark
cargo bench --bench resampler_bench resample_44100_to_48000

# Generate flamegraph
cargo bench --bench resampler_bench -- --profile-time=5

# Compare with baseline
cargo bench -- --save-baseline current
```

## Profiling Tools

- **criterion**: Throughput and latency measurement
- **perf**: Linux CPU profiling (`perf record -g`)
- **flamegraph**: Visual call graphs
- **tracy**: Memory allocation tracking
- **valgrind//cachegrind**: Cache performance analysis
