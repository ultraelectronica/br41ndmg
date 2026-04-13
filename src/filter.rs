use crate::sinc::normalized_sinc;
use crate::utils::validate_cutoff;
use crate::window::{Window, apply_window};

#[derive(Debug, Clone)]
pub struct FirKernel {
    taps: Vec<f64>,
    cutoff: f64,
    window: Window,
}

impl FirKernel {
    pub fn new(length: usize, cutoff: f64, window: Window) -> Self {
        let taps = fir_kernel(length, cutoff, window);
        Self {
            taps,
            cutoff,
            window,
        }
    }

    pub fn taps(&self) -> &[f64] {
        &self.taps
    }

    pub fn len(&self) -> usize {
        self.taps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.taps.is_empty()
    }

    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }

    pub fn window(&self) -> Window {
        self.window
    }
}

pub fn fir_kernel(length: usize, cutoff: f64, window: Window) -> Vec<f64> {
    if length == 0 {
        return Vec::new();
    }

    validate_cutoff(cutoff);

    let mut taps = windowed_sinc(length, cutoff, window);
    normalize_kernel(&mut taps);
    taps
}

fn normalize_kernel(kernel: &mut [f64]) {
    if kernel.is_empty() {
        return;
    }

    let sum: f64 = kernel.iter().sum();
    if sum.abs() <= f64::EPSILON {
        return;
    }

    let inv = 1.0 / sum;
    for value in kernel {
        *value *= inv;
    }
}

fn windowed_sinc(length: usize, cutoff: f64, window: Window) -> Vec<f64> {
    let window_values = apply_window(window, length);
    let center = (length as f64 - 1.0) * 0.5;

    let mut taps = Vec::with_capacity(length);
    for (n, window_value) in window_values.iter().enumerate() {
        let x = n as f64 - center;
        let sample = normalized_sinc(x, cutoff);
        taps.push(sample * window_value);
    }

    taps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_kernel_scales_sum_to_one() {
        let mut taps = vec![1.0, 1.0, 2.0];
        normalize_kernel(&mut taps);
        let sum: f64 = taps.iter().sum();
        assert!((sum - 1.0).abs() <= 1e-9);
    }
}
