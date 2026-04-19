use crate::ResampleError;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn can_use_sse2() -> bool {
    cfg!(target_feature = "sse2") || std::is_x86_feature_detected!("sse2")
}

#[derive(Debug, Clone)]
pub struct Resampler {
    input_rate: f64,
    output_rate: f64,
    ratio: f64,
}

impl Resampler {
    pub fn new<I, O>(input_rate: I, output_rate: O) -> Result<Self, ResampleError>
    where
        I: Into<f64>,
        O: Into<f64>,
    {
        let input_rate = input_rate.into();
        let output_rate = output_rate.into();

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
        let output_frames = self.output_len(input_frames);
        if output_frames == 0 {
            return Ok(Vec::new());
        }

        let mut output = vec![0.0; output_frames * channels];
        if channels == 2 {
            self.resample_interleaved_stereo_into(input, &mut output);
        } else {
            self.resample_interleaved_scalar_into(input, channels, &mut output);
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

    fn resample_interleaved_scalar_into(&self, input: &[f32], channels: usize, output: &mut [f32]) {
        let input_frames = input.len() / channels;
        let output_frames = output.len() / channels;
        let max_index = input_frames.saturating_sub(1);

        for frame_index in 0..output_frames {
            let position = (frame_index as f64) / self.ratio;
            let index = position.floor() as usize;
            let frac = (position - index as f64) as f32;

            let idx = index.min(max_index);
            let next = (idx + 1).min(max_index);
            let input_a = &input[idx * channels..(idx + 1) * channels];
            let input_b = &input[next * channels..(next + 1) * channels];
            let output_frame = &mut output[frame_index * channels..(frame_index + 1) * channels];
            interpolate_frame_scalar(output_frame, input_a, input_b, frac);
        }
    }

    fn resample_interleaved_stereo_into(&self, input: &[f32], output: &mut [f32]) {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if can_use_sse2() {
            let input_frames = input.len() / 2;
            let output_frames = output.len() / 2;
            let max_index = input_frames.saturating_sub(1);

            for frame_index in 0..output_frames {
                let position = (frame_index as f64) / self.ratio;
                let index = position.floor() as usize;
                let frac = (position - index as f64) as f32;

                let idx = index.min(max_index);
                let next = (idx + 1).min(max_index);
                let input_a = &input[idx * 2..(idx + 1) * 2];
                let input_b = &input[next * 2..(next + 1) * 2];
                let output_frame = &mut output[frame_index * 2..(frame_index + 1) * 2];

                unsafe {
                    interpolate_stereo_frame_sse2(output_frame, input_a, input_b, frac);
                }
            }

            return;
        }

        self.resample_interleaved_scalar_into(input, 2, output);
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
    pub fn new<I, O>(input_rate: I, output_rate: O, channels: usize) -> Result<Self, ResampleError>
    where
        I: Into<f64> + Copy,
        O: Into<f64> + Copy,
    {
        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(channels));
        }

        let resampler = Resampler::new(input_rate, output_rate)?;
        let input_rate = input_rate.into();
        let output_rate = output_rate.into();

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
        let use_simd = self.channels == 2 && {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            {
                can_use_sse2()
            }

            #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
            {
                false
            }
        };

        while self.next_position < total_frames as f64 - 1.0 {
            let index = self.next_position.floor() as usize;
            let frac = (self.next_position - index as f64) as f32;
            let start = written_frames * self.channels;
            let end = start + self.channels;
            let frame = &mut output[start..end];

            let input_a = self.frame_at(index, input, previous_frames_seen)?;
            let input_b = self.frame_at(index + 1, input, previous_frames_seen)?;

            if use_simd {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                unsafe {
                    interpolate_stereo_frame_sse2(frame, input_a, input_b, frac);
                }
            } else {
                interpolate_frame_scalar(frame, input_a, input_b, frac);
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

    fn frame_at<'a>(
        &'a self,
        frame_index: usize,
        input: &'a [f32],
        previous_frames_seen: usize,
    ) -> Result<&'a [f32], ResampleError> {
        if self.has_last_frame
            && previous_frames_seen > 0
            && frame_index == previous_frames_seen - 1
        {
            return Ok(&self.last_frame);
        }

        if frame_index < previous_frames_seen {
            return Err(ResampleError::BufferError(
                "streaming resampler does not retain enough history for this position".into(),
            ));
        }

        let local_frame = frame_index - previous_frames_seen;
        let start = local_frame * self.channels;
        let end = start + self.channels;
        input.get(start..end).ok_or_else(|| {
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

#[inline]
fn interpolate_frame_scalar(output: &mut [f32], input_a: &[f32], input_b: &[f32], frac: f32) {
    for ((sample, a), b) in output.iter_mut().zip(input_a).zip(input_b) {
        *sample = *a + (*b - *a) * frac;
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn interpolate_stereo_frame_sse2(
    output: &mut [f32],
    input_a: &[f32],
    input_b: &[f32],
    frac: f32,
) {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{
        __m128i, _mm_add_ps, _mm_castps_si128, _mm_castsi128_ps, _mm_loadl_epi64, _mm_mul_ps,
        _mm_set1_ps, _mm_storel_epi64, _mm_sub_ps,
    };
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{
        __m128i, _mm_add_ps, _mm_castps_si128, _mm_castsi128_ps, _mm_loadl_epi64, _mm_mul_ps,
        _mm_set1_ps, _mm_storel_epi64, _mm_sub_ps,
    };

    debug_assert!(output.len() >= 2);
    debug_assert!(input_a.len() >= 2);
    debug_assert!(input_b.len() >= 2);

    let a = _mm_castsi128_ps(unsafe { _mm_loadl_epi64(input_a.as_ptr() as *const __m128i) });
    let b = _mm_castsi128_ps(unsafe { _mm_loadl_epi64(input_b.as_ptr() as *const __m128i) });
    let delta = _mm_sub_ps(b, a);
    let result = _mm_add_ps(a, _mm_mul_ps(delta, _mm_set1_ps(frac)));

    unsafe {
        _mm_storel_epi64(
            output.as_mut_ptr() as *mut __m128i,
            _mm_castps_si128(result),
        );
    }
}
