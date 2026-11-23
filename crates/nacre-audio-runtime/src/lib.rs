//! Audio runtime: device lifecycle, realtime callback, offline rendering and WAV export.
//!
//! The control plane talks to [`AudioHost`] through
//! [`nacre_application::AudioHostPort`]. Device access and `cpal` are confined
//! to this crate.

mod command;
mod host;
mod offline;
mod processor;
mod sanitize;
mod shared;
mod wav;

pub use host::AudioHost;
pub use nacre_application::{AudioConfig, ParameterCommand as Command};
pub use offline::{OfflineRenderer, hash_samples};
pub use wav::{WavError, write_wav_f32};

/// Snapshot type published by the runtime.
pub type AudioSnapshot = nacre_application::AudioStatus;

/// Number of slots in the control-to-audio command queue.
pub const COMMAND_QUEUE_CAPACITY: usize = 256;

/// Maximum number of commands the callback applies per block.
pub const MAX_COMMANDS_PER_BLOCK: usize = 64;

/// Duration of the stop fade applied inside the callback.
pub const STOP_FADE_MS: f32 = 20.0;
