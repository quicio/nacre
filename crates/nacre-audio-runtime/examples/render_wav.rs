//! Renders ten seconds of audio to a WAV file without a device.
//!
//! Run with `cargo run -p nacre-audio-runtime --example render_wav -- out.wav`.

use std::error::Error;

use nacre_application::{AudioConfig, gain_db_to_linear};
use nacre_audio_runtime::{OfflineRenderer, hash_samples, write_wav_f32};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "nacre-preview.wav".to_owned());
    let config = AudioConfig::default();
    let renderer = OfflineRenderer::new(config, 220.0, gain_db_to_linear(-12.0))?;
    let frames = u64::from(config.sample_rate_hz()) * 10;
    let samples = renderer.render_frames(frames);

    write_wav_f32(&path, config.sample_rate_hz(), config.channels(), &samples)?;

    println!(
        "wrote {path}: {frames} frames, {} samples, hash={:016x}",
        samples.len(),
        hash_samples(&samples)
    );
    Ok(())
}
