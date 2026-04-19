use std::f32::consts::PI as PI_F32;
use std::f64::consts::PI as PI_F64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Window {
    Hann,
    Hamming,
    Blackman,
    Kaiser { beta: f64 },
}

pub fn apply_window(window: Window, length: usize) -> Vec<f64> {
    if length == 0 {
        return Vec::new();
    }

    if length == 1 {
        return vec![1.0];
    }

    let denom = (length as f64 - 1.0) * 0.5;
    let mut values = Vec::with_capacity(length);
    for n in 0..length {
        let t = (n as f64 - denom) / denom;
        values.push(window_value(window, t));
    }
    values
}

pub fn apply_window_f32(window: Window, length: usize) -> Vec<f32> {
    if length == 0 {
        return Vec::new();
    }

    if length == 1 {
        return vec![1.0];
    }

    let denom = (length as f32 - 1.0) * 0.5;
    let mut values = Vec::with_capacity(length);
    for n in 0..length {
        let t = (n as f32 - denom) / denom;
        values.push(window_value_f32(window, t));
    }
    values
}

pub(crate) fn window_value(window: Window, t: f64) -> f64 {
    match window {
        Window::Hann => hann_value(t),
        Window::Hamming => hamming_value(t),
        Window::Blackman => blackman_value(t),
        Window::Kaiser { beta } => kaiser_value(t, beta),
    }
}

pub(crate) fn window_value_f32(window: Window, t: f32) -> f32 {
    match window {
        Window::Hann => hann_value_f32(t),
        Window::Hamming => hamming_value_f32(t),
        Window::Blackman => blackman_value_f32(t),
        Window::Kaiser { beta } => kaiser_value_f32(t, beta as f32),
    }
}

fn hann_value(t: f64) -> f64 {
    0.5 * (1.0 + (PI_F64 * t).cos())
}

fn hann_value_f32(t: f32) -> f32 {
    0.5 * (1.0 + (PI_F32 * t).cos())
}

fn hamming_value(t: f64) -> f64 {
    0.54 + 0.46 * (PI_F64 * t).cos()
}

fn hamming_value_f32(t: f32) -> f32 {
    0.54 + 0.46 * (PI_F32 * t).cos()
}

fn blackman_value(t: f64) -> f64 {
    0.42 + 0.5 * (PI_F64 * t).cos() + 0.08 * (2.0 * PI_F64 * t).cos()
}

fn blackman_value_f32(t: f32) -> f32 {
    0.42 + 0.5 * (PI_F32 * t).cos() + 0.08 * (2.0 * PI_F32 * t).cos()
}

fn kaiser_value(t: f64, beta: f64) -> f64 {
    assert!(
        beta >= 0.0 && beta.is_finite(),
        "beta must be non-negative and finite"
    );
    let arg = (1.0 - t * t).max(0.0).sqrt();
    i0(beta * arg) / i0(beta)
}

fn kaiser_value_f32(t: f32, beta: f32) -> f32 {
    assert!(
        beta >= 0.0 && beta.is_finite(),
        "beta must be non-negative and finite"
    );
    let arg = (1.0 - t * t).max(0.0).sqrt();
    i0_f32(beta * arg) / i0_f32(beta)
}

fn i0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 3.75 {
        let y = (x / 3.75).powi(2);
        1.0 + y
            * (3.5156229
                + y * (3.0899424
                    + y * (1.2067492 + y * (0.2659732 + y * (0.0360768 + y * 0.0045813)))))
    } else {
        let y = 3.75 / ax;
        (ax.exp() / ax.sqrt())
            * (0.39894228
                + y * (0.01328592
                    + y * (0.00225319
                        + y * (-0.00157565
                            + y * (0.00916281
                                + y * (-0.02057706
                                    + y * (0.02635537 + y * (-0.01647633 + y * 0.00392377))))))))
    }
}

fn i0_f32(x: f32) -> f32 {
    let ax = x.abs();
    if ax < 3.75 {
        let y = (x / 3.75).powi(2);
        1.0 + y
            * (3.5156229
                + y * (3.0899424
                    + y * (1.2067492 + y * (0.2659732 + y * (0.0360768 + y * 0.0045813)))))
    } else {
        let y = 3.75 / ax;
        (ax.exp() / ax.sqrt())
            * (0.39894228
                + y * (0.01328592
                    + y * (0.00225319
                        + y * (-0.00157565
                            + y * (0.00916281
                                + y * (-0.02057706
                                    + y * (0.02635537 + y * (-0.01647633 + y * 0.00392377))))))))
    }
}
