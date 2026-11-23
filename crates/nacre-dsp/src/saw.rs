//! Band-limited saw oscillator with local 4× oversampling.
//!
//! The naive saw `2*phase - 1` has a discontinuity once per cycle. A single
//! PolyBLEP correction is applied at that wrap (the `+1 → -1` jump). No second
//! correction is applied mid-cycle: the mid-cycle correction belongs to a
//! square wave and would add an error to the saw.
//!
//! Spike S4 measured that PolyBLEP at 1× and 2× does not reach −60 dB of alias
//! at 1760 Hz. The oscillator therefore renders four samples per device sample
//! and decimates with a windowed-sinc low-pass. The oversampled stream lives in
//! a ring buffer (the 4× buffer) and the FIR coefficients are allocated by
//! [`SawOscillator::prepare`]; [`SawOscillator::process`] never allocates.
//!
//! Everything downstream of the oscillator (filter, VCA, saturation, DC block)
//! runs at the device rate.

/// Oversampling factor applied only inside the oscillator.
pub const OVERSAMPLING_FACTOR: usize = 4;

/// FIR taps per polyphase branch. Total taps are four times this value.
const TAPS_PER_PHASE: usize = 80;

/// Total length of the decimation FIR.
const FIR_TAPS: usize = OVERSAMPLING_FACTOR * TAPS_PER_PHASE;

/// Ring buffer size: the smallest power of two that holds the FIR history.
const RING_SIZE: usize = 512;

/// Ring buffer mask for the power-of-two index arithmetic.
const RING_MASK: usize = RING_SIZE - 1;

/// Output scaling that leaves room for the low-pass filter's Gibbs overshoot.
///
/// A band-limited saw overshoots its nominal ±1 range around the edge (about
/// 1.16 here). Clamping that overshoot would create a hard corner and defeat
/// the anti-aliasing, so the oscillator outputs at 0.8 and stays inside ±1.
const OUTPUT_GAIN: f32 = 0.8;

/// Kaiser window parameter for roughly −90 dB of stop-band attenuation.
const KAISER_BETA: f64 = 9.0;

const DEFAULT_SAMPLE_RATE_HZ: f32 = 48_000.0;
const DEFAULT_FREQUENCY_HZ: f32 = 440.0;
const MIN_FREQUENCY_HZ: f32 = 0.01;

/// Saw oscillator with a `f64` phase accumulator and 4× anti-aliased output.
#[derive(Clone, Debug)]
pub struct SawOscillator {
    phase: f64,
    frequency_hz: f32,
    sample_rate_hz: f32,
    max_block_frames: usize,
    decimation: Vec<f32>,
    ring: Vec<f32>,
    ring_head: usize,
    oversampled: Vec<f32>,
}

impl Default for SawOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl SawOscillator {
    /// Creates an oscillator prepared at 48 kHz and 440 Hz.
    ///
    /// All buffers are reserved here so the processing path stays allocation
    /// free.
    pub fn new() -> Self {
        let mut oscillator = Self {
            phase: 0.0,
            frequency_hz: DEFAULT_FREQUENCY_HZ,
            sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
            max_block_frames: 256,
            decimation: Vec::new(),
            ring: vec![0.0; RING_SIZE],
            ring_head: 0,
            oversampled: Vec::new(),
        };
        oscillator.rebuild_filters();
        oscillator
    }

    /// Prepares the oscillator for a sample rate, frequency and block size.
    ///
    /// Resets phase and filter state. Reallocates the decimation filter and the
    /// 4× scratch buffer; call from the control thread, never from the audio
    /// callback.
    pub fn prepare(&mut self, sample_rate_hz: f32, frequency_hz: f32, max_block_frames: usize) {
        self.sample_rate_hz = sanitize_sample_rate(sample_rate_hz, self.sample_rate_hz);
        self.frequency_hz =
            sanitize_frequency(frequency_hz, self.sample_rate_hz, self.frequency_hz);
        self.max_block_frames = max_block_frames.max(1);
        self.phase = 0.0;
        self.rebuild_filters();
    }

    /// Changes the frequency without resetting the phase.
    pub fn set_frequency(&mut self, sample_rate_hz: f32, frequency_hz: f32) {
        self.sample_rate_hz = sanitize_sample_rate(sample_rate_hz, self.sample_rate_hz);
        self.frequency_hz =
            sanitize_frequency(frequency_hz, self.sample_rate_hz, self.frequency_hz);
    }

