//! Resonant low-pass filter using the TPT state-variable topology.
//!
//! Topology-preserving transform (Zavalishin) with `f64` state. The cutoff is
//! clamped to `[20 Hz, 0.45 × sample rate]` and `Q` to `[0.5, 20]`, so `tan`
//! never sees the singularity at Nyquist. Coefficients are interpolated across
//! each block, which keeps LFO modulation continuous without a per-sample
//! tangent call.

use std::f32::consts::FRAC_1_SQRT_2;
use std::f64::consts::PI;

/// Lowest cutoff accepted by the filter, in hertz.
pub const MIN_CUTOFF_HZ: f32 = 20.0;
/// Highest cutoff as a fraction of the sample rate.
pub const MAX_CUTOFF_FRACTION: f32 = 0.45;
/// Lowest Q accepted by the filter.
pub const MIN_Q: f32 = 0.5;
/// Highest Q accepted by the filter.
pub const MAX_Q: f32 = 20.0;

/// TPT state-variable low-pass filter with a `f64` state.
#[derive(Clone, Debug)]
pub struct Svf {
    sample_rate_hz: f32,
    cutoff_hz: f32,
    resonance: f32,
    a1: f64,
    a2: f64,
    a3: f64,
    ic1eq: f64,
    ic2eq: f64,
}

impl Svf {
    /// Creates a filter at 48 kHz, 1 kHz cutoff and Butterworth Q.
    pub fn new(sample_rate_hz: f32) -> Self {
        let mut filter = Self {
            sample_rate_hz: 48_000.0,
            cutoff_hz: 1_000.0,
            resonance: FRAC_1_SQRT_2,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1eq: 0.0,
            ic2eq: 0.0,
        };
        filter.prepare(sample_rate_hz);
        filter
    }

    /// Prepares the filter for a sample rate and clears its state.
    pub fn prepare(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = if sample_rate_hz.is_finite() && sample_rate_hz > 0.0 {
            sample_rate_hz
        } else {
            48_000.0
        };
        self.reset_state();
        self.set_params(self.cutoff_hz, self.resonance);
    }

    /// Clears the integrator state without changing coefficients.
    pub fn reset(&mut self) {
        self.reset_state();
    }

    /// Sets fixed coefficients for the next block.
    pub fn set_params(&mut self, cutoff_hz: f32, resonance: f32) {
        self.cutoff_hz = sanitize_cutoff(cutoff_hz, self.sample_rate_hz);
        self.resonance = sanitize_resonance(resonance);
        let (a1, a2, a3) = coefficients(self.cutoff_hz, self.resonance, self.sample_rate_hz);
        self.a1 = a1;
        self.a2 = a2;
        self.a3 = a3;
    }

    /// Returns the current cutoff in hertz after sanitisation.
    pub const fn cutoff_hz(&self) -> f32 {
        self.cutoff_hz
    }

    /// Returns the current resonance after sanitisation.
    pub const fn resonance(&self) -> f32 {
        self.resonance
    }

    /// Processes a block while interpolating coefficients towards `end_*`.
    ///
    /// The first sample uses the coefficients left over from the previous
    /// block, so consecutive calls are continuous. Every output is sanitised
    /// and finite.
    pub fn process(&mut self, buffer: &mut [f32], end_cutoff_hz: f32, end_resonance: f32) {
        let end_cutoff_hz = sanitize_cutoff(end_cutoff_hz, self.sample_rate_hz);
        let end_resonance = sanitize_resonance(end_resonance);
        let (end_a1, end_a2, end_a3) =
            coefficients(end_cutoff_hz, end_resonance, self.sample_rate_hz);

        let frames = buffer.len();
        let denominator = frames.saturating_sub(1).max(1) as f64;

        for (index, sample) in buffer.iter_mut().enumerate() {
            let position = index as f64 / denominator;
            let a1 = self.a1 + (end_a1 - self.a1) * position;
            let a2 = self.a2 + (end_a2 - self.a2) * position;
            let a3 = self.a3 + (end_a3 - self.a3) * position;

            let input = f64::from(sanitize(*sample));
            let v3 = input - self.ic2eq;
            let v1 = a1 * self.ic1eq + a2 * v3;
            let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
            self.ic1eq = 2.0 * v1 - self.ic1eq;
            self.ic2eq = 2.0 * v2 - self.ic2eq;

            *sample = sanitize(v2 as f32);
        }

        self.a1 = end_a1;
        self.a2 = end_a2;
        self.a3 = end_a3;
        self.cutoff_hz = end_cutoff_hz;
        self.resonance = end_resonance;
    }

    fn reset_state(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }
}

impl Default for Svf {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

fn coefficients(cutoff_hz: f32, resonance: f32, sample_rate_hz: f32) -> (f64, f64, f64) {
    let g = (PI * f64::from(cutoff_hz) / f64::from(sample_rate_hz)).tan();
    let k = 1.0 / f64::from(resonance);
    let a1 = 1.0 / (1.0 + g * (g + k));
    let a2 = g * a1;
    let a3 = g * a2;

    if a1.is_finite() && a2.is_finite() && a3.is_finite() {
        (a1, a2, a3)
    } else {
        (0.0, 0.0, 0.0)
    }
}

fn sanitize_cutoff(candidate: f32, sample_rate_hz: f32) -> f32 {
    let maximum = (sample_rate_hz * MAX_CUTOFF_FRACTION).max(MIN_CUTOFF_HZ);
    if candidate.is_finite() {
        candidate.clamp(MIN_CUTOFF_HZ, maximum)
    } else {
        MIN_CUTOFF_HZ
    }
}

fn sanitize_resonance(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate.clamp(MIN_Q, MAX_Q)
    } else {
        FRAC_1_SQRT_2
    }
}

