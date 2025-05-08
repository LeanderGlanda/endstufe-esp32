use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::modem::Modem, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi, ScanMethod}};


const SSID: &str = "Leander";
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub fn setup_wifi(modem: Modem, sys_loop: EspSystemEventLoop, nvs: EspDefaultNvsPartition) -> anyhow::Result<()> {
    // Create and configure WiFi (using blocking APIs)
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop.clone(),
    )?;
    wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
        embedded_svc::wifi::ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            password: PASSWORD.try_into().unwrap(),
            scan_method: ScanMethod::FastScan,
            // You might add other fields as needed.
            ..Default::default()
        },
    ))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    log::info!("Wifi connected and up");

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("{:?}", ip_info);

    core::mem::forget(wifi);

    Ok(())
}