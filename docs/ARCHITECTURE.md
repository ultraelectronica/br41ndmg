# Architecture

## Module Map

| Module | Purpose |
|--------|---------|
| `lib.rs` | Public re-exports, crate root |
| `sinc.rs` | Sinc function and normalized sinc kernel |
| `window.rs` | Window functions (Hann, Hamming, Blackman, Kaiser) |
| `filter.rs` | FIR kernel generation and normalization |
| `io.rs` | Offline WAV decoding, encoding, and audio buffers |
| `polyphase.rs` | FilterBank and phase table management |
| `resampler.rs` | Main Resampler struct and processing pipeline |
| `error.rs` | Error types (InvalidSampleRate, InvalidRatio) |
| `utils.rs` | GCD and rational ratio helpers |

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
│  │ FilterBank    │──┼──► polyphase coefficient lookup
│  │ (future)      │  │
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

The `resample()` method performs linear interpolation:
1. Calculate output length from input length and ratio
2. For each output position, map back to input position
3. Interpolate between adjacent samples using fractional offset

### Future: Polyphase FilterBank

The `polyphase.rs` module will contain:
- Precomputed filter coefficients for M phases
- Efficient FIR dot product per output sample
- Reduced computation vs recomputing sinc each time

## Offline vs Real-time

**Offline (current)**: Processes entire input buffer, returns complete output. `io.rs` handles WAV decode to normalized interleaved `AudioBuffer`, channel-wise resampling, and 32-bit float WAV output.

**Real-time (future)**: Ring buffer with lookahead. Latency determined by filter length. No allocations in callback.

## Design Decisions

- **Type**: `f32` for input/output, `f64` for internal calculations
- **Window**: Configurable, Hann default
- **Filter length**: Configurable, 64-128 taps typical
- **Phases**: 16-32 for quality/performance balance
