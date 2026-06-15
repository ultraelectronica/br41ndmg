//! Error type for the crate.

use thiserror::Error;

/// All errors returned by `br41ndmg`.
#[derive(Error, Debug)]
pub enum ResampleError {
    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(f64),
    #[error("Invalid ratio")]
    InvalidRatio,
    #[error("Invalid filter configuration: {0}")]
    InvalidFilterConfig(String),
    #[error("Invalid channel count: {0}")]
    InvalidChannelCount(usize),
    #[error("Buffer error: {0}")]
    BufferError(String),
    #[error("Unsupported WAV format: {0}")]
    UnsupportedWavFormat(String),
    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),
    /// Decoding error from an optional codec (currently FLAC via claxon).
    #[cfg(feature = "flac")]
    #[error("FLAC error: {0}")]
    Flac(#[from] claxon::Error),
}
