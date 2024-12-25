pub struct TPA3116D2;

impl TPA3116D2 {
    pub fn new(/* parameters */) -> Self {
        TPA3116D2
    }

    #[allow(dead_code)]
    pub fn mute_all(&self) -> Result<(), anyhow::Error> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn mute_channel(&self, _channel: u8) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
