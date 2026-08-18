use br41ndmg::{PolyphaseFilterParams, Resampler, StreamingResampler, Window};
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

fn bench_resample(c: &mut Criterion) {
    let frames = 44_100;
    let mono = generate_signal(frames, 1);
    let stereo = generate_signal(frames, 2);
    let resampler = Resampler::new(44_100.0, 48_000.0).unwrap();
    let heavy = Resampler::with_filter_params(
        44_100.0,
        48_000.0,
        PolyphaseFilterParams {
            phases: 512,
            taps_per_phase: 95,
            window: Window::Blackman,
        },
    )
    .unwrap();
    let down = Resampler::new(48_000.0, 44_100.0).unwrap();
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

    group.throughput(Throughput::Elements(stereo.len() as u64));
    group.bench_function("stereo_48000_to_44100", |b| {
        b.iter(|| black_box(down.resample_interleaved(black_box(&stereo), 2).unwrap()))
    });

    group.throughput(Throughput::Elements(mono.len() as u64));
    group.bench_function("mono_48000_to_44100", |b| {
        b.iter(|| black_box(down.resample(black_box(&mono)).unwrap()))
    });

    group.throughput(Throughput::Elements(stereo.len() as u64));
    group.bench_function("stereo_44100_to_48000_phases_512_taps_95", |b| {
        b.iter(|| black_box(heavy.resample_interleaved(black_box(&stereo), 2).unwrap()))
    });

    group.finish();
}

fn bench_streaming(c: &mut Criterion) {
    let frames = 44_100;
    let channels = 2;
    let stereo = generate_signal(frames, channels);
    let chunk_frames = 512;
    let mut stream = StreamingResampler::new(44_100.0, 48_000.0, channels).unwrap();
    // ponytail: worst-case per-chunk output is ceil(chunk * ratio) + phase
    // wobble; 2x chunk frames covers it without stateful sizing math
    let mut output = vec![0.0f32; chunk_frames * 2 * channels];
    let mut group = c.benchmark_group("streaming");

    group.throughput(Throughput::Elements(stereo.len() as u64));
    group.bench_function("stereo_44100_to_48000_chunks_512", |b| {
        b.iter(|| {
            for chunk in stereo.chunks(chunk_frames * channels) {
                let written = stream
                    .process_into(black_box(chunk), black_box(&mut output))
                    .unwrap();
                black_box(written);
            }
            stream.reset();
        })
    });

    group.finish();
}

fn bench_filter_setup(c: &mut Criterion) {
    let heavy = PolyphaseFilterParams {
        phases: 512,
        taps_per_phase: 95,
        window: Window::Blackman,
    };
    let mut group = c.benchmark_group("filter_setup");

    group.bench_function("default_phases_256_taps_63", |b| {
        b.iter(|| Resampler::new(black_box(44_100.0), black_box(48_000.0)).unwrap())
    });

    group.bench_function("phases_512_taps_95", |b| {
        b.iter(|| Resampler::with_filter_params(44_100.0, 48_000.0, black_box(heavy)).unwrap())
    });

    group.finish();
}

criterion_group!(benches, bench_resample, bench_streaming, bench_filter_setup);
criterion_main!(benches);
