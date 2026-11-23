//! Realtime block processor shared by the device callback and tests.
//!
//! Everything here runs inside the audio callback. It must not allocate, lock,
//! perform I/O, log, or panic. The ring buffer, the temporary mono buffer and
//! the voice's 4× oscillator buffer are allocated by [`AudioProcessor::new`] on
//! the control thread.

use std::sync::Arc;
use std::time::Instant;

use nacre_application::{AudioConfig, ParameterCommand, VoiceParameter};
use nacre_dsp::{LfoDestination, LfoWaveform, Voice, VoiceParams, Waveform};
use rtrb::Consumer;

use crate::sanitize::{
    lfo_destination_code, lfo_shape_code, sanitize_frequency_with_fallback, sanitize_gain,
    waveform_code,
};
use crate::shared::SharedState;
use crate::{MAX_COMMANDS_PER_BLOCK, STOP_FADE_MS};

/// Gain smoothing time constant. Small enough to feel immediate, long enough
/// to avoid zipper noise.
const GAIN_SMOOTHING_MS: f32 = 10.0;

pub(crate) struct AudioProcessor {
    voice: Voice,
    commands: Consumer<ParameterCommand>,
    shared: Arc<SharedState>,
    sample_rate_hz: f32,
    channels: usize,
    temp: Vec<f32>,
    fade_level: f32,
    fade_target: f32,
    fade_step: f32,
    current_gain: f32,
    target_gain: f32,
    gain_smoothing: f32,
    stopping: bool,
}

impl AudioProcessor {
    pub(crate) fn new(
        config: AudioConfig,
        commands: Consumer<ParameterCommand>,
        shared: Arc<SharedState>,
    ) -> Self {
        let sample_rate_hz = config.sample_rate_hz() as f32;
        let linear_gain = shared.gain();

        let buffer_frames =
            usize::try_from(config.buffer_size_frames()).map_or(2_048, |frames| frames.max(1));

        let mut voice = Voice::new();
        voice.set_params(voice_params(&shared));
        voice.prepare(sample_rate_hz, buffer_frames);

        let fade_frames = (STOP_FADE_MS / 1_000.0) * sample_rate_hz;
        let smoothing_frames = (GAIN_SMOOTHING_MS / 1_000.0) * sample_rate_hz;

        Self {
            voice,
            commands,
            shared,
            sample_rate_hz,
            channels: usize::from(config.channels()),
            temp: vec![0.0; buffer_frames],
            fade_level: 0.0,
            fade_target: 1.0,
            fade_step: 1.0 / fade_frames.max(1.0),
            current_gain: linear_gain,
            target_gain: linear_gain,
            gain_smoothing: 1.0 - (-1.0 / smoothing_frames.max(1.0)).exp(),
            stopping: false,
        }
    }

    /// Processes one interleaved output block without allocating or blocking.
    pub(crate) fn process_block(&mut self, out: &mut [f32]) {
        let started = Instant::now();
        out.fill(0.0);

        if self.channels == 0 {
            return;
        }

        if self.shared.take_stop_request() {
            self.request_stop();
        }
        self.drain_commands();
        self.read_parameter_slots();

        let mut sum_squares = 0.0_f64;
        let mut peak = 0.0_f32;
        let mut processed_frames = 0_usize;

        let mut frames = out.chunks_exact_mut(self.channels);
        loop {
            let remaining_frames = frames.len();
            if remaining_frames == 0 {
                break;
            }
            let block_frames = remaining_frames.min(self.temp.len());
            self.voice.process(&mut self.temp[..block_frames]);

            for (index, frame) in frames.by_ref().take(block_frames).enumerate() {
                let raw = self.temp.get(index).copied().unwrap_or(0.0);
                let fade = self.next_fade();
                let gain = self.next_gain();
                let mut sample = if fade <= 0.0 || gain <= 0.0 {
                    0.0
                } else {
                    raw * fade * gain
                };
                if !sample.is_finite() {
                    sample = 0.0;
                }
                sample = sample.clamp(-1.0, 1.0);

                for channel in frame.iter_mut() {
                    *channel = sample;
                }

                let magnitude = f64::from(sample);
                sum_squares += magnitude * magnitude;
                peak = peak.max(sample.abs());
                processed_frames += 1;
            }
        }

        let rms = if processed_frames > 0 {
            (sum_squares / processed_frames as f64).sqrt() as f32
        } else {
            0.0
        };
        self.shared.store_meter(rms, peak);
        self.shared.add_frames(processed_frames as u64);

        let elapsed_seconds = started.elapsed().as_secs_f64();
        let block_seconds = processed_frames as f64 / f64::from(self.sample_rate_hz);
        let cpu_load = if block_seconds > 0.0 {
            (elapsed_seconds / block_seconds) as f32
        } else {
            0.0
        };
        self.shared.store_cpu_load(cpu_load);
    }

