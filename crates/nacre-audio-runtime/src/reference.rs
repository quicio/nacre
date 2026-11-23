//! Deterministic reference patches rendered without a device.
//!
//! Three patches exercise the voice in the three directions the product cares
//! about: a slow evolving drone, a pulsing bass and a metallic texture. They are
//! rendered by the same [`Voice`] used in the audio callback, with a fixed
//! event list: two runs with the same configuration produce identical samples.

use nacre_application::{AudioConfig, AudioConfigError, VoiceParameter};
use nacre_dsp::{LfoDestination, LfoWaveform, Voice, VoiceParams, Waveform};
use thiserror::Error;

use crate::wav::{WavError, write_wav_f32};

/// Errors produced while writing a reference render.
#[derive(Debug, Error)]
pub enum ReferenceError {
    /// The audio configuration is not valid.
    #[error(transparent)]
    Config(#[from] AudioConfigError),
    /// The WAV file could not be written.
    #[error(transparent)]
    Wav(#[from] WavError),
}

/// Duration of every reference render, in seconds.
pub const REFERENCE_DURATION_SECONDS: f32 = 120.0;

/// Frames rendered per block by the offline path.
const RENDER_BLOCK_FRAMES: usize = 256;

/// One of the three versioned reference patches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferencePatch {
    /// Slow evolving drone.
    Drone,
    /// Pulsing bass line.
    Bass,
    /// Metallic, resonant texture.
    Metal,
}

impl ReferencePatch {
    /// Every reference patch, in rendering order.
    pub const ALL: [Self; 3] = [Self::Drone, Self::Bass, Self::Metal];

    /// Stable name used by the CLI and the file name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Drone => "drone",
            Self::Bass => "bass",
            Self::Metal => "metal",
        }
    }

    /// Short human description of the patch.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Drone => "dron evolutivo: saw filtrada, ataque lento, LFO de cutoff",
            Self::Bass => "bajo pulsante: notas cortas cada segundo, resonancia media",
            Self::Metal => "textura metálica: resonancia alta, drive 12 dB, retriggers",
        }
    }

    /// Parses a patch name.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|patch| patch.name().eq_ignore_ascii_case(name))
    }
}

/// A parameter change scheduled for a given time, in seconds.
#[derive(Clone, Copy, Debug)]
struct AutomationPoint {
    at_seconds: f32,
    parameter: VoiceParameter,
    value: f32,
}

/// A note event: `Some(frequency)` starts a note, `None` releases it.
#[derive(Clone, Copy, Debug)]
struct NoteEvent {
    at_seconds: f32,
    frequency_hz: Option<f32>,
}

struct PatchPlan {
    params: VoiceParams,
    /// Final normalisation gain. It may exceed 1 because the voice already
    /// leaves 6 dB of headroom by design.
    render_gain: f32,
    automation: Vec<AutomationPoint>,
    notes: Vec<NoteEvent>,
}

impl PatchPlan {
    fn apply_automation(&self, time_seconds: f32, params: &mut VoiceParams, cursor: &mut usize) {
        while let Some(point) = self.automation.get(*cursor) {
            if point.at_seconds > time_seconds {
                break;
            }
            apply_parameter(params, point.parameter, point.value);
            *cursor += 1;
        }
    }

    fn apply_notes(
        &self,
        time_seconds: f32,
        params: &mut VoiceParams,
        voice: &mut Voice,
        cursor: &mut usize,
    ) {
        while let Some(event) = self.notes.get(*cursor) {
            if event.at_seconds > time_seconds {
                break;
            }
            match event.frequency_hz {
                Some(frequency_hz) => {
                    params.frequency_hz = frequency_hz;
                    voice.note_on();
                }
                None => voice.note_off(),
            }
            *cursor += 1;
        }
    }
}

fn apply_parameter(params: &mut VoiceParams, parameter: VoiceParameter, value: f32) {
    match parameter {
        VoiceParameter::FilterCutoff => params.filter_cutoff_hz = value,
        VoiceParameter::FilterResonance => params.filter_resonance = value,
        VoiceParameter::Attack => params.attack_ms = value,
        VoiceParameter::Decay => params.decay_ms = value,
        VoiceParameter::Sustain => params.sustain = value,
        VoiceParameter::Release => params.release_ms = value,
        VoiceParameter::LfoRate => params.lfo_rate_hz = value,
        VoiceParameter::LfoDepth => params.lfo_depth_percent = value,
        VoiceParameter::Drive => params.drive_db = value,
    }
}

