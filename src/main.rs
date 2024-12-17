use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::prelude::*;
use tokio::runtime::Runtime;
use tokio::task;

mod drivers;
mod i2c_helper;
mod api;
mod control;
mod web;

use crate::api::commands::SystemCommand;
use crate::drivers::{pcm1865::{self, PCM1865}, adau1467::ADAU1467, adau1962a::ADAU1962A, tpa3116d2::TPA3116D2};
use control::handle_command;

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

fn hardware_init() -> anyhow::Result<()> {
    let peripherals = Peripherals::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));

    call_every_command(shared_i2c.clone())?;
    setup_pcm1865(shared_i2c.clone())?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        log::info!("Running async tasks...");
        hardware_init()?;
        let web_task = task::spawn(web::handler::start_server());
        tokio::try_join!(web_task)?;
        Ok::<(), anyhow::Error>(())
    })?;

    //hardware_init()?;

    // Shared state if needed (e.g., a Mutex or similar)
    // let shared_state = Arc::new(());

    // Spawn other tasks
    // task::spawn(hardware::run(shared_state.clone()));

    // Start the web server
    // let web_task = task::spawn(web::handler::start_server());

    // Await all tasks
    // tokio::try_join!(hardware_task, web_task)?;

    Ok(())
}

fn setup_pcm1865(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    let mut pcm1865 = PCM1865::new(i2c, 0x4C);

    pcm1865.set_sck_xtal_selection(pcm1865::SckXtalSelection::Xtal)?;
    pcm1865.select_mode(true)?;
    pcm1865.master_clock_to_bit_clock_divider_value(2)?; // 12.228 Mhz BCLK
    pcm1865.bit_clock_to_left_right_clock_divider_value(64)?;
    pcm1865.auto_clock_detector_configuration(true)?;
    Ok(())
}

fn call_every_command(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    let mut pcm1865 = PCM1865::new(i2c, 0x4C);

    // Initialize components
    // let mut pcm1865 = PCM1865::new(/* args here */);
    let dsp = ADAU1467::new(/* args here */);
    let dac = ADAU1962A::new(/* args here */);
    let amplifier = TPA3116D2::new(/* args here */);

    // Example of issuing a command to the system
    let command1 = SystemCommand::SetVolume { channel: 1, level: 75 };
    handle_command(command1, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command2 = SystemCommand::MuteChannel { channel: 1 };
    handle_command(command2, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command3 = SystemCommand::SetInputSource { channel: 1, source_id: 1 };
    handle_command(command3, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command4 = SystemCommand::UnmuteChannel { channel: 1 };
    handle_command(command4, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    Ok(())
}