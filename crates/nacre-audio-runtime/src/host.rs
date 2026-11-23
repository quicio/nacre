//! `cpal`-backed audio host.
//!
//! This is the only module in the workspace that touches an audio device. All
//! realtime-sensitive work happens inside [`crate::processor::AudioProcessor`];
//! the host itself runs on the control thread.

use std::cell::{RefCell, RefMut};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig, SupportedBufferSize};
use nacre_application::{
    AudioConfig, AudioError, AudioHostPort, AudioState, AudioStatus, CommandError, ParameterCommand,
};

use crate::command::{CommandSender, command_queue};
use crate::processor::AudioProcessor;
use crate::shared::SharedState;

/// How long the control thread waits for the callback to finish its fade-out.
const FADE_WAIT_TIMEOUT_MS: u64 = 250;

struct PreparedOutput {
    device: cpal::Device,
    stream_config: StreamConfig,
    device_name: String,
    host_name: String,
    sample_rate_hz: u32,
    channels: u16,
    buffer_size_frames: u32,
    negotiated: bool,
}

/// Audio host backed by `cpal`.
///
/// The host owns the stream on the control thread. The callback owns only a
/// preallocated processor, an SPSC consumer, and atomics.
pub struct AudioHost {
    shared: Arc<SharedState>,
    stream: Option<cpal::Stream>,
    producer: RefCell<Option<CommandSender>>,
    status_template: AudioStatus,
}

impl AudioHost {
    /// Creates a host without opening a device.
    pub fn new() -> Self {
        Self {
            shared: Arc::new(SharedState::new(440.0, 0.25)),
            stream: None,
            producer: RefCell::new(None),
            status_template: AudioStatus::stopped(AudioConfig::default()),
        }
    }

    /// Inspects the default output device without starting a stream.
    ///
    /// The standalone application uses this to show the effective sample rate
    /// and buffer before the user presses Play.
    pub fn probe(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError> {
        let prepared = prepare_output(config)?;
        self.status_template = self.status_from(config, &prepared, AudioState::Stopped);
        Ok(self.snapshot())
    }

    /// Starts the output stream. Repeated calls while running are idempotent.
    pub fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError> {
        if self.stream.is_some() {
            return Ok(self.snapshot());
        }

        let prepared = prepare_output(config)?;
        self.shared.reset_counters();

        let (sender, consumer) = command_queue();
        // The DSP must run at the effective device rate, not the requested one.
        let effective_config = AudioConfig::new(
            prepared.sample_rate_hz,
            prepared.channels,
            prepared.buffer_size_frames,
        );
        let mut processor =
            AudioProcessor::new(effective_config, consumer, Arc::clone(&self.shared));
        let error_shared = Arc::clone(&self.shared);

        let stream = prepared
            .device
            .build_output_stream::<f32, _, _>(
                prepared.stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    processor.process_block(data);
                },
                move |_error: cpal::Error| {
                    error_shared.add_xrun();
                },
                None,
            )
            .map_err(|error| AudioError::StreamBuildFailed {
                detail: error.to_string(),
            })?;

        stream
            .play()
            .map_err(|error| AudioError::StreamStartFailed {
                detail: error.to_string(),
            })?;

        self.status_template = self.status_from(config, &prepared, AudioState::Running);
        *self.producer_mut() = Some(sender);
        self.stream = Some(stream);

        Ok(self.snapshot())
    }

    /// Stops the stream after the callback has finished its fade-out.
    pub fn stop(&mut self) {
        let Some(stream) = self.stream.take() else {
            *self.producer_mut() = None;
            self.status_template.state = AudioState::Stopped;
            return;
        };

        self.shared.clear_fade_finished();

        let stop_queued = {
            let mut producer = self.producer_mut();
            match producer.as_mut() {
                Some(sender) => sender.submit(ParameterCommand::Stop).is_ok(),
                None => false,
            }
        };
        if !stop_queued {
            // The queue may be full; an atomic flag always reaches the callback.
            self.shared.request_stop();
        }

        let deadline = Instant::now() + Duration::from_millis(FADE_WAIT_TIMEOUT_MS);
        while !self.shared.fade_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(1));
        }

        drop(stream);
        *self.producer_mut() = None;
        self.status_template.state = AudioState::Stopped;
    }

    /// Submits a command without blocking.
    ///
    /// Parameter changes are stored even while stopped so the next start uses
    /// the latest UI values. Transport commands require an active stream.
    pub fn submit(&self, command: ParameterCommand) -> Result<(), CommandError> {
        match command {
            ParameterCommand::SetFrequency { frequency_hz } => {
                let mut producer = self.producer_mut();
                match producer.as_mut() {
                    Some(sender) => self.submit_to_queue(sender, command),
                    None => {
                        self.shared.set_frequency_hz(frequency_hz);
                        Ok(())
                    }
                }
            }
            ParameterCommand::SetGain { linear_gain } => {
                let mut producer = self.producer_mut();
                match producer.as_mut() {
                    Some(sender) => self.submit_to_queue(sender, command),
                    None => {
                        self.shared.set_gain(linear_gain);
                        Ok(())
                    }
                }
            }
            ParameterCommand::Start | ParameterCommand::Stop => {
                let mut producer = self.producer_mut();
                match producer.as_mut() {
                    Some(sender) => self.submit_to_queue(sender, command),
                    None => Err(CommandError::NotRunning),
                }
            }
        }
    }

    /// Returns the latest effective configuration and health snapshot.
    pub fn snapshot(&self) -> AudioStatus {
        let mut status = self.status_template.clone();
        status.state = if self.stream.is_some() {
            AudioState::Running
        } else {
            AudioState::Stopped
        };
        status.health = self.shared.health();
        status
    }

    fn status_from(
        &self,
        requested: AudioConfig,
        prepared: &PreparedOutput,
        state: AudioState,
    ) -> AudioStatus {
        let mut status = AudioStatus::stopped(requested);
        status.state = state;
        status.device_name = Some(prepared.device_name.clone());
        status.host_name = Some(prepared.host_name.clone());
        status.sample_rate_hz = prepared.sample_rate_hz;
        status.channels = prepared.channels;
        status.buffer_size_frames = prepared.buffer_size_frames;
        status.negotiated = prepared.negotiated;
        status.health = self.shared.health();
        status
    }

    fn submit_to_queue(
        &self,
        sender: &mut CommandSender,
        command: ParameterCommand,
    ) -> Result<(), CommandError> {
        sender.submit(command).inspect_err(|_error| {
            self.shared.add_dropped_command();
        })
    }

    fn producer_mut(&self) -> RefMut<'_, Option<CommandSender>> {
        self.producer.borrow_mut()
    }
}

