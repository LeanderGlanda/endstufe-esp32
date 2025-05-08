use serde::{Deserialize, Serialize};

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
    pub fn handle(self) -> Response {
        match self {
            Command::SetVolume { level } if level <= 100 => {
                // set_volume(level);
                log::info!("SetVolume called");
                Response::Ok
            }
            Command::SetVolume { level } => Response::Err {
                message: format!("Level {} out of range (0-100)", level),
            },
            Command::Mute => {
                // mute();
                log::info!("Mute called");
                Response::Ok
            }
            Command::Unmute => {
                // unmute();
                log::info!("Unmute called");
                Response::Ok
            }
        }
    }
}
