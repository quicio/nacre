//! Minimal Nacre standalone application: transport, a one-octave gate keyboard,
//! the voice controls and device health. All composition lives here; audio
//! business logic stays in the application and runtime crates.

use std::time::{Duration, Instant};

use eframe::egui;
use nacre_application::{
    AppError, AudioConfig, AudioHostPort, AudioState, AudioStatus, CommandError, LfoDestination,
    LfoShape, Parameter, ParameterCommand, SetParameter, StartAudio, StopAudio, VoiceParameter,
    Waveform, gain_db_to_linear,
};
use nacre_audio_runtime::AudioHost;

/// Lowest oscillator frequency exposed by this slice, in hertz.
const MIN_FREQUENCY_HZ: f32 = 20.0;
/// Highest oscillator frequency exposed by this slice, in hertz.
const MAX_FREQUENCY_HZ: f32 = 2_000.0;
/// Lowest filter cutoff exposed by this slice, in hertz.
const MIN_CUTOFF_HZ: f32 = 20.0;
/// Highest filter cutoff exposed by this slice, in hertz.
const MAX_CUTOFF_HZ: f32 = 12_000.0;
/// Lowest master gain exposed by this slice, in decibels.
const MIN_GAIN_DB: f32 = -60.0;
/// Highest master gain exposed by this slice, in decibels.
const MAX_GAIN_DB: f32 = 0.0;

/// One-octave keyboard starting at A2.
const KEYBOARD: [(&str, f32); 8] = [
    ("A2", 110.0),
    ("B2", 123.47),
    ("C3", 130.81),
    ("D3", 146.83),
    ("E3", 164.81),
    ("F3", 174.61),
    ("G3", 196.0),
    ("A3", 220.0),
];

struct NacreApp {
    host: Box<dyn AudioHostPort>,
    status: AudioStatus,
    error: Option<String>,
    frequency_hz: f32,
    gain_db: f32,
    waveform: Waveform,
    cutoff_hz: f32,
    resonance: f32,
    attack_ms: f32,
    decay_ms: f32,
    sustain: f32,
    release_ms: f32,
    lfo_rate_hz: f32,
    lfo_depth_percent: f32,
    lfo_destination: LfoDestination,
    lfo_shape: LfoShape,
    drive_db: f32,
    gated_hz: Option<f32>,
    close_at: Option<Instant>,
}

