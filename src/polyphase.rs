use crate::ResampleError;
use crate::sinc::normalized_sinc;
use crate::window::{Window, window_value};

pub const DEFAULT_PHASES: usize = 256;
pub const DEFAULT_TAPS_PER_PHASE: usize = 63;
pub const DEFAULT_WINDOW: Window = Window::Blackman;

const DOWNSAMPLE_CUTOFF_MARGIN: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolyphaseFilterParams {
    pub phases: usize,
    pub taps_per_phase: usize,
    pub window: Window,
}

impl Default for PolyphaseFilterParams {
    fn default() -> Self {
        Self {
            phases: DEFAULT_PHASES,
            taps_per_phase: DEFAULT_TAPS_PER_PHASE,
            window: DEFAULT_WINDOW,
        }
    }
}

impl PolyphaseFilterParams {
    pub fn validate(&self) -> Result<(), ResampleError> {
        if self.phases == 0 {
            return Err(ResampleError::InvalidFilterConfig(
                "phase count must be non-zero".into(),
            ));
        }

        if self.taps_per_phase == 0 || self.taps_per_phase % 2 == 0 {
            return Err(ResampleError::InvalidFilterConfig(
                "tap count must be odd and non-zero".into(),
            ));
        }

        if let Window::Kaiser { beta } = self.window {
            if !beta.is_finite() || beta < 0.0 {
                return Err(ResampleError::InvalidFilterConfig(
                    "kaiser beta must be non-negative and finite".into(),
                ));
            }
        }

        Ok(())
    }
}

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
        Self::try_with_params(ratio, PolyphaseFilterParams::default())
            .expect("invalid polyphase filter configuration")
    }

    pub fn with_config(ratio: f64, phases: usize, taps_per_phase: usize, window: Window) -> Self {
        Self::try_with_config(ratio, phases, taps_per_phase, window)
            .expect("invalid polyphase filter configuration")
    }

    pub fn try_with_config(
        ratio: f64,
        phases: usize,
        taps_per_phase: usize,
        window: Window,
    ) -> Result<Self, ResampleError> {
        Self::try_with_params(
            ratio,
            PolyphaseFilterParams {
                phases,
                taps_per_phase,
                window,
            },
        )
    }

    pub fn try_with_params(
        ratio: f64,
        params: PolyphaseFilterParams,
    ) -> Result<Self, ResampleError> {
        if !ratio.is_finite() || ratio <= 0.0 {
            return Err(ResampleError::InvalidRatio);
        }

        params.validate()?;

        Ok(Self::build(ratio, params))
    }

    fn build(ratio: f64, params: PolyphaseFilterParams) -> Self {
        let cutoff = if ratio < 1.0 {
            0.5 * ratio * DOWNSAMPLE_CUTOFF_MARGIN
        } else {
            0.5
        };
        let radius = params.taps_per_phase / 2;
        let center = radius as f64;
        let mut coeffs = Vec::with_capacity(params.phases * params.taps_per_phase);

        for phase in 0..params.phases {
            let frac = phase as f64 / params.phases as f64;
            let mut phase_coeffs = Vec::with_capacity(params.taps_per_phase);
            let mut sum = 0.0;

            for tap in 0..params.taps_per_phase {
                let x = tap as f64 - center - frac;
                let window_t = if radius == 0 {
                    0.0
                } else {
                    (x / center).clamp(-1.0, 1.0)
                };
                let coeff = normalized_sinc(x, cutoff) * window_value(params.window, window_t);
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
            phases: params.phases,
            taps_per_phase: params.taps_per_phase,
            radius,
            cutoff,
            window: params.window,
            coeffs,
        }
    }

    pub fn params(&self) -> PolyphaseFilterParams {
        PolyphaseFilterParams {
            phases: self.phases,
            taps_per_phase: self.taps_per_phase,
            window: self.window,
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

    #[test]
    fn polyphase_params_reject_invalid_values() {
        let error = PolyphaseFilterParams {
            phases: 0,
            ..PolyphaseFilterParams::default()
        }
        .validate()
        .unwrap_err();
        assert!(error.to_string().contains("phase count"));

        let error = PolyphaseFilterParams {
            taps_per_phase: 32,
            ..PolyphaseFilterParams::default()
        }
        .validate()
        .unwrap_err();
        assert!(error.to_string().contains("tap count"));
    }
}
