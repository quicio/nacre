//! Sample-accurate ADSR envelope with smooth curves and click-free retrigger.
//!
//! Attack uses a one-pole curve normalised so that it reaches exactly 1.0 after
//! `attack_ms`. Decay and release use one-pole curves that reach 99.9 % of
//! their target after their nominal time (three-figure settling), then snap by
//! less than 0.1 % of the remaining distance.
//!
//! Retrigger always starts the attack from the current envelope value, never
//! from zero, so a new note during release does not produce a click.

/// Lowest accepted envelope time, in milliseconds.
pub const MIN_TIME_MS: f32 = 1.0;
/// Highest accepted envelope time, in milliseconds.
pub const MAX_TIME_MS: f32 = 10_000.0;

/// Natural logarithm of 1000: after this many time constants a one-pole has
/// settled to 99.9 % of its target.
const SETTLING_FACTOR: f64 = 6.907_755_278_982_137;

/// One-pole smoothing time used when the sustain level changes, in milliseconds.
const SUSTAIN_SMOOTHING_MS: f32 = 10.0;

/// Envelope segment currently running.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdsrStage {
    /// No note is active; the output is zero.
    Idle,
    /// Rising towards full level.
    Attack,
    /// Falling towards the sustain level.
    Decay,
    /// Holding the sustain level while the note is held.
    Sustain,
    /// Falling towards zero after note-off.
    Release,
}

/// ADSR times and levels, in milliseconds and linear amplitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdsrParams {
    /// Attack time in milliseconds.
    pub attack_ms: f32,
    /// Decay time in milliseconds.
    pub decay_ms: f32,
    /// Sustain level in `0.0..=1.0`.
    pub sustain: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack_ms: 10.0,
            decay_ms: 200.0,
            sustain: 0.7,
            release_ms: 300.0,
        }
    }
}

impl AdsrParams {
    /// Creates parameters from raw values, sanitising each one.
    pub fn new(attack_ms: f32, decay_ms: f32, sustain: f32, release_ms: f32) -> Self {
        Self {
            attack_ms,
            decay_ms,
            sustain,
            release_ms,
        }
        .sanitized()
    }

    /// Returns a copy with every value clamped into its accepted range.
    pub fn sanitized(self) -> Self {
        Self {
            attack_ms: sanitize_time_ms(self.attack_ms),
            decay_ms: sanitize_time_ms(self.decay_ms),
            release_ms: sanitize_time_ms(self.release_ms),
            sustain: sanitize_sustain(self.sustain),
        }
    }
}

/// ADSR envelope generator.
#[derive(Clone, Debug)]
pub struct Adsr {
    sample_rate_hz: f32,
    params: AdsrParams,
    stage: AdsrStage,
    level: f64,
    stage_frames: u64,
    attack_frames: u64,
    decay_frames: u64,
    release_frames: u64,
    attack_target: f64,
    attack_multiplier: f64,
    decay_multiplier: f64,
    release_multiplier: f64,
    sustain_multiplier: f64,
}

impl Adsr {
    /// Creates an idle envelope at 48 kHz with the default parameters.
    pub fn new() -> Self {
        let mut envelope = Self {
            sample_rate_hz: 48_000.0,
            params: AdsrParams::default(),
            stage: AdsrStage::Idle,
            level: 0.0,
            stage_frames: 0,
            attack_frames: 1,
            decay_frames: 1,
            release_frames: 1,
            attack_target: 1.0,
            attack_multiplier: 0.0,
            decay_multiplier: 0.0,
            release_multiplier: 0.0,
            sustain_multiplier: 0.0,
        };
        envelope.prepare(48_000.0);
        envelope
    }

    /// Prepares the envelope for a sample rate, resetting to idle.
    pub fn prepare(&mut self, sample_rate_hz: f32) {
        self.sample_rate_hz = if sample_rate_hz.is_finite() && sample_rate_hz > 0.0 {
            sample_rate_hz
        } else {
            48_000.0
        };
        self.reset();
        self.recompute_coefficients();
    }

    /// Applies new parameters. Progress is kept unless a segment is restarted.
    pub fn set_params(&mut self, params: AdsrParams) {
        let params = params.sanitized();
        if params == self.params {
            return;
        }

        self.params = params;
        let restart_attack = self.stage == AdsrStage::Attack;
        let restart_release = self.stage == AdsrStage::Release;
        self.recompute_coefficients();

        if restart_attack {
            self.begin_attack();
        }
        if restart_release {
            self.begin_release();
        }
    }

    /// Returns the current parameters.
    pub const fn params(&self) -> AdsrParams {
        self.params
    }

