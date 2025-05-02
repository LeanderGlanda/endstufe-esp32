use anyhow::Ok;
use embedded_svc::http::{Method};
use esp_idf_svc::{eventloop::EspSystemEventLoop, http::server::EspHttpServer, nvs::EspDefaultNvsPartition, sys::EspError, wifi::{BlockingWifi, EspWifi}};
use esp_idf_svc::hal::{modem::Modem, prelude::Peripherals};
use log::*;
use serde::Deserialize;
use esp_idf_svc::hal::i2s;

use crate::api::commands::Command;

const SSID: &str = "Leander";
// const PASSWORD: &str = env!("WIFI_PASSWORD");
const PASSWORD: &str = "Leander999";
const INDEX_HTML: &str = include_str!("http_server_page.html");
const MAX_LEN: usize = 128;
const STACK_SIZE: usize = 10240;

pub fn start_server(modem: Modem, sys_loop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> Result<EspHttpServer<'static>, anyhow::Error> {
    info!("Setting up webserver");

    let mut server = create_server()?;

    mount_routes(&mut server)?;

    Ok(server)
}

fn create_server() -> Result<EspHttpServer<'static>, anyhow::Error> {
    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    let server = EspHttpServer::new(&server_configuration)?;

    Ok(server)
}


/// Mountet alle API-Routen auf dem HTTP-Server
pub fn mount_routes(server: &mut EspHttpServer) -> Result<(), anyhow::Error> {
    server.fn_handler("/api", Method::Post, |mut req| {
        // 1) Body auslesen
        let mut buf = Vec::new();
        req.read(&mut buf)?;
        // 2) JSON -> Command
        let cmd: Command = serde_json::from_slice(&buf)
            .map_err(|e| anyhow::anyhow!(e))?;
        // 3) Command verarbeiten
        let resp = cmd.handle();
        // 4) Response -> JSON
        let body = serde_json::to_vec(&resp)?;
        // 5) Antwort senden
        let mut response = req.into_ok_response()?;
        response.write(&body)?;
        Ok(())
    })?;

    Ok(())
}