fn plan(patch: ReferencePatch) -> PatchPlan {
    match patch {
        ReferencePatch::Drone => drone_plan(),
        ReferencePatch::Bass => bass_plan(),
        ReferencePatch::Metal => metal_plan(),
    }
}

fn drone_plan() -> PatchPlan {
    PatchPlan {
        params: VoiceParams {
            frequency_hz: 110.0,
            waveform: Waveform::Saw,
            filter_cutoff_hz: 400.0,
            filter_resonance: 2.5,
            attack_ms: 4_000.0,
            decay_ms: 3_000.0,
            sustain: 0.85,
            release_ms: 5_000.0,
            lfo_rate_hz: 0.05,
            lfo_depth_percent: 35.0,
            lfo_destination: LfoDestination::Cutoff,
            lfo_waveform: LfoWaveform::Sine,
            drive_db: 3.0,
        },
        render_gain: 1.0,
        automation: vec![
            automation(0.0, VoiceParameter::FilterCutoff, 400.0),
            automation(30.0, VoiceParameter::FilterCutoff, 800.0),
            automation(60.0, VoiceParameter::FilterCutoff, 1_400.0),
            automation(90.0, VoiceParameter::FilterCutoff, 2_000.0),
            automation(112.0, VoiceParameter::FilterCutoff, 2_400.0),
        ],
        notes: vec![note(0.0, 110.0), note_off(112.0)],
    }
}

fn bass_plan() -> PatchPlan {
    const PATTERN: [f32; 4] = [55.0, 55.0, 49.0, 43.65];
    let mut notes = Vec::new();
    let mut beat = 0;
    while beat as f32 * 1.0 < REFERENCE_DURATION_SECONDS - 2.0 {
        let at_seconds = beat as f32;
        notes.push(note(at_seconds, PATTERN[beat % PATTERN.len()]));
        notes.push(note_off(at_seconds + 0.45));
        beat += 1;
    }

    PatchPlan {
        params: VoiceParams {
            frequency_hz: 55.0,
            waveform: Waveform::Saw,
            filter_cutoff_hz: 280.0,
            filter_resonance: 5.0,
            attack_ms: 5.0,
            decay_ms: 150.0,
            sustain: 0.25,
            release_ms: 120.0,
            lfo_rate_hz: 0.5,
            lfo_depth_percent: 0.0,
            lfo_destination: LfoDestination::Cutoff,
            lfo_waveform: LfoWaveform::Sine,
            drive_db: 6.0,
        },
        render_gain: 1.0,
        automation: Vec::new(),
        notes,
    }
}

fn metal_plan() -> PatchPlan {
    const PATTERN: [f32; 4] = [220.0, 233.08, 246.94, 207.65];
    let mut notes = Vec::new();
    let mut index = 0;
    while index as f32 * 6.0 < REFERENCE_DURATION_SECONDS - 4.0 {
        notes.push(note(index as f32 * 6.0, PATTERN[index % PATTERN.len()]));
        index += 1;
    }
    notes.push(note_off(REFERENCE_DURATION_SECONDS - 4.0));

    PatchPlan {
        params: VoiceParams {
            frequency_hz: 220.0,
            waveform: Waveform::Saw,
            filter_cutoff_hz: 1_800.0,
            filter_resonance: 14.0,
            attack_ms: 150.0,
            decay_ms: 2_500.0,
            sustain: 0.9,
            release_ms: 1_200.0,
            lfo_rate_hz: 0.7,
            lfo_depth_percent: 30.0,
            lfo_destination: LfoDestination::Cutoff,
            lfo_waveform: LfoWaveform::Sine,
            drive_db: 12.0,
        },
        render_gain: 2.0,
        automation: Vec::new(),
        notes,
    }
}

const fn automation(at_seconds: f32, parameter: VoiceParameter, value: f32) -> AutomationPoint {
    AutomationPoint {
        at_seconds,
        parameter,
        value,
    }
}

const fn note(at_seconds: f32, frequency_hz: f32) -> NoteEvent {
    NoteEvent {
        at_seconds,
        frequency_hz: Some(frequency_hz),
    }
}

const fn note_off(at_seconds: f32) -> NoteEvent {
    NoteEvent {
        at_seconds,
        frequency_hz: None,
    }
}

