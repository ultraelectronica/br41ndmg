# Test Plan

## Overview

Current testing strategy for validating resampler correctness, audio quality, and DSP characteristics.

The checked-in integration suite already covers:
- Impulse identity at 1:1 ratio and main-lobe symmetry when upsampling
- Low-frequency sine round-trip error and passband RMS preservation
- Downsampled sweep attenuation for out-of-band content
- DC stability, out-of-band tone suppression, and stereo round-trip quality
- Offline vs streaming equivalence and WAV I/O regressions

## Test Signals

### 1. Impulse Response

A single impulse (sample at t=0 with amplitude 1, all others 0) reveals the filter's impulse response.

**Generation**:
```rust
fn generate_impulse(n: usize) -> Vec<f32> {
    let mut signal = vec![0.0f32; n];
    signal[n/2] = 1.0;
    signal
}
```

**Expected behavior**:
- Output is the resampler's FIR kernel centered at impulse location
- Smooth decay in side lobes
- Symmetric about center
- No unexpected ringing or pre-echo

**Current validation**:
- 1:1 ratio preserves a centered impulse sample-for-sample
- 2x upsampling keeps the main lobe symmetric around the peak
- Check for NaN/Inf values implicitly through exact/approximate assertions

### 2. Sine Waves

Pure tones test frequency response and harmonic distortion.

**Generation**:
```rust
fn generate_sine(freq: f32, sample_rate: f32, duration: f32) -> Vec<f32> {
    let n = (sample_rate * duration) as usize;
    (0..n).map(|i| {
        let t = i as f32 / sample_rate;
        (2.0 * std::f32::consts::PI * freq * t).sin()
    }).collect()
}
```

**Test frequencies**:
- 100 Hz (low frequency)
- 1 kHz (mid frequency)
- 10 kHz (high frequency)
- 18 kHz (near Nyquist)

**Expected behavior**:
- Output amplitude matches input amplitude
- No frequency shift
- No aliasing artifacts visible in spectrum
- Stable amplitude over time

**Current validation**:
- Low-frequency round-trip RMSE stays below `8e-3`
- Passband RMS stays within 5% after resampling

### 3. Frequency Sweeps

Chirp signals reveal frequency-dependent behavior across the spectrum.

**Generation**:
```rust
fn generate_sweep(sample_rate: f32, duration: f32, 
                  f_start: f32, f_end: f32) -> Vec<f32> {
    let n = (sample_rate * duration) as usize;
    let k = (f_end - f_start) / duration;
    (0..n).map(|i| {
        let t = i as f32 / sample_rate;
        let phase = 2.0 * std::f32::consts::PI * (f_start * t + 0.5 * k * t * t);
        phase.sin()
    }).collect()
}
```

**Sweep ranges**:
- 20 Hz → 20 kHz (full audible range)
- 20 Hz → Nyquist (for Nyquist-rate conversions)

**Current validation**:
- Downsampled out-of-band sweep RMS stays below 20% of an in-band reference sweep

### 4. DC Signal

Constant signal tests DC response and gain.

**Generation**:
```rust
let dc = vec![1.0f32; 1000];
```

**Expected**: Output all samples with constant gain (may have edge effects at boundaries).

### 5. Silence

All zeros input tests handling of empty/negligible signal.

**Expected**: Output all zeros (no NaN, no artifacts).

## Checked-In Thresholds

| Test | Metric | Threshold |
|------|--------|-----------|
| Impulse | 1:1 identity | Exact within `1e-6` |
| Impulse | 2x symmetry | Local lobe mismatch <= `5e-4` |
| Sine | Round-trip RMSE | <= `8e-3` |
| Sine | Passband RMS drift | <= 5% |
| Sweep | Out-of-band attenuation | RMS < 20% of in-band sweep |
| DC | DC gain accuracy | <= `1e-3` after edge trim |
| Tone | 12 kHz -> 16 kHz suppression | RMS <= `0.05` |
| Stereo | Per-channel round-trip RMSE | <= `1e-2` |

## Pass/Fail Criteria

A checked-in test passes when:
1. All samples are finite (no NaN/Inf)
2. The specific signal metric meets its threshold
3. Output length equals expected
4. Boundary conditions handled gracefully

A test fails when:
1. Any sample is NaN or Inf
2. Any metric exceeds threshold
3. Crash or panic occurs
4. Output length is incorrect

## Test Categories

### Correctness Tests
- `test_empty_input` - Handle zero-length input
- `test_single_sample` - Handle minimal input
- `test_output_length` - Verify ratio calculation
- `test_deterministic` - Same input → same output

### DSP Tests
- `tests/impulse.rs` - Impulse identity and symmetry
- `tests/sine.rs` - Round-trip and passband tone preservation
- `tests/sweep.rs` - Out-of-band sweep attenuation
- `tests/quality_tests.rs` - DC response, alias suppression, stereo quality

### Conversion Tests
- `tests/resampler.rs` - Interleaved vs per-channel equivalence
- `tests/streaming.rs` - Offline vs streaming equivalence
- `tests/file_io.rs` - WAV conversion and layout preservation

### Edge Cases
- `test_very_small_ratio` - Extreme downsampling
- `test_very_large_ratio` - Extreme upsampling
- `test_identical_rates` - 1:1 ratio (passthrough)

## Next Validation Work

- Spectral measurements for THD, stopband attenuation, and passband ripple
- Longer real-world fixtures in addition to synthetic signals
- Benchmark-linked quality baselines for multiple filter settings
