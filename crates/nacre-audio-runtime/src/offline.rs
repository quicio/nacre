//! Deterministic offline rendering using the same oscillator as the runtime.

use nacre_application::{AudioConfig, AudioConfigError};
use nacre_dsp::SineOscillator;

use crate::sanitize::{sanitize_frequency_hz, sanitize_gain};
use crate::wav::{WavError, write_wav_f32};

const RENDER_BLOCK_FRAMES: usize = 256;

/// Renders a sine signal to an interleaved `f32` buffer without a device.
#[derive(Clone, Copy, Debug)]
pub struct OfflineRenderer {
    config: AudioConfig,
    frequency_hz: f32,
    linear_gain: f32,
}

impl OfflineRenderer {
    /// Creates an offline renderer after validating the audio configuration.
    pub fn new(
        config: AudioConfig,
        frequency_hz: f32,
        linear_gain: f32,
    ) -> Result<Self, AudioConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            frequency_hz: sanitize_frequency_hz(frequency_hz),
            linear_gain: sanitize_gain(linear_gain),
        })
    }

    /// Returns the validated configuration used by this renderer.
    pub const fn config(&self) -> AudioConfig {
        self.config
    }

    /// Renders exactly `frame_count` frames as interleaved samples.
    pub fn render_frames(&self, frame_count: u64) -> Vec<f32> {
        let frames = usize::try_from(frame_count).unwrap_or(usize::MAX);
        let channels = usize::from(self.config.channels());
        let sample_count = frames.saturating_mul(channels);

        let mut output = vec![0.0_f32; sample_count];
        let mut oscillator = SineOscillator::new();
        oscillator.prepare(self.config.sample_rate_hz() as f32, self.frequency_hz);

        let mut mono = vec![0.0_f32; RENDER_BLOCK_FRAMES];
        let mut frame_offset = 0_usize;
        while frame_offset < frames {
            let block_frames = (frames - frame_offset).min(RENDER_BLOCK_FRAMES);
            oscillator.process(&mut mono[..block_frames], 1.0);

            for (index, raw) in mono[..block_frames].iter().enumerate() {
                let value = (*raw * self.linear_gain).clamp(-1.0, 1.0);
                let sample_index = (frame_offset + index) * channels;
                for channel in 0..channels {
                    if let Some(sample) = output.get_mut(sample_index + channel) {
                        *sample = value;
                    }
                }
            }

            frame_offset += block_frames;
        }

        output
    }

    /// Renders a duration in seconds, rounding to the nearest frame.
    pub fn render_seconds(&self, seconds: f32) -> Vec<f32> {
        if !seconds.is_finite() || seconds <= 0.0 {
            return Vec::new();
        }

        let frames = (seconds * self.config.sample_rate_hz() as f32).round() as u64;
        self.render_frames(frames)
    }

    /// Renders frames and writes them to a `f32` PCM WAV file.
    pub fn write_wav(
        &self,
        path: impl AsRef<std::path::Path>,
        frame_count: u64,
    ) -> Result<(), WavError> {
        let samples = self.render_frames(frame_count);
        write_wav_f32(
            path,
            self.config.sample_rate_hz(),
            self.config.channels(),
            &samples,
        )
    }
}

/// Computes a stable FNV-1a hash over the little-endian bits of every sample.
pub fn hash_samples(samples: &[f32]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS;
    for sample in samples {
        for byte in sample.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use nacre_application::AudioConfig;

    use super::{OfflineRenderer, hash_samples};

    #[test]
    fn ten_seconds_at_48_khz_produces_the_expected_frame_count() {
        let renderer =
            OfflineRenderer::new(AudioConfig::default(), 220.0, 0.25).expect("valid config");
        let samples = renderer.render_frames(48_000 * 10);

        assert_eq!(samples.len(), 48_000 * 10 * 2);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn two_identical_renders_have_identical_samples_and_hash() {
        let renderer =
            OfflineRenderer::new(AudioConfig::default(), 220.0, 0.25).expect("valid config");
        let first = renderer.render_frames(48_000 * 10);
        let second = renderer.render_frames(48_000 * 10);

        assert_eq!(first.len(), second.len());
        for (first, second) in first.iter().zip(second.iter()) {
            assert_eq!(first.to_bits(), second.to_bits());
        }
        assert_eq!(hash_samples(&first), hash_samples(&second));
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        let error = OfflineRenderer::new(AudioConfig::new(48_000, 0, 256), 440.0, 0.5)
            .expect_err("zero channels must be rejected");
        assert!(error.to_string().contains("channel count"));
    }
}
