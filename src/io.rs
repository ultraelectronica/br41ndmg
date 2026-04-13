use crate::{ResampleError, Resampler};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer {
    sample_rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new(sample_rate: u32, channels: u16, samples: Vec<f32>) -> Result<Self, ResampleError> {
        if sample_rate == 0 {
            return Err(ResampleError::InvalidSampleRate(0.0));
        }

        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(0));
        }

        if samples.len() % channels as usize != 0 {
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

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn resample_to(&self, output_rate: u32) -> Result<Self, ResampleError> {
        let resampler = Resampler::new(self.sample_rate as f64, output_rate as f64)?;
        let samples = resampler.resample_interleaved(&self.samples, self.channels as usize)?;
        Self::new(output_rate, self.channels, samples)
    }
}

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

fn integer_scale(bits_per_sample: u16) -> f32 {
    (1_u64 << (bits_per_sample - 1)) as f32
}
