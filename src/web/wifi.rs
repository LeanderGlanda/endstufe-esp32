use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::modem::Modem,
    nvs::EspDefaultNvsPartition,
    sys::{wifi_interface_t_WIFI_IF_STA, WIFI_PROTOCOL_11B, WIFI_PROTOCOL_11G, WIFI_PROTOCOL_11N},
    wifi::{BlockingWifi, EspWifi, PmfConfiguration, ScanMethod, ScanSortMethod},
};

const SSID: &str = "Wollersberger";
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub fn setup_wifi(
    modem: Modem,
    sys_loop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
) -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    // Create and configure WiFi (using blocking APIs)
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop.clone(),
    )?;
    wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
        embedded_svc::wifi::ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            password: PASSWORD.try_into().unwrap(),
            scan_method: ScanMethod::CompleteScan(ScanSortMethod::Signal),
            pmf_cfg: PmfConfiguration::new_pmf_optional(),
            // You might add other fields as needed.
            ..Default::default()
        },
    ))?;

    for attempt in 0..3 {
        if !wifi
            .is_connected()
            .map_err(|_| anyhow::anyhow!("Error checking WiFi status"))?
        {
            wifi.start()?;
            set_wifi_protocols();
            match wifi.connect() {
                Ok(_) => {
                    log::info!("Connected to WiFi!");
                    wifi.wait_netif_up()?;
                    return Ok(wifi);
                }
                Err(e) => {
                    log::error!("Failed to connect to WiFi: {:#?}", e);
                    if e.code() == esp_idf_svc::sys::ESP_ERR_TIMEOUT {
                        log::error!("Failed to connect to WiFi: {}", e);
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }
    }

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("{:?}", ip_info);

    Ok(wifi)
}

fn set_wifi_protocols() {
    unsafe {
        // Only enable 802.11b, g and n on the STA interface
        // 802.11ax makes problems
        use esp_idf_svc::sys::*;
        esp_wifi_set_protocol(
            wifi_interface_t_WIFI_IF_STA,
            (WIFI_PROTOCOL_11B | WIFI_PROTOCOL_11G | WIFI_PROTOCOL_11N)
                .try_into()
                .unwrap(),
        );

        // Disable power‑save so the AP can’t sleep you out mid‑handshake
        // Does not seem to be required, so commented out for now
        // esp_wifi_set_ps(wifi_ps_type_t_WIFI_PS_NONE);
    }
}
