use embedded_svc::http::{Method};
use esp_idf_svc::{eventloop::EspSystemEventLoop, http::server::EspHttpServer, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi}};
use esp_idf_svc::hal::{modem::Modem, prelude::Peripherals};
use log::*;
use serde::Deserialize;
use std::{sync::{Arc, Mutex}, collections::VecDeque};
use esp_idf_hal::i2s::{I2S, I2SConfig, I2SOutput, I2SDataWidth, I2SMode, I2SClockSource};
use esp_idf_hal::prelude::*;
use esp_idf_sys::EspError;

const SSID: &str = "Leander";
const PASSWORD: &str = "hidden";
const INDEX_HTML: &str = include_str!("http_server_page.html");
const MAX_LEN: usize = 128;
const STACK_SIZE: usize = 10240;

#[derive(Deserialize)]
struct FormData<'a> {
    audio_data: &'a [u8], // This is the incoming raw audio data (e.g., PCM)
}

pub fn setup_webserver(modem: Modem, sys_loop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> Result<(), EspError> {
    info!("Setting up webserver");

    // Setup Wifi
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    connect_wifi(&mut wifi)?;

    let mut server = create_server()?;

    // Shared buffer for incoming audio data
    let buffer = Arc::new(Mutex::new(VecDeque::new()));

    server.fn_handler("/", Method::Get, |req| {
        req.into_ok_response()?
            .write_all(INDEX_HTML.as_bytes())
            .map(|_| ())
    })?;

    server.fn_handler::<anyhow::Error, _>("/upload", Method::Post, move |mut req| {
        let len = req.content_len().unwrap_or(0) as usize;

        if len > MAX_LEN {
            req.into_status_response(413)?
                .write_all("Request too big".as_bytes())?;
            return Ok(());
        }

        let mut buf = vec![0; len];
        req.read_exact(&mut buf)?;
        let mut resp = req.into_ok_response()?;

        // Assuming audio data is in raw PCM format
        let mut buffer_lock = buffer.lock().unwrap();
        for chunk in buf.chunks(2) { // 16-bit PCM samples (2 bytes)
            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
            buffer_lock.push_back(sample);
        }

        resp.write_all("Audio data received.".as_bytes())?;
        Ok(())
    })?;

    // Keep wifi and the server running beyond when main() returns (forever)
    core::mem::forget(wifi);
    core::mem::forget(server);

    // Now configure I2S for output
    let i2s = I2S::new();
    configure_i2s(&i2s)?;

    // Main loop to continuously send audio data via I2S
    loop {
        send_audio_to_i2s(&i2s, Arc::clone(&buffer));
    }
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<(), EspError> {
    let wifi_configuration = esp_idf_sys::wifi::Configuration::Client(esp_idf_sys::wifi::ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;
    Ok(())
}

fn create_server() -> Result<EspHttpServer<'static>, EspError> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    Ok(EspHttpServer::new(&server_configuration)?)
}

fn configure_i2s(i2s: &I2S) -> Result<(), EspError> {
    let config = I2SConfig {
        mode: I2SMode::Master,
        sample_rate: 48_000,
        word_size: I2SDataWidth::Bits16, // 16-bit audio samples
        ..Default::default()
    };
    i2s.configure(&config)?;
    Ok(())
}

fn send_audio_to_i2s(i2s: &I2S, buffer: Arc<Mutex<VecDeque<i16>>>) {
    let mut buffer_lock = buffer.lock().unwrap();
    if let Some(sample) = buffer_lock.pop_front() {
        // Write audio data to I2S
        if let Err(e) = i2s.write(&[sample]) {
            error!("Error sending audio data to I2S: {:?}", e);
        }
    }
}
