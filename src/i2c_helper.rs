use std::sync::{Arc, Mutex};
use esp_idf_svc::hal::{delay::BLOCK, i2c::I2cDriver};
use log;

pub fn read_registers_16bit_address(i2c: &Arc<Mutex<I2cDriver>>, i2c_device: u8, address: u16, register_count: usize) {
    let mut i2c = i2c.lock().expect("Failed to lock I2C driver");
    let mut register_values = Vec::with_capacity(register_count*4);
    i2c.write_read(i2c_device, &address.to_be_bytes(), &mut register_values, BLOCK).unwrap();
    println!("{:?}", register_values);
}

pub struct I2CDevice {
    address: u8,
    start_reg: u16,
    end_reg: u16,
    reg_size: u8, // 1 = 8-bit reg address + 1-byte value; 2 = 16-bit reg address + 4-byte value
}

pub fn pretty_register_dump(i2c: &Arc<Mutex<I2cDriver>>) {
    log::info!("\n--- Beginning Pretty Register Dump ---");

    let i2c_devices = [
        I2CDevice {
            address: 0x4A,  // PCM1865
            start_reg: 0x00,
            end_reg: 0x78,
            reg_size: 1,
        },
        I2CDevice {
            address: 0x04,  // ADAU1962A
            start_reg: 0x00,
            end_reg: 0x1F,
            reg_size: 1,
        },
        I2CDevice {
            address: 0x38,  // ADAU1467
            start_reg: 0x0000,
            end_reg: 0x0190, // example limit
            reg_size: 2,
        },
    ];

    for device in &i2c_devices {
        log::info!(
            "Device 0x{:02X} - Registers 0x{:04X} to 0x{:04X}:",
            device.address,
            device.start_reg,
            device.end_reg
        );

        let mut reg_values = Vec::new();

        for reg in device.start_reg..=device.end_reg {
            let mut i2c = i2c.lock().expect("Failed to lock I2C driver");

            let (reg_bytes, mut data): (Vec<u8>, Vec<u8>) = match device.reg_size {
                1 => (vec![reg as u8], vec![0u8; 1]),
                2 => (vec![(reg >> 8) as u8, (reg & 0xFF) as u8], vec![0u8; 4]),
                _ => continue,
            };

            let result = i2c.write_read(
                device.address,
                &reg_bytes,
                &mut data,
                esp_idf_svc::hal::delay::BLOCK,
            );

            match result {
                Ok(()) => {
                    let val = match device.reg_size {
                        1 => Some(u32::from(data[0])),
                        2 => Some(u32::from_be_bytes([data[0], data[1], data[2], data[3]])),
                        _ => None,
                    };
                    reg_values.push((reg, val));
                }
                Err(e) => {
                    log::error!(
                        "  Failed to read device 0x{:02X} at register 0x{:04X}: {:?}",
                        device.address,
                        reg,
                        e
                    );
                    reg_values.push((reg, None));
                }
            }

            if reg_values.len() == 8 || reg == device.end_reg {
                let mut output = format!("  0x{:04X}:", reg_values[0].0);
                for &(_, value) in &reg_values {
                    match value {
                        Some(v) => {
                            if device.reg_size == 1 {
                                output.push_str(&format!(" 0x{:02X}", v as u8));
                            } else {
                                output.push_str(&format!(" 0x{:08X}", v));
                            }
                        }
                        None => {
                            output.push_str(if device.reg_size == 1 { " --" } else { " --------" });
                        }
                    }
                }
                log::info!("{}", output);
                reg_values.clear();
            }
        }

        log::info!("");
    }

    log::info!("\n--- End of Pretty Register Dump ---");
}
