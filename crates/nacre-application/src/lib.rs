//! Ports, DTOs and use cases that connect the control plane to audio capability.
//!
//! This crate deliberately knows nothing about a concrete audio backend. The
//! runtime crate implements [`AudioHostPort`]; the standalone application wires
//! the two together.

use std::time::Duration;

use nacre_domain::SampleTime;
use thiserror::Error;

/// Device-independent audio configuration requested by the control plane.
///
/// The runtime may negotiate a different configuration with the device and
/// report the effective values in [`AudioStatus`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioConfig {
    sample_rate_hz: u32,
    channels: u16,
    buffer_size_frames: u32,
}

impl AudioConfig {
    /// Lowest sample rate accepted by the control plane, in hertz.
    pub const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
    /// Highest sample rate accepted by the control plane, in hertz.
    pub const MAX_SAMPLE_RATE_HZ: u32 = 192_000;
    /// Lowest accepted buffer size, in frames.
    pub const MIN_BUFFER_FRAMES: u32 = 32;
    /// Highest accepted buffer size, in frames.
    pub const MAX_BUFFER_FRAMES: u32 = 2_048;

    /// Creates a configuration whose values can be checked with [`Self::validate`].
    pub const fn new(sample_rate_hz: u32, channels: u16, buffer_size_frames: u32) -> Self {
        Self {
            sample_rate_hz,
            channels,
            buffer_size_frames,
        }
    }

    /// Returns the requested sample rate in hertz.
    pub const fn sample_rate_hz(self) -> u32 {
        self.sample_rate_hz
    }

    /// Returns the requested number of audio channels.
    pub const fn channels(self) -> u16 {
        self.channels
    }

    /// Returns the requested buffer size in frames.
    pub const fn buffer_size_frames(self) -> u32 {
        self.buffer_size_frames
    }

    /// Checks the sample rate, channel count and buffer size.
    pub fn validate(self) -> Result<(), AudioConfigError> {
        if !(Self::MIN_SAMPLE_RATE_HZ..=Self::MAX_SAMPLE_RATE_HZ).contains(&self.sample_rate_hz) {
            return Err(AudioConfigError::SampleRateOutOfRange {
                sample_rate_hz: self.sample_rate_hz,
            });
        }

        if !(1..=2).contains(&self.channels) {
            return Err(AudioConfigError::ChannelsOutOfRange {
                channels: self.channels,
            });
        }

        if !(Self::MIN_BUFFER_FRAMES..=Self::MAX_BUFFER_FRAMES).contains(&self.buffer_size_frames) {
            return Err(AudioConfigError::BufferSizeOutOfRange {
                buffer_size_frames: self.buffer_size_frames,
            });
        }

        Ok(())
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self::new(48_000, 2, 256)
    }
}

/// Errors produced by invalid audio configurations.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum AudioConfigError {
    /// The requested sample rate is outside the supported range.
    #[error("sample rate {sample_rate_hz} Hz is outside 8000..=192000 Hz")]
    SampleRateOutOfRange {
        /// Rejected sample rate in hertz.
        sample_rate_hz: u32,
    },
    /// The requested channel count is outside the supported range.
    #[error("channel count {channels} is outside 1..=2")]
    ChannelsOutOfRange {
        /// Rejected channel count.
        channels: u16,
    },
    /// The requested buffer size is outside the supported range.
    #[error("buffer size {buffer_size_frames} frames is outside 32..=2048 frames")]
    BufferSizeOutOfRange {
        /// Rejected buffer size in frames.
        buffer_size_frames: u32,
    },
}

/// Lifecycle state of the audio host as seen by the control plane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioState {
    /// No stream is active.
    Stopped,
    /// A stream is active and the callback is running.
    Running,
}

/// Health values published by the audio runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AudioHealth {
    /// Frames processed by the audio callback since the stream started.
    pub frames: SampleTime,
    /// Root-mean-square level of the last processed block.
    pub rms: f32,
    /// Peak level of the last processed block.
    pub peak: f32,
    /// Device errors or xrun reports observed by the runtime.
    pub xruns: u32,
    /// Commands rejected because the bounded queue was full.
    pub dropped_commands: u32,
    /// Estimated callback load as a fraction of the block deadline.
    pub cpu_load: f32,
}