    /// Starts a new note. The attack begins at the current level.
    pub fn note_on(&mut self) {
        self.begin_attack();
    }

    /// Starts the release segment from the current level.
    pub fn note_off(&mut self) {
        if self.stage != AdsrStage::Idle {
            self.begin_release();
        }
    }

    /// Immediately silences the envelope.
    pub fn reset(&mut self) {
        self.stage = AdsrStage::Idle;
        self.level = 0.0;
        self.stage_frames = 0;
    }

    /// Returns the current stage.
    pub const fn stage(&self) -> AdsrStage {
        self.stage
    }

    /// Returns whether the envelope still produces non-zero output.
    pub const fn is_active(&self) -> bool {
        !matches!(self.stage, AdsrStage::Idle)
    }

    /// Returns the current envelope level in `0.0..=1.0`.
    pub fn level(&self) -> f32 {
        self.level.clamp(0.0, 1.0) as f32
    }

    /// Advances one sample and returns the new level.
    pub fn next_sample(&mut self) -> f32 {
        match self.stage {
            AdsrStage::Idle => {
                self.level = 0.0;
            }
            AdsrStage::Attack => {
                self.level =
                    self.attack_target + (self.level - self.attack_target) * self.attack_multiplier;
                self.stage_frames += 1;
                if self.stage_frames >= self.attack_frames {
                    self.level = 1.0;
                    self.enter_decay();
                }
            }
            AdsrStage::Decay => {
                let sustain = f64::from(self.params.sustain);
                self.level = sustain + (self.level - sustain) * self.decay_multiplier;
                self.stage_frames += 1;
                if self.stage_frames >= self.decay_frames {
                    self.level = sustain;
                    self.stage = AdsrStage::Sustain;
                    self.stage_frames = 0;
                }
            }
            AdsrStage::Sustain => {
                let sustain = f64::from(self.params.sustain);
                self.level += (sustain - self.level) * self.sustain_multiplier;
            }
            AdsrStage::Release => {
                self.level *= self.release_multiplier;
                self.stage_frames += 1;
                if self.stage_frames >= self.release_frames || self.level < 1e-6 {
                    self.level = 0.0;
                    self.stage = AdsrStage::Idle;
                    self.stage_frames = 0;
                }
            }
        }

        if self.level.is_finite() {
            self.level.clamp(0.0, 1.0) as f32
        } else {
            self.level = 0.0;
            0.0
        }
    }

    fn begin_attack(&mut self) {
        self.stage = AdsrStage::Attack;
        self.stage_frames = 0;
        self.attack_frames = frames_for(self.params.attack_ms, self.sample_rate_hz);
        self.attack_multiplier = (-SETTLING_FACTOR / self.attack_frames as f64).exp();
        // Virtual target so the one-pole lands exactly on 1.0 after the attack
        // time, whatever the starting level is.
        let settle = (-SETTLING_FACTOR).exp();
        self.attack_target = (1.0 - self.level * settle) / (1.0 - settle);
    }

    fn enter_decay(&mut self) {
        if self.params.sustain >= 1.0 {
            self.stage = AdsrStage::Sustain;
            self.stage_frames = 0;
            self.level = 1.0;
        } else {
            self.stage = AdsrStage::Decay;
            self.stage_frames = 0;
        }
    }

    fn begin_release(&mut self) {
        self.stage = AdsrStage::Release;
        self.stage_frames = 0;
        self.release_frames = frames_for(self.params.release_ms, self.sample_rate_hz);
        self.release_multiplier = (-SETTLING_FACTOR / self.release_frames as f64).exp();
    }

    fn recompute_coefficients(&mut self) {
        let sample_rate_hz = self.sample_rate_hz;
        self.attack_frames = frames_for(self.params.attack_ms, sample_rate_hz);
        self.decay_frames = frames_for(self.params.decay_ms, sample_rate_hz);
        self.release_frames = frames_for(self.params.release_ms, sample_rate_hz);
        self.attack_multiplier = (-SETTLING_FACTOR / self.attack_frames as f64).exp();
        self.decay_multiplier = (-SETTLING_FACTOR / self.decay_frames as f64).exp();
        self.release_multiplier = (-SETTLING_FACTOR / self.release_frames as f64).exp();
        let smoothing_frames = (SUSTAIN_SMOOTHING_MS / 1_000.0) * sample_rate_hz;
        self.sustain_multiplier = 1.0 - (-1.0 / f64::from(smoothing_frames.max(1.0))).exp();

        if self.stage == AdsrStage::Attack {
            self.attack_target =
                (1.0 - self.level * (-SETTLING_FACTOR).exp()) / (1.0 - (-SETTLING_FACTOR).exp());
        }
    }
}

