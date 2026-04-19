use br41ndmg::Resampler;

const EPSILON: f32 = 1.0e-6;

fn generate_impulse(length: usize, index: usize) -> Vec<f32> {
    let mut signal = vec![0.0; length];
    signal[index] = 1.0;
    signal
}

#[test]
fn ratio_one_preserves_impulse_exactly() {
    let input = generate_impulse(129, 64);
    let output = Resampler::new(48_000.0, 48_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();

    assert_eq!(output.len(), input.len());
    for (index, (&actual, &expected)) in output.iter().zip(&input).enumerate() {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "sample {index} differed: {actual} vs {expected}"
        );
    }
}

#[test]
fn upsampled_impulse_has_symmetric_main_lobe() {
    let input = generate_impulse(129, 64);
    let output = Resampler::new(48_000.0, 96_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();
    let peak = 128;

    assert!(output[peak] > 0.99);
    for offset in 1..16 {
        let left = output[peak - offset];
        let right = output[peak + offset];
        assert!(
            (left - right).abs() <= 5.0e-4,
            "offset {offset} was not symmetric: {left} vs {right}"
        );
    }
}
