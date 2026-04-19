use br41ndmg::Resampler;

fn generate_sine(freq: f32, sample_rate: f32, duration: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    (0..samples)
        .map(|index| {
            let t = index as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
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

#[test]
fn low_frequency_sine_survives_round_trip() {
    let input = generate_sine(1_000.0, 44_100.0, 0.25);
    let upsampled = Resampler::new(44_100.0, 48_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();
    let round_trip = Resampler::new(48_000.0, 44_100.0)
        .unwrap()
        .resample(&upsampled)
        .unwrap();
    let trim = 128;
    let usable = input.len().min(round_trip.len()) - trim * 2;
    let input_trimmed = &input[trim..trim + usable];
    let output_trimmed = &round_trip[trim..trim + usable];

    assert!(rmse(input_trimmed, output_trimmed) <= 8.0e-3);
}

#[test]
fn passband_sine_keeps_its_rms() {
    let input = generate_sine(3_000.0, 48_000.0, 0.25);
    let output = Resampler::new(48_000.0, 44_100.0)
        .unwrap()
        .resample(&input)
        .unwrap();
    let trim = 128;
    let input_rms = rms(&input[trim..input.len() - trim]);
    let output_rms = rms(&output[trim..output.len() - trim]);

    assert!(((output_rms / input_rms) - 1.0).abs() <= 0.05);
}
