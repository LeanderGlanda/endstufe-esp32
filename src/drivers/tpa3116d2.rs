use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use esp_idf_svc::hal::{delay::BLOCK, i2c::I2cDriver};

pub struct TPA3116D2<'a> {
    i2c: Arc<Mutex<I2cDriver<'a>>>, // Use the lifetime parameter here
}

impl<'a> TPA3116D2<'a> {
    pub fn new(i2c: Arc<Mutex<I2cDriver<'a>>>) -> Self {
        TPA3116D2 { i2c }
    }

    #[allow(dead_code)]
    pub fn enable_speaker_outputs(&self, enabled: bool) -> Result<(), anyhow::Error> {
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(0x42, &[0x0, enabled as u8], BLOCK)?;
        i2c.write(0x42, &[0x2, enabled as u8], BLOCK)?;
        std::thread::sleep(Duration::from_millis(300));
        Ok(())
    }

    pub fn mute_speaker_outputs(&self, muted: bool) -> Result<(), anyhow::Error> {
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(0x42, &[0x4, muted as u8], BLOCK)?;
        i2c.write(0x42, &[0x5, muted as u8], BLOCK)?;
        std::thread::sleep(Duration::from_millis(1));
        Ok(())
    }
}
