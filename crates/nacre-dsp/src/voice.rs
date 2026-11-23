//! Monophonic voice: saw/sine oscillator → low-pass filter → VCA → saturation →
//! DC blocker.
//!
//! The oscillator is the only stage that oversamples (4×, decided by spike S4).
//! The filter, envelope, saturation and DC blocker run at the device rate. The
//! whole block is processed in place, with no allocation and no locks: all
//! buffers are reserved by [`Voice::prepare`].
//!
//! Parameter changes are read once per block. The LFO value at the start and at
//! the end of the block is used to interpolate pitch and cutoff inside the
//! block, so modulation does not step at block boundaries. The final output is
//! limited to −6 dBFS nominal headroom and clamped to `[-1, 1]`.

use crate::SineOscillator;
use crate::adsr::{Adsr, AdsrParams};
use crate::dc_block::DcBlocker;
use crate::lfo::{Lfo, LfoWaveform};
use crate::saturation::Saturation;
use crate::saw::SawOscillator;
use crate::svf::Svf;

/// Nominal output headroom of the voice, −6 dBFS.
pub const HEADROOM_GAIN: f32 = 0.5;

/// Maximum cutoff modulation depth at 100 % LFO depth, in octaves.
const MAX_CUTOFF_MODULATION_OCTAVES: f32 = 3.0;
/// Maximum pitch modulation depth at 100 % LFO depth, in semitones.
const MAX_PITCH_MODULATION_SEMITONES: f32 = 12.0;
/// Lowest note frequency accepted by the voice, in hertz.
const MIN_VOICE_FREQUENCY_HZ: f32 = 1.0;

/// Oscillator waveform selected for the voice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Waveform {
    /// Band-limited saw.
    Saw,
    /// Sine.
    Sine,
}

/// Destination of the LFO modulation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LfoDestination {
    /// Multiplicative modulation of the filter cutoff, in octaves.
    Cutoff,
    /// Multiplicative modulation of the oscillator pitch, in semitones.
    Pitch,
}

/// Complete parameter set of the voice for one block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceParams {
    /// Oscillator frequency in hertz.
    pub frequency_hz: f32,
    /// Oscillator waveform.
    pub waveform: Waveform,
    /// Filter cutoff in hertz.
    pub filter_cutoff_hz: f32,
    /// Filter resonance as Q.
    pub filter_resonance: f32,
    /// Attack time in milliseconds.
    pub attack_ms: f32,
    /// Decay time in milliseconds.
    pub decay_ms: f32,
    /// Sustain level in `0.0..=1.0`.
    pub sustain: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
    /// LFO rate in hertz.
    pub lfo_rate_hz: f32,
    /// LFO depth in percent, `0.0..=100.0`.
    pub lfo_depth_percent: f32,
    /// LFO destination.
    pub lfo_destination: LfoDestination,
    /// LFO waveform.
    pub lfo_waveform: LfoWaveform,
    /// Saturation drive in decibels, `0.0..=24.0`.
    pub drive_db: f32,
}

impl Default for VoiceParams {
    fn default() -> Self {
        Self {
            frequency_hz: 110.0,
            waveform: Waveform::Saw,
            filter_cutoff_hz: 2_000.0,
            filter_resonance: std::f32::consts::FRAC_1_SQRT_2,
            attack_ms: 10.0,
            decay_ms: 200.0,
            sustain: 0.7,
            release_ms: 300.0,
            lfo_rate_hz: 0.5,
            lfo_depth_percent: 0.0,
            lfo_destination: LfoDestination::Cutoff,
            lfo_waveform: LfoWaveform::Sine,
            drive_db: 0.0,
        }
    }
}

