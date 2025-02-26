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
mod control;
mod web;
mod sigmastudio;

use crate::api::commands::SystemCommand;
use crate::drivers::{pcm1865::{self, PCM1865}, adau1467::ADAU1467, adau1962a::ADAU1962A, tpa3116d2::TPA3116D2};
use control::handle_command;

// const AUDIO_DATA: &[u8] = include_bytes!("../HellsBells.raw");

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

fn hardware_init(shared_i2c: &Arc<Mutex<I2cDriver>>) -> anyhow::Result<()> {

    let mut tpa3116d2 = TPA3116D2::new(shared_i2c.clone());

    tpa3116d2.enable_speaker_outputs(false)?;  

    setup_pcm1865(shared_i2c.clone())?;

    setup_adau1962a(shared_i2c.clone())?;

    setup_adau1467(shared_i2c.clone())?;

    //setup_tpa3116d2(shared_i2c.clone())?;
    tpa3116d2.enable_speaker_outputs(true)?;  

    // call_every_command(shared_i2c.clone())?;

    // i2c_helper::pretty_register_dump(&shared_i2c);

    Ok(())
}

fn setup_wifi(modem: Modem, sys_loop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> anyhow::Result<()> {
    // Create and configure WiFi (using blocking APIs)
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop.clone(),
    )?;
    wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
        embedded_svc::wifi::ClientConfiguration {
            ssid: "Wollersberger".try_into().unwrap(),
            password: "hidden".try_into().unwrap(),
            // You might add other fields as needed.
            ..Default::default()
        },
    ))?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    log::info!("Connected to WiFi");

    core::mem::forget(wifi);

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    /*let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(async {
        log::info!("Running async tasks...");
        hardware_init()?;
        let web_task = task::spawn(web::handler::start_server());
        tokio::try_join!(web_task)?;
        Ok::<(), anyhow::Error>(())
    })?;*/

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));


    let bclk = peripherals.pins.gpio0;
    let ws = peripherals.pins.gpio1;
    let dout = peripherals.pins.gpio10;
    let mclk = Some(peripherals.pins.gpio11);

    const SAMPLE_RATE: u32 = 48_000;
    const SINE_FREQ: u32 = 440; // 440 Hz A4 tone
    const AMPLITUDE: i16 = i16::MAX / 1024; // Reduce amplitude to prevent distortion

    let config = StdConfig::philips(SAMPLE_RATE, DataBitWidth::Bits16);

    let mut i2s_driver = I2sDriver::new_std_tx(
        peripherals.i2s0,
        &config,
        bclk,
        dout,
        mclk,
        ws,
    )?;

    hardware_init(&shared_i2c)?;

    setup_wifi(peripherals.modem, sys_loop, nvs)?;

    log::info!("Playing back audio");

    let (tx, rx): (SyncSender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::sync_channel(64); // Größere Queue

    let socket = UdpSocket::bind("0.0.0.0:5004").expect("Failed to bind socket");
    //(socket.set_nonblocking(true).ok(); // Setze Non-Blocking Modus für bessere Performance

    {
        let tx = tx.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 1500];

            loop {
                match socket.recv_from(&mut buffer) {
                    Ok((size, _src)) => {
                        if let Ok(rtp) = RtpReader::new(&buffer[..size]) {
                            let _ = tx.send(rtp.payload().to_vec());
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_micros(500)); // Verhindert CPU-Überlastung
                    }
                }
            }
        });
    }

    log::info!("Enabling I2S");
    i2s_driver.tx_enable();

    // --- Consumer Thread (I2S Playback) ---
    {
        thread::spawn(move || {
            let mut buffer: VecDeque<u8> = VecDeque::new();
            let mut last_received_time = Instant::now();

            loop {
                if let Ok(samples) = rx.try_recv() {
                    buffer.extend(samples);
                    let elapsed_us = last_received_time.elapsed().as_micros();
                    if buffer.len() > 16384 {
                        log::warn!("Audio Buffer Overflow! Len: {}", buffer.len());
                    }
                    last_received_time = Instant::now();
                }

                if buffer.len() >= 8192 {
                    let data_to_send: Vec<u8> = buffer.drain(..8192).collect();
                    let timeout = TickType_t::Hz(2000);
                    if let Err(e) = i2s_driver.write_all(&data_to_send, timeout.into()) {
                        log::error!("I2S write error: {:?}", e);
                    }
                }

                thread::sleep(Duration::from_micros(500)); // Verhindert unnötige CPU-Auslastung
                unsafe {esp_idf_svc::sys::vTaskDelay(1); }
            }
        });
    }

    loop {
        thread::sleep(Duration::from_millis(100)); // Hauptthread entlasten
    }



    thread::sleep(Duration::from_millis(100));
    

    // web::handler::setup_webserver(peripherals.modem, sys_loop, nvs)?;

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
    log::info!("Setting up PCM1865");

    let mut pcm1865 = PCM1865::new(i2c, 0x4a);

    pcm1865.set_sck_xtal_selection(pcm1865::SckXtalSelection::Xtal)?;
    pcm1865.select_mode(true)?;
    pcm1865.master_clock_to_bit_clock_divider_value(2)?; // 12.228 Mhz BCLK
    pcm1865.bit_clock_to_left_right_clock_divider_value(64)?;
    pcm1865.auto_clock_detector_configuration(true)?;
    Ok(())
}

fn setup_adau1962a(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1962a");

    let mut adau1962a = ADAU1962A::new(i2c, 0x04);

    adau1962a.set_reset(true)?;
    adau1962a.master_power_up(true)?;
    adau1962a.set_xtal_oscillator_enabled(true)?;
    adau1962a.set_pll_input_source(adau1962a::MclkiXtaliOrDlrclkSelection::MclkiOrXtali)?;
    adau1962a.set_master_clock_fs_ratio(adau1962a::MasterClockFsRatio::Ratio512)?;
    adau1962a.set_dac_clock_select(adau1962a::DacClockSource::PLL)?;
    adau1962a.set_serial_audio_interface(adau1962a::AudioInterface::Stero)?;
    adau1962a.set_sample_rate_selection(adau1962a::SampleRate::FS192)?;
    adau1962a.set_dlrclk_polarity(false)?;
    adau1962a.set_serial_interface_master(true)?;
    adau1962a.set_master_mute(false)?;

    Ok(())
}

fn setup_adau1467(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1467");

    let mut adau1467 = ADAU1467::new(i2c, 0x38);

    adau1467.set_reset(true)?;
    adau1467.load_dsp_program()?;

    Ok(())
}

fn setup_tpa3116d2(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up TPA3116D2");

    let mut tpa3116d2 = TPA3116D2::new(i2c);

    tpa3116d2.enable_speaker_outputs(true)?;

    Ok(())
}


/*fn call_every_command(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    let mut pcm1865 = PCM1865::new(i2c, 0x4C);

    // Initialize components
    // let mut pcm1865 = PCM1865::new(/* args here */);
    let dsp = ADAU1467::new(/* args here */);
    // let dac = ADAU1962A::new(/* args here */);
    let amplifier = TPA3116D2::new(/* args here */);

    // Example of issuing a command to the system
    let command1 = SystemCommand::SetVolume { channel: 1, level: 75 };
    // handle_command(command1, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command2 = SystemCommand::MuteChannel { channel: 1 };
    // handle_command(command2, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command3 = SystemCommand::SetInputSource { channel: 1, source_id: 1 };
    // handle_command(command3, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    let command4 = SystemCommand::UnmuteChannel { channel: 1 };
    // handle_command(command4, &mut pcm1865, &dsp, &dac, &amplifier)?;
    
    Ok(())
}*/