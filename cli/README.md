# br41ndmg-cli

[![Crates.io](https://img.shields.io/crates/v/br41ndmg-cli)](https://crates.io/crates/br41ndmg-cli)
[![CI](https://github.com/ultraelectronica/br41ndmg/actions/workflows/ci.yml/badge.svg)](https://github.com/ultraelectronica/br41ndmg/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/ultraelectronica/br41ndmg/blob/master/LICENSE)

Interactive terminal resampler built on the [`br41ndmg`](https://crates.io/crates/br41ndmg) polyphase sinc library. Installs a `br41ndmg` binary that resamples `.wav`/`.flac` files to a target sample rate, one at a time or in batches.

## Install

```bash
cargo install br41ndmg-cli
```

## Usage

Run with no arguments (or a directory, or `-i`/`--interactive`) to open the
interactive file browser, or pass three positional arguments for the
non-interactive path:

```bash
# interactive browser, starts in the current directory
br41ndmg

# interactive browser, starting in a given folder
br41ndmg test_subjects/

# force interactive mode explicitly
br41ndmg -i

# non-interactive: single file -> explicit output path
br41ndmg input.flac output.wav 48000

# non-interactive: single file -> directory (auto-named <stem>_<rate>Hz.wav)
br41ndmg input.flac out_dir/ 48000

# non-interactive: batch every .wav/.flac in a folder -> out_dir/
br41ndmg test_subjects/ out_dir/ 48000
```

In the browser, navigate with arrow keys (or `j`/`k`), `Enter`/`Space` to open
or toggle files, `a` to select all, `c` to clear, and `p` to proceed. On the
settings screen pick a target rate from common presets (or type a custom one)
and choose the output directory by typing or browsing for it, then start.

From a source checkout, use `cargo run --release --bin br41ndmg -- <args>` in place of `br41ndmg`.

## Features

- Interactive TUI file browser with per-file progress
- Background-thread resampling so the UI stays responsive on large files
- WAV and FLAC input; 32-bit float WAV output
- Three-argument non-interactive path for scripting and batch conversion

See the [library crate](https://crates.io/crates/br41ndmg) for the resampling
engine, and the [project README](https://github.com/ultraelectronica/br41ndmg#readme) for full documentation.

## License

MIT
