# DSP Notes

## Sampling Theorem

The Nyquist-Shannon sampling theorem states that a bandlimited signal can be perfectly reconstructed from samples if the sampling frequency is at least twice the highest frequency component.

```
f_sampling ≥ 2 * f_max
```

When downsampling, the input must be lowpass filtered to prevent aliasing. When upsampling, a reconstruction filter removes images.

## Sinc Interpolation

The ideal reconstruction filter for continuous-time signals is the sinc function:

```
sinc(x) = sin(πx) / (πx)
```

For discrete resampling, we need to evaluate sinc at fractional positions to compute interpolated values. This is equivalent to ideal bandlimited interpolation.

### Normalized Sinc

For resampling with a given cutoff frequency `f_c` (normalized to Nyquist):

```
sinc(x) = sin(2πf_c * x) / (πx)
```

The implementation must handle the singularity at `x = 0` where `sinc(0) = 1`.

## Window Functions

Sinc has infinite support. We truncate it with a window function to create a finite FIR filter.

### Hann Window

```
w(n) = 0.5 * (1 - cos(2πn / N))
```

Good all-purpose window with smooth rolloff.

### Hamming Window

```
w(n) = 0.54 - 0.46 * cos(2πn / N)
```

Similar to Hann with higher first sidelobe.

### Blackman Window

```
w(n) = 0.42 - 0.5 * cos(2πn / N) + 0.08 * cos(4πn / N)
```

Better sidelobe suppression, wider main lobe.

### Kaiser Window

```
w(n) = I_0(β * sqrt(1 - (2n/N)^2)) / I_0(β)
```

Parameterized shape factor `β` controls tradeoff between main lobe width and sidelobe level.

## FIR Filter Design

Windowed sinc FIR kernel:

```
h(n) = sinc(2f_c * (n - M/2)) * w(n - M/2)
```

Where:
- `f_c`: Normalized cutoff frequency
- `M`: Filter length (odd)
- `w()`: Window function

## Polyphase Decomposition

Rather than computing sinc for each output sample, polyphase splits the FIR filter into `P` phases:

```
h_p(n) = h(nP + p)  for p = 0, 1, ..., P-1
```

At each output sample:
1. Determine fractional position `μ`
2. Select nearest phase `p = floor(μ * P) mod P`
3. Apply FIR filter `h_p` to input samples

This reduces computation by reusing precomputed coefficient sets.

## Aliasing and Imaging

**Aliasing**: When downsampling, frequencies above the new Nyquist fold back into the signal band. Prevented by lowpass filtering before decimation.

**Imaging**: When upsampling, spectral images appear at multiples of the original sample rate. Removed by lowpass interpolation filter.

## Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Passband ripple | < 0.1 dB | Max deviation in 0-0.9 Nyquist |
| Stopband attenuation | > 100 dB | Minimum attenuation above cutoff |
| Aliasing suppression | < -100 dB | Energy of aliased components |
| THD | < -120 dB | Harmonic distortion of pure tones |

## Known Conversion Ratios

Common audio sample rates and their LCM-based rational approximations:
- 44100 → 48000: 147/160
- 48000 → 44100: 160/147
- 44100 → 96000: 160/73.5 (use 2x upsampling then 2/3)
- 48000 → 192000: 4/1 (integer ratio)
