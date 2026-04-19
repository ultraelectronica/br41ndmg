use crate::utils::{validate_cutoff, validate_cutoff_f32};
use std::f32::consts::PI as PI_F32;
use std::f64::consts::PI as PI_F64;

const SMALL_T_F64: f64 = 1.0e-4;
const SMALL_T_F32: f32 = 1.0e-4;

fn sinc_from_t(t: f64) -> f64 {
    if t.abs() < SMALL_T_F64 {
        let t2 = t * t;
        return 1.0 - (t2 / 6.0) + (t2 * t2 / 120.0);
    }

    t.sin() / t
}

fn sinc_from_t_f32(t: f32) -> f32 {
    if t.abs() < SMALL_T_F32 {
        let t2 = t * t;
        return 1.0 - (t2 / 6.0) + (t2 * t2 / 120.0);
    }

    t.sin() / t
}

pub fn sinc(x: f64) -> f64 {
    sinc_from_t(PI_F64 * x)
}

pub fn sinc_f32(x: f32) -> f32 {
    sinc_from_t_f32(PI_F32 * x)
}

pub fn normalized_sinc(x: f64, cutoff: f64) -> f64 {
    2.0 * cutoff * sinc(2.0 * cutoff * x)
}

pub fn normalized_sinc_f32(x: f32, cutoff: f32) -> f32 {
    2.0 * cutoff * sinc_f32(2.0 * cutoff * x)
}

pub fn sinc_kernel(length: usize, cutoff: f64) -> Vec<f64> {
    if length == 0 {
        return Vec::new();
    }

    validate_cutoff(cutoff);

    if length == 1 {
        // One-tap FIR is identity.
        return vec![1.0];
    }

    // Even lengths are half-sample centered (Type II).
    let center = (length as f64 - 1.0) * 0.5;
    let mut kernel = Vec::with_capacity(length);
    for n in 0..length {
        let x = n as f64 - center;
        kernel.push(normalized_sinc(x, cutoff));
    }
    let sum: f64 = kernel.iter().sum();
    if sum.abs() > f64::EPSILON {
        let inv = 1.0 / sum;
        for v in &mut kernel {
            *v *= inv;
        }
    }
    kernel
}

pub fn sinc_kernel_f32(length: usize, cutoff: f32) -> Vec<f32> {
    if length == 0 {
        return Vec::new();
    }

    validate_cutoff_f32(cutoff);

    if length == 1 {
        return vec![1.0];
    }

    let center = (length as f32 - 1.0) * 0.5;
    let mut kernel = Vec::with_capacity(length);
    for n in 0..length {
        let x = n as f32 - center;
        kernel.push(normalized_sinc_f32(x, cutoff));
    }
    let sum: f32 = kernel.iter().sum();
    if sum.abs() > f32::EPSILON {
        let inv = 1.0 / sum;
        for v in &mut kernel {
            *v *= inv;
        }
    }
    kernel
}
