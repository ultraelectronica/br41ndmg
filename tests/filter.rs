use br41ndmg::filter::{FirKernel, fir_kernel};
use br41ndmg::window::Window;

const EPS: f64 = 1.0e-9;

fn assert_close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= EPS,
        "expected {a} to be within {EPS} of {b}"
    );
}

#[test]
fn fir_kernel_empty_length_is_empty() {
    let taps = fir_kernel(0, 0.45, Window::Hann);
    assert!(taps.is_empty());
}

#[test]
fn fir_kernel_length_one_normalizes_to_one() {
    let taps = fir_kernel(1, 0.45, Window::Hann);
    assert_eq!(taps.len(), 1);
    assert_close(taps[0], 1.0);
}

#[test]
fn fir_kernel_is_symmetric_odd() {
    let taps = fir_kernel(63, 0.45, Window::Hann);
    assert_eq!(taps.len(), 63);
    for i in 0..taps.len() {
        let j = taps.len() - 1 - i;
        assert_close(taps[i], taps[j]);
    }
}

#[test]
fn fir_kernel_even_length_is_valid() {
    let taps = fir_kernel(64, 0.45, Window::Hann);
    assert_eq!(taps.len(), 64);
    let sum: f64 = taps.iter().sum();
    assert_close(sum, 1.0);
}

#[test]
fn fir_kernel_is_normalized() {
    let taps = fir_kernel(64, 0.45, Window::Hamming);
    let sum: f64 = taps.iter().sum();
    assert_close(sum, 1.0);
}

#[test]
fn fir_kernel_builder_exposes_metadata() {
    let kernel = FirKernel::new(32, 0.4, Window::Blackman);
    assert_eq!(kernel.len(), 32);
    assert!(!kernel.is_empty());
    assert_close(kernel.cutoff(), 0.4);
    assert_eq!(kernel.window(), Window::Blackman);
}

#[test]
#[should_panic]
fn fir_kernel_rejects_invalid_cutoff() {
    let _ = fir_kernel(16, 0.0, Window::Hann);
}
