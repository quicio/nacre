//! Atomics shared between the control plane and the audio callback.
//!
//! Every field is a lock-free atomic. The callback only performs relaxed
//! loads/stores, except for the lifecycle flags that use acquire/release so the
//! control plane observes fade completion reliably.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use nacre_application::{AudioHealth, VoiceParameter};
use nacre_domain::SampleTime;

use crate::sanitize::{
    DEFAULT_ATTACK_MS, DEFAULT_DECAY_MS, DEFAULT_DRIVE_DB, DEFAULT_FILTER_CUTOFF_HZ,
    DEFAULT_FILTER_RESONANCE, DEFAULT_LFO_DEPTH_PERCENT, DEFAULT_LFO_RATE_HZ, DEFAULT_RELEASE_MS,
    DEFAULT_SUSTAIN, sanitize_frequency_hz, sanitize_gain, sanitize_voice_parameter,
};

pub(crate) struct SharedState {
    frequency_hz_bits: AtomicU32,
    gain_bits: AtomicU32,
    waveform: AtomicU32,
    lfo_shape: AtomicU32,
    lfo_destination: AtomicU32,
    filter_cutoff_bits: AtomicU32,
    filter_resonance_bits: AtomicU32,
    attack_ms_bits: AtomicU32,
    decay_ms_bits: AtomicU32,
    sustain_bits: AtomicU32,
    release_ms_bits: AtomicU32,
    lfo_rate_bits: AtomicU32,
    lfo_depth_bits: AtomicU32,
    drive_db_bits: AtomicU32,
    frames: AtomicU64,
    xruns: AtomicU32,
    dropped_commands: AtomicU32,
    rms_bits: AtomicU32,
    peak_bits: AtomicU32,
    cpu_load_bits: AtomicU32,
    stop_requested: AtomicBool,
    fade_finished: AtomicBool,
}

impl SharedState {
    pub(crate) fn new(frequency_hz: f32, linear_gain: f32) -> Self {
        Self {
            frequency_hz_bits: AtomicU32::new(sanitize_frequency_hz(frequency_hz).to_bits()),
            gain_bits: AtomicU32::new(sanitize_gain(linear_gain).to_bits()),
            waveform: AtomicU32::new(0),
            lfo_shape: AtomicU32::new(0),
            lfo_destination: AtomicU32::new(0),
            filter_cutoff_bits: AtomicU32::new(DEFAULT_FILTER_CUTOFF_HZ.to_bits()),
            filter_resonance_bits: AtomicU32::new(DEFAULT_FILTER_RESONANCE.to_bits()),
            attack_ms_bits: AtomicU32::new(DEFAULT_ATTACK_MS.to_bits()),
            decay_ms_bits: AtomicU32::new(DEFAULT_DECAY_MS.to_bits()),
            sustain_bits: AtomicU32::new(DEFAULT_SUSTAIN.to_bits()),
            release_ms_bits: AtomicU32::new(DEFAULT_RELEASE_MS.to_bits()),
            lfo_rate_bits: AtomicU32::new(DEFAULT_LFO_RATE_HZ.to_bits()),
            lfo_depth_bits: AtomicU32::new(DEFAULT_LFO_DEPTH_PERCENT.to_bits()),
            drive_db_bits: AtomicU32::new(DEFAULT_DRIVE_DB.to_bits()),
            frames: AtomicU64::new(0),
            xruns: AtomicU32::new(0),
            dropped_commands: AtomicU32::new(0),
            rms_bits: AtomicU32::new(0.0_f32.to_bits()),
            peak_bits: AtomicU32::new(0.0_f32.to_bits()),
            cpu_load_bits: AtomicU32::new(0.0_f32.to_bits()),
            stop_requested: AtomicBool::new(false),
            fade_finished: AtomicBool::new(false),
        }
    }

