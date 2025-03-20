use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::modem::Modem, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi}};

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