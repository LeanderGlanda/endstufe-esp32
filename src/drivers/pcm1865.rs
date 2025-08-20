use anyhow::{Error, Result};
use esp_idf_svc::hal::delay::BLOCK;
use esp_idf_svc::hal::i2c::I2cDriver;
use std::sync::{Arc, Mutex};

pub struct PCM1865<'a> {
    i2c: Arc<Mutex<I2cDriver<'a>>>, // Use the lifetime parameter here
    address: u8,
}

impl<'a> PCM1865<'a> {
    /// Creates a new instance of PCM1865 with a thread-safe I2C driver and device address
    pub fn new(i2c: Arc<Mutex<I2cDriver<'a>>>, address: u8) -> Self {
        PCM1865 { i2c, address }
    }

    /// Sets the input source for a specified channel
    #[allow(unused)]
    pub fn set_input_source(&self, channel: u8, source_id: u8) -> Result<()> {
        let register = match channel {
            1 => 0x3B,
            2 => 0x3C,
            3 => 0x3D,
            4 => 0x3E,
            _ => return Err(Error::msg("Invalid channel selected")),
        };

        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(self.address, &[register, source_id], BLOCK)?;
        Ok(())
    }

    /// Mutes or unmutes a specified channel
    #[allow(dead_code)]
    pub fn mute_channel(&self, channel: u8, mute: bool) -> Result<()> {
        let mute_register = 0x10 + channel;
        let mute_value = if mute { 0x01 } else { 0x00 };

        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(self.address, &[mute_register, mute_value], BLOCK)?;
        Ok(())
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

    #[allow(dead_code)]
    fn enable_automatic_clipping_suppression(&mut self, state: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x05, 0x1, if state { 0x1 } else { 0x00 })
    }

    /// Sets the SCK or XTAL selection bits in the corresponding register.
    pub fn set_sck_xtal_selection(
        &mut self,
        selection: SckXtalSelection,
    ) -> Result<(), anyhow::Error> {
        self.set_bits(0x20, 0b11000000, selection.value() << 6)
    }

    pub fn select_mode(&mut self, is_master: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x20, 0b00010000, if is_master { 0x1 << 4 } else { 0x0 })
    }

    pub fn auto_clock_detector_configuration(
        &mut self,
        enable_auto_detection: bool,
    ) -> Result<(), anyhow::Error> {
        self.set_bits(
            0x20,
            0b00000001,
            if enable_auto_detection { 0x1 } else { 0x0 },
        )
    }

    pub fn master_clock_to_bit_clock_divider_value(
        &mut self,
        divider: u8,
    ) -> Result<(), anyhow::Error> {
        if divider > 128 || divider == 0 {
            return Err(Error::msg(
                "Invalid divider. Must be a value between 1 and 128",
            ));
        }
        self.set_bits(0x26, 0b01111111, divider - 1)
    }

    pub fn bit_clock_to_left_right_clock_divider_value(
        &mut self,
        divider: u16,
    ) -> Result<(), anyhow::Error> {
        if divider > 256 || divider == 0 {
            return Err(Error::msg(
                "Invalid divider. Must be a value between 1 and 128",
            ));
        }
        self.set_bits(0x27, 0b01111111, (divider - 1).try_into().unwrap())
    }

    /// Selects the input for ADC1 or ADC2, Left/Right channel (regs 0x06..0x09).
    pub fn set_adc_input(
        &self,
        adc: AdcNumber,
        channel: AdcChannel,
        input: AdcInput,
        inverted: bool,
    ) -> Result<(), anyhow::Error> {
        let base_register = match adc {
            AdcNumber::Adc1 => 0x06,
            AdcNumber::Adc2 => 0x08,
        };

        let register = match channel {
            AdcChannel::Left => base_register,
            AdcChannel::Right => base_register + 1,
        };

        // Build register value
        let value = (if inverted { 1 << 7 } else { 0 })
            | (1 << 6)              // RSV bit must always be 1
            | (input as u8 & 0x3F); // lower 6 bits = selection

        self.set_bits(register, 0xFF, value)
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
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

#[derive(Clone, Copy, Debug)]
pub enum AdcNumber {
    Adc1,
    Adc2,
}

#[derive(Clone, Copy, Debug)]
pub enum AdcChannel {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum AdcInput {
    None = 0b000000,
    Vin1 = 0b000001,
    Vin2 = 0b000010,
    Vin2PlusVin1 = 0b000011,
    Vin3 = 0b000100,
    Vin3PlusVin1 = 0b000101,
    Vin3PlusVin2 = 0b000110,
    Vin3PlusVin2PlusVin1 = 0b000111,
    Vin4 = 0b001000,
    Vin4PlusVin1 = 0b001001,
    Vin4PlusVin2 = 0b001010,
    Vin4PlusVin2PlusVin1 = 0b001011,
    Vin4PlusVin3 = 0b001100,
    Vin4PlusVin3PlusVin1 = 0b001101,
    Vin4PlusVin3PlusVin2 = 0b001110,
    Vin4PlusVin3PlusVin2PlusVin1 = 0b001111,
    DiffVin1 = 0b010000,
    DiffVin4 = 0b100000,
    DiffVin1PlusDiffVin4 = 0b110000,
}