    /// Fills a mono block with the band-limited saw.
    ///
    /// The output is finite and clamped to `[-1.0, 1.0]` at the unit boundary.
    pub fn process(&mut self, out: &mut [f32]) {
        let frequency_hz = self.frequency_hz;
        self.process_glide(out, frequency_hz);
    }

    /// Fills a mono block, gliding linearly to `end_frequency_hz`.
    ///
    /// The phase is never reset. This is used for block-rate pitch modulation
    /// so that the modulation is smooth instead of stepped.
    pub fn process_glide(&mut self, out: &mut [f32], end_frequency_hz: f32) {
        let start = self.phase_increment();
        let end_frequency_hz =
            sanitize_frequency(end_frequency_hz, self.sample_rate_hz, self.frequency_hz);
        let end = f64::from(end_frequency_hz)
            / (f64::from(self.sample_rate_hz) * OVERSAMPLING_FACTOR as f64);

        let mut offset = 0_usize;
        while offset < out.len() {
            let chunk = (out.len() - offset).min(self.max_block_frames);
            let progress_start = offset as f64 / out.len() as f64;
            let progress_end = (offset + chunk) as f64 / out.len() as f64;
            let chunk_start = start + (end - start) * progress_start;
            let chunk_end = start + (end - start) * progress_end;
            self.process_chunk(&mut out[offset..offset + chunk], chunk_start, chunk_end);
            offset += chunk;
        }

        self.frequency_hz = end_frequency_hz;
    }

    /// Resets phase and the decimation filter state.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.ring.fill(0.0);
        self.ring_head = 0;
    }

    /// Returns the current normalised phase.
    pub const fn phase(&self) -> f64 {
        self.phase
    }

    /// Overwrites the phase, for waveform switches that keep phase continuity.
    pub fn set_phase(&mut self, phase: f64) {
        self.phase = if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.0
        };
    }

    /// Returns the current frequency in hertz after sanitisation.
    pub const fn frequency_hz(&self) -> f32 {
        self.frequency_hz
    }

    fn phase_increment(&self) -> f64 {
        let increment = f64::from(self.frequency_hz)
            / (f64::from(self.sample_rate_hz) * OVERSAMPLING_FACTOR as f64);
        if increment.is_finite() && increment > 0.0 {
            increment
        } else {
            0.0
        }
    }

    fn process_chunk(&mut self, out: &mut [f32], increment_start: f64, increment_end: f64) {
        let frames = out.len();
        if frames == 0 {
            return;
        }

        let step = if frames > 1 {
            (increment_end - increment_start) / frames as f64
        } else {
            0.0
        };

        // Fill the 4× scratch buffer first, then decimate. Both buffers were
        // reserved in `prepare`.
        let mut increment = increment_start;
        for frame in 0..frames {
            for sub_sample in 0..OVERSAMPLING_FACTOR {
                let value = self.raw_sample(increment);
                self.oversampled[frame * OVERSAMPLING_FACTOR + sub_sample] = value as f32;
                self.advance_phase(increment);
            }
            increment += step;
        }

        for (index, sample) in out.iter_mut().enumerate() {
            let start = index * OVERSAMPLING_FACTOR;
            for &value in &self.oversampled[start..start + OVERSAMPLING_FACTOR] {
                self.ring[self.ring_head] = value;
                self.ring_head = (self.ring_head + 1) & RING_MASK;
            }

            let mut accumulated = 0.0_f64;
            for (tap, coefficient) in self.decimation.iter().enumerate() {
                let ring_index = (self.ring_head + RING_SIZE - 1 - tap) & RING_MASK;
                accumulated += f64::from(self.ring[ring_index]) * f64::from(*coefficient);
            }

            *sample = (sanitize(accumulated as f32) * OUTPUT_GAIN).clamp(-1.0, 1.0);
        }
    }

    /// One PolyBLEP-corrected saw sample at the current phase.
    fn raw_sample(&self, increment: f64) -> f64 {
        let phase = self.phase;
        let naive = 2.0 * phase - 1.0;

        if increment <= 0.0 {
            return naive;
        }

        let correction = if phase < increment {
            let normalized = phase / increment;
            normalized + normalized - normalized * normalized - 1.0
        } else if phase > 1.0 - increment {
            let normalized = (phase - 1.0) / increment;
            normalized * normalized + normalized + normalized + 1.0
        } else {
            0.0
        };

        naive - correction
    }

    fn advance_phase(&mut self, increment: f64) {
        self.phase += increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }

    fn rebuild_filters(&mut self) {
        self.decimation = design_decimation_filter(self.sample_rate_hz, FIR_TAPS);
        self.ring.fill(0.0);
        self.ring_head = 0;
        self.oversampled = vec![0.0; self.max_block_frames * OVERSAMPLING_FACTOR];
    }
}

