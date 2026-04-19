# Architecture

## Module Map

| Module | Purpose |
|--------|---------|
| `lib.rs` | Public re-exports, crate root |
| `sinc.rs` | `f64` and `f32` sinc helpers and kernel builders |
| `window.rs` | `f64` and `f32` window generation |
| `filter.rs` | `f64` and `f32` FIR kernel generation |
| `io.rs` | Offline WAV decoding, encoding, and audio buffers |
| `polyphase.rs` | Reserved placeholder for a future filterbank |
| `resampler.rs` | Offline and streaming linear-interpolation resamplers |
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
│  │ linear       │──┼──► interpolate adjacent samples or frames
│  │ interpolate  │  │
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

The `resample()` method performs scalar linear interpolation:
1. Calculate output length from input length and ratio
2. For each output position, map back to input position
3. Interpolate between adjacent samples using fractional offset

The `resample_interleaved()` method keeps interleaved buffers in place instead of deinterleaving them first. For stereo input on `x86` and `x86_64`, it uses an SSE2 fast path to interpolate both channels together.

`StreamingResampler` shares the same interpolation model and retains one previous frame so chunked processing matches offline behavior.

### Future: Polyphase FilterBank

The `polyphase.rs` module will contain:
- Precomputed filter coefficients for M phases
- Efficient FIR dot product per output sample
- Reduced computation vs recomputing sinc each time

## Offline vs Real-time

**Offline (current)**: Processes the full input buffer and returns a new `Vec<f32>`. `io.rs` handles WAV decode to normalized interleaved `AudioBuffer` values and writes 32-bit float WAV output.

**Real-time (current)**: `StreamingResampler` keeps one input frame of history per channel plus a fractional read position. The caller provides reusable output buffers via `process_into()` and finishes the stream with `flush_into()`.

## Design Decisions

- **Type**: `f32` for audio samples, `f64` for sample-rate tracking and position math
- **DSP helper APIs**: `f64` and `f32` variants for sinc, window, and FIR generation
- **Window**: Configurable, Hann default
- **SIMD**: SSE2 stereo fast path where the frame layout is contiguous
- **Polyphase**: Deferred until the filterbank exists
