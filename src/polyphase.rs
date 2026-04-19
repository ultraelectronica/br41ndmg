use crate::sinc::normalized_sinc;
use crate::window::{Window, window_value};

pub const DEFAULT_PHASES: usize = 256;
pub const DEFAULT_TAPS_PER_PHASE: usize = 63;

const DOWNSAMPLE_CUTOFF_MARGIN: f64 = 0.95;

#[derive(Debug, Clone)]
pub struct PolyphaseFilterBank {
    phases: usize,
    taps_per_phase: usize,
    radius: usize,
    cutoff: f64,
    window: Window,
    coeffs: Vec<f32>,
}

impl PolyphaseFilterBank {
    pub fn new(ratio: f64) -> Self {
        Self::with_config(
            ratio,
            DEFAULT_PHASES,
            DEFAULT_TAPS_PER_PHASE,
            Window::Blackman,
        )
    }

    pub fn with_config(ratio: f64, phases: usize, taps_per_phase: usize, window: Window) -> Self {
        assert!(
            ratio.is_finite() && ratio > 0.0,
            "ratio must be positive and finite"
        );
        assert!(phases > 0, "phase count must be non-zero");
        assert!(
            taps_per_phase > 0 && taps_per_phase % 2 == 1,
            "tap count must be odd and non-zero"
        );

        let cutoff = if ratio < 1.0 {
            0.5 * ratio * DOWNSAMPLE_CUTOFF_MARGIN
        } else {
            0.5
        };
        let radius = taps_per_phase / 2;
        let center = radius as f64;
        let mut coeffs = Vec::with_capacity(phases * taps_per_phase);

        for phase in 0..phases {
            let frac = phase as f64 / phases as f64;
            let mut phase_coeffs = Vec::with_capacity(taps_per_phase);
            let mut sum = 0.0;

            for tap in 0..taps_per_phase {
                let x = tap as f64 - center - frac;
                let window_t = (x / center).clamp(-1.0, 1.0);
                let coeff = normalized_sinc(x, cutoff) * window_value(window, window_t);
                phase_coeffs.push(coeff);
                sum += coeff;
            }

            if sum.abs() > f64::EPSILON {
                let inv = 1.0 / sum;
                for coeff in &mut phase_coeffs {
                    *coeff *= inv;
                }
            }

            coeffs.extend(phase_coeffs.into_iter().map(|coeff| coeff as f32));
        }

        Self {
            phases,
            taps_per_phase,
            radius,
            cutoff,
            window,
            coeffs,
        }
    }

    pub fn phases(&self) -> usize {
        self.phases
    }

    pub fn taps_per_phase(&self) -> usize {
        self.taps_per_phase
    }

    pub fn radius(&self) -> usize {
        self.radius
    }

    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }

    pub fn window(&self) -> Window {
        self.window
    }

    pub fn left_offset(&self) -> isize {
        -(self.radius as isize)
    }

    pub fn phase_for(&self, frac: f64) -> &[f32] {
        let clamped = frac.clamp(0.0, 1.0);
        let phase = ((clamped * self.phases as f64).round() as usize).min(self.phases - 1);
        let start = phase * self.taps_per_phase;
        let end = start + self.taps_per_phase;
        &self.coeffs[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyphase_bank_builds_normalized_phases() {
        let bank = PolyphaseFilterBank::new(44_100.0 / 48_000.0);

        assert_eq!(bank.taps_per_phase(), DEFAULT_TAPS_PER_PHASE);
        assert_eq!(bank.radius(), DEFAULT_TAPS_PER_PHASE / 2);

        for phase in 0..bank.phases() {
            let coeffs =
                &bank.coeffs[phase * bank.taps_per_phase()..(phase + 1) * bank.taps_per_phase()];
            let sum: f32 = coeffs.iter().sum();
            assert!((sum - 1.0).abs() <= 1.0e-4);
        }
    }
}