/// Effective audio configuration and current health, safe to read from the UI.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioStatus {
    /// Current lifecycle state.
    pub state: AudioState,
    /// Name of the selected output device, if one was found.
    pub device_name: Option<String>,
    /// Name of the selected backend host, if one was found.
    pub host_name: Option<String>,
    /// Effective sample rate in hertz.
    pub sample_rate_hz: u32,
    /// Effective number of channels.
    pub channels: u16,
    /// Effective buffer size in frames.
    pub buffer_size_frames: u32,
    /// Whether the effective configuration differs from the requested one.
    pub negotiated: bool,
    /// Runtime health counters and levels.
    pub health: AudioHealth,
}

impl AudioStatus {
    /// Creates a stopped status for a requested configuration.
    pub fn stopped(config: AudioConfig) -> Self {
        Self {
            state: AudioState::Stopped,
            device_name: None,
            host_name: None,
            sample_rate_hz: config.sample_rate_hz(),
            channels: config.channels(),
            buffer_size_frames: config.buffer_size_frames(),
            negotiated: false,
            health: AudioHealth::default(),
        }
    }

    /// Returns the number of frames processed since the stream started.
    pub const fn frames(&self) -> SampleTime {
        self.health.frames
    }
}

impl Default for AudioStatus {
    fn default() -> Self {
        Self::stopped(AudioConfig::default())
    }
}

/// Errors exposed by an audio host implementation.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum AudioError {
    /// The selected backend cannot find an output device.
    #[error("no output device is available")]
    DeviceUnavailable,
    /// The device does not support a configuration compatible with the request.
    #[error("the output device does not support a compatible configuration: {detail}")]
    ConfigUnsupported {
        /// Backend-provided explanation.
        detail: String,
    },
    /// Building the output stream failed.
    #[error("failed to build the output stream: {detail}")]
    StreamBuildFailed {
        /// Backend-provided explanation.
        detail: String,
    },
    /// Starting the output stream failed.
    #[error("failed to start the output stream: {detail}")]
    StreamStartFailed {
        /// Backend-provided explanation.
        detail: String,
    },
    /// The requested configuration is not valid.
    #[error(transparent)]
    InvalidConfig(#[from] AudioConfigError),
}

/// Errors returned when submitting a command to the audio host.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum CommandError {
    /// The bounded command queue is full; the command was not accepted.
    #[error("audio command queue is full")]
    Full,
    /// The host has no active command path.
    #[error("audio host is not running")]
    NotRunning,
}

/// Commands that travel from the control plane to the audio callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParameterCommand {
    /// Sets the oscillator frequency in hertz.
    SetFrequency {
        /// Requested frequency in hertz.
        frequency_hz: f32,
    },
    /// Sets the master gain as a linear factor in `0.0..=1.0`.
    SetGain {
        /// Requested linear gain.
        linear_gain: f32,
    },
    /// Starts a note at the requested pitch.
    NoteOn {
        /// Frequency of the note in hertz.
        frequency_hz: f32,
    },
    /// Releases the current note.
    NoteOff,
    /// Selects the oscillator waveform.
    SetWaveform {
        /// Requested waveform.
        waveform: Waveform,
    },
    /// Selects one continuous voice parameter.
    SetVoiceParameter {
        /// Parameter to change.
        parameter: VoiceParameter,
        /// New value in the parameter's unit.
        value: f32,
    },
    /// Selects the LFO shape.
    SetLfoShape {
        /// Requested LFO shape.
        shape: LfoShape,
    },
    /// Selects the LFO destination.
    SetLfoDestination {
        /// Requested destination.
        destination: LfoDestination,
    },
    /// Starts the transport with a short fade-in.
    Start,
    /// Starts a short fade-out and stops the transport when it completes.
    Stop,
}

/// Oscillator waveform selected by the control plane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Waveform {
    /// Band-limited saw.
    Saw,
    /// Sine.
    Sine,
}

/// Shape generated by the LFO.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LfoShape {
    /// Smooth sinusoidal shape.
    Sine,
    /// Triangular shape.
    Triangle,
}

/// Destination of the LFO modulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LfoDestination {
    /// Filter cutoff, in octaves.
    Cutoff,
    /// Oscillator pitch, in semitones.
    Pitch,
}

