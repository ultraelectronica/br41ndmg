# Performance

## Profiling Results

No benchmark result snapshots are checked into the repository yet.

The current performance work focused on removing unnecessary interleaved-buffer copies and adding a stereo SIMD fast path.

## SIMD

- Target: `Resampler::resample_interleaved(..., 2)` and stereo `StreamingResampler::process_into()`
- ISA: SSE2 on `x86` and `x86_64`
- Fallback: scalar interpolation on unsupported targets and for non-stereo channel counts
- Main win: avoid the old deinterleave/resample/reinterleave flow and interpolate the two-channel frame directly

## Benchmarks

- `cargo bench --bench resampler_bench`
- Current Criterion cases:
  - `mono_44100_to_48000`
  - `stereo_44100_to_48000`
