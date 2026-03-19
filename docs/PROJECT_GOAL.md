# Project goal

Build a **production-quality polyphase sinc resampler in Rust** that can:

* resample audio between arbitrary sample rates
* preserve audio quality
* avoid aliasing/imaging artifacts
* run efficiently in real time
* be measurable, benchmarked, and documented like a serious DSP project

The end result is not just code. It is a **specialized audio engineering project** with strong math, tests, and performance work.

---

# 0%–10%: Requirements gathering and problem definition

This is where most projects fail. Do not code yet.

## 1) Define the exact problem

Write down the project in one sentence:

> “I am building a Rust resampler that converts PCM audio from any input sample rate to any output sample rate with high fidelity and predictable latency.”

Then define what is **in scope** and **out of scope**.

### In scope

* mono and stereo PCM audio first
* arbitrary rational and non-rational sample-rate ratios
* offline file processing first, then real-time streaming later
* quality-focused sinc/polyphase resampling

### Out of scope, initially

* pitch shifting
* time stretching
* AI upscaling
* compressed codecs as input/output
* plugin hosting
* fancy GUI

## 2) Define quality targets

Set measurable goals early:

* passband ripple target
* stopband attenuation target
* aliasing threshold
* acceptable latency
* CPU budget
* supported bit depth and sample format

Example target:

* **aliasing below -100 dB**
* **low-latency streaming support**
* **stable output for 44.1k → 48k, 48k → 96k, etc.**

## 3) Define user stories

Write simple use cases:

* “As a user, I can convert a WAV file from 44.1 kHz to 48 kHz.”
* “As a developer, I can use the resampler as a Rust library.”
* “As a performance tester, I can benchmark throughput and latency.”
* “As an audio engineer, I can inspect frequency response and artifacts.”

## 4) Requirements doc

Create a `REQUIREMENTS.md` with:

* objective
* supported formats
* non-goals
* quality targets
* performance targets
* testing goals
* future features

This doc becomes your contract.

---

# 10%–20%: Research and design documentation

Before implementation, write the design.

## Docs to create now

### `README.md`

High-level summary:

* what the project is
* why it exists
* features
* quick start
* examples
* roadmap

### `ARCHITECTURE.md`

Explain:

* module layout
* data flow
* resampling strategy
* offline vs real-time pipeline
* how filters are built and applied

### `DSP_NOTES.md`

Explain the math:

* sampling theorem
* sinc interpolation
* window functions
* FIR filters
* polyphase decomposition
* fractional delay
* aliasing and imaging

### `TEST_PLAN.md`

Explain:

* what signals you will test with
* how you will measure quality
* expected results
* acceptance criteria

### `BENCHMARK_PLAN.md`

Explain:

* what gets benchmarked
* what datasets are used
* target performance metrics
* how results are compared

## Design decisions to settle now

Choose:

* language level: Rust stable only
* numeric type: start with `f64` for correctness, later consider `f32`
* library type: crate library first
* processing style: offline batch first
* filter style: windowed sinc polyphase
* later optimization: SIMD and streaming

At this stage, the goal is **clarity**, not speed.

---

# 20%–30%: Repository setup and scaffolding

Now build the skeleton.

## Project structure

A clean layout might look like:

```text
extreme-src/
├── src/
│   ├── lib.rs
│   ├── sinc.rs
│   ├── window.rs
│   ├── filter.rs
│   ├── polyphase.rs
│   ├── resampler.rs
│   └── error.rs
├── tests/
│   ├── impulse.rs
│   ├── sine.rs
│   └── sweep.rs
├── benches/
│   └── resampler_bench.rs
├── examples/
│   ├── file_resample.rs
│   └── tone_resample.rs
├── docs/
│   ├── ARCHITECTURE.md
│   ├── DSP_NOTES.md
│   ├── TEST_PLAN.md
│   └── BENCHMARK_PLAN.md
├── Cargo.toml
├── Cargo.lock
├── README.md
└── .gitignore
```

## Add `.gitignore`

Ignore:

* `/target`
* profiling output
* temporary audio files
* editor junk
* generated benchmark artifacts

Keep:

* `Cargo.lock`
* source code
* docs
* examples
* tests

## Cargo setup

Add dependencies only when needed:

* `criterion` for benchmarks
* maybe `hound` or `symphonia` for audio file I/O later
* maybe `cpal` for real-time audio later

At this stage, the repo should compile even if the core logic is still empty.

---

# 30%–40%: Core math primitives

Now build the smallest useful DSP pieces.

## Implement the basic math modules

Start with:

### `sinc`

* exact and numerically safe implementation
* handle `x = 0` cleanly

### window functions

Start with one window:

* Hann
  Then later add:
* Hamming
* Blackman
* Kaiser

### filter kernel generation

Build a function that generates a finite sinc-based FIR kernel.

## What to verify

* sinc is symmetric
* window shape behaves correctly
* kernel coefficients are normalized
* kernel length is odd or at least centered properly

## Tests to write

* `sinc(0) == 1`
* window edges go to zero or near-zero where expected
* filter symmetry
* sum of coefficients near expected gain

This phase is about proving that your **math foundation is correct**.

---

# 40%–50%: Naive resampler prototype

Before polyphase, make something that works simply.

## Build the first working resampler

Implement a naive resampling pipeline:

* take input samples
* compute output sample positions
* interpolate using a simple method first
* verify the pipeline works end-to-end

Good first versions:

* linear interpolation
* cubic interpolation
* simple FIR-based interpolation

## Why this matters

It gives you:

* a working output
* a baseline for testing
* a correctness target
* a way to compare “simple” vs “high quality”

## Deliverables