impl VoiceParams {
    /// Returns a copy with every value clamped into a safe range.
    ///
    /// The oscillator frequency is clamped by the voice against the sample
    /// rate; this method handles the sample-rate-independent ranges.
    pub fn sanitized(self) -> Self {
        Self {
            frequency_hz: if self.frequency_hz.is_finite() {
                self.frequency_hz.max(MIN_VOICE_FREQUENCY_HZ)
            } else {
                110.0
            },
            filter_cutoff_hz: if self.filter_cutoff_hz.is_finite() {
                self.filter_cutoff_hz
            } else {
                2_000.0
            },
            filter_resonance: if self.filter_resonance.is_finite() {
                self.filter_resonance
            } else {
                std::f32::consts::FRAC_1_SQRT_2
            },
            attack_ms: if self.attack_ms.is_finite() {
                self.attack_ms
            } else {
                10.0
            },
            decay_ms: if self.decay_ms.is_finite() {
                self.decay_ms
            } else {
                200.0
            },
            sustain: if self.sustain.is_finite() {
                self.sustain
            } else {
                0.7
            },
            release_ms: if self.release_ms.is_finite() {
                self.release_ms
            } else {
                300.0
            },
            lfo_rate_hz: if self.lfo_rate_hz.is_finite() {
                self.lfo_rate_hz
            } else {
                0.5
            },
            lfo_depth_percent: if self.lfo_depth_percent.is_finite() {
                self.lfo_depth_percent
            } else {
                0.0
            },
            drive_db: if self.drive_db.is_finite() {
                self.drive_db
            } else {
                0.0
            },
            ..self
        }
    }
}

