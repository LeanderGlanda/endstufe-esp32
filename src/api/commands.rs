use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::hardware_context::{self, HardwareContext};

/// Kommando-Typen, alle über /api empfangenen Requests
#[derive(Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    SetVolume { level: u8 },
    Mute,
    Unmute,
}

/// Antwort-Typen, die wir serialisieren
#[derive(Serialize)]
#[serde(tag = "resp", rename_all = "snake_case")]
pub enum Response {
    Ok,
    Status { level: u8, muted: bool },
    Err { message: String },
}

impl Command {
    /// Verarbeite das Kommando und gib eine Antwort zurück
    pub fn handle(self, hardware_context: &Arc<HardwareContext<'_>>) -> Response {
        match self {
            Command::SetVolume { level } if level <= 100 => {
                log::info!("SetVolume called");
                hardware_context.adau1962a.lock().expect("Could not lock ADAU1962a driver").set_master_volume(level);
                Response::Ok
            }
            Command::SetVolume { level } => Response::Err {
                message: format!("Level {} out of range (0-100)", level),
            },
            Command::Mute => {
                // mute();
                log::info!("Mute called");
                hardware_context.tpa3116d2.lock().expect("Could not lock TPA3116d2 driver").mute_speaker_outputs(true);
                Response::Ok
            }
            Command::Unmute => {
                // unmute();
                log::info!("Unmute called");
                hardware_context.tpa3116d2.lock().expect("Could not lock TPA3116d2 driver").mute_speaker_outputs(false);
                Response::Ok
            }
        }
    }
}
