//! Soft saturation with gain compensation.
//!
//! The shaper is `tanh(drive * x) / drive`, which has unity slope at zero and
//! bounds the output by `1 / drive`. `drive` is `10^(db/20) - 1`, so 0 dB is
//! exactly transparent and 24 dB is a strong but finite saturation. A final
//! clamp keeps the output inside `[-1, 1]` even for corrupt input.
//!
//! The stage runs at the device rate. A memoryless shaper inevitably generates
//! harmonics above Nyquist when driven hard; the measured spectrum is recorded
//! in `HANDOFF.md` and the stage is not oversampled in this slice.

/// Lowest accepted drive, in decibels.
pub const MIN_DRIVE_DB: f32 = 0.0;
/// Highest accepted drive, in decibels.
pub const MAX_DRIVE_DB: f32 = 24.0;

/// Soft saturator with drive compensation.
#[derive(Clone, Debug)]
pub struct Saturation {
    drive_db: f32,
    drive: f64,
}

impl Default for Saturation {
    fn default() -> Self {
        Self::new()
    }
}

impl Saturation {
    /// Creates a saturator at 0 dB drive, which is transparent.
    pub fn new() -> Self {
        let mut saturation = Self {
            drive_db: 0.0,
            drive: 0.0,
        };
        saturation.set_drive_db(0.0);
        saturation
    }

    /// Sets the drive in decibels, clamped to `[0, 24]`.
    pub fn set_drive_db(&mut self, drive_db: f32) {
        self.drive_db = if drive_db.is_finite() {
            drive_db.clamp(MIN_DRIVE_DB, MAX_DRIVE_DB)
        } else {
            MIN_DRIVE_DB
        };
        self.drive = f64::from(10.0_f32.powf(self.drive_db / 20.0) - 1.0).max(0.0);
    }

    /// Returns the current drive in decibels.
    pub const fn drive_db(&self) -> f32 {
        self.drive_db
    }

    /// Saturates one sample.
    pub fn process_sample(&self, input: f32) -> f32 {
        if !input.is_finite() {
            return 0.0;
        }

        let output = if self.drive > 1e-9 {
            let value = f64::from(input) * self.drive;
            (value.tanh() / self.drive) as f32
        } else {
            input
        };

        if output.is_finite() {
            output.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// Saturates a block in place.
    pub fn process(&self, buffer: &mut [f32]) {
        for sample in buffer {
            *sample = self.process_sample(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::f64::consts::TAU;

    use super::{MAX_DRIVE_DB, Saturation};
    use crate::test_fft::{find_alias_peak, measure_peak_magnitude, real_spectrum};

    const SAMPLE_RATE_HZ: f32 = 48_000.0;
    const ANALYSIS_FRAMES: usize = 48_000;

    fn render_sine(frequency_hz: f32, amplitude: f32, drive_db: f32) -> Vec<f32> {
        let mut saturation = Saturation::new();
        saturation.set_drive_db(drive_db);

        (0..ANALYSIS_FRAMES)
            .map(|index| {
                let sample = (TAU * f64::from(frequency_hz) * index as f64
                    / f64::from(SAMPLE_RATE_HZ))
                .sin() as f32
                    * amplitude;
                saturation.process_sample(sample)
            })
            .collect()
    }

    #[test]
    fn zero_drive_is_transparent() {
        let saturation = Saturation::new();
        for input in [-1.0_f32, -0.25, 0.0, 0.25, 0.9] {
            assert_eq!(saturation.process_sample(input), input);
        }
    }

    #[test]
    fn output_is_bounded_for_extreme_and_corrupt_input() {
        let mut saturation = Saturation::new();
        saturation.set_drive_db(MAX_DRIVE_DB);

        for input in [-100.0_f32, -1.0, 0.0, 1.0, 100.0, f32::NAN, f32::INFINITY] {
            let output = saturation.process_sample(input);
            assert!(output.is_finite(), "input {input} produced {output}");
            assert!(output.abs() <= 1.0, "input {input} produced {output}");
        }
    }

    #[test]
    fn saturation_adds_harmonics_without_dc() {
        let samples = render_sine(440.0, 0.5, 12.0);
        let spectrum = real_spectrum(&samples);
        let fundamental = measure_peak_magnitude(&spectrum, 440, 0);
        let third_harmonic = measure_peak_magnitude(&spectrum, 1_320, 0);
        let dc = spectrum[0];

        assert!(fundamental > 0.1, "fundamental {fundamental}");
        assert!(
            third_harmonic > 1e-4,
            "third harmonic {third_harmonic} is not measurable"
        );
        assert!(dc < 1e-6, "DC {dc}");
    }

    #[test]
    fn output_never_exceeds_full_scale() {
        let samples = render_sine(440.0, 1.0, MAX_DRIVE_DB);
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn drive_is_clamped() {
        let mut saturation = Saturation::new();
        saturation.set_drive_db(-10.0);
        assert_eq!(saturation.drive_db(), 0.0);
        saturation.set_drive_db(100.0);
        assert_eq!(saturation.drive_db(), MAX_DRIVE_DB);
        saturation.set_drive_db(f32::NAN);
        assert_eq!(saturation.drive_db(), 0.0);
    }

    #[test]
    fn alias_at_maximum_drive_is_measured_and_bounded() {
        let samples = render_sine(440.0, 0.5, MAX_DRIVE_DB);
        let spectrum = real_spectrum(&samples);
        let fundamental_bin = (440.0 * ANALYSIS_FRAMES as f32 / SAMPLE_RATE_HZ) as usize;
        let fundamental = measure_peak_magnitude(&spectrum, fundamental_bin, 0);
        let (alias_bin, alias) = find_alias_peak(
            &spectrum,
            fundamental_bin,
            440.0,
            SAMPLE_RATE_HZ,
            ANALYSIS_FRAMES,
            SAMPLE_RATE_HZ / 2.0,
        );
        let relative_db = 20.0 * (alias / fundamental).log10();
        println!(
            "saturation 24 dB alias: {relative_db:.1} dB at {:.1} Hz",
            alias_bin as f32 * SAMPLE_RATE_HZ / ANALYSIS_FRAMES as f32
        );

        // `tanh` is analytic, so its harmonics decay exponentially. Even at
        // 24 dB on a half-scale sine the folded components stay far below the
        // −60 dB criterion; this stage does not need oversampling.
        assert!(relative_db < -60.0, "saturation alias {relative_db:.1} dB");
    }
}
