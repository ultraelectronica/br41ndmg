use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResampleError {
    #[error("Invalid sample rate: {0}")]
    InvalidSampleRate(f64),
    #[error("Invalid ratio")]
    InvalidRatio,
    #[error("Buffer error: {0}")]
    BufferError(String),
}