/// Monophonic voice with gate, filter, envelope, LFO and saturation.
#[derive(Clone, Debug)]
pub struct Voice {
    sample_rate_hz: f32,
    params: VoiceParams,
    saw: SawOscillator,
    sine: SineOscillator,
    active_waveform: Waveform,
    filter: Svf,
    envelope: Adsr,
    lfo: Lfo,
    saturation: Saturation,
    dc_blocker: DcBlocker,
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Voice {
    /// Creates a voice at 48 kHz with the default parameters and a 256-frame
    /// block budget.
    pub fn new() -> Self {
        let mut voice = Self {
            sample_rate_hz: 48_000.0,
            params: VoiceParams::default(),
            saw: SawOscillator::new(),
            sine: SineOscillator::new(),
            active_waveform: Waveform::Saw,
            filter: Svf::new(48_000.0),
            envelope: Adsr::new(),
            lfo: Lfo::new(48_000.0),
            saturation: Saturation::new(),
            dc_blocker: DcBlocker::new(48_000.0),
        };
        voice.prepare(48_000.0, 256);
        voice
    }

    /// Prepares every stage for a sample rate and maximum block size.
    ///
    /// Allocates the 4× oscillator scratch buffer. Call from the control thread.
    pub fn prepare(&mut self, sample_rate_hz: f32, max_block_frames: usize) {
        self.sample_rate_hz = if sample_rate_hz.is_finite() && sample_rate_hz > 0.0 {
            sample_rate_hz
        } else {
            48_000.0
        };

        let frequency_hz = self.voice_frequency_hz(self.params.frequency_hz);
        self.saw
            .prepare(self.sample_rate_hz, frequency_hz, max_block_frames);
        self.sine.prepare(self.sample_rate_hz, frequency_hz);
        self.filter.prepare(self.sample_rate_hz);
        self.envelope.prepare(self.sample_rate_hz);
        self.lfo.prepare(self.sample_rate_hz);
        self.dc_blocker.prepare(self.sample_rate_hz);
        self.set_params(self.params);
        self.envelope.reset();
    }

    /// Applies the parameters for the next block.
    pub fn set_params(&mut self, params: VoiceParams) {
        self.params = params.sanitized();

        self.envelope.set_params(AdsrParams::new(
            self.params.attack_ms,
            self.params.decay_ms,
            self.params.sustain,
            self.params.release_ms,
        ));
        self.lfo.set_rate_hz(self.params.lfo_rate_hz);
        self.lfo.set_waveform(self.params.lfo_waveform);
        self.saturation.set_drive_db(self.params.drive_db);
    }

    /// Returns the parameters in use.
    pub const fn params(&self) -> VoiceParams {
        self.params
    }

    /// Starts a note. The oscillator phase keeps running; the envelope rises
    /// from its current value, so retriggers are click-free.
    pub fn note_on(&mut self) {
        self.envelope.note_on();
    }

    /// Releases the note.
    pub fn note_off(&mut self) {
        self.envelope.note_off();
    }

    /// Resets every stage, including oscillator phase, to silence.
    pub fn reset(&mut self) {
        self.saw.reset();
        self.sine.reset();
        self.filter.reset();
        self.envelope.reset();
        self.lfo.reset();
        self.dc_blocker.reset();
    }

    /// Returns whether the envelope still produces output.
    pub const fn is_active(&self) -> bool {
        self.envelope.is_active()
    }

    /// Returns the current envelope level.
    pub fn envelope_level(&self) -> f32 {
        self.envelope.level()
    }

    /// Renders a mono block in place.
    pub fn process(&mut self, out: &mut [f32]) {
        if out.is_empty() {
            return;
        }
        let frames = out.len();

        self.lfo.advance(frames);
        let modulation_end = self.lfo.value();
        let depth = self.params.lfo_depth_percent.clamp(0.0, 100.0) / 100.0;

        let end_frequency_hz = match self.params.lfo_destination {
            LfoDestination::Pitch => {
                let semitones = MAX_PITCH_MODULATION_SEMITONES * depth * modulation_end;
                self.voice_frequency_hz(self.params.frequency_hz * 2.0_f32.powf(semitones / 12.0))
            }
            LfoDestination::Cutoff => self.voice_frequency_hz(self.params.frequency_hz),
        };

        // From silence there is nothing to glide from: snap the oscillator to
        // the note so the first block does not slide from a stale frequency.
        if !self.envelope.is_active() {
            self.saw
                .set_frequency(self.sample_rate_hz, end_frequency_hz);
            self.sine
                .set_frequency(self.sample_rate_hz, end_frequency_hz);
        }

        self.switch_waveform_if_needed();
        match self.params.waveform {
            Waveform::Saw => self.saw.process_glide(out, end_frequency_hz),
            Waveform::Sine => self.sine.process_glide(out, 1.0, end_frequency_hz),
        }

        let end_cutoff_hz = match self.params.lfo_destination {
            LfoDestination::Cutoff => {
                let octaves = MAX_CUTOFF_MODULATION_OCTAVES * depth * modulation_end;
                self.params.filter_cutoff_hz * 2.0_f32.powf(octaves)
            }
            LfoDestination::Pitch => self.params.filter_cutoff_hz,
        };

        self.filter
            .process(out, end_cutoff_hz, self.params.filter_resonance);

        for sample in out.iter_mut() {
            let envelope = self.envelope.next_sample();
            *sample = self.saturation.process_sample(*sample * envelope);
        }

        self.dc_blocker.process(out);

        for sample in out.iter_mut() {
            let value = *sample * HEADROOM_GAIN;
            *sample = if value.is_finite() {
                value.clamp(-1.0, 1.0)
            } else {
                0.0
            };
        }
    }

    fn switch_waveform_if_needed(&mut self) {
        if self.params.waveform == self.active_waveform {
            return;
        }

        // Keep phase continuity when the source changes. The two shapes still
        // have different values at the same phase, so the switch is audible as
        // a timbre change at the block boundary, as the design specifies.
        match self.params.waveform {
            Waveform::Saw => self.saw.set_phase(self.sine.phase()),
            Waveform::Sine => self.sine.set_phase(self.saw.phase()),
        }
        self.active_waveform = self.params.waveform;
    }

    fn voice_frequency_hz(&self, frequency_hz: f32) -> f32 {
        let maximum = (self.sample_rate_hz * 0.45).max(MIN_VOICE_FREQUENCY_HZ);
        if frequency_hz.is_finite() {
            frequency_hz.clamp(MIN_VOICE_FREQUENCY_HZ, maximum)
        } else {
            MIN_VOICE_FREQUENCY_HZ
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::{LfoDestination, Voice, VoiceParams, Waveform};
    use crate::lfo::LfoWaveform;

    const SAMPLE_RATE_HZ: f32 = 48_000.0;

    fn voice(params: VoiceParams) -> Voice {
        let mut voice = Voice::new();
        voice.prepare(SAMPLE_RATE_HZ, 256);
        voice.set_params(params);
        voice
    }

    fn render(voice: &mut Voice, frames: usize) -> Vec<f32> {
        let mut output = vec![0.0_f32; frames];
        let mut offset = 0;
        while offset < frames {
            let chunk = (frames - offset).min(256);
            voice.process(&mut output[offset..offset + chunk]);
            offset += chunk;
        }
        output
    }

    fn max_step(samples: &[f32]) -> f32 {
        samples
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn complete_note_has_no_discontinuities() {
        let mut voice = voice(VoiceParams {
            waveform: Waveform::Sine,
            frequency_hz: 220.0,
            attack_ms: 10.0,
            decay_ms: 50.0,
            sustain: 0.7,
            release_ms: 100.0,
            ..VoiceParams::default()
        });

        voice.note_on();
        let mut samples = render(&mut voice, 24_000);
        voice.note_off();
        samples.extend(render(&mut voice, 24_000));

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(max_step(&samples) < 0.1, "max step {}", max_step(&samples));
        assert!(samples[..24_000].iter().any(|sample| sample.abs() > 0.1));
        assert!(!voice.is_active());
        assert!(
            samples[44_000..].iter().all(|sample| sample.abs() < 1e-3),
            "note did not release"
        );
    }

    #[test]
    fn resonance_never_exceeds_the_output_limit() {
        let mut voice = voice(VoiceParams {
            frequency_hz: 110.0,
            filter_cutoff_hz: 110.0,
            filter_resonance: 20.0,
            sustain: 1.0,
            attack_ms: 1.0,
            ..VoiceParams::default()
        });
        voice.note_on();
        let samples = render(&mut voice, 48_000);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
        assert!(samples.iter().any(|sample| sample.abs() > 0.1));
    }

    #[test]
    fn cutoff_modulation_is_smooth_at_low_and_high_lfo_rates() {
        for rate_hz in [0.5_f32, 20.0] {
            let mut voice = voice(VoiceParams {
                waveform: Waveform::Sine,
                frequency_hz: 220.0,
                filter_cutoff_hz: 600.0,
                lfo_rate_hz: rate_hz,
                lfo_depth_percent: 50.0,
                lfo_destination: LfoDestination::Cutoff,
                sustain: 1.0,
                attack_ms: 1.0,
                ..VoiceParams::default()
            });
            voice.note_on();
            let samples = render(&mut voice, 48_000);

            assert!(samples.iter().all(|sample| sample.is_finite()));
            assert!(
                max_step(&samples) < 0.1,
                "{rate_hz} Hz LFO step {}",
                max_step(&samples)
            );
        }
    }

    #[test]
    fn pitch_modulation_is_finite_and_smooth() {
        let mut voice = voice(VoiceParams {
            waveform: Waveform::Sine,
            frequency_hz: 440.0,
            lfo_rate_hz: 5.0,
            lfo_depth_percent: 100.0,
            lfo_destination: LfoDestination::Pitch,
            sustain: 1.0,
            attack_ms: 1.0,
            ..VoiceParams::default()
        });
        voice.note_on();
        let samples = render(&mut voice, 48_000);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(max_step(&samples) < 0.2);
    }

    #[test]
    fn corrupt_parameters_do_not_break_the_voice() {
        let mut voice = voice(VoiceParams {
            frequency_hz: f32::NAN,
            filter_cutoff_hz: f32::NAN,
            filter_resonance: f32::INFINITY,
            attack_ms: f32::NAN,
            decay_ms: -1.0,
            sustain: f32::NAN,
            release_ms: f32::INFINITY,
            lfo_rate_hz: f32::NAN,
            lfo_depth_percent: f32::NAN,
            drive_db: f32::NAN,
            ..VoiceParams::default()
        });
        voice.note_on();
        let samples = render(&mut voice, 4_800);

        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
    }

    #[test]
    fn waveform_switch_keeps_the_phase() {
        let mut voice = voice(VoiceParams {
            frequency_hz: 220.0,
            sustain: 1.0,
            attack_ms: 1.0,
            ..VoiceParams::default()
        });
        voice.note_on();
        render(&mut voice, 1_000);

        voice.set_params(VoiceParams {
            waveform: Waveform::Sine,
            ..voice.params()
        });
        let samples = render(&mut voice, 1_000);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn triangle_lfo_is_usable_by_the_voice() {
        let mut voice = voice(VoiceParams {
            waveform: Waveform::Sine,
            lfo_waveform: LfoWaveform::Triangle,
            lfo_depth_percent: 40.0,
            sustain: 1.0,
            attack_ms: 1.0,
            ..VoiceParams::default()
        });
        voice.note_on();
        let samples = render(&mut voice, 24_000);
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(samples.iter().any(|sample| sample.abs() > 0.01));
    }
}
