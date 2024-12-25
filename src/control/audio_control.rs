use crate::drivers::adau1467::ADAU1467;

#[allow(dead_code)]
pub fn set_channel_volume(dsp: &ADAU1467, channel: u8, volume: u8) -> Result<(), anyhow::Error> {
    dsp.set_channel_volume(channel, volume)
}

#[allow(dead_code)]
pub fn mute_channel(dsp: &ADAU1467, channel: u8) -> Result<(), anyhow::Error> {
    dsp.mute_channel(channel)
}