/// Renders a reference patch as interleaved `f32` samples at the given config.
pub fn render_reference(
    patch: ReferencePatch,
    config: AudioConfig,
) -> Result<Vec<f32>, AudioConfigError> {
    render_for_duration(patch, config, REFERENCE_DURATION_SECONDS)
}

fn render_for_duration(
    patch: ReferencePatch,
    config: AudioConfig,
    duration_seconds: f32,
) -> Result<Vec<f32>, AudioConfigError> {
    config.validate()?;

    let duration_seconds = if duration_seconds.is_finite() && duration_seconds > 0.0 {
        duration_seconds
    } else {
        REFERENCE_DURATION_SECONDS
    };

    let sample_rate_hz = config.sample_rate_hz() as f32;
    let channels = usize::from(config.channels());
    let total_frames = (duration_seconds * sample_rate_hz).round() as usize;
    let mut output = vec![0.0_f32; total_frames * channels];

    let plan = plan(patch);
    let mut params = plan.params;
    let mut voice = Voice::new();
    voice.set_params(params);
    voice.prepare(sample_rate_hz, RENDER_BLOCK_FRAMES);

    let mut mono = vec![0.0_f32; RENDER_BLOCK_FRAMES];
    let mut automation_cursor = 0_usize;
    let mut note_cursor = 0_usize;
    let mut frame_offset = 0_usize;

    while frame_offset < total_frames {
        let block_frames = (total_frames - frame_offset).min(RENDER_BLOCK_FRAMES);
        let time_seconds = frame_offset as f32 / sample_rate_hz;

        plan.apply_automation(time_seconds, &mut params, &mut automation_cursor);
        plan.apply_notes(time_seconds, &mut params, &mut voice, &mut note_cursor);
        voice.set_params(params);
        voice.process(&mut mono[..block_frames]);

        for (index, sample) in mono[..block_frames].iter().enumerate() {
            let value = (*sample * plan.render_gain).clamp(-1.0, 1.0);
            let sample_index = (frame_offset + index) * channels;
            for channel in 0..channels {
                if let Some(slot) = output.get_mut(sample_index + channel) {
                    *slot = value;
                }
            }
        }

        frame_offset += block_frames;
    }

    Ok(output)
}

/// Renders a reference patch and writes it as a `f32` WAV file.
pub fn write_reference_wav(
    patch: ReferencePatch,
    config: AudioConfig,
    path: impl AsRef<std::path::Path>,
) -> Result<(), ReferenceError> {
    let samples = render_reference(patch, config)?;
    write_wav_f32(path, config.sample_rate_hz(), config.channels(), &samples)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use nacre_application::AudioConfig;

    use super::{REFERENCE_DURATION_SECONDS, ReferencePatch, plan, render_for_duration};
    use crate::hash_samples;

    #[test]
    fn every_patch_is_at_least_two_minutes() {
        let duration = REFERENCE_DURATION_SECONDS;
        for patch in ReferencePatch::ALL {
            assert!(duration >= 120.0);
            let plan = plan(patch);
            assert!(!plan.notes.is_empty(), "{} has no notes", patch.name());
            assert!(plan.render_gain > 0.0 && plan.render_gain <= 4.0);
        }
    }

    #[test]
    fn patch_names_parse_case_insensitively() {
        assert_eq!(ReferencePatch::parse("DRONE"), Some(ReferencePatch::Drone));
        assert_eq!(ReferencePatch::parse("bass"), Some(ReferencePatch::Bass));
        assert_eq!(ReferencePatch::parse("metal"), Some(ReferencePatch::Metal));
        assert_eq!(ReferencePatch::parse("other"), None);
    }

    #[test]
    fn short_renders_are_finite_bounded_and_deterministic() {
        let config = AudioConfig::new(48_000, 1, 256);
        for patch in ReferencePatch::ALL {
            let first = render_for_duration(patch, config, 5.0).expect("valid config");
            let second = render_for_duration(patch, config, 5.0).expect("valid config");

            assert_eq!(first.len(), 5 * 48_000);
            assert!(first.iter().all(|sample| sample.is_finite()));
            assert!(first.iter().all(|sample| sample.abs() <= 1.0));
            assert_eq!(hash_samples(&first), hash_samples(&second));
            assert!(
                first.iter().any(|sample| sample.abs() > 0.01),
                "{} is silent",
                patch.name()
            );
        }
    }
}