/// Continuous voice parameter controlled through [`Parameter`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VoiceParameter {
    /// Filter cutoff in hertz.
    FilterCutoff,
    /// Filter resonance as Q.
    FilterResonance,
    /// Envelope attack in milliseconds.
    Attack,
    /// Envelope decay in milliseconds.
    Decay,
    /// Envelope sustain level in `0.0..=1.0`.
    Sustain,
    /// Envelope release in milliseconds.
    Release,
    /// LFO rate in hertz.
    LfoRate,
    /// LFO depth in percent.
    LfoDepth,
    /// Saturation drive in decibels.
    Drive,
}

impl VoiceParameter {
    /// Returns the accepted range for this parameter.
    pub const fn range(self) -> (f32, f32) {
        match self {
            Self::FilterCutoff => (20.0, 20_000.0),
            Self::FilterResonance => (0.5, 20.0),
            Self::Attack | Self::Decay | Self::Release => (1.0, 10_000.0),
            Self::Sustain => (0.0, 1.0),
            Self::LfoRate => (0.01, 20.0),
            Self::LfoDepth => (0.0, 100.0),
            Self::Drive => (0.0, 24.0),
        }
    }
}

/// A control-plane parameter that can be validated before reaching audio.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Parameter {
    /// Oscillator frequency in hertz.
    Frequency,
    /// Master gain as a linear factor.
    Gain,
    /// One continuous voice parameter.
    Voice(VoiceParameter),
}

impl Parameter {
    /// Returns the accepted range for this parameter.
    pub const fn range(self) -> (f32, f32) {
        match self {
            Self::Frequency => (0.1, 20_000.0),
            Self::Gain => (0.0, 1.0),
            Self::Voice(parameter) => parameter.range(),
        }
    }

    /// Validates a value and converts it into a runtime command.
    pub fn command(self, value: f32) -> Result<ParameterCommand, ParameterError> {
        self.validate(value)?;

        Ok(match self {
            Self::Frequency => ParameterCommand::SetFrequency {
                frequency_hz: value,
            },
            Self::Gain => ParameterCommand::SetGain { linear_gain: value },
            Self::Voice(parameter) => ParameterCommand::SetVoiceParameter { parameter, value },
        })
    }

    /// Validates a value against this parameter's range.
    pub fn validate(self, value: f32) -> Result<(), ParameterError> {
        let (min, max) = self.range();
        if !value.is_finite() || value < min || value > max {
            return Err(ParameterError::OutOfRange {
                parameter: self,
                value,
                min,
                max,
            });
        }

        Ok(())
    }
}

/// Errors produced when validating control-plane parameters.
#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ParameterError {
    /// The value is not finite or lies outside the accepted range.
    #[error("{parameter:?} value {value} is outside {min}..={max}")]
    OutOfRange {
        /// Parameter that rejected the value.
        parameter: Parameter,
        /// Rejected value.
        value: f32,
        /// Inclusive lower bound.
        min: f32,
        /// Inclusive upper bound.
        max: f32,
    },
}

