use br41ndmg::Resampler;
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

fn generate_signal(frames: usize, channels: usize) -> Vec<f32> {
    let mut samples = Vec::with_capacity(frames * channels);

    for frame in 0..frames {
        let t = frame as f32;
        let base = (t * 0.013).sin() * 0.65 + (t * 0.0017).cos() * 0.35;

        for channel in 0..channels {
            let phase = channel as f32 * 0.21;
            samples.push(base + (t * 0.007 + phase).sin() * 0.1);
        }
    }

    samples
}

fn bench_resampler(c: &mut Criterion) {
    let frames = 44_100;
    let resampler = Resampler::new(44_100.0, 48_000.0).unwrap();
    let mono = generate_signal(frames, 1);
    let stereo = generate_signal(frames, 2);
    let mut group = c.benchmark_group("resample");

    group.throughput(Throughput::Elements(mono.len() as u64));
    group.bench_function("mono_44100_to_48000", |b| {
        b.iter(|| black_box(resampler.resample(black_box(&mono)).unwrap()))
    });

    group.throughput(Throughput::Elements(stereo.len() as u64));
    group.bench_function("stereo_44100_to_48000", |b| {
        b.iter(|| {
            black_box(
                resampler
                    .resample_interleaved(black_box(&stereo), 2)
                    .unwrap(),
            )
        })
    });

    group.finish();
}

criterion_group!(benches, bench_resampler);
criterion_main!(benches);
