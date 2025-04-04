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

    fn float_to_fixed_8_24(value: f32) -> u32 {
        (value * (1 << 23) as f32) as u32
    }

    pub fn set_subwoofer_gain(&self, target_gain_db: f32) -> Result<(), anyhow::Error> {

        // These base coefficient values were taken from Linkwitz-Riley 24 low pass filter with
        // 100Hz crossover frequency and 0dB gain setting in SigmaStudio
        const BASE_COEFFS: [f32; 3] = [
            2.67111819123059E-06,
            5.34223638246117E-06,
            2.67111819123059E-06,
        ];

        // Current register addresses for relevant filter coefficients
        const B2_ADDR: u16 = 38 * 4;
        const B1_ADDR: u16 = 39 * 4;
        const B0_ADDR: u16 = 40 * 4;

        let registers = [B2_ADDR, B1_ADDR, B0_ADDR];

        let gain_factor = 10f32.powf(target_gain_db / 20.0);
        let new_coeffs: Vec<u32> = BASE_COEFFS
            .iter()
            .map(|&b| Self::float_to_fixed_8_24(b * gain_factor))
            .collect();

        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");


        let mut current_value = [0u8; 12];
        i2c.write_read(self.address, &B2_ADDR.to_le_bytes(), &mut current_value, BLOCK)?;
        println!("{:?}", current_value);
        
        for (i, &coeff) in new_coeffs.iter().enumerate() {
            let data = vec![
                (registers[i] >> 8) as u8,
                registers[i] as u8,
                (coeff >> 24) as u8,
                (coeff >> 16) as u8,
                (coeff >> 8) as u8,
                coeff as u8,
            ];
            i2c.write(self.address,&data, BLOCK)?;
        }

        
        let mut current_value = [0u8; 12];
        i2c.write_read(self.address, &B2_ADDR.to_le_bytes(), &mut current_value, BLOCK)?;
        println!("{:?}", current_value);
        
        Ok(())
    }
}
