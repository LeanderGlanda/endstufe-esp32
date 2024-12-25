pub mod audio_control;
pub mod gpio_control;

use crate::api::commands::SystemCommand;
use crate::drivers::{pcm1865::PCM1865, adau1467::ADAU1467, adau1962a::ADAU1962A, tpa3116d2::TPA3116D2};

#[allow(dead_code)]
pub fn handle_command(command: SystemCommand, pcm1865: &mut PCM1865, dsp: &ADAU1467, _dac: &ADAU1962A, amplifier: &TPA3116D2) -> Result<(), anyhow::Error> {
    match command {
        SystemCommand::SetVolume { channel, level } => {
            dsp.set_channel_volume(channel, level)
        }
        SystemCommand::MuteChannel { channel } => {
            amplifier.mute_channel(channel)
        }
        SystemCommand::SetInputSource { channel, source_id } => {
            pcm1865.set_input_source(channel, source_id)
        }
        _ => Err(anyhow::Error::msg("Unknown command")),
    }
}
