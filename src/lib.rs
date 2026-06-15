//! `br41ndmg` — a polyphase sinc audio resampler for Rust.
//!
//! The crate resamples `f32` audio with a precomputed polyphase sinc filter
//! bank. It exposes offline and streaming APIs, reads/writes WAV files, and
//! (with the default `flac` feature) decodes FLAC input.
//!
//! # Quick start
//!
//! ```
//! use br41ndmg::Resampler;
//!
//! let resampler = Resampler::new(44_100.0_f32, 48_000.0_f32)?;
//! let input = vec![0.0_f32; 1024];
//! let output = resampler.resample(&input)?;
//! # Ok::<(), br41ndmg::ResampleError>(())
//! ```
//!
//! See the [`Resampler`] and [`StreamingResampler`] types for processing, and
//! [`io`] for file helpers.

pub mod error;
pub mod filter;
pub mod io;
pub mod polyphase;
pub mod resampler;
pub mod sinc;
pub mod utils;
pub mod window;

pub use error::ResampleError;
pub use io::AudioBuffer;
pub use polyphase::PolyphaseFilterParams;
pub use resampler::{Resampler, StreamingResampler};
pub use window::Window;
