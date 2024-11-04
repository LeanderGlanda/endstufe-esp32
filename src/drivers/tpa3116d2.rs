pub struct TPA3116D2;

impl TPA3116D2 {
    pub fn new(/* parameters */) -> Self {
        TPA3116D2
    }

    pub fn mute_all(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub fn mute_channel(&self, channel: u8) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
