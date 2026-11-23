//! Value sanitisation at the control/audio boundary.

/// Safe fallback used when a frequency cannot be interpreted.
pub(crate) const DEFAULT_FREQUENCY_HZ: f32 = 440.0;
/// Lowest frequency accepted by the control-plane parameter range.
pub(crate) const MIN_FREQUENCY_HZ: f32 = 0.1;
/// Highest frequency accepted by the control-plane parameter range.
pub(crate) const MAX_FREQUENCY_HZ: f32 = 20_000.0;

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
