//! Device lifecycle checks that require a real output device.
//!
//! Run explicitly with:
//! `cargo test -p nacre-audio-runtime --test device_lifecycle -- --ignored`

use std::thread;
use std::time::Duration;

use nacre_application::{AudioConfig, AudioHostPort, AudioState, ParameterCommand};
use nacre_audio_runtime::AudioHost;

#[test]
#[ignore = "requires a working audio output device"]
fn twenty_start_stop_cycles_leave_no_stuck_state() {
    let config = AudioConfig::default();
    let mut host = AudioHost::new();

    host.submit(ParameterCommand::SetGain { linear_gain: 0.0 })
        .expect("parameter accepted while stopped");

    for cycle in 0..20 {
        let status = host
            .start(config)
            .unwrap_or_else(|error| panic!("cycle {cycle} failed to start: {error}"));
        assert_eq!(status.state, AudioState::Running);
        thread::sleep(Duration::from_millis(10));
        host.stop();
        assert_eq!(host.status().state, AudioState::Stopped);
    }
}
