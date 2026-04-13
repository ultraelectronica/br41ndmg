# Realtime

## Callback Rules

- Use `StreamingResampler` and reuse it for the lifetime of the stream.
- Preallocate the output buffer and reuse it across callbacks.
- Call `output_samples_for(input_frames)` before `process_into()` to size or validate the callback buffer.
- Keep file I/O, logging, allocation growth, and control-thread work out of the audio callback.
- Call `flush_into()` only when the stream is ending, not between normal callback blocks.

## Threading Model

- Construct the resampler and any reusable buffers on a control thread.
- Move only the prepared `StreamingResampler` and fixed-capacity buffers into the audio thread.
- Avoid sharing mutable state through blocking locks in the callback.

## Buffer Sizes

- Input and output are interleaved frame buffers.
- `process_into()` writes a variable number of output frames depending on the current fractional phase.
- `output_frames_for()` and `output_samples_for()` report the exact output required for the next chunk.
- `flush_frames()` and `flush_samples()` report the remaining tail that will be emitted at end-of-stream.

## Latency Tradeoffs

- The current linear-interpolation streaming path has a one-frame lookahead requirement.
- This gives a bounded algorithmic latency of one input frame.
- End-of-stream output is emitted by `flush_into()`, which repeats the final frame to match offline edge clamping.

## Latency Tradeoffs

[Latency considerations]
