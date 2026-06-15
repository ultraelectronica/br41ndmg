//! Offline audio file I/O.
//!
//! [`AudioBuffer`] is the normalized, interleaved `f32` container used across
//! the crate. [`read_wav`] / [`write_wav`] handle PCM and float WAV files, and
//! [`read_flac`] (available with the `flac` feature) decodes FLAC into the same
//! buffer type. [`read_audio`] dispatches on the file extension so callers do
//! not need to pick the decoder themselves.

use crate::{ResampleError, Resampler};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

/// Normalized, interleaved `f32` audio with sample-rate and channel metadata.
///
/// Samples are stored interleaved (frame-major) and scaled to the `[-1.0, 1.0]`
/// range regardless of the source bit depth. The buffer is cheap to clone and
/// owns its samples, which can be taken with [`AudioBuffer::into_samples`].
#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl AudioBuffer {
    /// Create a buffer from interleaved `f32` samples.
    ///
    /// Returns [`ResampleError::InvalidSampleRate`] when `sample_rate` is zero,
    /// [`ResampleError::InvalidChannelCount`] when `channels` is zero, and
    /// [`ResampleError::BufferError`] when the sample count is not a whole
    /// number of frames.
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, ResampleError> {
        if sample_rate == 0 {
            return Err(ResampleError::InvalidSampleRate(0.0));
        }

        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(0));
        }

        if !samples.len().is_multiple_of(channels as usize) {
            return Err(ResampleError::BufferError(
                "interleaved sample count must be divisible by channel count".into(),
            ));
        }

        Ok(Self {
            sample_rate,
            channels,
            samples,
        })
    }

    /// Source sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Interleaved sample slice.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Consume the buffer and return the owned interleaved samples.
    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    /// Number of frames (`samples.len() / channels`).
    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// `true` when the buffer holds no samples.
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Resample to `output_rate`, preserving the channel layout.
    pub fn resample_to(&self, output_rate: u32) -> Result<Self, ResampleError> {
        let resampler = Resampler::new(self.sample_rate as f64, output_rate as f64)?;
        let samples = resampler.resample_interleaved(&self.samples, self.channels as usize)?;
        Self::new(output_rate, self.channels, samples)
    }
}

/// Read a WAV file into a normalized [`AudioBuffer`].
///
/// Accepted inputs: 8/16/24/32-bit integer PCM and 32-bit float. Integer
/// samples are scaled to `[-1.0, 1.0]`.
pub fn read_wav<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, ResampleError> {
    let mut reader = WavReader::open(path)?;
    let spec = reader.spec();
    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        (SampleFormat::Int, 8 | 16 | 24 | 32) => {
            let scale = integer_scale(spec.bits_per_sample);
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<Vec<_>, _>>()?
        }
        (format, bits) => {
            return Err(ResampleError::UnsupportedWavFormat(format!(
                "{format:?} with {bits} bits per sample"
            )));
        }
    };

    AudioBuffer::new(spec.sample_rate, spec.channels, samples)
}

/// Write an [`AudioBuffer`] as a 32-bit float WAV file.
///
/// Samples are clamped to `[-1.0, 1.0]` before writing.
pub fn write_wav<P: AsRef<Path>>(path: P, buffer: &AudioBuffer) -> Result<(), ResampleError> {
    let spec = WavSpec {
        channels: buffer.channels,
        sample_rate: buffer.sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let mut writer = WavWriter::create(path, spec)?;
    for sample in buffer.samples() {
        writer.write_sample(sample.clamp(-1.0, 1.0))?;
    }
    writer.finalize()?;
    Ok(())
}

/// Read any supported audio file into a normalized [`AudioBuffer`].
///
/// The decoder is selected from the file extension: `.wav` uses [`read_wav`]
/// and `.flac` uses [`read_flac`] (requires the `flac` feature). Unknown
/// extensions return [`ResampleError::UnsupportedWavFormat`].
pub fn read_audio<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, ResampleError> {
    let path = path.as_ref();
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("wav") => read_wav(path),
        #[cfg(feature = "flac")]
        Some("flac") => read_flac(path),
        other => Err(ResampleError::UnsupportedWavFormat(format!(
            "no decoder for extension {other:?}; expected .wav or .flac"
        ))),
    }
}

/// Read a FLAC file into a normalized [`AudioBuffer`].
///
/// This is available with the `flac` feature (enabled by default). FLAC supports
/// 4/8/12/16/20/24/32-bit samples; all are decoded to `f32` and scaled to the
/// `[-1.0, 1.0]` range using the source bit depth.
#[cfg(feature = "flac")]
pub fn read_flac<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, ResampleError> {
    let mut reader = claxon::FlacReader::open(path)?;
    let streaminfo = reader.streaminfo();
    let bits_per_sample = streaminfo.bits_per_sample;
    let channels: u16 = u16::try_from(streaminfo.channels).map_err(|_| {
        ResampleError::UnsupportedWavFormat(format!(
            "FLAC channel count {} exceeds u16",
            streaminfo.channels
        ))
    })?;
    let sample_rate = streaminfo.sample_rate;

    if bits_per_sample == 0 {
        return Err(ResampleError::UnsupportedWavFormat(
            "FLAC stream reports 0 bits per sample".into(),
        ));
    }

    let scale = integer_scale(bits_per_sample as u16);
    let samples = reader
        .samples()
        .map(|sample| sample.map(|value: i32| value as f32 / scale))
        .collect::<Result<Vec<_>, _>>()?;

    AudioBuffer::new(sample_rate, channels, samples)
}

fn integer_scale(bits_per_sample: u16) -> f32 {
    (1_u64 << (bits_per_sample - 1)) as f32
}
