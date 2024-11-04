use drivers::pcm1865::{self, PCM1865};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::prelude::*;

mod drivers;
mod i2c_helper;

/// Lets do some planning of this:
/// The GPIOs are only accessible to RP2040 right now, which means either ESP32 sends commands to RP2040 or RP2040 configures them in a way fixed. But that's little bit bad because then amplifier muting doesn't work like it should.
/// This means there needs to be an interface between RP2040 and ESP32, which can be nothing for now but needs to be added later.
/// Required interface functions are:
/// * Accessing GPIOs for controlling the TPA3116D2 ICs
/// * PCM9211 Reset GPIO
/// * ADAU1962A Reset GPIO
/// 
/// This application has to configure all the ICs. 
/// 

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let mut i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let mut pcm1865 = PCM1865::new(i2c, 0x4C);

    pcm1865.set_sck_xtal_selection(pcm1865::SckXtalSelection::Xtal)?;
    pcm1865.select_mode(true)?;
    pcm1865.master_clock_to_bit_clock_divider_value(2)?; // 12.228 Mhz BCLK
    pcm1865.bit_clock_to_left_right_clock_divider_value(64)?;
    pcm1865.auto_clock_detector_configuration(true)?;


    Ok(())
}