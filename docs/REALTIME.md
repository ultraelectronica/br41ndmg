# Realtime

How to use `StreamingResampler` safely inside a real-time audio callback.

## Callback Rules

- Use a single `StreamingResampler` and reuse it for the lifetime of the stream.
- Preallocate the output buffer once and reuse it across callbacks.
- Call [`output_samples_for(input_frames)`](../src/resampler.rs) before `process_into` to size or validate the callback buffer.
- Keep file I/O, logging, allocation growth, and control-thread work out of the audio callback.
- Call `flush_into` only when the stream is ending, not between normal callback blocks.
- After `flush_into`, call `reset` before reusing the resampler for a new stream.

## Threading Model

- Construct the resampler and any reusable buffers on a control thread.
- Move only the prepared `StreamingResampler` and fixed-capacity buffers into the audio thread.
- Avoid sharing mutable state through blocking locks in the callback.

## Buffer Sizes

- Input and output are interleaved frame buffers.
- `process_into` writes a variable number of output frames depending on the current fractional phase and how much lookahead is available.
- `output_frames_for` and `output_samples_for` report the exact output required for the next chunk — never allocate inside the callback, size from these.
- `flush_frames` and `flush_samples` report the remaining tail emitted at end-of-stream.

## Latency Tradeoffs

- The streaming path uses the **same polyphase sinc filter** as the offline path. Its algorithmic latency equals the filter radius in input frames (`taps_per_phase / 2`, 31 frames by default).
- `process_into` only emits an output sample once its lookahead window (`filter.radius()` input frames ahead) is fully available, so steady-state output lags the input by that radius.
- Output positions are derived from the output-frame counter as `index / ratio` — the identical expression used offline — so streaming output is **bit-for-bit identical** to the offline resampler regardless of how the input is chunked.
- End-of-stream output is emitted by `flush_into`, which repeats the final input frame to match the offline path's edge clamping.

## Minimal Callback Sketch

```rust
use br41ndmg::StreamingResampler;

// Prepared once on the control thread:
let mut stream = StreamingResampler::new(44_100.0, 48_000.0, 2)?;
let mut output = vec![0.0_f32; 0]; // grown once, then reused

// Inside the audio callback, with `input` being this block's interleaved frames:
let need = stream.output_samples_for(input.len() / 2);
output.resize(need.max(output.len()), 0.0); // grow only the first time
let written = stream.process_into(input, &mut output[..need])?;
let frames_out = &output[..written * 2];
# Ok::<(), br41ndmg::ResampleError>(())
```
