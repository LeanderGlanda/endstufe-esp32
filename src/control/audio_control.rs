use crate::drivers::adau1467::ADAU1467;

pub fn set_channel_volume(dsp: &ADAU1467, channel: u8, volume: u8) -> Result<(), anyhow::Error> {
    dsp.set_channel_volume(channel, volume)
}

pub fn mute_channel(dsp: &ADAU1467, channel: u8) -> Result<(), anyhow::Error> {
    dsp.mute_channel(channel)
}
