use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::i2c::I2cDriver;
use esp_idf_svc::hal::delay::BLOCK;

use anyhow::{Error, Result};

pub struct ADAU1962A<'a> {
    i2c: Arc<Mutex<I2cDriver<'a>>>, // Use the lifetime parameter here
    address: u8,
}

impl<'a> ADAU1962A<'a> {
    pub fn new(i2c: Arc<Mutex<I2cDriver<'a>>>, address: u8) -> Self {
        ADAU1962A { i2c, address }
    }

    fn set_bits(&self, register: u8, mask: u8, value: u8) -> Result<()> {
        log::info!("Settings bits");
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        
        // Step 1: Read the current value of the register
        let mut current_value = [0u8; 1];
        i2c.write_read(self.address, &[register], &mut current_value, BLOCK)?;
        
        // Step 2: Modify the specific bits
        let new_value = (current_value[0] & !mask) | (value & mask);
        
        // Step 3: Write the modified value back
        i2c.write(self.address, &[register, new_value], BLOCK)?;
        Ok(())
    }
}
