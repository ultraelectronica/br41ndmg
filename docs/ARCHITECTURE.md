# Architecture

## Module Map

| Module | Purpose |
|--------|---------|
| `lib.rs` | Public re-exports, crate root |
| `sinc.rs` | `f64` and `f32` sinc helpers and kernel builders |
| `window.rs` | `f64` and `f32` window generation |
| `filter.rs` | `f64` and `f32` FIR kernel generation |
| `io.rs` | Offline WAV decoding, encoding, and audio buffers |
| `polyphase.rs` | Polyphase sinc filter-bank construction and phase lookup |
| `resampler.rs` | Offline and streaming polyphase sinc resamplers |
| `error.rs` | Error types (InvalidSampleRate, InvalidRatio) |
| `utils.rs` | Small shared validation helpers |

## Data Flow

```
Input Samples
     │
     ▼
┌─────────────────────┐
│   Resampler         │
│  ┌───────────────┐  │
│  │ ratio calc    │  │
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │ position     │──┼──► fractional position in input
│  │ mapping      │  │
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │ phase select │──┼──► choose precomputed sinc phase
│  └───────────────┘  │
│  ┌───────────────┐  │
│  │ FIR dot      │──┼──► convolve windowed-sinc taps with source frames
│  │ product      │  │
│  └───────────────┘  │
└─────────────────────┘
     │
     ▼
Output Samples
```

## Current Implementation

### Resampler (resampler.rs)

The `Resampler` struct holds:
- `input_rate`: Source sample rate in Hz
- `output_rate`: Target sample rate in Hz  
- `ratio`: Derived output_rate / input_rate
- `filter`: Precomputed polyphase sinc coefficient table

The `resample()` method performs polyphase sinc interpolation:
1. Calculate output length from input length and ratio
2. For each output position, map back to input position
3. Pick the nearest precomputed phase for the fractional offset
4. Convolve the phase taps against edge-extended input samples

The `resample_interleaved()` method keeps interleaved buffers in place instead of deinterleaving them first. For stereo input on `x86` and `x86_64`, it uses an SSE2 fast path to accumulate both channels together through the FIR loop.

`StreamingResampler` shares the same polyphase model, buffers the history needed for the filter radius, and flushes the remaining tail with last-frame edge extension so chunked processing matches offline behavior.

### Polyphase Filter Bank

The `polyphase.rs` module contains:
- A precomputed coefficient table for fractional phases
- A default 63-tap Blackman-windowed sinc kernel bank with 256 phases
- Cutoff selection that preserves full-band upsampling and applies margin during downsampling

## Offline vs Real-time

**Offline (current)**: Processes the full input buffer and returns a new `Vec<f32>`. `io.rs` handles WAV decode to normalized interleaved `AudioBuffer` values and writes 32-bit float WAV output.

**Real-time (current)**: `StreamingResampler` keeps enough input history to cover the filter radius plus a fractional read position. The caller provides reusable output buffers via `process_into()` and finishes the stream with `flush_into()`.

## Design Decisions

- **Type**: `f32` for audio samples, `f64` for sample-rate tracking and position math
- **DSP helper APIs**: `f64` and `f32` variants for sinc, window, and FIR generation
- **Window**: Blackman-windowed sinc for the default polyphase bank
- **SIMD**: SSE2 stereo fast path where the interleaved frame layout is contiguous
- **Polyphase**: Shared offline/streaming implementation so quality behavior stays aligned
