use br41ndmg::Resampler;

fn deinterleave(input: &[f32], channels: usize) -> Vec<Vec<f32>> {
    let mut per_channel = vec![Vec::with_capacity(input.len() / channels); channels];

    for frame in input.chunks_exact(channels) {
        for (channel, sample) in frame.iter().enumerate() {
            per_channel[channel].push(*sample);
        }
    }

    per_channel
}

fn rms(signal: &[f32]) -> f32 {
    let energy: f32 = signal.iter().map(|sample| sample * sample).sum();
    (energy / signal.len() as f32).sqrt()
}

fn rmse(a: &[f32], b: &[f32]) -> f32 {
    let error: f32 = a
        .iter()
        .zip(b)
        .map(|(&lhs, &rhs)| {
            let diff = lhs - rhs;
            diff * diff
        })
        .sum();
    (error / a.len() as f32).sqrt()
}

fn generate_sine(freq: f32, sample_rate: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|index| {
            let t = index as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

#[test]
fn dc_signal_stays_stable_across_ratio_change() {
    let input = vec![0.25; 4_096];
    let output = Resampler::new(44_100.0, 48_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();

    for &sample in &output[128..output.len() - 128] {
        assert!((sample - 0.25).abs() <= 1.0e-3);
    }
}

#[test]
fn downsampling_suppresses_out_of_band_tone() {
    let input = generate_sine(12_000.0, 48_000.0, 12_000);
    let output = Resampler::new(48_000.0, 16_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();
    let trim = 128;
    let output_rms = rms(&output[trim..output.len() - trim]);

    assert!(output_rms <= 0.05);
}

#[test]
fn stereo_round_trip_preserves_channel_separation() {
    let left = generate_sine(800.0, 44_100.0, 4_096);
    let right = generate_sine(2_400.0, 44_100.0, 4_096);
    let right_scaled: Vec<f32> = right.iter().map(|sample| sample * 0.7).collect();
    let mut input = Vec::with_capacity(left.len() * 2);
    for (&l, &r) in left.iter().zip(&right_scaled) {
        input.push(l);
        input.push(r);
    }

    let upsampled = Resampler::new(44_100.0, 48_000.0)
        .unwrap()
        .resample_interleaved(&input, 2)
        .unwrap();
    let round_trip = Resampler::new(48_000.0, 44_100.0)
        .unwrap()
        .resample_interleaved(&upsampled, 2)
        .unwrap();
    let channels = deinterleave(&round_trip, 2);
    let trim = 128;
    let usable = left.len().min(channels[0].len()) - trim * 2;

    assert!(
        rmse(
            &left[trim..trim + usable],
            &channels[0][trim..trim + usable]
        ) <= 1.0e-2
    );
    assert!(
        rmse(
            &right_scaled[trim..trim + usable],
            &channels[1][trim..trim + usable],
        ) <= 1.0e-2
    );
}
