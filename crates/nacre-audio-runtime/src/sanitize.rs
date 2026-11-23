//! Value sanitisation at the control/audio boundary.

use nacre_application::{LfoDestination, LfoShape, VoiceParameter, Waveform};

/// Safe fallback used when a frequency cannot be interpreted.
pub(crate) const DEFAULT_FREQUENCY_HZ: f32 = 440.0;
/// Lowest frequency accepted by the control-plane parameter range.
pub(crate) const MIN_FREQUENCY_HZ: f32 = 0.1;
/// Highest frequency accepted by the control-plane parameter range.
pub(crate) const MAX_FREQUENCY_HZ: f32 = 20_000.0;

/// Default filter cutoff, in hertz.
pub(crate) const DEFAULT_FILTER_CUTOFF_HZ: f32 = 2_000.0;
/// Default filter resonance.
pub(crate) const DEFAULT_FILTER_RESONANCE: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// Default envelope attack, in milliseconds.
pub(crate) const DEFAULT_ATTACK_MS: f32 = 10.0;
/// Default envelope decay, in milliseconds.
pub(crate) const DEFAULT_DECAY_MS: f32 = 200.0;
/// Default envelope sustain level.
pub(crate) const DEFAULT_SUSTAIN: f32 = 0.7;
/// Default envelope release, in milliseconds.
pub(crate) const DEFAULT_RELEASE_MS: f32 = 300.0;
/// Default LFO rate, in hertz.
pub(crate) const DEFAULT_LFO_RATE_HZ: f32 = 0.5;
/// Default LFO depth, in percent.
pub(crate) const DEFAULT_LFO_DEPTH_PERCENT: f32 = 0.0;
/// Default saturation drive, in decibels.
pub(crate) const DEFAULT_DRIVE_DB: f32 = 0.0;

pub(crate) fn sanitize_frequency_hz(candidate: f32) -> f32 {
    if candidate.is_finite() && candidate > 0.0 {
        candidate.clamp(MIN_FREQUENCY_HZ, MAX_FREQUENCY_HZ)
    } else {
        DEFAULT_FREQUENCY_HZ
    }
}

pub(crate) fn sanitize_gain(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(crate) fn sanitize_frequency_with_fallback(candidate: f32, fallback: f32) -> f32 {
    if candidate.is_finite() && candidate > 0.0 {
        candidate.clamp(MIN_FREQUENCY_HZ, MAX_FREQUENCY_HZ)
    } else {
        sanitize_frequency_hz(fallback)
    }
}

/// Encodes the waveform in a shared atomic slot.
pub(crate) const fn waveform_code(waveform: Waveform) -> u32 {
    match waveform {
        Waveform::Saw => 0,
        Waveform::Sine => 1,
    }
}

/// Encodes the LFO shape in a shared atomic slot.
pub(crate) const fn lfo_shape_code(shape: LfoShape) -> u32 {
    match shape {
        LfoShape::Sine => 0,
        LfoShape::Triangle => 1,
    }
}

/// Encodes the LFO destination in a shared atomic slot.
pub(crate) const fn lfo_destination_code(destination: LfoDestination) -> u32 {
    match destination {
        LfoDestination::Cutoff => 0,
        LfoDestination::Pitch => 1,
    }
}

/// Clamps a voice parameter to its authoritative range.
///
/// A non-finite value keeps the documented default instead of poisoning the
/// DSP state.
pub(crate) fn sanitize_voice_parameter(parameter: VoiceParameter, value: f32) -> f32 {
    let (min, max) = parameter.range();
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        match parameter {
            VoiceParameter::FilterCutoff => DEFAULT_FILTER_CUTOFF_HZ,
            VoiceParameter::FilterResonance => DEFAULT_FILTER_RESONANCE,
            VoiceParameter::Attack => DEFAULT_ATTACK_MS,
            VoiceParameter::Decay => DEFAULT_DECAY_MS,
            VoiceParameter::Sustain => DEFAULT_SUSTAIN,
            VoiceParameter::Release => DEFAULT_RELEASE_MS,
            VoiceParameter::LfoRate => DEFAULT_LFO_RATE_HZ,
            VoiceParameter::LfoDepth => DEFAULT_LFO_DEPTH_PERCENT,
            VoiceParameter::Drive => DEFAULT_DRIVE_DB,
        }
    }
}

#[cfg(test)]
mod tests {
    use nacre_application::VoiceParameter;

    use super::{
        DEFAULT_ATTACK_MS, DEFAULT_FILTER_CUTOFF_HZ, DEFAULT_FILTER_RESONANCE,
        sanitize_voice_parameter,
    };

    #[test]
    fn voice_parameters_are_clamped_to_their_ranges() {
        assert_eq!(
            sanitize_voice_parameter(VoiceParameter::FilterCutoff, 100_000.0),
            20_000.0
        );
        assert_eq!(
            sanitize_voice_parameter(VoiceParameter::FilterCutoff, -1.0),
            20.0
        );
        assert_eq!(sanitize_voice_parameter(VoiceParameter::Sustain, 2.0), 1.0);
        assert_eq!(sanitize_voice_parameter(VoiceParameter::Drive, 30.0), 24.0);
    }

    #[test]
    fn non_finite_voice_parameters_use_documented_defaults() {
        assert_eq!(
            sanitize_voice_parameter(VoiceParameter::FilterCutoff, f32::NAN),
            DEFAULT_FILTER_CUTOFF_HZ
        );
        assert_eq!(
            sanitize_voice_parameter(VoiceParameter::FilterResonance, f32::INFINITY),
            DEFAULT_FILTER_RESONANCE
        );
        assert_eq!(
            sanitize_voice_parameter(VoiceParameter::Attack, f32::NEG_INFINITY),
            DEFAULT_ATTACK_MS
        );
    }
}
