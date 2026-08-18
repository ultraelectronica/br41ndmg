# Performance

## Profiling Results

Baseline snapshot: [benchmarks/baseline/x86_64_linux.csv](../benchmarks/baseline/x86_64_linux.csv),
generated with [benches/baseline_csv.py](../benches/baseline_csv.py) from a full
`cargo bench --bench resampler_bench` run.

- **CPU**: AMD Ryzen 7 7735HS with Radeon Graphics (8C/16T)
- **rustc**: 1.95.0 (59807616e 2026-04-14)
- **Date**: 2026-08-18
- **Command**: `cargo bench --bench resampler_bench`

### Offline resampling (44,100 frames, default filter 256 phases / 63 taps)

| Benchmark | Median | Throughput |
|-----------|--------|------------|
| mono 44.1k -> 48k | 2.84 ms | 15.5 Melem/s |
| stereo 44.1k -> 48k (SSE2) | 3.00 ms | 29.4 Melem/s |
| mono 48k -> 44.1k | 2.35 ms | 18.8 Melem/s |
| stereo 48k -> 44.1k (SSE2) | 2.48 ms | 35.6 Melem/s |
| stereo 44.1k -> 48k, 512 phases / 95 taps | 4.05 ms | 21.8 Melem/s |

A second of stereo 44.1 kHz audio resamples in ~3.0 ms (~330x faster than
real time); the heavier 512/95 filter still runs ~245x faster than real time.

### Streaming (stereo 44.1k -> 48k, 512-frame chunks)

| Benchmark | Median | Throughput |
|-----------|--------|------------|
| chunks of 512 frames | 9.54 ms | 9.2 Melem/s |

Chunked streaming costs ~3x the offline stereo path (history buffering,
per-chunk state, and scalar-friendly loop shape) but still runs ~100x faster
than real time. This case shows the most run-to-run jitter of the suite.

### Filter setup (one-time cost per resampler)

| Filter | Median |
|--------|--------|
| default 256 phases / 63 taps | 437 µs |
| 512 phases / 95 taps | 1.32 ms |

### Regenerating the baseline

```bash
cargo bench --bench resampler_bench
python3 benches/baseline_csv.py
```

The script overwrites `benchmarks/baseline/x86_64_linux.csv`; keep one CSV per
reference machine. Compare a change against the checked-in baseline with
`cargo bench -- --load-baseline` style Criterion workflows or by diffing the
CSV.

### Profiling tooling status

`perf` was not available on the baseline machine; timing data above is pure
Criterion measurement. Hot-path attribution still relies on the structure of
`src/resampler.rs` (phase lookup + FIR accumulation). Run `perf record -g` or
`cargo bench -- --profile-time=5` on a machine with the tools installed to
extend this with call-graph data.

## SIMD

- Target: `Resampler::resample_interleaved(..., 2)` and stereo `StreamingResampler::process_into()`
- ISA: SSE2 on `x86` and `x86_64`
- Fallback: scalar FIR accumulation on unsupported targets and for non-stereo channel counts
- Main win: accumulate both stereo channels together while the scalar path handles arbitrary channel counts
- Measured effect: stereo SSE2 converts 88.2k samples at 29.4 Melem/s vs 15.5 Melem/s for mono scalar — about 1.9x the per-sample rate. A forced-scalar comparison build is still pending; see BENCHMARK_PLAN.md

## Benchmarks

- `cargo bench --bench resampler_bench`
- Current Criterion cases:
  - `resample/mono_44100_to_48000`
  - `resample/stereo_44100_to_48000`
  - `resample/mono_48000_to_44100`
  - `resample/stereo_48000_to_44100`
  - `resample/stereo_44100_to_48000_phases_512_taps_95`
  - `streaming/stereo_44100_to_48000_chunks_512`
  - `filter_setup/default_phases_256_taps_63`
  - `filter_setup/phases_512_taps_95`
