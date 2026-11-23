//! Pure digital signal processing building blocks.

use thiserror::Error;

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

#[cfg(test)]
mod tests {
    use super::{BlockSize, DspError, SampleRate, apply_gain};

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
}
