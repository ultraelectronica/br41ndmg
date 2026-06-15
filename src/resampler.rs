use crate::ResampleError;
use crate::polyphase::{PolyphaseFilterBank, PolyphaseFilterParams};

#[cfg(target_arch = "x86")]
use core::arch::x86 as simd;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as simd;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn can_use_sse2() -> bool {
    cfg!(target_feature = "sse2") || std::is_x86_feature_detected!("sse2")
}

/// Offline polyphase sinc resampler.
///
/// Construct once and reuse for any number of buffers at the same rate pair.
/// `input_rate`/`output_rate` may be `f32`, `f64`, or integer types via the
/// `Into<f64>` bounds.
///
/// ```
/// use br41ndmg::Resampler;
///
/// let resampler = Resampler::new(44_100.0_f32, 48_000.0_f32)?;
/// let output = resampler.resample(&[0.0_f32; 512])?;
/// # Ok::<(), br41ndmg::ResampleError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Resampler {
    input_rate: f64,
    output_rate: f64,
    ratio: f64,
    filter: PolyphaseFilterBank,
}

impl Resampler {
    /// Build a resampler with the default filter parameters.
    pub fn new<I, O>(input_rate: I, output_rate: O) -> Result<Self, ResampleError>
    where
        I: Into<f64>,
        O: Into<f64>,
    {
        Self::with_filter_params(input_rate, output_rate, PolyphaseFilterParams::default())
    }

    /// Build a resampler with a custom [`PolyphaseFilterParams`].
    pub fn with_filter_params<I, O>(
        input_rate: I,
        output_rate: O,
        filter_params: PolyphaseFilterParams,
    ) -> Result<Self, ResampleError>
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
            filter: PolyphaseFilterBank::try_with_params(ratio, filter_params)?,
        })
    }

    pub fn input_rate(&self) -> f64 {
        self.input_rate
    }

    /// Target sample rate in Hz.
    pub fn output_rate(&self) -> f64 {
        self.output_rate
    }

    /// `output_rate / input_rate`.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Snapshot of the active filter parameters.
    pub fn filter_params(&self) -> PolyphaseFilterParams {
        self.filter.params()
    }

    fn output_len(&self, input_len: usize) -> usize {
        ((input_len as f64) * self.ratio).round() as usize
    }

    /// Resample a mono (`channels == 1`) interleaved buffer.
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

        if !input.len().is_multiple_of(channels) {
            return Err(ResampleError::BufferError(
                "interleaved input length must be divisible by channel count".into(),
            ));
        }

        let input_frames = input.len() / channels;
        let output_frames = self.output_len(input_frames);
        if output_frames == 0 {
            return Ok(Vec::new());
        }

        let mut output = vec![0.0; output_frames * channels];
        if channels == 1 {
            self.resample_mono_into(input, &mut output);
        } else if channels == 2 {
            self.resample_interleaved_stereo_into(input, &mut output);
        } else {
            self.resample_interleaved_scalar_into(input, channels, &mut output);
        }

        Ok(output)
    }

    /// Resample a mono buffer. Equivalent to `resample_interleaved(input, 1)`.
    pub fn resample(&self, input: &[f32]) -> Result<Vec<f32>, ResampleError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        let output_len = self.output_len(input.len());
        if output_len == 0 {
            return Ok(Vec::new());
        }

        let mut output = vec![0.0; output_len];
        self.resample_mono_into(input, &mut output);
        Ok(output)
    }

    fn phase_for_position(&self, position: f64) -> (isize, &[f32]) {
        let base = position.floor() as isize;
        let frac = position - base as f64;
        (base, self.filter.phase_for(frac))
    }

    fn resample_mono_into(&self, input: &[f32], output: &mut [f32]) {
        let left_offset = self.filter.left_offset();

        for (output_index, sample) in output.iter_mut().enumerate() {
            let position = output_index as f64 / self.ratio;
            let (base, coeffs) = self.phase_for_position(position);
            *sample = convolve_mono(input, base, left_offset, coeffs);
        }
    }

    fn resample_interleaved_scalar_into(&self, input: &[f32], channels: usize, output: &mut [f32]) {
        let input_frames = input.len() / channels;
        let output_frames = output.len() / channels;
        let left_offset = self.filter.left_offset();

        for frame_index in 0..output_frames {
            let position = frame_index as f64 / self.ratio;
            let (base, coeffs) = self.phase_for_position(position);
            let output_frame = &mut output[frame_index * channels..(frame_index + 1) * channels];
            convolve_interleaved_scalar(
                output_frame,
                input,
                channels,
                input_frames,
                base,
                left_offset,
                coeffs,
            );
        }
    }

    fn resample_interleaved_stereo_into(&self, input: &[f32], output: &mut [f32]) {
        let input_frames = input.len() / 2;
        let output_frames = output.len() / 2;
        let left_offset = self.filter.left_offset();

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        if can_use_sse2() {
            for frame_index in 0..output_frames {
                let position = frame_index as f64 / self.ratio;
                let (base, coeffs) = self.phase_for_position(position);
                let output_frame = &mut output[frame_index * 2..(frame_index + 1) * 2];

                unsafe {
                    convolve_stereo_offline_sse2(
                        output_frame,
                        input,
                        input_frames,
                        base,
                        left_offset,
                        coeffs,
                    );
                }
            }

            return;
        }

        self.resample_interleaved_scalar_into(input, 2, output);
    }
}