* `Resampler` struct
* `process(&[f64]) -> Vec<f64>`
* basic example program
* a tiny test tone conversion example

## What to document

Update `README.md` with:

* how to run the example
* what the output is supposed to be
* limitations of the prototype

---

# 50%–60%: High-quality polyphase sinc implementation

This is the real project.

## Build the polyphase filterbank

Instead of computing sinc from scratch for every output sample, precompute filters for multiple fractional phases.

### You need to decide:

* number of phases
* number of taps
* cutoff frequency
* window type
* normalization strategy

## Internal design

You will likely want:

* a `FilterBank`
* a `PhaseIndex`
* an `Interpolator`
* a `ResamplerConfig`

## Processing pipeline

1. determine output sample time
2. map it to an input fractional position
3. choose the nearest phase
4. apply FIR dot product
5. emit output sample

## What to test

* impulse response
* sine wave stability
* sweep response
* known sample-rate conversions like 44.1k → 48k

## Documentation to add

In `ARCHITECTURE.md`, explain:

* why polyphase exists
* how phase tables work
* why it is faster than computing sinc every time
* how latency depends on filter length

---

# 60%–70%: File I/O and realistic audio integration

Now make it useful on real audio.

## Add offline file support

Support:

* WAV first
* then possibly FLAC/MP3/M4A through a decoder crate later

## Recommended flow

* read audio file
* convert samples into a common internal format
* resample
* write new file

## Why offline first

Because it is easier to test:

* no timing jitter
* no callback restrictions
* no audio device issues
* easier debugging

## Real-world test files

Use:

* sine waves
* impulse files
* pink noise
* music clips
* sweep tones

## Documentation

Add an `examples/` section to README:

* sample commands
* input/output examples
* supported formats

---

# 70%–80%: Testing, measurement, and quality validation

This phase separates a hobby project from a specialist project.

## Create automated tests

### Correctness tests

* output length checks
* boundary behavior
* deterministic output for fixed input
* no panics on short buffers

### DSP tests

* impulse response
* sine preservation
* frequency sweep
* aliasing detection
* DC signal preservation

### Regression tests

Store known input/output pairs and compare future runs against them.

## Create benchmark suite

Measure:

* throughput
* latency
* memory allocations
* scaling with number of taps
* scaling with number of phases

## Add quality metrics

You can track:

* SNR
* THD
* spectral error
* maximum aliasing energy

## Documentation

Update `TEST_PLAN.md` with:

* exact test signals
* expected pass/fail conditions
* how to interpret failures

This is where your project starts looking serious.

---

# 80%–90%: Performance engineering and refinement

Now optimize only after correctness is proven.

## Optimizations to consider

* switch internal buffers to cache-friendly layouts
* precompute and reuse filter tables
* reduce allocations
* use slice-based processing
* add SIMD where safe
* profile hot paths before changing them

## Performance priorities

1. correctness
2. stable output
3. predictability
4. speed
5. advanced optimizations

## Things to avoid too early

* premature SIMD
* overcomplicated threading
* GPU offload
* micro-optimizations without profiling

## Add profiling docs

Create:

* `PERFORMANCE.md`
* flamegraph outputs
* benchmark comparisons before/after each optimization

The best specialist projects show not just code, but **proof of improvement**.

---

# 90%–95%: Real-time streaming version

Once offline processing is solid, adapt it for real-time audio.

## Real-time requirements

* no heap allocation in the audio callback
* no blocking locks
* bounded latency
* safe ring buffers
* graceful underrun handling

## Live pipeline

* input stream
* resampler
* output stream

## Important constraint

Real-time audio code is stricter than offline code.
Anything that might stall must be moved out of the callback.

## Documentation

Add `REALTIME.md`:

* what is allowed in the callback
* threading model
* buffer sizes
* latency tradeoffs

---

# 95%–100%: Polish, docs, release, and portfolio quality

This is where the project becomes presentable.

## Final docs to polish

### README

Should answer:

* what is it
* why does it exist
* how to build
* how to run
* how to test
* how to benchmark

### CHANGELOG

Track:

* features added
* bug fixes
* algorithm improvements
* benchmark changes

### API docs

Document:

* public structs
* configuration fields
* process methods
* error types

### Example gallery

Add examples for:

* tone conversion
* file conversion
* impulse test
* benchmark run

## Release checklist

* all tests pass
* benchmarks run cleanly
* docs are complete
* examples work
* code is formatted
* no obvious allocation issues in hot path
* version tagged

---

# Recommended build order summary

Here is the actual development order I would follow:

## Phase A: Define

* write requirements
* write docs
* decide scope

## Phase B: Scaffold

* create repo
* add modules
* add `.gitignore`
* add README skeleton

## Phase C: Math

* sinc
* windows
* FIR kernel
* basic tests

## Phase D: Prototype

* naive resampler
* one working output example

## Phase E: Polyphase

* precomputed phase table
* real high-quality conversion

## Phase F: Quality

* impulse / sine / sweep tests
* benchmark suite
* frequency response checks

## Phase G: Integration

* file I/O
* real audio examples
* maybe live streaming

## Phase H: Optimization

* profile
* tune
* add SIMD where it matters

## Phase I: Polish

* finalize docs
* stabilize API
* tag release

---

# What “done” looks like

Your project is complete when:

* it converts audio cleanly between sample rates
* it has repeatable tests
* it has clear docs
* it has benchmark evidence
* it can be used as a library
* it can be shown in a portfolio as a real DSP engineering project

That is the point where you have not just a repo, but a **specialization path**.

---

# The single most important thing to remember

Do **not** jump straight into optimization.

The right order is:

**requirements → docs → math → prototype → polyphase → tests → benchmarks → optimization → real-time → polish**
