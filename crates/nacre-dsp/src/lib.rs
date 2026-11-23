//! Pure digital signal processing building blocks.

use std::f64::consts::TAU;

use thiserror::Error;

const DEFAULT_SAMPLE_RATE_HZ: f32 = 48_000.0;
const DEFAULT_FREQUENCY_HZ: f32 = 440.0;
const MIN_FREQUENCY_HZ: f32 = 0.1;

/// Errors produced when validating DSP configuration values.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum DspError {
    /// A sample rate of zero has no physical meaning.
    #[error("sample rate must be greater than 0 Hz")]
    ZeroSampleRate,
    /// Audio cannot be processed in empty blocks.
    #[error("block size must be greater than 0 frames")]
    ZeroBlockSize,
}

/// Sample rate measured in hertz.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleRate(u32);

impl SampleRate {
    /// Creates a non-zero sample rate.
    pub const fn new(sample_rate_hz: u32) -> Result<Self, DspError> {
        if sample_rate_hz == 0 {
            return Err(DspError::ZeroSampleRate);
        }

        Ok(Self(sample_rate_hz))
    }

    /// Returns the sample rate in hertz.
    pub const fn hz(self) -> u32 {
        self.0
    }
}

/// Audio block size measured in frames.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockSize(u32);

impl BlockSize {
    /// Creates a non-zero block size.
    pub const fn new(block_size_frames: u32) -> Result<Self, DspError> {
        if block_size_frames == 0 {
            return Err(DspError::ZeroBlockSize);
        }

        Ok(Self(block_size_frames))
    }

    /// Returns the block size in frames.
    pub const fn frames(self) -> u32 {
        self.0
    }
}

/// Multiplies every sample by a linear gain factor.
pub fn apply_gain(samples: &mut [f32], linear_gain: f32) {
    for sample in samples {
        *sample *= linear_gain;
    }
}

/// Phase-continuous sine oscillator with a `f64` phase accumulator.
///
/// The phase is normalised to `[0, 1)` and never reset when the frequency
/// changes, so changes do not introduce a click. Invalid parameters are
/// sanitised at the DSP boundary instead of producing NaN or infinity.
#[derive(Clone, Debug)]
pub struct SineOscillator {
    phase: f64,
    phase_increment: f64,
    sample_rate_hz: f32,
    frequency_hz: f32,
}

impl Default for SineOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl SineOscillator {
    /// Creates an oscillator prepared at 48 kHz and 440 Hz.
    pub fn new() -> Self {
        let mut oscillator = Self {
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            frequency_hz: DEFAULT_FREQUENCY_HZ,
        };
        oscillator.recompute_increment();
        oscillator
    }

    /// Prepares the oscillator for a sample rate and frequency, resetting phase.
    pub fn prepare(&mut self, sample_rate_hz: f32, frequency_hz: f32) {
        self.sample_rate_hz = sanitize_sample_rate(sample_rate_hz, self.sample_rate_hz);
        self.frequency_hz =
            sanitize_frequency(frequency_hz, self.sample_rate_hz, self.frequency_hz);
        self.recompute_increment();
        self.phase = 0.0;
    }

    /// Changes the frequency without resetting the current phase.
    ///
    /// Non-finite or non-positive frequencies keep the previous valid value.
    pub fn set_frequency(&mut self, sample_rate_hz: f32, frequency_hz: f32) {
        self.sample_rate_hz = sanitize_sample_rate(sample_rate_hz, self.sample_rate_hz);
        self.frequency_hz =
            sanitize_frequency(frequency_hz, self.sample_rate_hz, self.frequency_hz);
        self.recompute_increment();
    }

    /// Fills a mono block with the oscillator signal multiplied by `gain`.
    ///
    /// The output is always finite and clamped to `[-1.0, 1.0]`. A zero gain
    /// produces exact digital silence while phase keeps advancing.
    pub fn process(&mut self, out: &mut [f32], gain: f32) {
        let gain = if gain.is_finite() {
            gain.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let silent = gain <= 0.0;

        for sample in out {
            let value = if silent {
                0.0
            } else {
                let raw = (TAU * self.phase).sin() as f32 * gain;
                if raw.is_finite() {
                    raw.clamp(-1.0, 1.0)
                } else {
                    0.0
                }
            };
            *sample = value;

            self.phase += self.phase_increment;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            } else if self.phase < 0.0 {
                self.phase += 1.0;
            }
        }
    }

