use std::sync::Arc;

use anyhow::Ok;
use embedded_svc::http::{Headers, Method};
use esp_idf_svc::{eventloop::EspSystemEventLoop, http::server::EspHttpServer, io::{Read, Write}, nvs::EspDefaultNvsPartition, sys::EspError, wifi::{BlockingWifi, EspWifi}};
use esp_idf_svc::hal::{modem::Modem, prelude::Peripherals};
use log::*;
use serde::Deserialize;
use esp_idf_svc::hal::i2s;

use crate::{api::commands::Command, hardware_context::HardwareContext};

const INDEX_HTML: &str = include_str!("http_server_page.html");
const MAX_LEN: usize = 128;
const STACK_SIZE: usize = 10240;

pub fn start_server(hardware_context: Arc<HardwareContext<'static>>) -> Result<EspHttpServer<'static>, anyhow::Error> {
    info!("Setting up webserver");

    let mut server = create_server()?;

    mount_routes(&mut server, hardware_context)?;

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
pub fn mount_routes(server: &mut EspHttpServer, hardware_context: Arc<HardwareContext<'static>>) -> Result<(), anyhow::Error> {

    let hardware_context_clone = Arc::clone(&hardware_context);

    server.fn_handler("/api", Method::Post, move |mut req| {
        log::info!("Incoming request");

        let len = req.content_len().unwrap_or(0) as usize;
        if len > MAX_LEN {
            req.into_status_response(413)?
                .write_all("Request too big".as_bytes())?;
            return Ok(());
        }

        let mut buf = vec![0; len];
        req.read_exact(&mut buf)?;
        
        // 2) JSON -> Command
        let cmd: Command = serde_json::from_slice(&buf)
            .map_err(|e| anyhow::anyhow!(e))?;
        // 3) Command verarbeiten
        let resp = cmd.handle(&hardware_context_clone);
        // 4) Response -> JSON
        let body = serde_json::to_vec(&resp)?;
        // 5) Antwort senden
        let mut response = req.into_ok_response()?;
        response.write(&body)?;
        Ok(())
    })?;

    Ok(())
}