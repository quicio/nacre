//! Audio runtime configuration, isolated from device access.

use nacre_dsp::{BlockSize, DspError, SampleRate};
use thiserror::Error;

/// Errors produced by invalid audio configurations.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AudioConfigError {
    /// At least one audio channel is required.
    #[error("audio configuration must have at least one channel")]
    ZeroChannels,
    /// A DSP configuration value is invalid.
    #[error(transparent)]
    Dsp(#[from] DspError),
}

/// Device-independent audio configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioConfig {
    sample_rate_hz: u32,
    channels: u16,
    buffer_size_frames: u32,
}

impl AudioConfig {
    /// Creates a configuration whose values can be checked with [`Self::validate`].
    pub const fn new(sample_rate_hz: u32, channels: u16, buffer_size_frames: u32) -> Self {
        Self {
            sample_rate_hz,
            channels,
            buffer_size_frames,
        }
    }

    /// Returns the sample rate in hertz.
    pub const fn sample_rate_hz(self) -> u32 {
        self.sample_rate_hz
    }

    /// Returns the number of audio channels.
    pub const fn channels(self) -> u16 {
        self.channels
    }

    /// Returns the buffer size in frames.
    pub const fn buffer_size_frames(self) -> u32 {
        self.buffer_size_frames
    }

    /// Checks the sample rate, channel count and buffer size.
    pub fn validate(self) -> Result<(), AudioConfigError> {
        SampleRate::new(self.sample_rate_hz)?;

        if self.channels == 0 {
            return Err(AudioConfigError::ZeroChannels);
        }

        BlockSize::new(self.buffer_size_frames)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nacre_dsp::DspError;

    use super::{AudioConfig, AudioConfigError};

    #[test]
    fn accepts_standard_audio_configuration() {
        let config = AudioConfig::new(48_000, 2, 256);

        assert_eq!(config.validate(), Ok(()));
        assert_eq!(config.sample_rate_hz(), 48_000);
        assert_eq!(config.channels(), 2);
        assert_eq!(config.buffer_size_frames(), 256);
    }

    #[test]
    fn rejects_zero_buffer_size() {
        let config = AudioConfig::new(48_000, 2, 0);

        assert_eq!(
            config.validate(),
            Err(AudioConfigError::Dsp(DspError::ZeroBlockSize))
        );
    }

    #[test]
    fn rejects_zero_channels() {
        let config = AudioConfig::new(48_000, 0, 256);

        assert_eq!(config.validate(), Err(AudioConfigError::ZeroChannels));
    }
}
