use crate::drivers::tpa3116d2::TPA3116D2;

#[allow(dead_code)]
pub fn mute_speakers(amplifier: &TPA3116D2) -> Result<(), anyhow::Error> {
    amplifier.mute_all()
}