/// Chunked, low-latency polyphase resampler for interleaved audio.
///
/// Designed for real-time pipelines: feed variable-size input chunks via
/// [`process_into`](Self::process_into) into a reusable output buffer, then
/// drain the tail with [`flush_into`](Self::flush_into). The same polyphase
/// filter math as [`Resampler`] is used, so streaming output is bit-for-bit
/// equivalent to the offline path.
///
/// ```
/// use br41ndmg::StreamingResampler;
///
/// let mut stream = StreamingResampler::new(44_100.0_f32, 48_000.0_f32, 2)?;
/// let input = vec![0.0_f32; 256 * 2];
/// let mut output = vec![0.0; stream.output_samples_for(256)];
/// let written = stream.process_into(&input, &mut output)?;
/// # Ok::<(), br41ndmg::ResampleError>(())
/// ```
#[derive(Debug, Clone)]
pub struct StreamingResampler {
    resampler: Resampler,
    channels: usize,
    input_frames_received: usize,
    output_frames_emitted: usize,
    history_start_frame: usize,
    history: Vec<f32>,
    finished: bool,
}

impl StreamingResampler {
    /// Build a streaming resampler with default filter parameters.
    pub fn new<I, O>(input_rate: I, output_rate: O, channels: usize) -> Result<Self, ResampleError>
    where
        I: Into<f64> + Copy,
        O: Into<f64> + Copy,
    {
        Self::with_filter_params(
            input_rate,
            output_rate,
            channels,
            PolyphaseFilterParams::default(),
        )
    }

    /// Build a streaming resampler with custom filter parameters.
    pub fn with_filter_params<I, O>(
        input_rate: I,
        output_rate: O,
        channels: usize,
        filter_params: PolyphaseFilterParams,
    ) -> Result<Self, ResampleError>
    where
        I: Into<f64> + Copy,
        O: Into<f64> + Copy,
    {
        if channels == 0 {
            return Err(ResampleError::InvalidChannelCount(channels));
        }

        let resampler = Resampler::with_filter_params(input_rate, output_rate, filter_params)?;

        Ok(Self {
            resampler,
            channels,
            input_frames_received: 0,
            output_frames_emitted: 0,
            history_start_frame: 0,
            history: Vec::new(),
            finished: false,
        })
    }