fn sanitize(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate
    } else {
        0.0
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

/// Designs a Kaiser-windowed sinc low-pass for 4× to 1× decimation.
///
/// Pass band up to 0.42 × device rate, stop band from 0.5 × device rate. The
/// cutoff is normalised to the oversampled rate.
fn design_decimation_filter(sample_rate_hz: f32, taps: usize) -> Vec<f32> {
    let cutoff_normalized = 0.46 * sample_rate_hz / (sample_rate_hz * OVERSAMPLING_FACTOR as f32);
    let center = (taps - 1) as f64 / 2.0;
    let mut coefficients = vec![0.0_f64; taps];
    let mut sum = 0.0_f64;

    for (index, coefficient) in coefficients.iter_mut().enumerate() {
        let offset = index as f64 - center;
        let sinc = if offset.abs() < 1e-12 {
            2.0 * f64::from(cutoff_normalized)
        } else {
            (std::f64::consts::TAU * f64::from(cutoff_normalized) * offset).sin()
                / (std::f64::consts::PI * offset)
        };
        let ratio = offset / center;
        let window = if ratio.abs() <= 1.0 {
            bessel_i0(KAISER_BETA * (1.0 - ratio * ratio).max(0.0).sqrt()) / bessel_i0(KAISER_BETA)
        } else {
            0.0
        };

        *coefficient = sinc * window;
        sum += *coefficient;
    }

    let scale = if sum.abs() > f64::EPSILON {
        1.0 / sum
    } else {
        0.0
    };
    coefficients
        .into_iter()
        .map(|coefficient| (coefficient * scale) as f32)
        .collect()
}

/// Zeroth-order modified Bessel function of the first kind, series expansion.
fn bessel_i0(value: f64) -> f64 {
    let mut sum = 1.0_f64;
    let mut term = 1.0_f64;
    let squared = value * value / 4.0;

    for order in 1..64 {
        term *= squared / f64::from(order * order);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }

    sum
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::f64::consts::TAU;

    use super::{FIR_TAPS, SawOscillator, design_decimation_filter};
    use crate::test_fft::{find_alias_peak, measure_peak_magnitude, real_spectrum};

    const SAMPLE_RATE_HZ: f32 = 48_000.0;
    /// 1 s at 48 kHz: every test frequency lands exactly on a 1 Hz bin.
    const ANALYSIS_FRAMES: usize = 48_000;
    const WARMUP_FRAMES: usize = 9_600;

    fn cents_error(measured_hz: f32, expected_hz: f32) -> f32 {
        1_200.0 * (measured_hz / expected_hz).log2()
    }

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

    fn render_saw(frequency_hz: f32) -> Vec<f32> {
        let mut oscillator = SawOscillator::new();
        oscillator.prepare(SAMPLE_RATE_HZ, frequency_hz, 256);

        let mut total = vec![0.0_f32; WARMUP_FRAMES + ANALYSIS_FRAMES];
        let mut offset = 0;
        while offset < total.len() {
            let chunk = (total.len() - offset).min(256);
            oscillator.process(&mut total[offset..offset + chunk]);
            offset += chunk;
        }

        total.split_off(WARMUP_FRAMES)
    }

    #[test]
    fn frequency_is_within_one_cent_at_110_440_and_1760_hz() {
        for frequency_hz in [110.0, 440.0, 1760.0] {
            let samples = render_saw(frequency_hz);
            let measured = measure_frequency_hz(&samples, SAMPLE_RATE_HZ);
            let error = cents_error(measured, frequency_hz);
            assert!(
                error.abs() < 1.0,
                "{frequency_hz} Hz measured {measured} Hz ({error} cents)"
            );
        }
    }

    #[test]
    fn alias_stays_below_minus_60_db_at_110_440_and_1760_hz() {
        for frequency_hz in [110.0, 440.0, 1760.0] {
            let samples = render_saw(frequency_hz);
            let spectrum = real_spectrum(&samples);
            let fundamental_bin = (frequency_hz * ANALYSIS_FRAMES as f32 / SAMPLE_RATE_HZ) as usize;
            let fundamental = measure_peak_magnitude(&spectrum, fundamental_bin, 1);
            let (alias_bin, alias) = find_alias_peak(
                &spectrum,
                fundamental_bin,
                frequency_hz,
                SAMPLE_RATE_HZ,
                ANALYSIS_FRAMES,
                SAMPLE_RATE_HZ / 2.0,
            );
            let relative_db = 20.0 * (alias / fundamental).log10();
            println!(
                "saw alias {frequency_hz} Hz: {relative_db:.1} dB at {:.1} Hz",
                alias_bin as f32 * SAMPLE_RATE_HZ / ANALYSIS_FRAMES as f32
            );
            assert!(
                relative_db < -60.0,
                "{frequency_hz} Hz alias {relative_db:.1} dB"
            );
        }
    }

    #[test]
    fn frequency_change_keeps_phase_and_has_no_step() {
        let mut oscillator = SawOscillator::new();
        oscillator.prepare(SAMPLE_RATE_HZ, 110.0, 256);

        // 3600 frames at 110 Hz leave the phase at exactly 0.25, far from the
        // wrap, where a saw has no natural discontinuity. The decimation filter
        // is already warm after 3600 frames.
        let mut block = vec![0.0_f32; 3_600];
        oscillator.process(&mut block);
        assert!((oscillator.phase() - 0.25).abs() < 1e-9);
        let phase_before = oscillator.phase();
        let previous = block[block.len() - 1];

        oscillator.set_frequency(SAMPLE_RATE_HZ, 440.0);
        assert_eq!(oscillator.phase(), phase_before);

        let mut sample = [0.0_f32];
        oscillator.process(&mut sample);

        // Four oversampled steps at 440 Hz advance one output-frame worth of
        // phase, with no reset.
        let expected_phase = (phase_before + 440.0 / f64::from(SAMPLE_RATE_HZ)).rem_euclid(1.0);
        assert!(
            (oscillator.phase() - expected_phase).abs() < 1e-9,
            "unexpected phase after the change: {}",
            oscillator.phase()
        );

        // The filter delays the signal by a few samples, so the exact sample
        // value is not the raw saw; what matters is that the phase never jumps.
        // A reset would place the output near the start of the cycle (±1).
        let step = (sample[0] - previous).abs();
        assert!(step < 0.05, "sample discontinuity {step}");

        // And after the filter has settled, the pitch is the new one.
        let mut tail = vec![0.0_f32; ANALYSIS_FRAMES];
        oscillator.process(&mut tail);
        let measured = measure_frequency_hz(&tail, SAMPLE_RATE_HZ);
        assert!(
            cents_error(measured, 440.0).abs() < 1.0,
            "post-change pitch {measured} Hz"
        );
    }

    #[test]
    fn output_is_finite_and_bounded() {
        let samples = render_saw(1_000.0);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn decimation_filter_has_unity_dc_gain_and_deep_stop_band() {
        let coefficients = design_decimation_filter(SAMPLE_RATE_HZ, FIR_TAPS);
        let dc: f32 = coefficients.iter().sum();
        assert!((dc - 1.0).abs() < 1e-5, "DC gain {dc}");

        let response = |normalized_frequency: f64| {
            let mut real = 0.0_f64;
            let mut imaginary = 0.0_f64;
            for (index, coefficient) in coefficients.iter().enumerate() {
                let angle = TAU * normalized_frequency * index as f64;
                real += f64::from(*coefficient) * angle.cos();
                imaginary -= f64::from(*coefficient) * angle.sin();
            }
            (real * real + imaginary * imaginary).sqrt()
        };

        // Nyquist of the device rate is 0.125 of the 4× rate.
        for normalized_frequency in [0.125, 0.1386, 0.25, 0.5] {
            let magnitude = response(normalized_frequency);
            println!("FIR response at {normalized_frequency}: {magnitude:.3e}");
            assert!(
                magnitude < 1e-3,
                "stop-band response {magnitude} at {normalized_frequency}"
            );
        }
    }

    #[test]
    fn reset_returns_to_a_silent_filter_state() {
        let mut oscillator = SawOscillator::new();
        oscillator.prepare(SAMPLE_RATE_HZ, 440.0, 256);
        let mut block = vec![0.0_f32; 1_024];
        oscillator.process(&mut block);
        oscillator.reset();

        assert_eq!(oscillator.phase(), 0.0);
        assert!(oscillator.ring.iter().all(|sample| *sample == 0.0));

        let mut first = vec![0.0_f32; 1];
        oscillator.process(&mut first);
        assert!(first[0].is_finite());
        assert!(first[0].abs() <= 1.0);
    }
}