/// Errors produced by application use cases.
#[derive(Debug, Error)]
pub enum AppError {
    /// The audio host failed.
    #[error(transparent)]
    Audio(#[from] AudioError),
    /// The audio host rejected a command.
    #[error(transparent)]
    Command(#[from] CommandError),
    /// A parameter value is invalid.
    #[error(transparent)]
    Parameter(#[from] ParameterError),
}

/// Converts a gain in decibels into a linear factor, clamping to `0.0..=1.0`.
pub fn gain_db_to_linear(gain_db: f32) -> f32 {
    if !gain_db.is_finite() {
        return 0.0;
    }

    let linear = 10.0_f32.powf(gain_db / 20.0);
    if linear.is_finite() {
        linear.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Port implemented by an audio host.
///
/// The application layer only depends on this trait. `nacre-audio-runtime`
/// provides the concrete implementation backed by the device.
pub trait AudioHostPort {
    /// Starts audio processing with the requested configuration.
    fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError>;

    /// Stops audio processing, applying the runtime's fade-out.
    fn stop(&mut self);

    /// Submits a command without blocking the caller.
    fn submit(&self, command: ParameterCommand) -> Result<(), CommandError>;

    /// Returns the latest effective configuration and health snapshot.
    fn status(&self) -> AudioStatus;
}

/// Provides a monotonic time source.
pub trait Clock {
    /// Returns the elapsed monotonic time since an implementation-defined origin.
    fn now(&self) -> Duration;
}

/// Renders audio into a caller-owned sample buffer.
pub trait Renderer {
    /// Error reported by the renderer implementation.
    type Error;

    /// Fills `output` with rendered samples.
    fn render(&mut self, output: &mut [f32]) -> Result<(), Self::Error>;
}

/// Use case that starts audio through the host port.
pub struct StartAudio<'a> {
    host: &'a mut dyn AudioHostPort,
}

impl<'a> StartAudio<'a> {
    /// Creates the use case against a host port.
    pub fn new(host: &'a mut dyn AudioHostPort) -> Self {
        Self { host }
    }

    /// Starts audio and returns the effective configuration.
    pub fn execute(&mut self, config: AudioConfig) -> Result<AudioStatus, AppError> {
        self.host.start(config).map_err(Into::into)
    }
}

/// Use case that stops audio through the host port.
pub struct StopAudio<'a> {
    host: &'a mut dyn AudioHostPort,
}

impl<'a> StopAudio<'a> {
    /// Creates the use case against a host port.
    pub fn new(host: &'a mut dyn AudioHostPort) -> Self {
        Self { host }
    }

    /// Stops audio. Stopping an already stopped host is a no-op.
    pub fn execute(&mut self) {
        self.host.stop();
    }
}

/// Use case that validates a parameter and submits it to the host port.
pub struct SetParameter<'a> {
    host: &'a mut dyn AudioHostPort,
}

impl<'a> SetParameter<'a> {
    /// Creates the use case against a host port.
    pub fn new(host: &'a mut dyn AudioHostPort) -> Self {
        Self { host }
    }

    /// Validates and submits `value` for `parameter`.
    pub fn execute(&mut self, parameter: Parameter, value: f32) -> Result<(), AppError> {
        let command = parameter.command(value)?;
        self.host.submit(command)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::cell::RefCell;
    use std::convert::Infallible;

    use super::{
        AppError, AudioConfig, AudioConfigError, AudioError, AudioHostPort, AudioState,
        AudioStatus, Clock, CommandError, Parameter, ParameterCommand, ParameterError, Renderer,
        SetParameter, StartAudio, StopAudio, VoiceParameter, gain_db_to_linear,
    };

    fn assert_object_safe<T: ?Sized>() {}

    #[test]
    fn ports_are_object_safe() {
        assert_object_safe::<dyn AudioHostPort>();
        assert_object_safe::<dyn Clock>();
        assert_object_safe::<dyn Renderer<Error = Infallible>>();
    }

    #[test]
    fn default_configuration_is_the_requested_preference() {
        let config = AudioConfig::default();

        assert_eq!(config.sample_rate_hz(), 48_000);
        assert_eq!(config.channels(), 2);
        assert_eq!(config.buffer_size_frames(), 256);
        assert_eq!(config.validate(), Ok(()));
    }

    #[test]
    fn accepts_configuration_boundaries() {
        for config in [
            AudioConfig::new(8_000, 1, 32),
            AudioConfig::new(192_000, 2, 2_048),
        ] {
            assert_eq!(config.validate(), Ok(()));
        }
    }

    #[test]
    fn rejects_out_of_range_configuration() {
        assert_eq!(
            AudioConfig::new(7_999, 2, 256).validate(),
            Err(AudioConfigError::SampleRateOutOfRange {
                sample_rate_hz: 7_999
            })
        );
        assert_eq!(
            AudioConfig::new(192_001, 2, 256).validate(),
            Err(AudioConfigError::SampleRateOutOfRange {
                sample_rate_hz: 192_001
            })
        );
        assert_eq!(
            AudioConfig::new(48_000, 0, 256).validate(),
            Err(AudioConfigError::ChannelsOutOfRange { channels: 0 })
        );
        assert_eq!(
            AudioConfig::new(48_000, 3, 256).validate(),
            Err(AudioConfigError::ChannelsOutOfRange { channels: 3 })
        );
        assert_eq!(
            AudioConfig::new(48_000, 2, 31).validate(),
            Err(AudioConfigError::BufferSizeOutOfRange {
                buffer_size_frames: 31
            })
        );
        assert_eq!(
            AudioConfig::new(48_000, 2, 2_049).validate(),
            Err(AudioConfigError::BufferSizeOutOfRange {
                buffer_size_frames: 2_049
            })
        );
    }

    #[test]
    fn gain_conversion_is_clamped_and_sanitized() {
        assert_eq!(gain_db_to_linear(0.0), 1.0);
        assert!((gain_db_to_linear(-60.0) - 0.001).abs() < 1e-6);
        assert_eq!(gain_db_to_linear(6.0), 1.0);
        assert_eq!(gain_db_to_linear(f32::NAN), 0.0);
    }

    #[test]
    fn parameter_validation_rejects_out_of_range_before_audio() {
        assert!(matches!(
            Parameter::Frequency.command(0.0),
            Err(ParameterError::OutOfRange {
                parameter: Parameter::Frequency,
                ..
            })
        ));
        assert!(matches!(
            Parameter::Gain.command(1.5),
            Err(ParameterError::OutOfRange {
                parameter: Parameter::Gain,
                ..
            })
        ));
        assert_eq!(
            Parameter::Frequency.command(440.0),
            Ok(ParameterCommand::SetFrequency {
                frequency_hz: 440.0
            })
        );
        assert_eq!(
            Parameter::Gain.command(0.5),
            Ok(ParameterCommand::SetGain { linear_gain: 0.5 })
        );
    }

    #[test]
    fn voice_parameters_validate_and_map_to_commands() {
        assert_eq!(VoiceParameter::FilterCutoff.range(), (20.0, 20_000.0));
        assert_eq!(VoiceParameter::Sustain.range(), (0.0, 1.0));

        assert!(matches!(
            Parameter::Voice(VoiceParameter::Sustain).validate(1.5),
            Err(ParameterError::OutOfRange { .. })
        ));
        assert_eq!(
            Parameter::Voice(VoiceParameter::Attack).command(250.0),
            Ok(ParameterCommand::SetVoiceParameter {
                parameter: VoiceParameter::Attack,
                value: 250.0,
            })
        );
    }

    #[derive(Default)]
    struct FakeHost {
        started: Vec<AudioConfig>,
        stops: usize,
        status: AudioStatus,
    }

    impl AudioHostPort for FakeHost {
        fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError> {
            self.started.push(config);
            let mut status = AudioStatus::stopped(config);
            status.state = AudioState::Running;
            self.status = status.clone();
            Ok(status)
        }

        fn stop(&mut self) {
            self.stops += 1;
            self.status.state = AudioState::Stopped;
        }

        fn submit(&self, _command: ParameterCommand) -> Result<(), CommandError> {
            // `submit` takes `&self` in the port; the fake only needs to record
            // command validation in these tests, so this test double is read-only.
            Ok(())
        }

        fn status(&self) -> AudioStatus {
            self.status.clone()
        }
    }

    struct RecordingHost {
        commands: RefCell<Vec<ParameterCommand>>,
    }

    impl AudioHostPort for RecordingHost {
        fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError> {
            Ok(AudioStatus::stopped(config))
        }

        fn stop(&mut self) {}

        fn submit(&self, command: ParameterCommand) -> Result<(), CommandError> {
            self.commands.borrow_mut().push(command);
            Ok(())
        }

        fn status(&self) -> AudioStatus {
            AudioStatus::stopped(AudioConfig::default())
        }
    }

    #[test]
    fn start_and_stop_use_cases_delegate_to_the_port() {
        let mut host = FakeHost::default();

        let status = StartAudio::new(&mut host)
            .execute(AudioConfig::default())
            .expect("start should succeed");
        assert_eq!(status.state, AudioState::Running);
        assert_eq!(host.started, vec![AudioConfig::default()]);

        StopAudio::new(&mut host).execute();
        assert_eq!(host.stops, 1);
    }

    #[test]
    fn set_parameter_rejects_invalid_values_without_touching_the_port() {
        let mut host = RecordingHost {
            commands: RefCell::new(Vec::new()),
        };

        let error = SetParameter::new(&mut host)
            .execute(Parameter::Gain, 1.5)
            .expect_err("gain above one must be rejected");

        assert!(matches!(
            error,
            AppError::Parameter(ParameterError::OutOfRange { .. })
        ));
        assert!(host.commands.borrow().is_empty());
    }

    #[test]
    fn set_parameter_submits_valid_values() {
        let mut host = RecordingHost {
            commands: RefCell::new(Vec::new()),
        };

        SetParameter::new(&mut host)
            .execute(Parameter::Frequency, 440.0)
            .expect("valid frequency");

        assert_eq!(
            host.commands.borrow().as_slice(),
            &[ParameterCommand::SetFrequency {
                frequency_hz: 440.0
            }]
        );
    }
}