    /// Input-frame position for the `output_index`-th output sample.
    ///
    /// Uses the exact same expression as the offline path
    /// (`output_index / ratio`) so streaming stays bit-for-bit identical to
    /// offline resampling regardless of how the input is chunked.
    fn position_for(&self, output_index: usize) -> f64 {
        output_index as f64 / self.resampler.ratio()
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

    pub fn filter_params(&self) -> PolyphaseFilterParams {
        self.resampler.filter_params()
    }

    /// Algorithmic latency in input frames, equal to the filter radius.
    pub fn latency_frames(&self) -> usize {
        self.resampler.filter.radius()
    }

    /// Number of output frames the next `process_into` call will emit for an
    /// input of `input_frames` frames.
    pub fn output_frames_for(&self, input_frames: usize) -> usize {
        if input_frames == 0 || self.finished {
            return 0;
        }

        let total_frames = self.input_frames_received + input_frames;
        let lookahead = self.resampler.filter.radius() as f64;
        let mut emitted = self.output_frames_emitted;

        while self.position_for(emitted) + lookahead < total_frames as f64 {
            emitted += 1;
        }

        emitted - self.output_frames_emitted
    }

    /// `output_frames_for(input_frames) * channels` — output sample count to
    /// allocate for the next `process_into` call.
    pub fn output_samples_for(&self, input_frames: usize) -> usize {
        self.output_frames_for(input_frames) * self.channels
    }

    /// Number of output frames [`flush_into`](Self::flush_into) will emit.
    pub fn flush_frames(&self) -> usize {
        if self.finished || self.input_frames_received == 0 {
            return 0;
        }

        let target_output_frames = self.resampler.output_len(self.input_frames_received);
        target_output_frames.saturating_sub(self.output_frames_emitted)
    }

    /// `flush_frames() * channels` — output sample count to allocate for the
    /// final flush.
    pub fn flush_samples(&self) -> usize {
        self.flush_frames() * self.channels
    }

    /// Resample `input` into the front of `output`.
    ///
    /// Returns the number of whole frames written. `output` must hold at least
    /// [`output_samples_for(input.len() / channels)`](Self::output_samples_for)
    /// samples.
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

        self.history.extend_from_slice(input);
        self.input_frames_received += input_frames;

        let total_frames = self.input_frames_received;
        let lookahead = self.resampler.filter.radius() as f64;
        let left_offset = self.resampler.filter.left_offset();
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

        while self.position_for(self.output_frames_emitted) + lookahead < total_frames as f64 {
            let (base, coeffs) = self
                .resampler
                .phase_for_position(self.position_for(self.output_frames_emitted));
            let start = written_frames * self.channels;
            let end = start + self.channels;
            let output_frame = &mut output[start..end];

            if use_simd {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                unsafe {
                    convolve_stereo_history_sse2(
                        output_frame,
                        &self.history,
                        self.history_start_frame,
                        total_frames,
                        base,
                        left_offset,
                        coeffs,
                        false,
                    )?;
                }
            } else {
                convolve_history_scalar(
                    output_frame,
                    &self.history,
                    self.history_start_frame,
                    total_frames,
                    self.channels,
                    base,
                    left_offset,
                    coeffs,
                    false,
                )?;
            }

            written_frames += 1;
            self.output_frames_emitted += 1;
        }

        self.trim_history();
        Ok(written_frames)
    }