    fn drain_commands(&mut self) {
        for _ in 0..MAX_COMMANDS_PER_BLOCK {
            match self.commands.pop() {
                Ok(ParameterCommand::SetFrequency { frequency_hz }) => {
                    let value =
                        sanitize_frequency_with_fallback(frequency_hz, self.shared.frequency_hz());
                    self.shared.set_frequency_hz(value);
                }
                Ok(ParameterCommand::SetGain { linear_gain }) => {
                    let value = sanitize_gain(linear_gain);
                    self.shared.set_gain(value);
                    self.target_gain = value;
                }
                Ok(ParameterCommand::NoteOn { frequency_hz }) => {
                    let value =
                        sanitize_frequency_with_fallback(frequency_hz, self.shared.frequency_hz());
                    self.shared.set_frequency_hz(value);
                    self.voice.note_on();
                }
                Ok(ParameterCommand::NoteOff) => {
                    self.voice.note_off();
                }
                Ok(ParameterCommand::SetWaveform { waveform }) => {
                    self.shared.set_waveform(waveform_code(waveform));
                }
                Ok(ParameterCommand::SetVoiceParameter { parameter, value }) => {
                    self.shared.set_voice_parameter(parameter, value);
                }
                Ok(ParameterCommand::SetLfoShape { shape }) => {
                    self.shared.set_lfo_shape(lfo_shape_code(shape));
                }
                Ok(ParameterCommand::SetLfoDestination { destination }) => {
                    self.shared
                        .set_lfo_destination(lfo_destination_code(destination));
                }
                Ok(ParameterCommand::Start) => {
                    self.stopping = false;
                    self.fade_target = 1.0;
                    self.shared.clear_fade_finished();
                }
                Ok(ParameterCommand::Stop) => {
                    self.request_stop();
                }
                Err(_) => break,
            }
        }
    }

    fn read_parameter_slots(&mut self) {
        self.voice.set_params(voice_params(&self.shared));
        self.target_gain = self.shared.gain();
    }

    fn request_stop(&mut self) {
        self.fade_target = 0.0;
        self.stopping = true;
    }

    fn next_fade(&mut self) -> f32 {
        let value = self.fade_level;

        if self.fade_level < self.fade_target {
            self.fade_level = (self.fade_level + self.fade_step).min(self.fade_target);
        } else if self.fade_level > self.fade_target {
            self.fade_level = (self.fade_level - self.fade_step).max(self.fade_target);
        }

        if self.stopping && self.fade_level <= 0.0 {
            self.fade_level = 0.0;
            self.stopping = false;
            self.shared.mark_fade_finished();
        }

        value
    }

    fn next_gain(&mut self) -> f32 {
        let next = self.current_gain + (self.target_gain - self.current_gain) * self.gain_smoothing;
        self.current_gain = if next.is_finite() {
            next.clamp(0.0, 1.0)
        } else {
            self.target_gain
        };
        self.current_gain
    }
}

fn voice_params(shared: &SharedState) -> VoiceParams {
    VoiceParams {
        frequency_hz: shared.frequency_hz(),
        waveform: match shared.waveform() {
            1 => Waveform::Sine,
            _ => Waveform::Saw,
        },
        filter_cutoff_hz: shared.voice_parameter(VoiceParameter::FilterCutoff),
        filter_resonance: shared.voice_parameter(VoiceParameter::FilterResonance),
        attack_ms: shared.voice_parameter(VoiceParameter::Attack),
        decay_ms: shared.voice_parameter(VoiceParameter::Decay),
        sustain: shared.voice_parameter(VoiceParameter::Sustain),
        release_ms: shared.voice_parameter(VoiceParameter::Release),
        lfo_rate_hz: shared.voice_parameter(VoiceParameter::LfoRate),
        lfo_depth_percent: shared.voice_parameter(VoiceParameter::LfoDepth),
        lfo_destination: match shared.lfo_destination() {
            1 => LfoDestination::Pitch,
            _ => LfoDestination::Cutoff,
        },
        lfo_waveform: match shared.lfo_shape() {
            1 => LfoWaveform::Triangle,
            _ => LfoWaveform::Sine,
        },
        drive_db: shared.voice_parameter(VoiceParameter::Drive),
    }
}

