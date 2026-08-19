//! STFT spectrogram computation and terminal rendering.
//!
//! [`Spectrogram::compute`] mono-mixes an [`AudioBuffer`], runs a
//! Hann-windowed STFT (FFT size 1024, adaptive hop so the column count caps
//! at [`MAX_COLUMNS`]), and stores one dB value per (column, bin). Rendering
//! max-pools that grid into terminal cells and maps dB through a fixed color
//! LUT, so scroll and zoom never recompute the FFT.

use br41ndmg::io::AudioBuffer;
use ratatui::style::Color;
use rustfft::{FftPlanner, num_complex::Complex};

pub const FFT_SIZE: usize = 1024;
pub const BINS: usize = FFT_SIZE / 2;
pub const DB_FLOOR: f32 = -90.0;
/// Cap on stored STFT columns; the hop grows to keep huge files bounded
/// (~33 MB grid worst case).
pub const MAX_COLUMNS: usize = 16_384;

/// Magma-style 16-stop LUT; index 0 is the dB floor, index 15 is full scale.
const LUT: [(u8, u8, u8); 16] = [
    (10, 7, 32),
    (22, 11, 57),
    (40, 11, 84),
    (66, 15, 108),
    (91, 20, 113),
    (114, 25, 95),
    (136, 32, 66),
    (158, 40, 36),
    (178, 52, 23),
    (199, 72, 9),
    (216, 97, 7),
    (231, 127, 27),
    (240, 159, 66),
    (248, 192, 103),
    (253, 224, 146),
    (252, 255, 214),
];

/// Map a dB value in `[DB_FLOOR, 0]` to a LUT color.
pub fn color(db: f32) -> Color {
    let t = ((db - DB_FLOOR) / -DB_FLOOR).clamp(0.0, 1.0);
    let (r, g, b) = LUT[(t * (LUT.len() - 1) as f32).round() as usize];
    Color::Rgb(r, g, b)
}

pub struct Spectrogram {
    /// dB per (column, bin), column-major: `db[col * BINS + bin]`.
    db: Vec<f32>,
    columns: usize,
    hop: usize,
    pub sample_rate: u32,
}

impl Spectrogram {
    pub fn compute(buffer: &AudioBuffer) -> Self {
        let samples = buffer.samples();
        let channels = buffer.channels() as usize;
        let frames = buffer.frame_count();
        let base_hop = FFT_SIZE / 2;
        // ponytail: single mip level, hop scales with file length. Per-octave
        // mipmaps if zoomed-in detail on hour-long files ever matters.
        let hop = base_hop * (frames / base_hop).div_ceil(MAX_COLUMNS).max(1);
        let columns = if frames == 0 {
            1
        } else {
            (1 + frames.saturating_sub(FFT_SIZE) / hop).clamp(1, MAX_COLUMNS)
        };

        let window: Vec<f32> = (0..FFT_SIZE)
            .map(|n| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * n as f32 / FFT_SIZE as f32).cos()))
            .collect();

        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let mut buf = vec![Complex::new(0.0_f32, 0.0); FFT_SIZE];
        let mut scratch = vec![Complex::new(0.0_f32, 0.0); fft.get_inplace_scratch_len()];

        // Full-scale sine through a Hann window peaks near FFT_SIZE/4, so that
        // is the 0 dB reference.
        let ref_mag = FFT_SIZE as f32 / 4.0;
        let mut db = vec![DB_FLOOR; columns * BINS];

        for col in 0..columns {
            let start = col * hop;
            buf.fill(Complex::new(0.0, 0.0));
            for n in 0..FFT_SIZE {
                let frame = start + n;
                if frame >= frames {
                    break;
                }
                let mut sum = 0.0_f32;
                for ch in 0..channels {
                    sum += samples[frame * channels + ch];
                }
                buf[n].re = sum / channels as f32 * window[n];
            }
            fft.process_with_scratch(&mut buf, &mut scratch);
            for bin in 0..BINS {
                let mag = buf[bin].norm();
                let value = 20.0 * (mag / ref_mag).log10();
                db[col * BINS + bin] = value.max(DB_FLOOR);
            }
        }

        Spectrogram {
            db,
            columns,
            hop,
            sample_rate: buffer.sample_rate(),
        }
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Max-pooled dB over a half-open grid region; empty regions return the
    /// floor.
    pub fn max_db(&self, cols: std::ops::Range<usize>, bins: std::ops::Range<usize>) -> f32 {
        let cols = cols.start.min(self.columns)..cols.end.min(self.columns);
        let bins = bins.start.min(BINS)..bins.end.min(BINS);
        let mut best = DB_FLOOR;
        for col in cols {
            for bin in bins.clone() {
                let value = self.db[col * BINS + bin];
                if value > best {
                    best = value;
                }
            }
        }
        best
    }

    /// Time in seconds at the start of grid column `col`.
    pub fn time_at(&self, col: usize) -> f64 {
        col as f64 * self.hop as f64 / self.sample_rate as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_buffer(rate: u32, freq: f32, seconds: f32) -> AudioBuffer {
        let frames = (rate as f32 * seconds) as usize;
        let samples: Vec<f32> = (0..frames)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / rate as f32).sin())
            .collect();
        AudioBuffer::new(rate, 1, samples).unwrap()
    }

    #[test]
    fn sine_peaks_in_expected_bin_near_full_scale() {
        let rate = 44_100;
        let freq = 440.0_f32;
        let spectro = Spectrogram::compute(&sine_buffer(rate, freq, 1.0));

        let mut best_bin = 0;
        let mut best_db = f32::MIN;
        for bin in 0..BINS {
            let value = spectro.max_db(0..spectro.columns(), bin..bin + 1);
            if value > best_db {
                best_db = value;
                best_bin = bin;
            }
        }

        let expected = (freq as f64 * FFT_SIZE as f64 / rate as f64).round() as usize;
        assert!(
            (best_bin as i64 - expected as i64).abs() <= 2,
            "peak bin {best_bin}, expected ~{expected}"
        );
        assert!(
            best_db > -2.0,
            "peak {best_db} dB should be near full scale"
        );
    }

    #[test]
    fn silence_floors_everywhere() {
        let buffer = AudioBuffer::new(48_000, 1, vec![0.0; 48_000]).unwrap();
        let spectro = Spectrogram::compute(&buffer);
        assert_eq!(spectro.max_db(0..spectro.columns(), 0..BINS), DB_FLOOR);
    }

    #[test]
    fn columns_cap_on_long_input() {
        let spectro = Spectrogram::compute(&sine_buffer(44_100, 440.0, 300.0));
        assert!(spectro.columns() <= MAX_COLUMNS);
        assert!(spectro.columns() > 0);
    }
}
