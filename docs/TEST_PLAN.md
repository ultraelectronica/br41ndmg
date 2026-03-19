# Test Plan

## Overview

Comprehensive testing strategy for validating resampler correctness, audio quality, and DSP characteristics.

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

**Validation**:
- Peak amplitude within 3dB of input
- Side lobe levels below -80dB
- Check for NaN/Inf values

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

**Validation**:
- SNR > 100dB for sine preservation
- THD < -100dB
- Peak frequency within 0.1% of expected

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

**Validation**:
- No sudden amplitude jumps
- Smooth spectral response
- No aliasing in downsampled regions

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

## Quality Thresholds

| Test | Metric | Threshold |
|------|--------|-----------|
| Impulse | Peak amplitude | ±0.5 dB of input |
| Impulse | Side lobe level | < -80 dB |
| Sine | SNR | > 100 dB |
| Sine | THD | < -100 dB |
| Sweep | Amplitude variation | < 1 dB across band |
| DC | DC gain accuracy | ±0.1 dB |
| All | NaN/Inf check | Zero occurrences |
| All | Output length | Matches expected ratio |

## Pass/Fail Criteria

A test passes when:
1. All samples are finite (no NaN/Inf)
2. Quality metrics meet thresholds
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
- `test_impulse_response` - Verify filter characteristics
- `test_sine_preservation` - Frequency accuracy
- `test_dc_gain` - DC response
- `test_silence` - Zero signal handling

### Conversion Tests
- `test_44100_to_48000` - Common ratio
- `test_48000_to_44100` - Inverse ratio
- `test_upsample_2x` - Simple integer ratio
- `test_downsample_2x` - Simple integer ratio
- `test_96000_to_48000` - Large ratio

### Edge Cases
- `test_very_small_ratio` - Extreme downsampling
- `test_very_large_ratio` - Extreme upsampling
- `test_identical_rates` - 1:1 ratio (passthrough)
