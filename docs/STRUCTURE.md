# br41ndmg

```
br41ndmg/
├── Cargo.toml              # Workspace manifest and dependencies
├── .gitignore              # Ignores: /target, *.wav, flamegraphs
├── README.md               # Overview, quick start, roadmap
├── CHANGELOG.md            # Feature, fix, and perf history
│
├── src/
│   ├── lib.rs              # Public re-exports, crate root
│   ├── sinc.rs             # Core sinc function, safe x=0 branch
│   ├── window.rs           # Hann, Blackman, Kaiser windowing
│   ├── filter.rs           # FIR kernel generation and normalization
│   ├── io.rs               # WAV read/write and AudioBuffer helpers
│   ├── polyphase.rs        # FilterBank, phase table
│   ├── resampler.rs        # Offline and streaming resamplers
│   ├── error.rs            # Error enum with thiserror
│   └── utils.rs            # GCD, rational ratio helpers
│
├── tests/
│   ├── impulse.rs          # Impulse response, symmetry tests
│   ├── sine.rs             # Tone preservation, SNR verification
│   ├── sweep.rs            # Aliasing, stopband energy checks
│   └── quality_tests.rs    # Regression tests (expanded)
│
├── benches/
│   └── resampler_bench.rs  # Criterion: throughput and latency
│
├── examples/
│   ├── resample_file.rs    # WAV in → WAV out via hound
│   └── tone_resample.rs    # Synthetic sine written as WAV output
│
└── docs/
    ├── ARCHITECTURE.md     # Module map, data flow
    ├── DSP_NOTES.md        # Sinc math, polyphase theory
    ├── TEST_PLAN.md        # Signals, thresholds, criteria
    ├── BENCHMARK_PLAN.md   # Datasets, targets, comparison
    ├── REQUIREMENTS.md     # Scope, quality targets, non-goals
    ├── PERFORMANCE.md      # Profiling results, SIMD notes
    └── REALTIME.md         # Callback rules, latency tradeoffs
```
