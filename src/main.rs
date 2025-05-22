#![allow(unused)]

use std::collections::VecDeque;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::time::{Duration, Instant};

use drivers::{adau1467, adau1962a, tpa3116d2};
use embedded_svc::http::Headers;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::netif::EspNetif;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::i2s::config::{DataBitWidth, StdConfig};
use esp_idf_svc::hal::i2s::I2sDriver;
use esp_idf_svc::hal::i2s::config::ClockSource;
use esp_idf_svc::hal::i2s::config::Config;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::{peripheral, prelude::*};
use esp_idf_svc::http::Method;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{TickType_t, MALLOC_CAP_INTERNAL};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use rtp_rs::RtpReader;

mod drivers;
mod i2c_helper;
mod api;
mod web;
mod sigmastudio;
mod hardware_init;
mod linkwitz_riley_coeffs;
mod hardware_context;

use crate::drivers::{pcm1865::{self, PCM1865}, adau1467::ADAU1467, adau1962a::ADAU1962A, tpa3116d2::TPA3116D2};

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
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));

    let hardware_context = Arc::new(hardware_context::HardwareContext::new(shared_i2c));

    if HARDWARE_CONNECTED {
        hardware_init::hardware_init(hardware_context.clone())?;

        log::info!("Hardware init complete");
    }

    if ENABLE_WEB {
        web::wifi::setup_wifi(peripherals.modem, sys_loop, nvs)?;
    
        let web_server = web::server::start_server(hardware_context)?;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
    

    Ok(())
}