fn sanitize(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::f64::consts::{PI, TAU};

    use super::{MIN_CUTOFF_HZ, Svf};

    const SAMPLE_RATE_HZ: f32 = 48_000.0;
    const WARMUP_FRAMES: usize = 12_000;
    const MEASURE_FRAMES: usize = 24_000;

    fn model_magnitude(frequency_hz: f32, cutoff_hz: f32, q: f32, sample_rate_hz: f32) -> f64 {
        let u = (PI * f64::from(frequency_hz) / f64::from(sample_rate_hz)).tan()
            / (PI * f64::from(cutoff_hz) / f64::from(sample_rate_hz)).tan();
        let denominator = ((1.0 - u * u).powi(2) + (u / f64::from(q)).powi(2)).sqrt();
        1.0 / denominator
    }

    fn measure_gain_db(frequency_hz: f32, cutoff_hz: f32, q: f32) -> f64 {
        let mut filter = Svf::new(SAMPLE_RATE_HZ);
        filter.set_params(cutoff_hz, q);

        let mut input = vec![0.0_f32; WARMUP_FRAMES + MEASURE_FRAMES];
        for (index, sample) in input.iter_mut().enumerate() {
            *sample = (TAU * f64::from(frequency_hz) * index as f64 / f64::from(SAMPLE_RATE_HZ))
                .sin() as f32
                * 0.5;
        }

        let mut output = input.clone();
        filter.process(&mut output, cutoff_hz, q);

        let input_rms = rms(&input[WARMUP_FRAMES..]);
        let output_rms = rms(&output[WARMUP_FRAMES..]);
        20.0 * (output_rms / input_rms).log10()
    }

    fn rms(samples: &[f32]) -> f64 {
        let sum: f64 = samples
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum();
        (sum / samples.len() as f64).sqrt()
    }

    #[test]
    fn frequency_response_matches_the_model_within_one_db() {
        let cutoff_hz = 1_000.0;
        let q = std::f32::consts::FRAC_1_SQRT_2;

        for frequency_hz in [100.0, 1_000.0, 10_000.0] {
            let measured = measure_gain_db(frequency_hz, cutoff_hz, q);
            let expected =
                20.0 * model_magnitude(frequency_hz, cutoff_hz, q, SAMPLE_RATE_HZ).log10();
            println!("{frequency_hz} Hz: measured {measured:.3} dB, model {expected:.3} dB");
            assert!(
                (measured - expected).abs() < 1.0,
                "{frequency_hz} Hz: measured {measured:.3} dB, model {expected:.3} dB"
            );
        }
    }

    #[test]
    fn cutoff_sweep_is_finite_for_low_mid_and_high_q() {
        for q in [0.5_f32, 5.0, 20.0] {
            let mut filter = Svf::new(SAMPLE_RATE_HZ);
            filter.set_params(20.0, q);

            let mut phase = 0.0_f64;
            let mut peak = 0.0_f32;
            let blocks = 2 * SAMPLE_RATE_HZ as usize / 64;
            for block in 0..blocks {
                let progress = block as f32 / blocks as f32;
                let cutoff_hz = MIN_CUTOFF_HZ * (20_000.0_f32 / MIN_CUTOFF_HZ).powf(progress);
                let mut buffer = vec![0.0_f32; 64];
                for sample in &mut buffer {
                    *sample = (phase * TAU).sin() as f32 * 0.5;
                    phase = (phase + 220.0 / f64::from(SAMPLE_RATE_HZ)).rem_euclid(1.0);
                }
                filter.process(&mut buffer, cutoff_hz, q);
                for sample in &buffer {
                    assert!(sample.is_finite(), "non-finite output at Q {q}");
                    peak = peak.max(sample.abs());
                }
            }

            assert!(peak < 50.0, "Q {q} produced an unbounded peak of {peak}");
        }
    }

    #[test]
    fn corrupt_input_does_not_propagate() {
        let mut filter = Svf::new(SAMPLE_RATE_HZ);
        let mut buffer = [f32::NAN, f32::INFINITY, -f32::INFINITY, 0.5];
        filter.process(&mut buffer, 1_000.0, 0.7);

        assert!(buffer.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn coefficients_are_interpolated_between_blocks() {
        let mut filter = Svf::new(SAMPLE_RATE_HZ);
        filter.set_params(200.0, 0.7);

        let mut buffer = vec![0.1_f32; 32];
        filter.process(&mut buffer, 8_000.0, 0.7);

        assert_eq!(filter.cutoff_hz(), 8_000.0);
        assert!(buffer.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn out_of_range_parameters_are_clamped() {
        let mut filter = Svf::new(SAMPLE_RATE_HZ);
        filter.set_params(-10.0, 100.0);
        assert_eq!(filter.cutoff_hz(), MIN_CUTOFF_HZ);
        assert_eq!(filter.resonance(), super::MAX_Q);

        filter.set_params(f32::NAN, f32::NAN);
        assert_eq!(filter.cutoff_hz(), MIN_CUTOFF_HZ);
        assert_eq!(filter.resonance(), std::f32::consts::FRAC_1_SQRT_2);
    }
}