impl NacreApp {
    fn new() -> Self {
        let config = AudioConfig::default();
        let mut host = AudioHost::new();
        let (status, error) = match host.probe(config) {
            Ok(status) => (status, None),
            Err(error) => (
                AudioStatus::stopped(config),
                Some(format!("audio no disponible: {error}")),
            ),
        };

        let mut app = Self {
            host: Box::new(host),
            status,
            error,
            frequency_hz: 110.0,
            gain_db: -12.0,
            waveform: Waveform::Saw,
            cutoff_hz: 1_500.0,
            resonance: std::f32::consts::FRAC_1_SQRT_2,
            attack_ms: 10.0,
            decay_ms: 200.0,
            sustain: 0.7,
            release_ms: 300.0,
            lfo_rate_hz: 0.5,
            lfo_depth_percent: 0.0,
            lfo_destination: LfoDestination::Cutoff,
            lfo_shape: LfoShape::Sine,
            drive_db: 0.0,
            gated_hz: None,
            close_at: std::env::var("NACRE_AUTOCLOSE_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map(|milliseconds| Instant::now() + Duration::from_millis(milliseconds)),
        };
        app.push_all_parameters();

        if env_flag("NACRE_AUTOSTART") {
            app.start_audio();
        }
        if env_flag("NACRE_AUTOGATE") {
            app.gate_on(app.frequency_hz);
        }

        app
    }

    fn start_audio(&mut self) {
        self.push_all_parameters();

        match StartAudio::new(self.host.as_mut()).execute(AudioConfig::default()) {
            Ok(status) => {
                self.status = status;
                self.error = None;
            }
            Err(error) => {
                self.status = self.host.status();
                self.error = Some(format!("no se pudo iniciar el audio: {error}"));
            }
        }
    }

    fn stop_audio(&mut self) {
        if self.gated_hz.is_some() {
            self.gate_off();
        }
        StopAudio::new(self.host.as_mut()).execute();
        self.status = self.host.status();
    }

    fn gate_on(&mut self, frequency_hz: f32) {
        if self.status.state != AudioState::Running {
            return;
        }

        self.frequency_hz = frequency_hz;
        self.push_parameter(Parameter::Frequency, frequency_hz);
        self.submit(ParameterCommand::NoteOn { frequency_hz });
        self.gated_hz = Some(frequency_hz);
    }

    fn gate_off(&mut self) {
        self.submit(ParameterCommand::NoteOff);
        self.gated_hz = None;
    }

    fn push_parameter(&mut self, parameter: Parameter, value: f32) {
        if let Err(error) = SetParameter::new(self.host.as_mut()).execute(parameter, value)
            && !matches!(error, AppError::Command(CommandError::Full))
        {
            self.error = Some(error.to_string());
        }
    }

    fn push_all_parameters(&mut self) {
        self.push_parameter(Parameter::Frequency, self.frequency_hz);
        self.push_parameter(Parameter::Gain, gain_db_to_linear(self.gain_db));
        for (parameter, value) in [
            (VoiceParameter::FilterCutoff, self.cutoff_hz),
            (VoiceParameter::FilterResonance, self.resonance),
            (VoiceParameter::Attack, self.attack_ms),
            (VoiceParameter::Decay, self.decay_ms),
            (VoiceParameter::Sustain, self.sustain),
            (VoiceParameter::Release, self.release_ms),
            (VoiceParameter::LfoRate, self.lfo_rate_hz),
            (VoiceParameter::LfoDepth, self.lfo_depth_percent),
            (VoiceParameter::Drive, self.drive_db),
        ] {
            self.push_parameter(Parameter::Voice(parameter), value);
        }
        self.submit(ParameterCommand::SetWaveform {
            waveform: self.waveform,
        });
        self.submit(ParameterCommand::SetLfoShape {
            shape: self.lfo_shape,
        });
        self.submit(ParameterCommand::SetLfoDestination {
            destination: self.lfo_destination,
        });
    }

    fn submit(&mut self, command: ParameterCommand) {
        match self.host.submit(command) {
            Ok(()) | Err(CommandError::NotRunning) | Err(CommandError::Full) => {}
        }
    }

    fn status_line(&self) -> String {
        let state = match self.status.state {
            AudioState::Running => "reproduciendo",
            AudioState::Stopped if self.status.device_name.is_some() => "audio listo",
            AudioState::Stopped => "audio detenido",
        };
        format!(
            "{state} · {} Hz · {} canales · {} frames{}",
            self.status.sample_rate_hz,
            self.status.channels,
            self.status.buffer_size_frames,
            if self.status.negotiated {
                " · configuración negociada"
            } else {
                ""
            }
        )
    }

    fn transport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.status.state == AudioState::Stopped,
                    egui::Button::new("Play"),
                )
                .clicked()
            {
                self.start_audio();
            }

            if ui
                .add_enabled(
                    self.status.state == AudioState::Running,
                    egui::Button::new("Stop"),
                )
                .clicked()
            {
                self.stop_audio();
            }

            let gate_label = match self.gated_hz {
                Some(_) => "Gate ●",
                None => "Gate",
            };
            if ui
                .add_enabled(
                    self.status.state == AudioState::Running,
                    egui::Button::new(gate_label),
                )
                .clicked()
            {
                if self.gated_hz.is_some() {
                    self.gate_off();
                } else {
                    self.gate_on(self.frequency_hz);
                }
            }
        });
    }

    fn keyboard(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Nota");
            for (label, frequency_hz) in KEYBOARD {
                let active = self
                    .gated_hz
                    .is_some_and(|gated| (gated - frequency_hz).abs() < 0.01);
                let response = ui.add_enabled(
                    self.status.state == AudioState::Running,
                    egui::Button::new(label).selected(active),
                );
                if response.clicked() {
                    if active {
                        self.gate_off();
                    } else {
                        self.gate_on(frequency_hz);
                    }
                }
            }
        });
    }
}

