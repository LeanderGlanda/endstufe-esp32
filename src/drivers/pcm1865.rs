
use anyhow::{Error, Ok};
use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;

pub struct PCM1865<'a> {
    i2c: I2cDriver<'a>,
    address: u8,
}

impl<'a> PCM1865<'a> {
    /// Creates a new instance of PCM1865 with an I2C driver and device address
    pub fn new(i2c: I2cDriver<'a>, address: u8) -> Self {
        PCM1865 { i2c, address }
    }

    /// Sets the input source for a specified channel
    pub fn set_input_source(&mut self, channel: u8, source_id: u8) -> Result<(), anyhow::Error> {
        let register = match channel {
            1 => 0x3B,
            2 => 0x3C,
            3 => 0x3D,
            4 => 0x3E,
            _ => return Err(Error::msg("Invalid channel selected")),
        };
        self.i2c.write(self.address, &[register, source_id], BLOCK)?;
        Ok(())
    }

    /// Mutes or unmutes a specified channel
    pub fn mute_channel(&mut self, channel: u8, mute: bool) -> Result<(), anyhow::Error> {
        let mute_register = 0x10; // Hypothetical mute register
        let mute_value = if mute { 0x01 } else { 0x00 };
        self.i2c.write(self.address, &[mute_register, mute_value], BLOCK)?;
        Ok(())
    }

    fn set_bits(&mut self, register: u8, mask: u8, value: u8) -> Result<(), anyhow::Error> {
        // Step 1: Read the current value of the register
        let mut current_value = [0u8; 1];
        self.i2c.write_read(self.address, &[register], &mut current_value, BLOCK)?;
        
        // Step 2: Modify the specific bits
        let new_value = (current_value[0] & !mask) | (value & mask);
        
        // Step 3: Write the modified value back
        self.i2c.write(self.address, &[register, new_value], BLOCK)?;
        Ok(())
    }

    fn enable_automatic_clipping_suppression(&mut self, state: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x05, 0x1, if state {0x1} else {0x00})
    }

    /// Sets the SCK or XTAL selection bits in the corresponding register.
    pub fn set_sck_xtal_selection(&mut self, selection: SckXtalSelection) -> Result<(), anyhow::Error> {
        self.set_bits(0x20, 0b11000000, selection.value() << 6)
    }

    pub fn select_mode(&mut self, is_master: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x20, 0b00010000, if is_master {0x1 << 4} else {0x0})
    }

    pub fn auto_clock_detector_configuration(&mut self, enable_auto_detection: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x20, 0b00000001, if enable_auto_detection {0x1} else {0x0})
    }

    pub fn master_clock_to_bit_clock_divider_value(&mut self, divider: u8) -> Result<(), anyhow::Error> {
        if divider > 128 || divider == 0 {
            return Err(Error::msg("Invalid divider. Must be a value between 1 and 128"))
        }
        self.set_bits(0x38, 0b01111111, divider - 1)
    }

    pub fn bit_clock_to_left_right_clock_divider_value(&mut self, divider: u16) -> Result<(), anyhow::Error> {
        if divider > 256 || divider == 0 {
            return Err(Error::msg("Invalid divider. Must be a value between 1 and 128"))
        }
        self.set_bits(0x38, 0b01111111, (divider - 1).try_into().unwrap())
    }



}


#[derive(Clone, Copy)]
pub enum SckXtalSelection {
    SckOrXtal, // 00
    Sck,       // 01
    Xtal,      // 10
}

impl SckXtalSelection {
    pub fn value(&self) -> u8 {
        match self {
            SckXtalSelection::SckOrXtal => 0b00,
            SckXtalSelection::Sck => 0b01,
            SckXtalSelection::Xtal => 0b10,
        }
    }
}