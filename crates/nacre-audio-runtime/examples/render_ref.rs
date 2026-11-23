//! Renders the three reference patches offline, without opening a device.
//!
//! Run with `cargo run -p nacre-audio-runtime --example render_ref -- <patch>`
//! where `<patch>` is `drone`, `bass`, `metal` or `all`. Files are written to
//! `assets/reference/` as 48 kHz stereo `f32` WAV and the process exits by
//! itself. Nothing is played through the speakers.

use std::error::Error;
use std::fs;
use std::path::Path;

use nacre_application::AudioConfig;
use nacre_audio_runtime::{
    REFERENCE_DURATION_SECONDS, ReferencePatch, hash_samples, render_reference, write_wav_f32,
};

fn main() -> Result<(), Box<dyn Error>> {
    let name = std::env::args().nth(1).unwrap_or_else(|| "all".to_owned());
    let config = AudioConfig::default();
    let output_dir = Path::new("assets/reference");
    fs::create_dir_all(output_dir)?;

    let patches: Vec<ReferencePatch> = if name.eq_ignore_ascii_case("all") {
        ReferencePatch::ALL.to_vec()
    } else {
        vec![
            ReferencePatch::parse(&name)
                .ok_or_else(|| format!("unknown patch '{name}'; use drone, bass, metal or all"))?,
        ]
    };

    for patch in patches {
        let samples = render_reference(patch, config)?;
        let path = output_dir.join(format!("{}.wav", patch.name()));
        write_wav_f32(&path, config.sample_rate_hz(), config.channels(), &samples)?;
        println!(
            "{}: {} — {} frames, {:.1} s, {:.1} MiB, hash={:016x}",
            patch.name(),
            path.display(),
            samples.len() / usize::from(config.channels()),
            REFERENCE_DURATION_SECONDS,
            samples.len() as f64 * 4.0 / (1024.0 * 1024.0),
            hash_samples(&samples),
        );
    }

    Ok(())
}