    /// Emit the stream tail after the last input chunk. Marks the resampler
    /// finished; call [`reset`](Self::reset) to reuse it.
    pub fn flush_into(&mut self, output: &mut [f32]) -> Result<usize, ResampleError> {
        let output_capacity_frames = self.validate_output_buffer(output)?;
        let required_output_frames = self.flush_frames();

        if output_capacity_frames < required_output_frames {
            return Err(ResampleError::BufferError(format!(
                "output buffer is too small: need {required_output_frames} frames, have {output_capacity_frames}"
            )));
        }

        if self.input_frames_received == 0 {
            return Ok(0);
        }

        let total_frames = self.input_frames_received;
        let left_offset = self.resampler.filter.left_offset();
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

        for frame_index in 0..required_output_frames {
            let (base, coeffs) = self
                .resampler
                .phase_for_position(self.position_for(self.output_frames_emitted));
            let start = frame_index * self.channels;
            let end = start + self.channels;
            let output_frame = &mut output[start..end];

            if use_simd {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                unsafe {
                    convolve_stereo_history_sse2(
                        output_frame,
                        &self.history,
                        self.history_start_frame,
                        total_frames,
                        base,
                        left_offset,
                        coeffs,
                        true,
                    )?;
                }
            } else {
                convolve_history_scalar(
                    output_frame,
                    &self.history,
                    self.history_start_frame,
                    total_frames,
                    self.channels,
                    base,
                    left_offset,
                    coeffs,
                    true,
                )?;
            }

            self.output_frames_emitted += 1;
        }

        self.finished = true;
        Ok(required_output_frames)
    }

    /// Clear all history and counters, returning the resampler to a fresh state.
    pub fn reset(&mut self) {
        self.input_frames_received = 0;
        self.output_frames_emitted = 0;
        self.history_start_frame = 0;
        self.history.clear();
        self.finished = false;
    }

    fn validate_interleaved_buffer(&self, buffer: &[f32]) -> Result<usize, ResampleError> {
        if !buffer.len().is_multiple_of(self.channels) {
            return Err(ResampleError::BufferError(
                "interleaved input length must be divisible by channel count".into(),
            ));
        }

        Ok(buffer.len() / self.channels)
    }

    fn validate_output_buffer(&self, buffer: &[f32]) -> Result<usize, ResampleError> {
        if !buffer.len().is_multiple_of(self.channels) {
            return Err(ResampleError::BufferError(
                "output buffer length must be divisible by channel count".into(),
            ));
        }

        Ok(buffer.len() / self.channels)
    }

    fn trim_history(&mut self) {
        if self.history.is_empty() {
            return;
        }

        // Position of the next output sample to be emitted.
        let next_position = self.position_for(self.output_frames_emitted);
        let keep_from = (next_position.floor().max(0.0) as usize)
            .saturating_sub(self.resampler.filter.radius());
        if keep_from <= self.history_start_frame {
            return;
        }

        let drop_frames = keep_from - self.history_start_frame;
        let drop_samples = drop_frames * self.channels;
        self.history.drain(0..drop_samples);
        self.history_start_frame = keep_from;
    }
}

#[inline]
fn clamp_frame_index(frame_index: isize, input_frames: usize) -> usize {
    if input_frames <= 1 {
        return 0;
    }

    frame_index.clamp(0, input_frames as isize - 1) as usize
}

#[inline]
fn accumulate_frame_scalar(output: &mut [f32], input: &[f32], coeff: f32) {
    for (sample, input_sample) in output.iter_mut().zip(input) {
        *sample += *input_sample * coeff;
    }
}

fn convolve_mono(input: &[f32], base: isize, left_offset: isize, coeffs: &[f32]) -> f32 {
    let mut accum = 0.0;

    for (tap, coeff) in coeffs.iter().enumerate() {
        let input_index = clamp_frame_index(base + left_offset + tap as isize, input.len());
        accum += input[input_index] * *coeff;
    }

    accum
}

fn convolve_interleaved_scalar(
    output: &mut [f32],
    input: &[f32],
    channels: usize,
    input_frames: usize,
    base: isize,
    left_offset: isize,
    coeffs: &[f32],
) {
    output.fill(0.0);

    for (tap, coeff) in coeffs.iter().enumerate() {
        let input_frame_index = clamp_frame_index(base + left_offset + tap as isize, input_frames);
        let start = input_frame_index * channels;
        let end = start + channels;
        accumulate_frame_scalar(output, &input[start..end], *coeff);
    }
}

