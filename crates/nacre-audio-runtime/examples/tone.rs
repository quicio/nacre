//! Plays a short 440 Hz tone and exits by itself.
//!
//! Run with `cargo run -p nacre-audio-runtime --example tone`.

use std::thread;
use std::time::Duration;

use nacre_application::{AudioConfig, ParameterCommand};
use nacre_audio_runtime::AudioHost;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AudioConfig::default();
    let mut host = AudioHost::new();

    host.submit(ParameterCommand::SetFrequency {
        frequency_hz: 440.0,
    })?;
    host.submit(ParameterCommand::SetGain { linear_gain: 0.2 })?;

    let status = host.start(config)?;
    println!(
        "host={} device={} sample_rate={} Hz channels={} buffer={} frames negotiated={}",
        status.host_name.as_deref().unwrap_or("unknown"),
        status.device_name.as_deref().unwrap_or("unknown"),
        status.sample_rate_hz,
        status.channels,
        status.buffer_size_frames,
        status.negotiated,
    );

    thread::sleep(Duration::from_secs(2));
    host.stop();
    println!("stopped");
    Ok(())
}
