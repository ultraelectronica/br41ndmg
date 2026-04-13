use br41ndmg::{Resampler, StreamingResampler};

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

fn collect_stream_output(
    stream: &mut StreamingResampler,
    input: &[f32],
    chunk_frames: &[usize],
) -> Vec<f32> {
    let mut output = Vec::new();
    let channels = stream.channels();
    let mut frame_offset = 0;

    for &chunk_frames in chunk_frames {
        let start = frame_offset * channels;
        let end = start + chunk_frames * channels;
        let chunk = &input[start..end];
        let mut scratch = vec![0.0; stream.output_samples_for(chunk_frames)];
        let written_frames = stream.process_into(chunk, &mut scratch).unwrap();
        output.extend_from_slice(&scratch[..written_frames * channels]);
        frame_offset += chunk_frames;
    }

    assert_eq!(frame_offset * channels, input.len());

    let mut tail = vec![0.0; stream.flush_samples()];
    let written_frames = stream.flush_into(&mut tail).unwrap();
    output.extend_from_slice(&tail[..written_frames * channels]);

    output
}

#[test]
fn streaming_matches_offline_for_chunked_mono() {
    let input: Vec<f32> = (0..32)
        .map(|index| (((index as f32) * 0.37).sin() * 0.75) + ((index % 5) as f32 * 0.05))
        .collect();
    let offline = Resampler::new(44_100.0, 48_000.0)
        .unwrap()
        .resample(&input)
        .unwrap();

    let mut stream = StreamingResampler::new(44_100.0, 48_000.0, 1).unwrap();
    let chunked = collect_stream_output(&mut stream, &input, &[1, 4, 3, 7, 2, 5, 10]);

    assert_close(&chunked, &offline);
}

#[test]
fn streaming_matches_offline_for_chunked_stereo() {
    let mut input = Vec::new();
    for index in 0..24 {
        input.push((index as f32 * 0.2).sin() * 0.5);
        input.push((index as f32 * 0.13).cos() * 0.25);
    }

    let offline = Resampler::new(48_000.0, 44_100.0)
        .unwrap()
        .resample_interleaved(&input, 2)
        .unwrap();

    let mut stream = StreamingResampler::new(48_000.0, 44_100.0, 2).unwrap();
    let chunked = collect_stream_output(&mut stream, &input, &[2, 1, 5, 3, 4, 9]);

    assert_close(&chunked, &offline);
}

#[test]
fn streaming_rejects_processing_after_flush_without_reset() {
    let mut stream = StreamingResampler::new(44_100.0, 48_000.0, 1).unwrap();
    let input = [0.0_f32, 1.0, 0.5, -0.5];
    let mut output = vec![0.0; stream.output_samples_for(input.len())];

    let _ = stream.process_into(&input, &mut output).unwrap();

    let mut tail = vec![0.0; stream.flush_samples()];
    let _ = stream.flush_into(&mut tail).unwrap();

    let mut next_output = vec![0.0; stream.output_samples_for(2)];
    let error = stream
        .process_into(&[0.25, 0.75], &mut next_output)
        .unwrap_err();
    assert!(error.to_string().contains("cannot process after flush"));
}
