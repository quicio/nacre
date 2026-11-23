//! Minimal `f32` PCM WAV writer.
//!
//! The slice-by-slice encoder keeps the dependency tree small. If export grows
//! metadata or more formats in a later slice, this module is the place to
//! reconsider the decision.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use thiserror::Error;

const WAV_HEADER_BYTES: u32 = 36;
const BYTES_PER_SAMPLE: u32 = 4;
const IEEE_FLOAT_FORMAT: u16 = 3;

/// Errors produced while writing a WAV file.
#[derive(Debug, Error)]
pub enum WavError {
    /// The file could not be created or written.
    #[error("I/O error while writing WAV: {0}")]
    Io(#[from] std::io::Error),
    /// A WAV file needs at least one channel.
    #[error("WAV channel count must be greater than zero")]
    ZeroChannels,
    /// Interleaved samples must contain whole frames.
    #[error("sample count {samples} is not divisible by channel count {channels}")]
    MisalignedSamples {
        /// Number of samples provided.
        samples: usize,
        /// Number of channels requested.
        channels: u16,
    },
    /// The data does not fit in the 32-bit WAV size fields.
    #[error("WAV data exceeds the 32-bit RIFF size limit")]
    TooLarge,
}

/// Writes interleaved `f32` samples as an IEEE-float PCM WAV file.
pub fn write_wav_f32(
    path: impl AsRef<Path>,
    sample_rate_hz: u32,
    channels: u16,
    samples: &[f32],
) -> Result<(), WavError> {
    if channels == 0 {
        return Err(WavError::ZeroChannels);
    }
    let channels_usize = usize::from(channels);
    if !samples.len().is_multiple_of(channels_usize) {
        return Err(WavError::MisalignedSamples {
            samples: samples.len(),
            channels,
        });
    }

    let data_bytes = samples
        .len()
        .checked_mul(BYTES_PER_SAMPLE as usize)
        .ok_or(WavError::TooLarge)?;
    let data_bytes = u32::try_from(data_bytes).map_err(|_| WavError::TooLarge)?;
    let riff_size = WAV_HEADER_BYTES
        .checked_add(data_bytes)
        .ok_or(WavError::TooLarge)?;
    let block_align = channels
        .checked_mul(BYTES_PER_SAMPLE as u16)
        .ok_or(WavError::TooLarge)?;
    let byte_rate = sample_rate_hz
        .checked_mul(u32::from(block_align))
        .ok_or(WavError::TooLarge)?;

    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(b"RIFF")?;
    writer.write_all(&riff_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;
    writer.write_all(b"fmt ")?;
    writer.write_all(&16_u32.to_le_bytes())?;
    writer.write_all(&IEEE_FLOAT_FORMAT.to_le_bytes())?;
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate_hz.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&(BYTES_PER_SAMPLE as u16 * 8).to_le_bytes())?;
    writer.write_all(b"data")?;
    writer.write_all(&data_bytes.to_le_bytes())?;

    for sample in samples {
        writer.write_all(&sample.to_le_bytes())?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{WavError, write_wav_f32};

    static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temporary_path() -> std::path::PathBuf {
        let counter = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "nacre-wav-test-{}-{counter}.wav",
            std::process::id()
        ))
    }

    #[test]
    fn header_is_a_valid_float_wav() {
        let path = temporary_path();
        let samples = [0.0_f32, 0.25, -0.25, 0.5, -0.5, 0.0];

        write_wav_f32(&path, 48_000, 2, &samples).expect("write succeeds");
        let bytes = fs::read(&path).expect("read back");
        fs::remove_file(&path).expect("cleanup");

        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().expect("data size")),
            samples.len() as u32 * 4
        );
        assert_eq!(
            u16::from_le_bytes(bytes[20..22].try_into().expect("format")),
            3
        );
        assert_eq!(
            u16::from_le_bytes(bytes[22..24].try_into().expect("channels")),
            2
        );
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().expect("sample rate")),
            48_000
        );
        assert_eq!(bytes.len(), 44 + samples.len() * 4);
    }

    #[test]
    fn rejects_zero_channels_and_misaligned_samples() {
        let path = temporary_path();

        assert!(matches!(
            write_wav_f32(&path, 48_000, 0, &[]),
            Err(WavError::ZeroChannels)
        ));
        assert!(matches!(
            write_wav_f32(&path, 48_000, 2, &[0.0]),
            Err(WavError::MisalignedSamples { .. })
        ));
    }
}