    /// Resets the phase to zero at the next block boundary.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Returns the current frequency in hertz after sanitisation.
    pub const fn frequency_hz(&self) -> f32 {
        self.frequency_hz
    }

    /// Returns the current sample rate in hertz after sanitisation.
    pub const fn sample_rate_hz(&self) -> f32 {
        self.sample_rate_hz
    }

    fn recompute_increment(&mut self) {
        let increment = f64::from(self.frequency_hz) / f64::from(self.sample_rate_hz);
        self.phase_increment = if increment.is_finite() && increment > 0.0 {
            increment
        } else {
            0.0
        };
    }
}

fn sanitize_sample_rate(candidate: f32, fallback: f32) -> f32 {
    if candidate.is_finite() && candidate > 0.0 {
        candidate
    } else if fallback.is_finite() && fallback > 0.0 {
        fallback
    } else {
        DEFAULT_SAMPLE_RATE_HZ
    }
}

fn sanitize_frequency(candidate: f32, sample_rate_hz: f32, fallback: f32) -> f32 {
    let nyquist_hz = sample_rate_hz * 0.499;
    let max_hz = nyquist_hz.max(MIN_FREQUENCY_HZ);

    if candidate.is_finite() && candidate > 0.0 {
        return candidate.clamp(MIN_FREQUENCY_HZ, max_hz);
    }

    if fallback.is_finite() && fallback > 0.0 {
        fallback.clamp(MIN_FREQUENCY_HZ, max_hz)
    } else {
        MIN_FREQUENCY_HZ
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::{BlockSize, DspError, SampleRate, SineOscillator, apply_gain};

    fn measure_frequency_hz(samples: &[f32], sample_rate_hz: f32) -> f32 {
        let mut crossings = 0_u32;
        let mut first = 0.0_f64;
        let mut last = 0.0_f64;

        for index in 1..samples.len() {
            let previous = samples[index - 1];
            let current = samples[index];
            if previous <= 0.0 && current > 0.0 {
                let denominator = f64::from(current - previous);
                let fraction = if denominator == 0.0 {
                    0.0
                } else {
                    -f64::from(previous) / denominator
                };
                let position = (index - 1) as f64 + fraction;
                if crossings == 0 {
                    first = position;
                }
                last = position;
                crossings += 1;
            }
        }

        if crossings < 2 {
            return 0.0;
        }

        let cycles = f64::from(crossings - 1);
        (cycles * f64::from(sample_rate_hz) / (last - first)) as f32
    }

    fn cents_error(measured_hz: f32, expected_hz: f32) -> f32 {
        1_200.0 * (measured_hz / expected_hz).log2()
    }

    #[test]
    fn unity_gain_preserves_samples() {
        let expected = [0.0_f32, 0.25, -0.5, 1.0];
        let mut samples = expected;

        apply_gain(&mut samples, 1.0);

        for (&actual, expected) in samples.iter().zip(expected) {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn rejects_zero_sample_rate() {
        assert_eq!(SampleRate::new(0), Err(DspError::ZeroSampleRate));
        assert_eq!(SampleRate::new(48_000).map(SampleRate::hz), Ok(48_000));
    }

    #[test]
    fn rejects_zero_block_size() {
        assert_eq!(BlockSize::new(0), Err(DspError::ZeroBlockSize));
        assert_eq!(BlockSize::new(256).map(BlockSize::frames), Ok(256));
    }

    #[test]
    fn reference_frequency_is_within_one_cent() {
        let sample_rate_hz = 48_000.0;
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(sample_rate_hz, 440.0);

        let mut samples = vec![0.0_f32; (sample_rate_hz as usize) * 10];
        oscillator.process(&mut samples, 1.0);

        let measured = measure_frequency_hz(&samples, sample_rate_hz);
        assert!(
            cents_error(measured, 440.0).abs() < 1.0,
            "measured {measured} Hz"
        );
    }

    #[test]
    fn low_frequency_is_within_one_cent_and_has_no_dc_offset() {
        let sample_rate_hz = 48_000.0;
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(sample_rate_hz, 20.0);

        let mut samples = vec![0.0_f32; (sample_rate_hz as usize) * 10];
        oscillator.process(&mut samples, 1.0);

        let measured = measure_frequency_hz(&samples, sample_rate_hz);
        assert!(
            cents_error(measured, 20.0).abs() < 1.0,
            "measured {measured} Hz"
        );

        let mean =
            samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / samples.len() as f64;
        assert!(mean.abs() < 1e-4, "DC offset {mean}");
    }

    #[test]
    fn frequency_change_keeps_phase_continuous() {
        let sample_rate_hz = 48_000.0;
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(sample_rate_hz, 220.0);

        let mut block = vec![0.0_f32; 1_000];
        oscillator.process(&mut block, 1.0);
        let phase_before = oscillator.phase;

        oscillator.set_frequency(sample_rate_hz, 880.0);
        let mut sample = [0.0_f32];
        oscillator.process(&mut sample, 1.0);

        let expected_increment = 880.0_f64 / sample_rate_hz as f64;
        let expected = (std::f64::consts::TAU * phase_before).sin() as f32;
        assert!(
            (sample[0] - expected).abs() < 1e-5,
            "phase was reset: expected {expected}, got {}",
            sample[0]
        );

        let expected_phase = (phase_before + expected_increment).rem_euclid(1.0);
        assert!(
            (oscillator.phase - expected_phase).abs() < 1e-9,
            "unexpected post-change phase: {}",
            oscillator.phase
        );
    }

    #[test]
    fn low_frequency_change_has_no_sample_discontinuity() {
        let sample_rate_hz = 48_000.0;
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(sample_rate_hz, 20.0);

        let mut block = vec![0.0_f32; 1_000];
        oscillator.process(&mut block, 1.0);
        let previous = block[block.len() - 1];

        oscillator.set_frequency(sample_rate_hz, 21.0);
        let mut sample = [0.0_f32];
        oscillator.process(&mut sample, 1.0);

        assert!(
            (sample[0] - previous).abs() < 0.01,
            "discontinuity {}",
            (sample[0] - previous).abs()
        );
    }

    #[test]
    fn sanitizes_non_finite_parameters_and_clamps_output() {
        let sample_rate_hz = 48_000.0;
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(sample_rate_hz, 440.0);

        let mut samples = vec![0.0_f32; 512];
        oscillator.process(&mut samples, f32::NAN);
        assert!(samples.iter().all(|sample| sample.to_bits() == 0));

        oscillator.process(&mut samples, f32::INFINITY);
        assert!(samples.iter().all(|sample| sample.to_bits() == 0));

        oscillator.process(&mut samples, 10.0);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));

        oscillator.set_frequency(sample_rate_hz, f32::NAN);
        assert_eq!(oscillator.frequency_hz(), 440.0);
        oscillator.set_frequency(sample_rate_hz, f32::INFINITY);
        assert_eq!(oscillator.frequency_hz(), 440.0);
        oscillator.set_frequency(sample_rate_hz, 0.0);
        assert_eq!(oscillator.frequency_hz(), 440.0);
    }

    #[test]
    fn zero_gain_is_exact_silence() {
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(48_000.0, 1_000.0);

        let mut samples = vec![1.0_f32; 1_024];
        oscillator.process(&mut samples, 0.0);

        assert!(samples.iter().all(|sample| sample.to_bits() == 0));
    }

    #[test]
    fn reset_returns_phase_to_zero() {
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(48_000.0, 440.0);

        let mut samples = vec![0.0_f32; 128];
        oscillator.process(&mut samples, 1.0);
        oscillator.reset();
        let mut first = [1.0_f32];
        oscillator.process(&mut first, 1.0);

        assert_eq!(first[0].to_bits(), 0.0_f32.to_bits());
    }
}
