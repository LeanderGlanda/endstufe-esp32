use std::{sync::{Arc, Mutex}, time::Duration};

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

    /// Set the RESET pin of the ADAU1962a
    /// reset = true means the device is turned on
    pub fn set_reset(&mut self, reset: bool) -> Result<(), anyhow::Error> {
        let mut i2c = self.i2c.lock().expect("Failed to lock I2C driver");
        i2c.write(0x42, &[0x8, reset as u8], BLOCK)?;
        std::thread::sleep(Duration::from_millis(300));
        Ok(())
    }

    pub fn master_power_up(&mut self, power_up: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x00, 0b00000001, if power_up {0x1} else {0x0})
    }

    pub fn set_xtal_oscillator_enabled(&mut self, enabled: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x00, 0b00110000, if enabled {0x0} else {0x3 << 4})
    }

    pub fn set_pll_input_source(&mut self, selection: MclkiXtaliOrDlrclkSelection) -> Result<(), anyhow::Error> {
        self.set_bits(0x00, 0b11000000, selection.value() << 6)
    }

    /// Set the Master Clock Select ratio   
    pub fn set_master_clock_fs_ratio(&mut self, ratio: MasterClockFsRatio) -> Result<(), anyhow::Error> {
        self.set_bits(0x00, 0b00000110, ratio.value() << 1)
    }

    pub fn set_dac_clock_select(&mut self, selection: DacClockSource) -> Result<(), anyhow::Error> {
        self.set_bits(0x01, 0b00000001, selection.value() << 0)
    }

    pub fn set_serial_audio_interface(&mut self, selection: AudioInterface) -> Result<(), anyhow::Error> {
        self.set_bits(0x06, 0b00111000, selection.value() << 3)
    }

    pub fn set_sample_rate_selection(&mut self, selection: SampleRate) -> Result<(), anyhow::Error> {
        self.set_bits(0x06, 0b00000110, selection.value() << 1)
    }

    pub fn set_master_mute(&mut self, muted: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x06, 0b00000001, if muted {0x1} else {0x0})
    }

    pub fn set_dlrclk_polarity(&mut self, inverted: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x07, 0b00100000, if inverted {0x1 << 5} else {0x0})
    }

    pub fn set_serial_interface_master(&mut self, is_master: bool) -> Result<(), anyhow::Error> {
        self.set_bits(0x07, 0b00000001, if is_master {0x1} else {0x0})
    }

}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum MclkiXtaliOrDlrclkSelection {
    MclkiOrXtali, // 00
    Dlrclk,       // 01
}

impl MclkiXtaliOrDlrclkSelection {
    pub fn value(&self) -> u8 {
        match self {
            MclkiXtaliOrDlrclkSelection::MclkiOrXtali => 0b00,
            MclkiXtaliOrDlrclkSelection::Dlrclk => 0b01,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
/// Ratio is referenced to 48kHz, it is divided down for higher sampling rates
pub enum MasterClockFsRatio {
    Ratio256,
    Ratio384,
    Ratio512,
    Ratio768,
}

impl MasterClockFsRatio {
    pub fn value(&self) -> u8 {
        match self {
            MasterClockFsRatio::Ratio256 => 0b00,
            MasterClockFsRatio::Ratio384 => 0b01,
            MasterClockFsRatio::Ratio512 => 0b10,
            MasterClockFsRatio::Ratio768 => 0b11,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum DacClockSource {
    PLL,
    MCLKIorXTALI,
}

impl DacClockSource {
    pub fn value(&self) -> u8 {
        match self {
            DacClockSource::PLL => 0b0,
            DacClockSource::MCLKIorXTALI => 0b1,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum AudioInterface {
    Stero,
    TDM2,
    TDM4,
    TDM8,
    TDM16,
}

impl AudioInterface {
    pub fn value(&self) -> u8 {
        match self {
            AudioInterface::Stero => 0b000,
            AudioInterface::TDM2 => 0b001,
            AudioInterface::TDM4 => 0b010,
            AudioInterface::TDM8 => 0b011,
            AudioInterface::TDM16 => 0b100,
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum SampleRate {
    FS48,
    FS96,
    FS192,
    FS192LowPropagationDelay,
}

impl SampleRate {
    pub fn value(&self) -> u8 {
        match self {
            SampleRate::FS48 => 0b00,
            SampleRate::FS96 => 0b01,
            SampleRate::FS192 => 0b10,
            SampleRate::FS192LowPropagationDelay => 0b11,
        }
    }
}