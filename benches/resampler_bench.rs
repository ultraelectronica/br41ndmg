use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_resampler(c: &mut Criterion) {
    c.bench_function("resample", |b| {
        b.iter(|| black_box(42));
    });
}

criterion_group!(benches, bench_resampler);
criterion_main!(benches);
