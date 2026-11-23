//! One-pole DC blocker.
//!
//! `y[n] = x[n] - x[n-1] + R * y[n-1]` with `R = 1 - 2π·5/fs`, a corner near
//! 5 Hz. The state is `f64` so a ten-second hold does not accumulate error, and
//! non-finite input is replaced by zero before it reaches the filter.

const CUTOFF_HZ: f64 = 5.0;

/// One-pole DC blocker with `f64` state.
#[derive(Clone, Debug)]
pub struct DcBlocker {
    coefficient: f64,
    previous_input: f64,
    previous_output: f64,
}

impl DcBlocker {
    /// Creates a DC blocker for a sample rate.
    pub fn new(sample_rate_hz: f32) -> Self {
        let mut blocker = Self {
            coefficient: 0.0,
            previous_input: 0.0,
            previous_output: 0.0,
        };
        blocker.prepare(sample_rate_hz);
        blocker
    }

    /// Recomputes the pole for a sample rate and clears the state.
    pub fn prepare(&mut self, sample_rate_hz: f32) {
        let sample_rate_hz = if sample_rate_hz.is_finite() && sample_rate_hz > 0.0 {
            f64::from(sample_rate_hz)
        } else {
            48_000.0
        };
        self.coefficient =
            (1.0 - 2.0 * std::f64::consts::PI * CUTOFF_HZ / sample_rate_hz).clamp(0.0, 0.999_999);
        self.reset();
    }

    /// Clears the filter state.
    pub fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }

    /// Removes DC from a block in place.
    ///
    /// The output is sanitised but not clamped: the voice applies its headroom
    /// and final limit after this stage, so a resonant peak is not clipped here
    /// only to be scaled afterwards.
    pub fn process(&mut self, buffer: &mut [f32]) {
        for sample in buffer {
            let input = if sample.is_finite() {
                f64::from(*sample)
            } else {
                0.0
            };
            let output = input - self.previous_input + self.coefficient * self.previous_output;
            self.previous_input = input;
            self.previous_output = if output.is_finite() { output } else { 0.0 };

            *sample = if self.previous_output.is_finite() {
                self.previous_output as f32
            } else {
                0.0
            };
        }
    }

    /// Returns the pole coefficient, for tests and diagnostics.
    pub const fn coefficient(&self) -> f64 {
        self.coefficient
    }
}

impl Default for DcBlocker {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::f64::consts::TAU;

    use super::DcBlocker;

    const SAMPLE_RATE_HZ: f32 = 48_000.0;

    #[test]
    fn removes_a_dc_offset_from_an_asymmetric_saw() {
        let mut blocker = DcBlocker::new(SAMPLE_RATE_HZ);
        let mut phase = 0.0_f64;
        let mut last_second = Vec::new();

        let total_frames = SAMPLE_RATE_HZ as usize * 10;
        for index in 0..total_frames {
            let saw = (2.0 * phase - 1.0) as f32;
            let mut buffer = [saw + 0.25];
            blocker.process(&mut buffer);
            phase = (phase + 110.0 / f64::from(SAMPLE_RATE_HZ)).rem_euclid(1.0);

            if index >= total_frames - SAMPLE_RATE_HZ as usize {
                last_second.push(buffer[0]);
            }
        }

        let mean: f64 = last_second
            .iter()
            .map(|sample| f64::from(*sample))
            .sum::<f64>()
            / last_second.len() as f64;
        assert!(mean.abs() < 0.001, "DC offset {mean}");
    }

    #[test]
    fn does_not_propagate_non_finite_values() {
        let mut blocker = DcBlocker::new(SAMPLE_RATE_HZ);
        let mut buffer = [f32::NAN, f32::INFINITY, -f32::INFINITY, 0.5, -0.5];
        blocker.process(&mut buffer);

        assert!(buffer.iter().all(|sample| sample.is_finite()));
        assert!(buffer.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn attenuates_a_constant_input() {
        let mut blocker = DcBlocker::new(SAMPLE_RATE_HZ);
        let mut buffer = vec![1.0_f32; SAMPLE_RATE_HZ as usize * 2];
        blocker.process(&mut buffer);

        assert!(buffer[0].abs() > 0.0);
        assert!(buffer[buffer.len() - 1].abs() < 0.01);
    }

    #[test]
    fn pole_is_below_one() {
        let blocker = DcBlocker::new(SAMPLE_RATE_HZ);
        assert!(blocker.coefficient() < 1.0);
        assert!(blocker.coefficient() > 0.99);
    }

    #[test]
    fn sine_above_the_corner_passes_through() {
        let mut blocker = DcBlocker::new(SAMPLE_RATE_HZ);
        let mut buffer: Vec<f32> = (0..SAMPLE_RATE_HZ as usize)
            .map(|index| (TAU * 100.0 * index as f64 / f64::from(SAMPLE_RATE_HZ)).sin() as f32)
            .collect();
        blocker.process(&mut buffer);

        let peak = buffer[4_800..]
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        assert!(peak > 0.95, "100 Hz peak {peak}");
    }
}