impl eframe::App for NacreApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.status = self.host.status();

        ui.heading("Nacre");
        ui.label(self.status_line());

        if let Some(device) = &self.status.device_name {
            let host = self
                .status
                .host_name
                .as_deref()
                .unwrap_or("host desconocido");
            ui.label(format!("dispositivo: {device} ({host})"));
        }

        ui.separator();
        self.transport(ui);
        self.keyboard(ui);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let frequency_changed = ui
                .add(
                    egui::Slider::new(&mut self.frequency_hz, MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ)
                        .logarithmic(true)
                        .text("Frecuencia (Hz)"),
                )
                .changed();
            if frequency_changed {
                self.push_parameter(Parameter::Frequency, self.frequency_hz);
                if self.gated_hz.is_some() {
                    self.gated_hz = Some(self.frequency_hz);
                }
            }

            ui.horizontal(|ui| {
                ui.label("Forma");
                for waveform in [Waveform::Saw, Waveform::Sine] {
                    let label = match waveform {
                        Waveform::Saw => "Saw",
                        Waveform::Sine => "Sine",
                    };
                    if ui
                        .selectable_label(self.waveform == waveform, label)
                        .clicked()
                    {
                        self.waveform = waveform;
                        self.submit(ParameterCommand::SetWaveform { waveform });
                    }
                }
            });

            ui.separator();
            ui.label("Filtro");
            let cutoff_changed = ui
                .add(
                    egui::Slider::new(&mut self.cutoff_hz, MIN_CUTOFF_HZ..=MAX_CUTOFF_HZ)
                        .logarithmic(true)
                        .suffix(" Hz")
                        .text("Cutoff"),
                )
                .changed();
            if cutoff_changed {
                self.push_parameter(
                    Parameter::Voice(VoiceParameter::FilterCutoff),
                    self.cutoff_hz,
                );
            }

            let resonance_changed = ui
                .add(egui::Slider::new(&mut self.resonance, 0.5..=20.0).text("Resonancia (Q)"))
                .changed();
            if resonance_changed {
                self.push_parameter(
                    Parameter::Voice(VoiceParameter::FilterResonance),
                    self.resonance,
                );
            }

            ui.separator();
            ui.label("Envolvente");
            let attack_changed = ui
                .add(
                    egui::Slider::new(&mut self.attack_ms, 1.0..=5_000.0)
                        .logarithmic(true)
                        .suffix(" ms")
                        .text("Ataque"),
                )
                .changed();
            if attack_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::Attack), self.attack_ms);
            }

            let decay_changed = ui
                .add(
                    egui::Slider::new(&mut self.decay_ms, 1.0..=5_000.0)
                        .logarithmic(true)
                        .suffix(" ms")
                        .text("Decay"),
                )
                .changed();
            if decay_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::Decay), self.decay_ms);
            }

            let sustain_changed = ui
                .add(egui::Slider::new(&mut self.sustain, 0.0..=1.0).text("Sustain"))
                .changed();
            if sustain_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::Sustain), self.sustain);
            }

            let release_changed = ui
                .add(
                    egui::Slider::new(&mut self.release_ms, 1.0..=5_000.0)
                        .logarithmic(true)
                        .suffix(" ms")
                        .text("Release"),
                )
                .changed();
            if release_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::Release), self.release_ms);
            }

            ui.separator();
            ui.label("LFO");
            let lfo_rate_changed = ui
                .add(
                    egui::Slider::new(&mut self.lfo_rate_hz, 0.01..=20.0)
                        .logarithmic(true)
                        .suffix(" Hz")
                        .text("Velocidad"),
                )
                .changed();
            if lfo_rate_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::LfoRate), self.lfo_rate_hz);
            }

            let lfo_depth_changed = ui
                .add(
                    egui::Slider::new(&mut self.lfo_depth_percent, 0.0..=100.0)
                        .suffix(" %")
                        .text("Profundidad"),
                )
                .changed();
            if lfo_depth_changed {
                self.push_parameter(
                    Parameter::Voice(VoiceParameter::LfoDepth),
                    self.lfo_depth_percent,
                );
            }

            ui.horizontal(|ui| {
                ui.label("Destino");
                for destination in [LfoDestination::Cutoff, LfoDestination::Pitch] {
                    let label = match destination {
                        LfoDestination::Cutoff => "Cutoff",
                        LfoDestination::Pitch => "Pitch",
                    };
                    if ui
                        .selectable_label(self.lfo_destination == destination, label)
                        .clicked()
                    {
                        self.lfo_destination = destination;
                        self.submit(ParameterCommand::SetLfoDestination { destination });
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Forma LFO");
                for shape in [LfoShape::Sine, LfoShape::Triangle] {
                    let label = match shape {
                        LfoShape::Sine => "Seno",
                        LfoShape::Triangle => "Triángulo",
                    };
                    if ui
                        .selectable_label(self.lfo_shape == shape, label)
                        .clicked()
                    {
                        self.lfo_shape = shape;
                        self.submit(ParameterCommand::SetLfoShape { shape });
                    }
                }
            });

            ui.separator();
            ui.label("Saturación y salida");
            let drive_changed = ui
                .add(
                    egui::Slider::new(&mut self.drive_db, 0.0..=24.0)
                        .suffix(" dB")
                        .text("Drive"),
                )
                .changed();
            if drive_changed {
                self.push_parameter(Parameter::Voice(VoiceParameter::Drive), self.drive_db);
            }

            let gain_changed = ui
                .add(
                    egui::Slider::new(&mut self.gain_db, MIN_GAIN_DB..=MAX_GAIN_DB)
                        .suffix(" dB")
                        .text("Ganancia maestra"),
                )
                .changed();
            if gain_changed {
                self.push_parameter(Parameter::Gain, gain_db_to_linear(self.gain_db));
            }
        });

        ui.separator();

        let health = self.status.health;
        ui.label(format!(
            "RMS {:.1} dB · Peak {:.1} dB · frames {} · xruns {} · descartes {} · CPU {:.0} %",
            level_db(health.rms),
            level_db(health.peak),
            health.frames.get(),
            health.xruns,
            health.dropped_commands,
            health.cpu_load * 100.0,
        ));

        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::RED, error);
        }

        if let Some(deadline) = self.close_at
            && Instant::now() >= deadline
        {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.host.stop();
    }
}