impl Default for AudioHost {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHostPort for AudioHost {
    fn start(&mut self, config: AudioConfig) -> Result<AudioStatus, AudioError> {
        AudioHost::start(self, config)
    }

    fn stop(&mut self) {
        AudioHost::stop(self);
    }

    fn submit(&self, command: ParameterCommand) -> Result<(), CommandError> {
        AudioHost::submit(self, command)
    }

    fn status(&self) -> AudioStatus {
        AudioHost::snapshot(self)
    }
}

impl Drop for AudioHost {
    fn drop(&mut self) {
        AudioHost::stop(self);
    }
}

fn prepare_output(config: AudioConfig) -> Result<PreparedOutput, AudioError> {
    config.validate()?;

    let host = select_host();
    let host_name = host.id().name().to_owned();
    let device = host
        .default_output_device()
        .ok_or(AudioError::DeviceUnavailable)?;
    let device_name = device.to_string();

    let supported =
        device
            .supported_output_configs()
            .map_err(|error| AudioError::ConfigUnsupported {
                detail: error.to_string(),
            })?;

    let mut selected: Option<(u16, u32, SupportedBufferSize, i32)> = None;
    for range in supported {
        if range.sample_format() != SampleFormat::F32 {
            continue;
        }

        let channels = range.channels();
        if channels == 0 || channels > 2 {
            continue;
        }

        let sample_rate_hz = config
            .sample_rate_hz()
            .clamp(range.min_sample_rate(), range.max_sample_rate());
        let channel_penalty = i32::from(channels != config.channels());
        let rate_penalty = i32::from(sample_rate_hz != config.sample_rate_hz());
        let score = channel_penalty * 2 + rate_penalty;

        let replace = match selected {
            None => true,
            Some((_, _, _, best_score)) => score < best_score,
        };
        if replace {
            selected = Some((channels, sample_rate_hz, *range.buffer_size(), score));
        }
    }

    let Some((channels, sample_rate_hz, supported_buffer, _score)) = selected else {
        return Err(AudioError::ConfigUnsupported {
            detail: "no f32 output configuration with one or two channels".to_owned(),
        });
    };

    let buffer_size_frames = match supported_buffer {
        SupportedBufferSize::Range { min, max } => config.buffer_size_frames().clamp(min, max),
        SupportedBufferSize::Unknown => config.buffer_size_frames(),
    };

    let negotiated = sample_rate_hz != config.sample_rate_hz()
        || channels != config.channels()
        || buffer_size_frames != config.buffer_size_frames();

    let stream_config = StreamConfig {
        channels,
        sample_rate: sample_rate_hz,
        buffer_size: BufferSize::Fixed(buffer_size_frames),
    };

    Ok(PreparedOutput {
        device,
        stream_config,
        device_name,
        host_name,
        sample_rate_hz,
        channels,
        buffer_size_frames,
        negotiated,
    })
}

fn select_host() -> cpal::Host {
    #[cfg(target_os = "linux")]
    if let Some(host_id) = cpal::available_hosts()
        .into_iter()
        .find(|host_id| host_id.name().eq_ignore_ascii_case("pipewire"))
        && let Ok(host) = cpal::host_from_id(host_id)
    {
        return host;
    }

    cpal::default_host()
}

#[cfg(test)]
mod tests {
    use nacre_application::{AudioConfig, AudioError};

    use super::prepare_output;

    #[test]
    fn invalid_configuration_is_rejected_before_touching_the_device() {
        let error = match prepare_output(AudioConfig::new(48_000, 3, 256)) {
            Ok(_) => panic!("three channels must be rejected"),
            Err(error) => error,
        };

        assert!(matches!(error, AudioError::InvalidConfig(_)));
    }
}
