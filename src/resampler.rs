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

    fn output_len(&self, input_len: usize) -> usize {
        ((input_len as f64) * self.ratio).round() as usize
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

        let output_len = self.output_len(input.len());
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

#[derive(Debug, Clone)]
pub struct StreamingResampler {
    resampler: Resampler,
    channels: usize,
    step: f64,
    next_position: f64,
    input_frames_seen: usize,
    output_frames_emitted: usize,
    last_frame: Vec<f32>,
    has_last_frame: bool,
    finished: bool,
}

impl StreamingResampler {
    pub fn new(input_rate: f64, output_rate: f64, channels: usize) -> Result<Self, ResampleError> {
        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(channels));
        }

        let resampler = Resampler::new(input_rate, output_rate)?;

        Ok(Self {
            resampler,
            channels,
            step: input_rate / output_rate,
            next_position: 0.0,
            input_frames_seen: 0,
            output_frames_emitted: 0,
            last_frame: vec![0.0; channels],
            has_last_frame: false,
            finished: false,
        })
    }

    pub fn input_rate(&self) -> f64 {
        self.resampler.input_rate()
    }

    pub fn output_rate(&self) -> f64 {
        self.resampler.output_rate()
    }

    pub fn ratio(&self) -> f64 {
        self.resampler.ratio()
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn latency_frames(&self) -> usize {
        1
    }

    pub fn output_frames_for(&self, input_frames: usize) -> usize {
        if input_frames == 0 || self.finished {
            return 0;
        }

        let total_frames = self.input_frames_seen + input_frames;
        let mut next_position = self.next_position;
        let mut output_frames = 0;

        while next_position < total_frames as f64 - 1.0 {
            output_frames += 1;
            next_position += self.step;
        }

        output_frames
    }

    pub fn output_samples_for(&self, input_frames: usize) -> usize {
        self.output_frames_for(input_frames) * self.channels
    }

    pub fn flush_frames(&self) -> usize {
        if self.finished || !self.has_last_frame {
            return 0;
        }

        let target_output_frames = self.resampler.output_len(self.input_frames_seen);
        target_output_frames.saturating_sub(self.output_frames_emitted)
    }

    pub fn flush_samples(&self) -> usize {
        self.flush_frames() * self.channels
    }

    pub fn process_into(
        &mut self,
        input: &[f32],
        output: &mut [f32],
    ) -> Result<usize, ResampleError> {
        if self.finished {
            return Err(ResampleError::BufferError(
                "cannot process after flush; call reset before reusing the streaming resampler"
                    .into(),
            ));
        }

        let input_frames = self.validate_interleaved_buffer(input)?;
        let output_capacity_frames = self.validate_output_buffer(output)?;
        let required_output_frames = self.output_frames_for(input_frames);

        if output_capacity_frames < required_output_frames {
            return Err(ResampleError::BufferError(format!(
                "output buffer is too small: need {required_output_frames} frames, have {output_capacity_frames}"
            )));
        }

        if input_frames == 0 {
            return Ok(0);
        }

        let previous_frames_seen = self.input_frames_seen;
        let total_frames = previous_frames_seen + input_frames;
        let mut written_frames = 0;

        while self.next_position < total_frames as f64 - 1.0 {
            let index = self.next_position.floor() as usize;
            let frac = (self.next_position - index as f64) as f32;
            let start = written_frames * self.channels;
            let end = start + self.channels;
            let frame = &mut output[start..end];

            for (channel, sample) in frame.iter_mut().enumerate() {
                let a = self.sample_at(index, channel, input, previous_frames_seen)?;
                let b = self.sample_at(index + 1, channel, input, previous_frames_seen)?;
                *sample = a + (b - a) * frac;
            }

            written_frames += 1;
            self.output_frames_emitted += 1;
            self.next_position += self.step;
        }

        self.store_last_frame(input);
        self.has_last_frame = true;
        self.input_frames_seen = total_frames;

        Ok(written_frames)
    }

    pub fn flush_into(&mut self, output: &mut [f32]) -> Result<usize, ResampleError> {
        let output_capacity_frames = self.validate_output_buffer(output)?;
        let required_output_frames = self.flush_frames();

        if output_capacity_frames < required_output_frames {
            return Err(ResampleError::BufferError(format!(
                "output buffer is too small: need {required_output_frames} frames, have {output_capacity_frames}"
            )));
        }

        if !self.has_last_frame {
            return Ok(0);
        }

        for frame_index in 0..required_output_frames {
            let start = frame_index * self.channels;
            let end = start + self.channels;
            output[start..end].copy_from_slice(&self.last_frame);
        }

        self.output_frames_emitted += required_output_frames;
        self.next_position += self.step * required_output_frames as f64;
        self.finished = true;

        Ok(required_output_frames)
    }

    pub fn reset(&mut self) {
        self.next_position = 0.0;
        self.input_frames_seen = 0;
        self.output_frames_emitted = 0;
        self.last_frame.fill(0.0);
        self.has_last_frame = false;
        self.finished = false;
    }

    fn validate_interleaved_buffer(&self, buffer: &[f32]) -> Result<usize, ResampleError> {
        if buffer.len() % self.channels != 0 {
            return Err(ResampleError::BufferError(
                "interleaved input length must be divisible by channel count".into(),
            ));
        }

        Ok(buffer.len() / self.channels)
    }

    fn validate_output_buffer(&self, buffer: &[f32]) -> Result<usize, ResampleError> {
        if buffer.len() % self.channels != 0 {
            return Err(ResampleError::BufferError(
                "output buffer length must be divisible by channel count".into(),
            ));
        }

        Ok(buffer.len() / self.channels)
    }

    fn sample_at(
        &self,
        frame_index: usize,
        channel: usize,
        input: &[f32],
        previous_frames_seen: usize,
    ) -> Result<f32, ResampleError> {
        if self.has_last_frame
            && previous_frames_seen > 0
            && frame_index == previous_frames_seen - 1
        {
            return Ok(self.last_frame[channel]);
        }

        if frame_index < previous_frames_seen {
            return Err(ResampleError::BufferError(
                "streaming resampler does not retain enough history for this position".into(),
            ));
        }

        let local_frame = frame_index - previous_frames_seen;
        let offset = local_frame * self.channels + channel;
        input.get(offset).copied().ok_or_else(|| {
            ResampleError::BufferError(
                "streaming resampler read beyond the current input chunk".into(),
            )
        })
    }

    fn store_last_frame(&mut self, input: &[f32]) {
        let start = input.len() - self.channels;
        self.last_frame.copy_from_slice(&input[start..]);
    }
}
