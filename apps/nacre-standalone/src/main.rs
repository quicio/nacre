//! Minimal Nacre standalone application: transport, frequency, master gain and
//! device health. All composition lives here; audio business logic stays in the
//! application and runtime crates.

use std::time::{Duration, Instant};

use eframe::egui;
use nacre_application::{
    AppError, AudioConfig, AudioHostPort, AudioState, AudioStatus, CommandError, Parameter,
    SetParameter, StartAudio, StopAudio, gain_db_to_linear,
};
use nacre_audio_runtime::AudioHost;

/// Lowest oscillator frequency exposed by this slice, in hertz.
const MIN_FREQUENCY_HZ: f32 = 20.0;
/// Highest oscillator frequency exposed by this slice, in hertz.
const MAX_FREQUENCY_HZ: f32 = 2_000.0;
/// Lowest master gain exposed by this slice, in decibels.
const MIN_GAIN_DB: f32 = -60.0;
/// Highest master gain exposed by this slice, in decibels.
const MAX_GAIN_DB: f32 = 0.0;

struct NacreApp {
    host: Box<dyn AudioHostPort>,
    status: AudioStatus,
    error: Option<String>,
    frequency_hz: f32,
    gain_db: f32,
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
            frequency_hz: 440.0,
            gain_db: -12.0,
            close_at: std::env::var("NACRE_AUTOCLOSE_MS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .map(|milliseconds| Instant::now() + Duration::from_millis(milliseconds)),
        };
        app.push_parameter(Parameter::Frequency, app.frequency_hz);
        app.push_parameter(Parameter::Gain, gain_db_to_linear(app.gain_db));

        if env_flag("NACRE_AUTOSTART") {
            app.start_audio();
        }

        app
    }

    fn start_audio(&mut self) {
        self.push_parameter(Parameter::Frequency, self.frequency_hz);
        self.push_parameter(Parameter::Gain, gain_db_to_linear(self.gain_db));

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
        StopAudio::new(self.host.as_mut()).execute();
        self.status = self.host.status();
    }

    fn push_parameter(&mut self, parameter: Parameter, value: f32) {
        if let Err(error) = SetParameter::new(self.host.as_mut()).execute(parameter, value)
            && !matches!(error, AppError::Command(CommandError::Full))
        {
            self.error = Some(error.to_string());
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
        });

        let frequency_changed = ui
            .add(
                egui::Slider::new(&mut self.frequency_hz, MIN_FREQUENCY_HZ..=MAX_FREQUENCY_HZ)
                    .logarithmic(true)
                    .text("Frecuencia (Hz)"),
            )
            .changed();
        if frequency_changed {
            self.push_parameter(Parameter::Frequency, self.frequency_hz);
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
            .with_inner_size([480.0, 240.0])
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

    #[test]
    fn workspace_metadata_is_present() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nacre-standalone");
        assert_eq!(env!("CARGO_PKG_VERSION"), "0.1.0");
    }

    #[test]
    fn gain_slider_range_maps_to_linear_factors() {
        assert!(gain_db_to_linear(-12.0) < 0.3);
        assert_eq!(gain_db_to_linear(0.0), 1.0);
    }
}