impl Drop for NacreApp {
    fn drop(&mut self) {
        self.host.stop();
    }
}

fn level_db(level: f32) -> f32 {
    if level > 0.0 {
        20.0 * level.log10()
    } else {
        -120.0
    }
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| {
        !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
    })
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([560.0, 720.0])
            .with_title("Nacre"),
        ..Default::default()
    };

    eframe::run_native(
        "Nacre",
        options,
        Box::new(|_creation_context| Ok(Box::new(NacreApp::new()))),
    )
}

#[cfg(test)]
mod tests {
    use nacre_application::gain_db_to_linear;

    use super::{KEYBOARD, MAX_GAIN_DB, MIN_GAIN_DB};

    #[test]
    fn workspace_metadata_is_present() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nacre-standalone");
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }

    #[test]
    fn gain_slider_range_maps_to_linear_factors() {
        assert!(gain_db_to_linear(MIN_GAIN_DB) < 0.3);
        assert_eq!(gain_db_to_linear(MAX_GAIN_DB), 1.0);
    }

    #[test]
    fn keyboard_covers_one_octave_in_order() {
        assert_eq!(KEYBOARD.len(), 8);
        assert!(KEYBOARD.windows(2).all(|pair| pair[0].1 < pair[1].1));
        assert_eq!(KEYBOARD[0].1, 110.0);
        assert_eq!(KEYBOARD[7].1, 220.0);
    }
}
