//! Real-audio integration tests driven by `test_subjects/` FLAC files.
//!
//! These tests exercise the full decode → resample → encode path on actual
//! music. `test_subjects/` is gitignored, so the tests auto-skip when the
//! fixtures are absent (e.g. on CI or after a fresh clone). Run them locally
//! with the songs present:
//!
//! ```text
//! cargo test --test real_audio
//! ```

#![cfg(feature = "flac")]

use br41ndmg::io::{read_audio, read_flac, write_wav};
use br41ndmg::{Resampler, StreamingResampler};
use std::path::{Path, PathBuf};

const TEST_SUBJECTS_DIR: &str = "test_subjects";

fn subjects() -> Vec<PathBuf> {
    let dir = Path::new(TEST_SUBJECTS_DIR);
    if !dir.is_dir() {
        return Vec::new();
    }

    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("flac"))
        .collect();
    paths.sort();
    paths
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let energy: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (energy / samples.len() as f64) as f32
}

fn max_abs(samples: &[f32]) -> f32 {
    samples.iter().copied().fold(0.0_f32, |a, b| a.max(b.abs()))
}

#[test]
fn flac_files_decode_to_stereo_44_1khz() {
    for path in subjects() {
        let buffer =
            read_flac(&path).unwrap_or_else(|err| panic!("decode {}: {err}", path.display()));
        assert_eq!(buffer.channels(), 2, "{}", path.display());
        assert_eq!(buffer.sample_rate(), 44_100, "{}", path.display());
        assert!(buffer.frame_count() > 1_000, "{}", path.display());
        assert!(
            buffer.samples().iter().all(|s| s.is_finite()),
            "{}",
            path.display()
        );
    }
}

#[test]
fn offline_upsample_matches_streaming_on_real_audio() {
    for path in subjects() {
        let input = read_audio(&path).expect("decode");
        // Cap runtime: take a bounded window from the middle of the track.
        let channels = input.channels() as usize;
        let start = input.frame_count() / 3 * channels;
        let window = 20_000 * channels; // ~0.45s of stereo audio
        let input = &input.samples()[start..start + window.min(input.samples().len() - start)];

        let resampler = Resampler::new(44_100.0, 48_000.0).unwrap();
        let offline = resampler.resample_interleaved(input, channels).unwrap();

        let mut stream = StreamingResampler::new(44_100.0, 48_000.0, channels).unwrap();
        let mut streaming = Vec::with_capacity(offline.len());

        // Feed the data through in non-uniform chunks to stress history trimming.
        let chunk_plan = [1_024_usize, 7, 2_048, 13, 1, 4_096, 511];
        let mut offset = 0;
        loop {
            for &frames in &chunk_plan {
                if offset >= input.len() {
                    break;
                }
                let take = (frames * channels).min(input.len() - offset);
                let chunk = &input[offset..offset + take];
                let frames_in = chunk.len() / channels;
                let mut out = vec![0.0_f32; stream.output_samples_for(frames_in)];
                let written = stream.process_into(chunk, &mut out).unwrap();
                streaming.extend_from_slice(&out[..written * channels]);
                offset += take;
            }
            if offset >= input.len() {
                break;
            }
        }
        let mut tail = vec![0.0_f32; stream.flush_samples()];
        let written = stream.flush_into(&mut tail).unwrap();
        streaming.extend_from_slice(&tail[..written * channels]);

        assert_eq!(
            streaming.len(),
            offline.len(),
            "length mismatch for {}",
            path.display()
        );
        let max_diff = streaming
            .iter()
            .zip(&offline)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_diff <= 1.0e-5,
            "{}: streaming vs offline max diff {max_diff}",
            path.display()
        );
    }
}

#[test]
fn downsample_round_trip_preserves_energy_band() {
    for path in subjects() {
        let input = read_audio(&path).expect("decode");
        let channels = input.channels() as usize;
        // Take a central ~0.7s window.
        let start = input.frame_count() / 2 * channels;
        let window = 30_000 * channels;
        let input = &input.samples()[start..start + window.min(input.samples().len() - start)];

        // Trim filter startup/transient edges before measuring energy.
        let trim = 4_096 * channels;

        let input_energy = rms(&input[trim..input.len() - trim]);

        let up = Resampler::new(44_100.0, 96_000.0)
            .unwrap()
            .resample_interleaved(input, channels)
            .unwrap();
        let back = Resampler::new(96_000.0, 44_100.0)
            .unwrap()
            .resample_interleaved(&up, channels)
            .unwrap();

        assert!(back.len() > trim * 2, "{}", path.display());
        let round_trip_energy = rms(&back[trim..back.len() - trim]);
        let ratio = round_trip_energy / input_energy;
        assert!(
            (ratio - 1.0).abs() <= 0.25,
            "{}: round-trip energy ratio {} outside [0.75, 1.25]",
            path.display(),
            ratio
        );
        // No clipping artifacts introduced.
        assert!(max_abs(&back) <= 1.05, "{}", path.display());
    }
}

#[test]
fn resample_then_write_wav_round_trip() {
    let path = match subjects().into_iter().next() {
        Some(p) => p,
        None => return,
    };

    let input = read_audio(&path).expect("decode");
    let channels = input.channels() as usize;
    let window = 20_000 * channels;
    let input = input.into_samples();
    let input = &input[..window.min(input.len())];

    let output = Resampler::new(44_100.0, 48_000.0)
        .unwrap()
        .resample_interleaved(input, channels)
        .unwrap();

    let out_path = std::env::temp_dir().join(format!(
        "br41ndmg_real_audio_{}.wav",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let buffer = br41ndmg::io::AudioBuffer::new(48_000, channels as u16, output.clone()).unwrap();
    write_wav(&out_path, &buffer).expect("write");
    let reloaded = read_audio(&out_path).expect("read back");
    std::fs::remove_file(&out_path).ok();

    assert_eq!(reloaded.sample_rate(), 48_000);
    assert_eq!(reloaded.channels(), channels as u16);
    assert_eq!(reloaded.frame_count(), output.len() / channels);
}