impl Default for Adsr {
    fn default() -> Self {
        Self::new()
    }
}

fn frames_for(time_ms: f32, sample_rate_hz: f32) -> u64 {
    let frames = (time_ms / 1_000.0 * sample_rate_hz).round();
    if frames.is_finite() && frames >= 1.0 {
        frames as u64
    } else {
        1
    }
}

fn sanitize_time_ms(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate.clamp(MIN_TIME_MS, MAX_TIME_MS)
    } else {
        MIN_TIME_MS
    }
}

fn sanitize_sustain(candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::{Adsr, AdsrParams, AdsrStage};

    const SAMPLE_RATE_HZ: f32 = 48_000.0;

    fn envelope() -> Adsr {
        let mut envelope = Adsr::new();
        envelope.prepare(SAMPLE_RATE_HZ);
        envelope.set_params(AdsrParams::new(10.0, 100.0, 0.6, 200.0));
        envelope
    }

    fn run(envelope: &mut Adsr, frames: usize) -> Vec<f32> {
        (0..frames).map(|_| envelope.next_sample()).collect()
    }

    #[test]
    fn attack_reaches_unity_in_the_nominal_time() {
        let mut envelope = envelope();
        envelope.note_on();

        let samples = run(&mut envelope, 480);
        assert_eq!(envelope.stage(), AdsrStage::Decay);
        assert!((samples[479] - 1.0).abs() < 1e-6);
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1] + 1e-6));
    }

    #[test]
    fn decay_reaches_sustain_in_the_nominal_time() {
        let mut envelope = envelope();
        envelope.note_on();

        run(&mut envelope, 480);
        let samples = run(&mut envelope, 4_800);

        assert_eq!(envelope.stage(), AdsrStage::Sustain);
        assert!((envelope.level() - 0.6).abs() < 1e-3);
        assert!(samples.windows(2).all(|pair| pair[0] >= pair[1] - 1e-6));
    }

    #[test]
    fn sustain_level_is_stable_for_ten_seconds() {
        let mut envelope = envelope();
        envelope.note_on();
        run(&mut envelope, 480 + 4_800);
        let samples = run(&mut envelope, 480_000);

        assert_eq!(envelope.stage(), AdsrStage::Sustain);
        for sample in samples.iter().skip(480) {
            assert!((sample - 0.6).abs() < 0.01, "sustain drifted to {sample}");
        }
    }

    #[test]
    fn release_falls_to_zero_in_the_nominal_time() {
        let mut envelope = envelope();
        envelope.note_on();
        run(&mut envelope, 5_280);
        let sustain_level = envelope.level();

        envelope.note_off();
        let samples = run(&mut envelope, 9_600);

        assert_eq!(envelope.stage(), AdsrStage::Idle);
        assert!(samples.windows(2).all(|pair| pair[0] >= pair[1] - 1e-6));
        assert!(samples[0] <= sustain_level + 1e-6);
        assert!(samples.iter().all(|sample| (0.0..=1.0).contains(sample)));
    }

    #[test]
    fn retrigger_during_release_starts_from_the_current_level() {
        let mut envelope = envelope();
        envelope.note_on();
        run(&mut envelope, 5_280);
        envelope.note_off();
        run(&mut envelope, 2_000);

        let level_before = envelope.level();
        envelope.note_on();
        let level_after = envelope.level();

        assert_eq!(envelope.stage(), AdsrStage::Attack);
        assert!(
            (level_after - level_before).abs() < 1e-6,
            "retrigger jumped from {level_before} to {level_after}"
        );

        let samples = run(&mut envelope, 480);
        assert_eq!(envelope.stage(), AdsrStage::Decay);
        assert!((samples[479] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sustain_of_one_skips_decay() {
        let mut envelope = Adsr::new();
        envelope.prepare(SAMPLE_RATE_HZ);
        envelope.set_params(AdsrParams::new(10.0, 100.0, 1.0, 200.0));
        envelope.note_on();

        run(&mut envelope, 480);
        assert_eq!(envelope.stage(), AdsrStage::Sustain);
        assert_eq!(envelope.level(), 1.0);
    }

    #[test]
    fn invalid_parameters_are_sanitized() {
        let params = AdsrParams::new(f32::NAN, -10.0, 2.0, f32::INFINITY);
        assert_eq!(params.attack_ms, 1.0);
        assert_eq!(params.decay_ms, 1.0);
        assert_eq!(params.sustain, 1.0);
        assert_eq!(params.release_ms, 1.0);

        let mut envelope = Adsr::new();
        envelope.prepare(f32::NAN);
        envelope.set_params(params);
        envelope.note_on();
        let samples = run(&mut envelope, 100);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }
}
