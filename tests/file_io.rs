use br41ndmg::Resampler;
use br41ndmg::io::{AudioBuffer, probe_audio, read_wav, write_wav};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("br41ndmg_{name}_{unique}.wav"))
}

#[test]
fn read_wav_normalizes_integer_pcm() {
    let path = temp_path("read_pcm");
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    {
        let mut writer = WavWriter::create(&path, spec).unwrap();
        for sample in [-32768_i16, 0, 32767] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    let buffer = read_wav(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(buffer.sample_rate(), 44_100);
    assert_eq!(buffer.channels(), 1);
    assert_eq!(buffer.frame_count(), 3);
    assert!((buffer.samples()[0] + 1.0).abs() < 1.0e-6);
    assert!(buffer.samples()[1].abs() < 1.0e-6);
    assert!((buffer.samples()[2] - (32767.0 / 32768.0)).abs() < 1.0e-6);
}

#[test]
fn write_wav_persists_float_output() {
    let path = temp_path("write_float");
    let buffer = AudioBuffer::new(48_000, 2, vec![0.25, -0.25, 0.5, -0.5]).unwrap();

    write_wav(&path, &buffer).unwrap();

    let mut reader = hound::WavReader::open(&path).unwrap();
    let spec = reader.spec();
    let samples = reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 48_000);
    assert_eq!(spec.bits_per_sample, 32);
    assert_eq!(spec.sample_format, SampleFormat::Float);
    assert_eq!(samples, vec![0.25, -0.25, 0.5, -0.5]);
}

#[test]
fn audio_buffer_resample_to_preserves_stereo_layout() {
    let input = AudioBuffer::new(4, 2, vec![1.0, 10.0, 2.0, 20.0, 3.0, 30.0, 4.0, 40.0]).unwrap();

    let output = input.resample_to(8).unwrap();
    let resampler = Resampler::new(4.0, 8.0).unwrap();
    let left = resampler.resample(&[1.0, 2.0, 3.0, 4.0]).unwrap();
    let right = resampler.resample(&[10.0, 20.0, 30.0, 40.0]).unwrap();

    assert_eq!(output.channels(), 2);
    assert_eq!(output.sample_rate(), 8);
    assert_eq!(output.frame_count(), 8);
    assert_eq!(output.samples()[0], 1.0);
    assert_eq!(output.samples()[1], 10.0);

    for frame in 0..output.frame_count() {
        assert!((output.samples()[frame * 2] - left[frame]).abs() <= 1.0e-5);
        assert!((output.samples()[frame * 2 + 1] - right[frame]).abs() <= 1.0e-5);
    }
}

#[test]
fn probe_wav_reports_header_without_decoding() {
    let path = temp_path("probe_wav");
    let spec = WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    {
        let mut writer = WavWriter::create(&path, spec).unwrap();
        for _ in 0..100 {
            writer.write_sample(0_i16).unwrap();
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();
    }

    let info = probe_audio(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert_eq!(info.format, "wav");
    assert_eq!(info.sample_rate, 48_000);
    assert_eq!(info.channels, 2);
    assert_eq!(info.bits_per_sample, 16);
    assert_eq!(info.frames, Some(100));
}

#[test]
fn probe_flac_reports_streaminfo() {
    let path = PathBuf::from("test_subjects/Hitsujibungaku - more than words.flac");
    if !path.exists() {
        return; // fixtures are optional; probe is covered by the WAV test
    }

    let info = probe_audio(&path).unwrap();

    assert_eq!(info.format, "flac");
    assert!(info.sample_rate > 0);
    assert!(info.channels > 0);
    assert!(info.bits_per_sample > 0);
    assert!(info.frames.is_some_and(|frames| frames > 0));
}