#[cfg(test)]
mod tests {
    #![allow(unsafe_code)]
    #![allow(clippy::float_cmp)]

    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::sync::Arc;

    use nacre_application::{
        AudioConfig, LfoDestination, LfoShape, ParameterCommand, VoiceParameter, Waveform,
    };

    use super::AudioProcessor;
    use crate::command::command_queue;
    use crate::shared::SharedState;
    use crate::{COMMAND_QUEUE_CAPACITY, STOP_FADE_MS};

    thread_local! {
        static COUNTING: Cell<bool> = const { Cell::new(false) };
        static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    }

    struct CountingAllocator;

    // SAFETY: every method forwards to `System` with the same layout it was
    // given, and only adds a thread-local counter around the allocation.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            if COUNTING.with(Cell::get) {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
            // SAFETY: `System` is the platform allocator and `layout` is valid.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            if COUNTING.with(Cell::get) {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
            // SAFETY: `System` is the platform allocator and `layout` is valid.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            if COUNTING.with(Cell::get) {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
            // SAFETY: the caller guarantees `pointer` and `layout` are valid for
            // a reallocation to `new_size`, which `System` handles unchanged.
            unsafe { System.realloc(pointer, layout, new_size) }
        }

        unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
            // SAFETY: the caller guarantees the pointer/layout pair came from
            // this allocator, which forwards to `System`.
            unsafe { System.dealloc(pointer, layout) }
        }
    }

    #[global_allocator]
    static GLOBAL: CountingAllocator = CountingAllocator;

    fn processor() -> (
        Arc<SharedState>,
        crate::command::CommandSender,
        AudioProcessor,
    ) {
        let shared = Arc::new(SharedState::new(440.0, 0.2));
        let (sender, consumer) = command_queue();
        let processor = AudioProcessor::new(AudioConfig::default(), consumer, Arc::clone(&shared));
        (shared, sender, processor)
    }

    #[test]
    fn callback_path_does_not_allocate_over_ten_thousand_blocks() {
        let (_shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: 220.0,
            })
            .expect("note accepted");

        for index in 0..10_000 {
            let frequency_hz = 220.0 + (index % 100) as f32;
            let _ = sender.submit(ParameterCommand::SetFrequency { frequency_hz });
            if index % 3 == 0 {
                let _ = sender.submit(ParameterCommand::SetVoiceParameter {
                    parameter: VoiceParameter::FilterCutoff,
                    value: 200.0 + (index % 1_000) as f32,
                });
            }
            if index % 500 == 0 {
                let _ = sender.submit(ParameterCommand::NoteOff);
                let _ = sender.submit(ParameterCommand::NoteOn {
                    frequency_hz: 110.0,
                });
            }

            ALLOCATIONS.with(|count| count.set(0));
            COUNTING.with(|counting| counting.set(true));
            processor.process_block(&mut output);
            COUNTING.with(|counting| counting.set(false));

            let allocations = ALLOCATIONS.with(Cell::get);
            assert_eq!(
                allocations, 0,
                "block {index} allocated {allocations} times"
            );
        }
    }

    #[test]
    fn note_on_produces_audio_and_note_off_returns_to_silence() {
        let (_shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::SetVoiceParameter {
                parameter: VoiceParameter::Release,
                value: 20.0,
            })
            .expect("release accepted");
        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: 220.0,
            })
            .expect("note accepted");
        for _ in 0..40 {
            processor.process_block(&mut output);
        }
        assert!(output.iter().any(|sample| sample.abs() > 0.001));

        sender
            .submit(ParameterCommand::NoteOff)
            .expect("note off accepted");
        for _ in 0..80 {
            processor.process_block(&mut output);
        }
        assert!(output.iter().all(|sample| sample.abs() < 0.001));
    }

    #[test]
    fn voice_parameter_commands_reach_the_shared_slots() {
        let (shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        for command in [
            ParameterCommand::SetVoiceParameter {
                parameter: VoiceParameter::FilterCutoff,
                value: 800.0,
            },
            ParameterCommand::SetVoiceParameter {
                parameter: VoiceParameter::Attack,
                value: 50.0,
            },
            ParameterCommand::SetVoiceParameter {
                parameter: VoiceParameter::Drive,
                value: f32::NAN,
            },
            ParameterCommand::SetWaveform {
                waveform: Waveform::Sine,
            },
            ParameterCommand::SetLfoShape {
                shape: LfoShape::Triangle,
            },
            ParameterCommand::SetLfoDestination {
                destination: LfoDestination::Pitch,
            },
            ParameterCommand::NoteOn {
                frequency_hz: 330.0,
            },
        ] {
            sender.submit(command).expect("command accepted");
        }

        processor.process_block(&mut output);

        assert_eq!(shared.voice_parameter(VoiceParameter::FilterCutoff), 800.0);
        assert_eq!(shared.voice_parameter(VoiceParameter::Attack), 50.0);
        assert_eq!(shared.voice_parameter(VoiceParameter::Drive), 0.0);
        assert_eq!(shared.waveform(), 1);
        assert_eq!(shared.lfo_shape(), 1);
        assert_eq!(shared.lfo_destination(), 1);
        assert_eq!(shared.frequency_hz(), 330.0);
    }

    #[test]
    fn stop_fade_lasts_twenty_milliseconds_and_then_stays_silent() {
        let (shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let block_frames = config.buffer_size_frames() as usize;
        let mut output = vec![0.0_f32; block_frames * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: 220.0,
            })
            .expect("note accepted");
        for _ in 0..10 {
            processor.process_block(&mut output);
        }
        assert!(output.iter().any(|sample| sample.abs() > 0.001));

        sender
            .submit(ParameterCommand::Stop)
            .expect("stop command accepted");

        let fade_blocks = ((STOP_FADE_MS / 1_000.0) * config.sample_rate_hz() as f32
            / block_frames as f32)
            .ceil() as usize
            + 1;
        let mut blocks = 0;
        while !shared.fade_finished() && blocks <= fade_blocks + 1 {
            processor.process_block(&mut output);
            blocks += 1;
        }

        assert!(shared.fade_finished(), "fade did not finish");
        assert!(
            blocks <= fade_blocks + 1,
            "fade took {blocks} blocks, expected about {fade_blocks}"
        );

        processor.process_block(&mut output);
        assert!(output.iter().all(|sample| sample.to_bits() == 0));
    }

    #[test]
    fn start_command_fades_back_in_after_stop() {
        let (shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: 220.0,
            })
            .expect("note accepted");
        sender
            .submit(ParameterCommand::Stop)
            .expect("stop command accepted");
        for _ in 0..40 {
            processor.process_block(&mut output);
        }
        assert!(shared.fade_finished());

        sender
            .submit(ParameterCommand::Start)
            .expect("start command accepted");
        for _ in 0..40 {
            processor.process_block(&mut output);
        }
        assert!(!shared.fade_finished());
        assert!(output.iter().any(|sample| sample.abs() > 0.001));
    }

    #[test]
    fn invalid_parameter_commands_are_sanitized() {
        let (shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::SetFrequency {
                frequency_hz: f32::NAN,
            })
            .expect("queue accepts the command");
        sender
            .submit(ParameterCommand::SetGain {
                linear_gain: f32::INFINITY,
            })
            .expect("queue accepts the command");
        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: f32::NEG_INFINITY,
            })
            .expect("queue accepts the command");

        processor.process_block(&mut output);

        assert_eq!(shared.frequency_hz(), 440.0);
        assert_eq!(shared.gain(), 0.0);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn health_snapshot_updates_after_one_block() {
        let (shared, mut sender, mut processor) = processor();
        let config = AudioConfig::default();
        let mut output =
            vec![0.0_f32; config.buffer_size_frames() as usize * usize::from(config.channels())];

        sender
            .submit(ParameterCommand::NoteOn {
                frequency_hz: 220.0,
            })
            .expect("note accepted");
        for _ in 0..4 {
            processor.process_block(&mut output);
        }

        let health = shared.health();
        assert_eq!(
            health.frames.get(),
            u64::from(config.buffer_size_frames()) * 4
        );
        assert!(health.peak > 0.0);
        assert!(health.rms > 0.0);
        assert!(health.cpu_load.is_finite());
    }

    #[test]
    fn queue_capacity_constant_is_the_contract_value() {
        assert_eq!(COMMAND_QUEUE_CAPACITY, 256);
    }
}
