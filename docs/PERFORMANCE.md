# Performance

## Profiling Results

No benchmark result snapshots are checked into the repository yet.

The current performance work focuses on the polyphase sinc FIR loop, avoiding interleaved-buffer copies, and keeping a stereo SIMD fast path for the most common interleaved case.

## SIMD

- Target: `Resampler::resample_interleaved(..., 2)` and stereo `StreamingResampler::process_into()`
- ISA: SSE2 on `x86` and `x86_64`
- Fallback: scalar FIR accumulation on unsupported targets and for non-stereo channel counts
- Main win: accumulate both stereo channels together while the scalar path handles arbitrary channel counts

## Benchmarks

- `cargo bench --bench resampler_bench`
- Current Criterion cases:
  - `mono_44100_to_48000`
  - `stereo_44100_to_48000`