fn history_frame(
    history: &[f32],
    history_start_frame: usize,
    total_frames: usize,
    channels: usize,
    frame_index: isize,
    allow_future_edge: bool,
) -> Result<&[f32], ResampleError> {
    if history.is_empty() {
        return Err(ResampleError::BufferError(
            "streaming resampler has no input history".into(),
        ));
    }

    let latest_frame = total_frames.saturating_sub(1) as isize;
    let clamped = if frame_index < 0 {
        0
    } else if frame_index > latest_frame {
        if allow_future_edge {
            latest_frame
        } else {
            return Err(ResampleError::BufferError(
                "streaming resampler read beyond the current input chunk".into(),
            ));
        }
    } else {
        frame_index
    };

    if clamped < history_start_frame as isize {
        return Err(ResampleError::BufferError(
            "streaming resampler trimmed required history".into(),
        ));
    }

    let local_frame = clamped as usize - history_start_frame;
    let start = local_frame * channels;
    let end = start + channels;
    history.get(start..end).ok_or_else(|| {
        ResampleError::BufferError("streaming resampler history lookup failed".into())
    })
}

#[allow(clippy::too_many_arguments)]
fn convolve_history_scalar(
    output: &mut [f32],
    history: &[f32],
    history_start_frame: usize,
    total_frames: usize,
    channels: usize,
    base: isize,
    left_offset: isize,
    coeffs: &[f32],
    allow_future_edge: bool,
) -> Result<(), ResampleError> {
    output.fill(0.0);

    for (tap, coeff) in coeffs.iter().enumerate() {
        let frame = history_frame(
            history,
            history_start_frame,
            total_frames,
            channels,
            base + left_offset + tap as isize,
            allow_future_edge,
        )?;
        accumulate_frame_scalar(output, frame, *coeff);
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn convolve_stereo_offline_sse2(
    output: &mut [f32],
    input: &[f32],
    input_frames: usize,
    base: isize,
    left_offset: isize,
    coeffs: &[f32],
) {
    let mut accum = simd::_mm_setzero_ps();

    for (tap, coeff) in coeffs.iter().enumerate() {
        let frame_index = clamp_frame_index(base + left_offset + tap as isize, input_frames);
        let start = frame_index * 2;
        let samples = unsafe { load_stereo_frame_sse2(&input[start..start + 2]) };
        let scaled = simd::_mm_mul_ps(samples, simd::_mm_set1_ps(*coeff));
        accum = simd::_mm_add_ps(accum, scaled);
    }

    unsafe {
        store_stereo_frame_sse2(output, accum);
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(clippy::too_many_arguments)]
#[target_feature(enable = "sse2")]
unsafe fn convolve_stereo_history_sse2(
    output: &mut [f32],
    history: &[f32],
    history_start_frame: usize,
    total_frames: usize,
    base: isize,
    left_offset: isize,
    coeffs: &[f32],
    allow_future_edge: bool,
) -> Result<(), ResampleError> {
    let mut accum = simd::_mm_setzero_ps();

    for (tap, coeff) in coeffs.iter().enumerate() {
        let frame = history_frame(
            history,
            history_start_frame,
            total_frames,
            2,
            base + left_offset + tap as isize,
            allow_future_edge,
        )?;
        let samples = unsafe { load_stereo_frame_sse2(frame) };
        let scaled = simd::_mm_mul_ps(samples, simd::_mm_set1_ps(*coeff));
        accum = simd::_mm_add_ps(accum, scaled);
    }

    unsafe {
        store_stereo_frame_sse2(output, accum);
    }

    Ok(())
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn load_stereo_frame_sse2(frame: &[f32]) -> simd::__m128 {
    debug_assert!(frame.len() >= 2);
    simd::_mm_castsi128_ps(unsafe { simd::_mm_loadl_epi64(frame.as_ptr() as *const simd::__m128i) })
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn store_stereo_frame_sse2(output: &mut [f32], accum: simd::__m128) {
    debug_assert!(output.len() >= 2);
    unsafe {
        simd::_mm_storel_epi64(
            output.as_mut_ptr() as *mut simd::__m128i,
            simd::_mm_castps_si128(accum),
        );
    }
}
