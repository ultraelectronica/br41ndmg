# Usage Guide

This is the end-user guide for `br41ndmg`. It covers installation, the common
processing patterns, file I/O, filter tuning, real-time streaming, and
error handling. For API details run `cargo doc --open`; for theory see
[DSP_NOTES.md](DSP_NOTES.md); for real-time rules see [REALTIME.md](REALTIME.md).

## Table of contents

1. [Installation](#installation)
2. [The three entry points](#the-three-entry-points)
3. [Offline resampling](#offline-resampling)
4. [File I/O (WAV and FLAC)](#file-io-wav-and-flac)
5. [Custom filter parameters](#custom-filter-parameters)
6. [DSP helper APIs](#dsp-helper-apis)
7. [Real-time streaming](#real-time-streaming)
8. [Channel layouts](#channel-layouts)
9. [Error handling](#error-handling)
10. [Examples and benchmarks](#examples-and-benchmarks)
11. [Tips and pitfalls](#tips-and-pitfalls)

---

## Installation

```toml
[dependencies]
br41ndmg = "0.1"
```

The default features include FLAC input support (via the pure-Rust `claxon`
decoder). For a leaner, WAV-only build:

```toml
[dependencies]
br41ndmg = { version = "0.1", default-features = false }
```

Requires Rust **edition 2024** (Rust 1.85+).

---

## The three entry points

| Type | Use it when |
|------|-------------|
| [`Resampler`](../src/resampler.rs) | You have the whole signal in memory and want a `Vec<f32>` back. |
| [`StreamingResampler`](../src/resampler.rs) | You process audio in chunks (real-time callback, ring buffer, file streaming). |
| [`io::AudioBuffer`](../src/io.rs) + `read_audio` / `write_wav` | You are working from files and want metadata (rate, channels) carried with the samples. |

Both resampler types use the **same polyphase sinc filter**, so they produce
identical samples for the same input — `StreamingResampler` just lets you feed
the input in arbitrary-sized chunks.

---

## Offline resampling

### Mono buffer

```rust
use br41ndmg::Resampler;

let resampler = Resampler::new(44_100.0_f32, 48_000.0_f32)?;
let input: Vec<f32> = /* mono samples in [-1, 1] */;
let output = resampler.resample(&input)?;
# Ok::<(), br41ndmg::ResampleError>(())
```

### Interleaved multichannel buffer

```rust
use br41ndmg::Resampler;

let resampler = Resampler::new(44_100.0, 48_000.0)?;
let stereo: Vec<f32> = /* interleaved [L, R, L, R, ...] */;
let output = resampler.resample_interleaved(&stereo, 2)?;
# Ok::<(), br41ndmg::ResampleError>(())
```

`resample_interleaved` keeps the data interleaved (no de/re-interleave pass).
Stereo uses an SSE2 fast path on x86/x86_64; other channel counts use a scalar
path that works for any number of channels.

Sample-rate arguments accept `f32`, `f64`, or integer types — anything that
converts into `f64`.

---

## File I/O (WAV and FLAC)

```rust
use br41ndmg::io::{read_audio, write_wav};

// read_audio picks the decoder from the extension: .wav or .flac
let input = read_audio("song.flac")?;
println!(
    "{} ch, {} Hz, {} frames",
    input.channels(),
    input.sample_rate(),
    input.frame_count(),
);

let output = input.resample_to(48_000)?; // preserves channels
write_wav("song_48k.wav", &output)?;
# Ok::<(), br41ndmg::ResampleError>(())
```

- **Input**: WAV (8/16/24/32-bit PCM, 32-bit float) and FLAC (4–32-bit, with the
  `flac` feature). Integer samples are normalized to `[-1.0, 1.0]`.
- **Output**: 32-bit float WAV. Samples are clamped to `[-1.0, 1.0]` on write.
- For explicit decoders use [`read_wav`](../src/io.rs) / [`read_flac`](../src/io.rs).

`AudioBuffer::resample_to` is a convenience wrapper that builds a `Resampler`
internally. Construct your own `Resampler` if you want to reuse it across many
buffers or tune the filter.

### Command-line tool

The CLI ships in a separate package (`br41ndmg-cli`) but installs a binary
named `br41ndmg`:

```bash
cargo install br41ndmg-cli
```

The tool has two modes: an **interactive file browser** and a **non-interactive
three-argument** path.

**Interactive mode** — run with no arguments, with a single directory, or with
`-i`/`--interactive [dir]`:

```bash
br41ndmg                # browser, starts in the current directory
br41ndmg test_subjects/  # browser, starting in that folder
br41ndmg -i              # force interactive mode
```

In the browser:

- `↑`/`↓` (or `j`/`k`) move the cursor.
- `Enter` opens a directory or toggles a file's selection; `Space` toggles.
- `u` goes up one directory level.
- `a` selects every audio file in the current directory; `c` clears.
- `p` proceeds to the settings screen (enabled once at least one file is
  selected). There you choose a **target sample rate** (a list of common
  presets, or "Custom…" to type your own) and an **output directory** (type it
  or browse for it ncdu-style: `Enter`/`→` to open a folder, `←`/`u` to go up,
  `m`/`Space` to use the current directory). Move between fields with
  `Tab`/`↑`/`↓`, `Enter` to edit one, and `Enter` on **Start resampling** to
  begin. A progress bar reports each file; large FLACs are resampled on a
  background thread so the UI stays responsive.
- `q`/`Esc` quits.

**Non-interactive mode** — `br41ndmg <input> <output> <target_sample_rate>`.
Both single files and whole directories are supported, and the output target
decides the filename:

- If `<output>` ends in `.wav` (and is not an existing directory), it is used
  verbatim as the output path.
- Otherwise `<output>` is treated as a directory (created if missing) and the
  file is auto-named `<original-stem>_<rate>Hz.wav`.
- If `<input>` is a directory, every `.wav`/`.flac` inside it is resampled
  into `<output>` using that naming rule (batch mode).

```bash
# Single file to an explicit path
br41ndmg input.flac output.wav 48000

# Single file into a directory (auto-named input_48000Hz.wav)
br41ndmg input.flac out_dir/ 48000

# Batch: resample every song in a folder into out_dir/
br41ndmg test_subjects/ out_dir/ 48000
```

From a source checkout, use `cargo run --release --bin br41ndmg -- <args>`.

---

## Custom filter parameters

The default bank is a 63-tap, 256-phase Blackman-windowed sinc. Tune it for
sharper rolloff (more taps) or finer fractional-delay resolution (more phases):

```rust
use br41ndmg::{PolyphaseFilterParams, Resampler, Window};

let params = PolyphaseFilterParams {
    phases: 512,
    taps_per_phase: 127,   // must be odd and non-zero
    window: Window::Blackman,
};
let resampler = Resampler::with_filter_params(44_100.0, 96_000.0, params)?;
# Ok::<(), br41ndmg::ResampleError>(())
```

Tradeoffs:

- **`taps_per_phase`** (odd): larger → sharper transition band, more stopband
  attenuation, longer latency (`taps_per_phase / 2` input frames), more CPU.
- **`phases`**: larger → finer fractional-delay quantization, smaller passband
  ripple, larger coefficient table (memory = `phases * taps_per_phase * 4` bytes).
- **`window`**: `Blackman` (default) gives strong sidelobes; `Hann`/`Hamming`
  are narrower main-lobe; `Kaiser { beta }` lets you dial the tradeoff directly.

Cutoff is chosen automatically: full Nyquist when upsampling, scaled by the
ratio (with a small margin) when downsampling, to suppress aliasing.

---

## DSP helper APIs

Independent of the resampler, the crate exposes the building blocks for custom
FIR design in both `f64` and `f32`:

```rust
use br41ndmg::filter::fir_kernel;
use br41ndmg::window::Window;

let taps = fir_kernel(63, 0.45, Window::Hamming);
assert!((taps.iter().sum::<f64>() - 1.0).abs() < 1e-9);
```

- [`sinc`](../src/sinc.rs): `sinc`, `normalized_sinc`, `sinc_kernel`.
- [`window`](../src/window.rs): `Window` enum, `apply_window`.
- [`filter`](../src/filter.rs): `FirKernel`, `fir_kernel`.

These are useful for inspecting the design or building related DSP.

---

## Real-time streaming

`StreamingResampler` processes interleaved audio in variable-size chunks. Size
the output buffer from the predictor methods, never by guessing:

```rust
use br41ndmg::StreamingResampler;

let mut stream = StreamingResampler::new(44_100.0_f32, 48_000.0_f32, 2)?;

for chunk in input_chunks {
    let frames_in = chunk.len() / 2;
    let mut out = vec![0.0_f32; stream.output_samples_for(frames_in)];
    let written = stream.process_into(chunk, &mut out)?;
    play(&out[..written * 2]);
}

// Drain the latency tail at end-of-stream:
let mut tail = vec![0.0_f32; stream.flush_samples()];
let written = stream.flush_into(&mut tail)?;
play(&tail[..written * 2]);

// Reuse the resampler for another stream:
stream.reset();
# Ok::<(), br41ndmg::ResampleError>(())
```

Key points (see [REALTIME.md](REALTIME.md) for the full callback discipline):

- `output_samples_for(n)` / `output_frames_for(n)` predict exactly how many
  samples the next `process_into` will emit for `n` input frames.
- `process_into` returns the number of **frames** written.
- `flush_samples()` / `flush_frames()` predict the end-of-stream tail.
- `latency_frames()` returns the algorithmic latency in input frames
  (`taps_per_phase / 2`, 31 by default).
- `process_into` after `flush_into` is an error; call `reset()` first.
- Output is bit-for-bit identical to `Resampler` for the same input.

---

## Channel layouts

All processing expects **interleaved** samples (`[L, R, L, R, ...]` for stereo).
`resample_interleaved(input, channels)` requires `input.len()` to be a whole
multiple of `channels`. Per-channel (planar) buffers should be resampled
one channel at a time with `resample()`.

---

## Error handling

Everything fallible returns [`ResampleError`](../src/error.rs):

| Variant | When |
|---------|------|
| `InvalidSampleRate` | A sample rate is zero or non-finite. |
| `InvalidRatio` | The derived ratio is non-finite or non-positive. |
| `InvalidFilterConfig` | `PolyphaseFilterParams` fails validation (zero/even taps, bad Kaiser beta). |
| `InvalidChannelCount` | `channels == 0`. |
| `BufferError` | Mis-sized interleaved/output buffers, processing after flush, etc. |
| `UnsupportedWavFormat` | Unknown bit depth or file extension. |
| `Wav` / `Flac` | Decoder I/O or format errors. |

`ResampleError` implements `std::error::Error` (via `thiserror`), so it composes
with `?` and `Box<dyn Error>` as in the examples.

---

## Examples and benchmarks

```bash
# Resample a file (WAV or FLAC in, WAV out)
br41ndmg input.flac output.wav 48000

# Batch-resample a whole folder (auto-named <stem>_<rate>Hz.wav)
br41ndmg test_subjects/ out_dir/ 48000

# Generate and resample a synthetic tone
cargo run --release --example tone_resample -- tone.wav

# Run the test suite (includes real-audio tests if test_subjects/ is present)
cargo test

# Run only the real-audio integration tests
cargo test --test real_audio

# Benchmarks
cargo bench --bench resampler_bench
```

---

## Tips and pitfalls

- **Reuse a `Resampler`** across buffers with the same rate pair — building the
  filter table is the expensive part.
- **Don't allocate in the audio callback.** Pre-size buffers from
  `output_samples_for` and grow them once on the control thread.
- **Output amplitude** stays in `[-1, 1]` for in-range input; the polyphase
  filter is DC-normalized so gain is unity in the passband.
- **Edge clamping**: the first and last few output samples see edge-extended
  input (clamped to the boundary frame). Trim `latency_frames()` samples from
  the head/tail if you need a clean steady-state window.
- **FLAC is a default feature.** If your target doesn't need it, disable
  default features to drop the `claxon` dependency.
