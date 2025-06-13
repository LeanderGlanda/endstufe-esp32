use std::sync::{Arc, Mutex};

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

mod api;
mod drivers;
mod encoder;
mod hardware_context;
mod hardware_control;
mod hardware_init;
mod i2c_helper;
mod linkwitz_riley_coeffs;
mod sigmastudio;
mod web;
mod sticky_limiter;


const HARDWARE_CONNECTED: bool = true;
const ENABLE_WEB: bool = true;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio12;
    let scl = peripherals.pins.gpio13;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));

    let hardware_context = Arc::new(hardware_context::HardwareContext::new(shared_i2c));

    let mut handle = None;

    if HARDWARE_CONNECTED {
        hardware_init::hardware_init(hardware_context.clone())?;

        let hardware_context_clone = hardware_context.clone();

        handle = Some(std::thread::spawn(move || {
            hardware_control::hardware_control(
                peripherals.pins.gpio35.into(),
                peripherals.pins.gpio36.into(),
                peripherals.pins.gpio37.into(),
                peripherals.pins.gpio38.into(),
                peripherals.pins.gpio39.into(),
                peripherals.pins.gpio40.into(),
                peripherals.pins.gpio41.into(),
                peripherals.pins.gpio42.into(),
                peripherals.ledc.into(),
                peripherals.pcnt0,
                hardware_context_clone,
            )
        }));

        log::info!("Hardware init complete");
    }

    if ENABLE_WEB {
        let _wifi = web::wifi::setup_wifi(peripherals.modem, sys_loop, nvs)?;

        let _web_server = web::server::start_server(hardware_context)?;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    if handle.is_some() {
        handle.unwrap().join().expect("Hardware control thread panicked")?;
    }
    

    Ok(())
}
