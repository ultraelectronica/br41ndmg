# Architecture

## Module Map

- `lib.rs` - Public re-exports, crate root
- `sinc.rs` - Sinc function implementation
- `window.rs` - Window functions (Hann, Blackman, Kaiser)
- `filter.rs` - FIR kernel generation and normalization
- `polyphase.rs` - FilterBank and phase table
- `resampler.rs` - ResamplerConfig and process()
- `error.rs` - Error enum
- `utils.rs` - GCD and rational ratio helpers

## Data Flow

[Data flow diagram]
