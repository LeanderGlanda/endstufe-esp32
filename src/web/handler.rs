use embedded_svc::http::{Method};
use esp_idf_svc::{eventloop::EspSystemEventLoop, http::server::EspHttpServer, nvs::EspDefaultNvsPartition, sys::EspError, wifi::{BlockingWifi, EspWifi}};
use esp_idf_svc::hal::{modem::Modem, prelude::Peripherals};
use log::*;
use serde::Deserialize;
use esp_idf_svc::hal::i2s;

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

    let mut server = create_server()?;

    server.fn_handler("/volume", esp_idf_svc::http::Method::Get, |req| {
        let respone = "Hello";
        Err(req.into_response(200, Some(respone), &[]))
    })?;

    Ok(())
}

fn create_server() -> Result<EspHttpServer<'static>, EspError> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    Ok(EspHttpServer::new(&server_configuration).unwrap())
}
