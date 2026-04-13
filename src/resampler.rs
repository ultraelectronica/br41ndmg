use crate::ResampleError;

#[derive(Debug, Clone)]
pub struct Resampler {
    input_rate: f64,
    output_rate: f64,
    ratio: f64,
}

impl Resampler {
    pub fn new(input_rate: f64, output_rate: f64) -> Result<Self, ResampleError> {
        if !input_rate.is_finite() || input_rate <= 0.0 {
            return Err(ResampleError::InvalidSampleRate(input_rate));
        }

        if !output_rate.is_finite() || output_rate <= 0.0 {
            return Err(ResampleError::InvalidSampleRate(output_rate));
        }

        let ratio = output_rate / input_rate;
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err(ResampleError::InvalidRatio);
        }

        Ok(Self {
            input_rate,
            output_rate,
            ratio,
        })
    }

    pub fn input_rate(&self) -> f64 {
        self.input_rate
    }

    pub fn output_rate(&self) -> f64 {
        self.output_rate
    }

    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    pub fn resample_interleaved(
        &self,
        input: &[f32],
        channels: usize,
    ) -> Result<Vec<f32>, ResampleError> {
        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(channels));
        }

        if input.is_empty() {
            return Ok(Vec::new());
        }

        if input.len() % channels != 0 {
            return Err(ResampleError::BufferError(
                "interleaved input length must be divisible by channel count".into(),
            ));
        }

        if channels == 1 {
            return self.resample(input);
        }

        let input_frames = input.len() / channels;
        let mut per_channel = vec![Vec::with_capacity(input_frames); channels];

        for frame in input.chunks_exact(channels) {
            for (channel, sample) in frame.iter().enumerate() {
                per_channel[channel].push(*sample);
            }
        }

        let mut resampled_channels = Vec::with_capacity(channels);
        for channel in per_channel {
            resampled_channels.push(self.resample(&channel)?);
        }

        let output_frames = resampled_channels[0].len();
        let mut output = Vec::with_capacity(output_frames * channels);
        for frame_index in 0..output_frames {
            for channel in &resampled_channels {
                output.push(channel[frame_index]);
            }
        }

        Ok(output)
    }

    pub fn resample(&self, input: &[f32]) -> Result<Vec<f32>, ResampleError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let output_len = ((input.len() as f64) * self.ratio).round() as usize;
        if output_len == 0 {
            return Ok(Vec::new());
        }

        let mut output = Vec::with_capacity(output_len);
        let max_index = input.len().saturating_sub(1);

        for i in 0..output_len {
            let position = (i as f64) / self.ratio;
            let index = position.floor() as usize;
            let frac = (position - (index as f64)) as f32;

            let idx = index.min(max_index);
            let next = (idx + 1).min(max_index);
            let a = input[idx];
            let b = input[next];
            output.push(a + (b - a) * frac);
        }

        Ok(output)
    }
}