    pub(crate) fn reset_counters(&self) {
        self.frames.store(0, Ordering::Relaxed);
        self.xruns.store(0, Ordering::Relaxed);
        self.dropped_commands.store(0, Ordering::Relaxed);
        self.rms_bits.store(0.0_f32.to_bits(), Ordering::Relaxed);
        self.peak_bits.store(0.0_f32.to_bits(), Ordering::Relaxed);
        self.cpu_load_bits
            .store(0.0_f32.to_bits(), Ordering::Relaxed);
        self.stop_requested.store(false, Ordering::Release);
        self.fade_finished.store(false, Ordering::Release);
    }

    pub(crate) fn frequency_hz(&self) -> f32 {
        f32::from_bits(self.frequency_hz_bits.load(Ordering::Relaxed))
    }

    pub(crate) fn set_frequency_hz(&self, frequency_hz: f32) {
        let value = sanitize_frequency_hz(frequency_hz);
        self.frequency_hz_bits
            .store(value.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn gain(&self) -> f32 {
        f32::from_bits(self.gain_bits.load(Ordering::Relaxed))
    }

    pub(crate) fn set_gain(&self, linear_gain: f32) {
        let value = sanitize_gain(linear_gain);
        self.gain_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    /// Waveform code: 0 = saw, 1 = sine.
    pub(crate) fn waveform(&self) -> u32 {
        self.waveform.load(Ordering::Relaxed)
    }

    pub(crate) fn set_waveform(&self, waveform: u32) {
        self.waveform.store(waveform, Ordering::Relaxed);
    }

    /// LFO shape code: 0 = sine, 1 = triangle.
    pub(crate) fn lfo_shape(&self) -> u32 {
        self.lfo_shape.load(Ordering::Relaxed)
    }

    pub(crate) fn set_lfo_shape(&self, shape: u32) {
        self.lfo_shape.store(shape, Ordering::Relaxed);
    }

    /// LFO destination code: 0 = cutoff, 1 = pitch.
    pub(crate) fn lfo_destination(&self) -> u32 {
        self.lfo_destination.load(Ordering::Relaxed)
    }

    pub(crate) fn set_lfo_destination(&self, destination: u32) {
        self.lfo_destination.store(destination, Ordering::Relaxed);
    }

    pub(crate) fn voice_parameter(&self, parameter: VoiceParameter) -> f32 {
        f32::from_bits(self.voice_slot(parameter).load(Ordering::Relaxed))
    }

    pub(crate) fn set_voice_parameter(&self, parameter: VoiceParameter, value: f32) {
        let sanitized = sanitize_voice_parameter(parameter, value);
        self.voice_slot(parameter)
            .store(sanitized.to_bits(), Ordering::Relaxed);
    }

    fn voice_slot(&self, parameter: VoiceParameter) -> &AtomicU32 {
        match parameter {
            VoiceParameter::FilterCutoff => &self.filter_cutoff_bits,
            VoiceParameter::FilterResonance => &self.filter_resonance_bits,
            VoiceParameter::Attack => &self.attack_ms_bits,
            VoiceParameter::Decay => &self.decay_ms_bits,
            VoiceParameter::Sustain => &self.sustain_bits,
            VoiceParameter::Release => &self.release_ms_bits,
            VoiceParameter::LfoRate => &self.lfo_rate_bits,
            VoiceParameter::LfoDepth => &self.lfo_depth_bits,
            VoiceParameter::Drive => &self.drive_db_bits,
        }
    }

    pub(crate) fn add_frames(&self, frames: u64) {
        self.frames.fetch_add(frames, Ordering::Relaxed);
    }

    pub(crate) fn add_dropped_command(&self) {
        self.dropped_commands.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn add_xrun(&self) {
        self.xruns.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn store_meter(&self, rms: f32, peak: f32) {
        let rms = if rms.is_finite() {
            rms.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let peak = if peak.is_finite() {
            peak.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.rms_bits.store(rms.to_bits(), Ordering::Relaxed);
        self.peak_bits.store(peak.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn store_cpu_load(&self, cpu_load: f32) {
        let value = if cpu_load.is_finite() {
            cpu_load.clamp(0.0, 100.0)
        } else {
            0.0
        };
        self.cpu_load_bits.store(value.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
    }

    pub(crate) fn take_stop_request(&self) -> bool {
        self.stop_requested.swap(false, Ordering::AcqRel)
    }

    pub(crate) fn mark_fade_finished(&self) {
        self.fade_finished.store(true, Ordering::Release);
    }

    pub(crate) fn clear_fade_finished(&self) {
        self.fade_finished.store(false, Ordering::Release);
    }

    pub(crate) fn fade_finished(&self) -> bool {
        self.fade_finished.load(Ordering::Acquire)
    }

    pub(crate) fn health(&self) -> AudioHealth {
        AudioHealth {
            frames: SampleTime::new(self.frames.load(Ordering::Relaxed)),
            rms: f32::from_bits(self.rms_bits.load(Ordering::Relaxed)),
            peak: f32::from_bits(self.peak_bits.load(Ordering::Relaxed)),
            xruns: self.xruns.load(Ordering::Relaxed),
            dropped_commands: self.dropped_commands.load(Ordering::Relaxed),
            cpu_load: f32::from_bits(self.cpu_load_bits.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;

    use nacre_application::VoiceParameter;
    use nacre_domain::SampleTime;

    use super::SharedState;
    use crate::sanitize::DEFAULT_ATTACK_MS;

    #[test]
    fn parameter_slots_cross_threads_without_unsafe() {
        let shared = Arc::new(SharedState::new(440.0, 0.5));
        let writer = Arc::clone(&shared);

        let handle = thread::spawn(move || {
            writer.set_frequency_hz(880.0);
            writer.set_gain(0.25);
        });
        handle.join().expect("writer thread should not panic");

        assert_eq!(shared.frequency_hz(), 880.0);
        assert_eq!(shared.gain(), 0.25);
    }

    #[test]
    fn invalid_slots_are_sanitized() {
        let shared = SharedState::new(440.0, 0.5);

        shared.set_frequency_hz(f32::NAN);
        shared.set_gain(f32::INFINITY);

        assert_eq!(shared.frequency_hz(), 440.0);
        assert_eq!(shared.gain(), 0.0);
    }

    #[test]
    fn voice_parameter_slots_are_sanitized_independently() {
        let shared = SharedState::new(440.0, 0.5);

        shared.set_voice_parameter(VoiceParameter::FilterCutoff, 100_000.0);
        shared.set_voice_parameter(VoiceParameter::Attack, f32::NAN);
        shared.set_waveform(1);
        shared.set_lfo_destination(1);
        shared.set_lfo_shape(1);

        assert_eq!(
            shared.voice_parameter(VoiceParameter::FilterCutoff),
            20_000.0
        );
        assert_eq!(
            shared.voice_parameter(VoiceParameter::Attack),
            DEFAULT_ATTACK_MS
        );
        assert_eq!(shared.waveform(), 1);
        assert_eq!(shared.lfo_destination(), 1);
        assert_eq!(shared.lfo_shape(), 1);
    }

    #[test]
    fn health_reads_counters_and_frames() {
        let shared = SharedState::new(440.0, 0.5);
        shared.add_frames(512);
        shared.add_xrun();
        shared.add_dropped_command();
        shared.store_meter(0.5, 0.75);
        shared.store_cpu_load(0.25);

        let health = shared.health();
        assert_eq!(health.frames, SampleTime::new(512));
        assert_eq!(health.xruns, 1);
        assert_eq!(health.dropped_commands, 1);
        assert_eq!(health.rms, 0.5);
        assert_eq!(health.peak, 0.75);
        assert_eq!(health.cpu_load, 0.25);
    }
}
