use std::{sync::{Arc, Mutex}, time::Duration};

use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::hal::delay::BLOCK;

use anyhow::{Error, Result};

pub struct ADAU1467<'a> {
    i2c: Arc<Mutex<I2cDriver<'a>>>, // Use the lifetime parameter here
    address: u8,
}
impl<'a> ADAU1467<'a> {
    pub fn new(i2c: Arc<Mutex<I2cDriver<'a>>>, address: u8) -> Self {
        ADAU1467 { i2c, address }
    }

    fn set_bits(&self, register: u16, mask: u16, value: u16) -> Result<()> {
        log::info!("Settings bits");
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        
        // Step 1: Read the current value of the register
        let mut current_value = [0u8; 2];
        i2c.write_read(self.address, &register.to_le_bytes(), &mut current_value, BLOCK)?;
        
        // Step 2: Modify the specific bits
        let new_value = (u16::from_le_bytes(current_value) & !mask) | (value & mask);
        
        let mut data_to_write = Vec::with_capacity(4);
        data_to_write.extend_from_slice(&register.to_le_bytes());
        data_to_write.extend_from_slice(&new_value.to_le_bytes());

        // Step 3: Write the modified value back
        i2c.write(self.address, &data_to_write, BLOCK)?;
        Ok(())
    }

    /// Set the RESET pin of the ADAU1467
    /// reset = true means the device is turned on
    pub fn set_reset(&mut self, reset: bool) -> Result<(), anyhow::Error> {
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(0x42, &[0x7, reset as u8], BLOCK)?;
        std::thread::sleep(Duration::from_millis(15));
        Ok(())
    }

    fn clear_panic(&self) -> Result<(), anyhow::Error> {
        self.set_bits(0xF421, 0b1, 0b1);
        self.set_bits(0xF421, 0b1, 0b0);
        Ok(())
    }

    pub fn load_dsp_program(&self) -> Result<(), anyhow::Error> {
        crate::sigmastudio::interop::load_sigmastudio_dsp_program(&self.i2c);
        self.clear_panic()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn mute_channel(&self, _channel: u8) -> Result<(), anyhow::Error> {
        // DSP muting logic
        Ok(())
    }
}
