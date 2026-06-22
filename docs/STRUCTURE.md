# br41ndmg

```
br41ndmg/
├── Cargo.toml              # Workspace manifest and dependencies
├── .gitignore              # Ignores: /target, *.wav, flamegraphs
├── README.md               # Overview, quick start, roadmap
│
├── src/
│   ├── lib.rs              # Public re-exports, crate root
│   ├── sinc.rs             # f64/f32 sinc helpers and kernel builders
│   ├── window.rs           # f64/f32 Hann, Hamming, Blackman, Kaiser
│   ├── filter.rs           # f64/f32 FIR kernel generation and normalization
│   ├── io.rs               # WAV/FLAC read, WAV write, and AudioBuffer helpers
│   ├── polyphase.rs        # Polyphase sinc filter-bank builder and phase lookup
│   ├── resampler.rs        # Offline and streaming polyphase resamplers
│   ├── error.rs            # Error enum with thiserror
│   ├── utils.rs            # Shared validation helpers
│   └── bin/
│       └── br41ndmg.rs     # CLI: .wav/.flac in → .wav out (`cargo install`)
│
├── tests/
│   ├── file_io.rs          # WAV normalization, write, and layout tests
│   ├── filter.rs           # FIR helper tests
│   ├── resampler.rs        # Offline interleaved resampler tests
│   ├── streaming.rs        # Chunked streaming equivalence tests
│   ├── impulse.rs          # Impulse identity and symmetry checks
│   ├── sine.rs             # Sine round-trip and passband RMS checks
│   ├── sweep.rs            # Sweep attenuation regression checks
│   ├── quality_tests.rs    # DC, alias suppression, and stereo quality checks
│   └── real_audio.rs       # End-to-end tests on test_subjects/*.flac (skip if absent)
│
├── benches/
│   └── resampler_bench.rs  # Criterion mono/stereo resampling benches
│
├── examples/
│   └── tone_resample.rs    # Synthetic sine written as WAV output
│
└── docs/
    ├── USAGE.md            # End-user guide and recipes
    ├── ARCHITECTURE.md     # Module map, data flow
    ├── DSP_NOTES.md        # Sinc math, polyphase theory
    ├── REALTIME.md         # Callback rules, latency tradeoffs
    ├── TEST_PLAN.md        # Signals, thresholds, and checked-in coverage
    ├── BENCHMARK_PLAN.md   # Current coverage and next perf work
    ├── REQUIREMENTS.md     # Scope, capabilities, non-goals
    ├── PERFORMANCE.md      # Current performance notes
    └── PROJECT_GOAL.md     # Phase-by-phase build plan
```
