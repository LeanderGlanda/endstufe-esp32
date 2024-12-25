use std::sync::{Arc, Mutex};

use log;
use esp_idf_svc::hal::i2c::I2cDriver;

pub struct I2CDevice {
    address: u8,
    start_reg: u8,
    end_reg: u8,
}


pub fn pretty_register_dump(i2c: &Arc<Mutex<I2cDriver>>) {
    log::info!("\n--- Beginning Pretty Register Dump ---");

    // Konfiguration der ICs mit ihren Adressbereichen
    let i2c_devices = [
        I2CDevice {
            address: 0x4A,  // PCM1865
            start_reg: 0x00,
            end_reg: 0x78,
        },
        I2CDevice {
            address: 0x38,  // ADAU1962A
            start_reg: 0x00,
            end_reg: 0x1F,
        },
    ];

    for device in &i2c_devices {
        log::info!(
            "Device 0x{:02X} - Registers 0x{:02X} to 0x{:02X}:",
            device.address,
            device.start_reg,
            device.end_reg
        );

        for reg in device.start_reg..=device.end_reg {
            let mut i2c = i2c.lock().expect("Failed to lock I2C driver");
            let mut data = [0u8; 1];

            // Schreibe das Register, das gelesen werden soll
            if let Err(e) = i2c.write_read(device.address, &[reg], &mut data, esp_idf_svc::hal::delay::BLOCK) {
                log::error!(
                    "  Failed to access device 0x{:02X} at register 0x{:02X}: {:?}",
                    device.address,
                    reg,
                    e
                );
                continue;
            }

            log::info!("  Register 0x{:02X}: 0x{:02X}", reg, data[0]);
        }
        log::info!("");
    }

    log::info!("\n--- End of Pretty Register Dump ---");
}