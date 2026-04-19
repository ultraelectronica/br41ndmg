use br41ndmg::Resampler;

const EPSILON: f32 = 1.0e-6;

fn assert_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (index, (&a, &b)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (a - b).abs() <= EPSILON,
            "sample {index} differed: {a} vs {b}"
        );
    }
}

fn deinterleave(input: &[f32], channels: usize) -> Vec<Vec<f32>> {
    let mut per_channel = vec![Vec::with_capacity(input.len() / channels); channels];

    for frame in input.chunks_exact(channels) {
        for (channel, sample) in frame.iter().enumerate() {
            per_channel[channel].push(*sample);
        }
    }

    per_channel
}

fn reinterleave(channels: &[Vec<f32>]) -> Vec<f32> {
    let frames = channels[0].len();
    let mut output = Vec::with_capacity(frames * channels.len());

    for frame in 0..frames {
        for channel in channels {
            output.push(channel[frame]);
        }
    }

    output
}

#[test]
fn stereo_interleaved_matches_per_channel_resampling() {
    let mut input = Vec::new();
    for index in 0..64 {
        let t = index as f32;
        input.push((t * 0.21).sin() * 0.7 + (t * 0.03).cos() * 0.1);
        input.push((t * 0.17).cos() * 0.4 - (t * 0.05).sin() * 0.2);
    }

    let resampler = Resampler::new(44_100.0, 48_000.0).unwrap();
    let actual = resampler.resample_interleaved(&input, 2).unwrap();
    let expected = reinterleave(
        &deinterleave(&input, 2)
            .into_iter()
            .map(|channel| resampler.resample(&channel).unwrap())
            .collect::<Vec<_>>(),
    );

    assert_close(&actual, &expected);
}

#[test]
fn multichannel_interleaved_matches_per_channel_resampling() {
    let mut input = Vec::new();
    for index in 0..48 {
        let t = index as f32;
        input.push((t * 0.11).sin() * 0.6);
        input.push((t * 0.07).cos() * 0.5);
        input.push(((index % 9) as f32 - 4.0) * 0.1);
    }

    let resampler = Resampler::new(48_000.0, 44_100.0).unwrap();
    let actual = resampler.resample_interleaved(&input, 3).unwrap();
    let expected = reinterleave(
        &deinterleave(&input, 3)
            .into_iter()
            .map(|channel| resampler.resample(&channel).unwrap())
            .collect::<Vec<_>>(),
    );

    assert_close(&actual, &expected);
}
