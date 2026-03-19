use std::f64::consts::PI;

const SMALL_T: f64 = 1.0e-4;

fn sinc_from_t(t: f64) -> f64 {
    let abs_t = t.abs();
    if abs_t < SMALL_T {
        let t2 = t * t;
        return 1.0 - (t2 / 6.0) + (t2 * t2 / 120.0);
    }

    t.sin() / t
}

pub fn sinc(x: f64) -> f64 {
    sinc_from_t(PI * x)
}

pub fn normalized_sinc(x: f64, cutoff: f64) -> f64 {
    2.0 * cutoff * sinc(2.0 * cutoff * x)
}

pub fn sinc_kernel(length: usize, cutoff: f64) -> Vec<f64> {
    if length == 0 {
        return Vec::new();
    }

    if length == 1 {
        return vec![1.0];
    }

    let center = (length as f64 - 1.0) * 0.5;
    let mut kernel = Vec::with_capacity(length);
    for n in 0..length {
        let x = n as f64 - center;
        kernel.push(normalized_sinc(x, cutoff));
    }
    let sum: f64 = kernel.iter().sum();
    if sum != 0.0 {
        let inv = 1.0 / sum;
        for v in &mut kernel {
            *v *= inv;
        }
    }
    kernel
}
