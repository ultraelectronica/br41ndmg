use br41ndmg::Resampler;

fn generate_sweep(sample_rate: f32, duration: f32, f_start: f32, f_end: f32) -> Vec<f32> {
    let samples = (sample_rate * duration) as usize;
    let k = (f_end - f_start) / duration;

    (0..samples)
        .map(|index| {
            let t = index as f32 / sample_rate;
            let phase = 2.0 * std::f32::consts::PI * (f_start * t + 0.5 * k * t * t);
            phase.sin()
        })
        .collect()
}

fn rms(signal: &[f32]) -> f32 {
    let energy: f32 = signal.iter().map(|sample| sample * sample).sum();
    (energy / signal.len() as f32).sqrt()
}

#[test]
fn downsampling_attenuates_out_of_band_sweep() {
    let in_band = generate_sweep(48_000.0, 0.25, 300.0, 6_000.0);
    let out_of_band = generate_sweep(48_000.0, 0.25, 9_000.0, 18_000.0);
    let resampler = Resampler::new(48_000.0, 16_000.0).unwrap();
    let in_band_output = resampler.resample(&in_band).unwrap();
    let out_of_band_output = resampler.resample(&out_of_band).unwrap();
    let trim = 128;
    let in_band_rms = rms(&in_band_output[trim..in_band_output.len() - trim]);
    let out_of_band_rms = rms(&out_of_band_output[trim..out_of_band_output.len() - trim]);

    assert!(out_of_band_rms < in_band_rms * 0.2);
}
