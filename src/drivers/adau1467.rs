use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;

use anyhow::Result;

use crate::linkwitz_riley_coeffs::LinkwitzRileyCoeffs;

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
        i2c.write_read(
            self.address,
            &register.to_le_bytes(),
            &mut current_value,
            BLOCK,
        )?;

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
        self.set_bits(0xF421, 0b1, 0b1)?;
        self.set_bits(0xF421, 0b1, 0b0)?;
        Ok(())
    }

    pub fn load_dsp_program(&self) -> Result<(), anyhow::Error> {
        crate::sigmastudio::interop::load_sigmastudio_dsp_program(&self.i2c);
        self.clear_panic()?;
        Ok(())
    }

    #[allow(unused)]
    pub fn read_second_page_select_reg(&self) -> Result<(), anyhow::Error> {
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");

        let mut current_value = [0u8; 2];
        i2c.write_read(
            self.address,
            &(0xF899 as u16).to_le_bytes(),
            &mut current_value,
            BLOCK,
        )?;

        log::info!("Second page select: {:?}", current_value);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn mute_channel(&self, _channel: u8) -> Result<(), anyhow::Error> {
        // DSP muting logic
        Ok(())
    }

    pub fn safeload_write(
        &self,
        data: &[u32],
        reg_addr: u16,
        lowerpage: bool,
    ) -> Result<(), anyhow::Error> {
        if data.len() > 5 {
            return Err(anyhow::anyhow!("Data slice must not exceed 5 elements"));
        }

        let mut buf: Vec<u8> = Vec::with_capacity(2 + 20 + 4 + 4);

        // Safeload address 0x6000
        buf.extend_from_slice(&[0x60, 0x00]);

        // Add the 5 data words (pad with zeros if needed)
        for i in 0..5 {
            let word = data.get(i).copied().unwrap_or(0);
            buf.extend_from_slice(&word.to_be_bytes());
        }

        // Safeload data address
        buf.extend_from_slice(&(reg_addr as u32).to_be_bytes());

        // If writting to upper page, then add a word with 0
        if !lowerpage {
            buf.extend_from_slice(&0_u32.to_be_bytes());
        }

        // Safeload lower/upper page setting / words to write
        buf.extend_from_slice(&(data.len() as u32).to_be_bytes());

        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(self.address, &buf, BLOCK)?;

        Ok(())
    }

    #[allow(unused)]
    fn float_to_fixed_8_24(value: f32) -> u32 {
        (value * (1 << 23) as f32) as u32
    }

    pub fn set_subwoofer_gain(&self, target_gain_db: f32) -> Result<(), anyhow::Error> {
        // Make sure to write new filter coeffs to both crossover filters, as there is one for each channel
        const CROSSOVER1_LOWPASS_FILTER1_BASE_ADDR: u16 = 52;
        const CROSSOVER2_LOWPASS_FILTER1_BASE_ADDR: u16 = 62;

        let coeffs = LinkwitzRileyCoeffs::new(192000.0, 100.0, target_gain_db as f64);

        log::debug!("Filter coefficients: {:?}", coeffs);

        self.safeload_write(
            &coeffs.lowpass_filter1.to_fixed(),
            CROSSOVER1_LOWPASS_FILTER1_BASE_ADDR,
            true,
        )?;
        self.safeload_write(
            &coeffs.lowpass_filter1.to_fixed(),
            CROSSOVER2_LOWPASS_FILTER1_BASE_ADDR,
            true,
        )?;

        Ok(())
    }
}
