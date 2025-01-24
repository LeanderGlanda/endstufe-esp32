#![allow(static_mut_refs)]

use std::{sync::{Arc, Mutex, OnceLock}, time::Duration};

use esp_idf_svc::hal::i2c::I2cDriver;

extern "C" {
    fn load_sigmastudio_program_adau1467();
}
static mut I2C: OnceLock<Arc<Mutex<I2cDriver>>> = OnceLock::new();

pub fn load_sigmastudio_dsp_program(local_i2c: &Arc<Mutex<I2cDriver>>) {
    // This is safe as we call load_sigmastudio_program_adau1467 which then uses the i2c before we return here.
    let static_i2c = unsafe { std::mem::transmute::<&Arc<Mutex<I2cDriver<'_>>>, &Arc<Mutex<I2cDriver<'static>>>>(local_i2c) };

    unsafe{ I2C.set(static_i2c.clone()).unwrap_or_else(|_| panic!("I2C already initialized!")); }
    
    log::info!("Executing C Code");

    unsafe {
        load_sigmastudio_program_adau1467();
    }


    // Make sure the I2C in here gets invalidated so it can't be used after the function returns
    // when returning, the I2C may get invalid.
    unsafe { I2C.take() };

}

#[no_mangle]
pub extern "C" fn sleep_ms(microseconds: u64) {
    std::thread::sleep(Duration::from_millis(microseconds));
}

#[no_mangle]
pub extern "C" fn i2c_write(i2c_address: u8, buffer: *const u8, length: i32) -> i32 {

    let mut i2c = unsafe { I2C.get().expect("I2C not initialized").lock().unwrap() };

    // Safety: You need to ensure that `buffer` is valid and points to `length` bytes.
    if !buffer.is_null() && length > 0 {
        unsafe {
            let slice = std::slice::from_raw_parts(buffer, length as usize);
            i2c.write(i2c_address, slice, esp_idf_svc::hal::delay::BLOCK).unwrap();
        }
    } else {
        eprintln!("Invalid buffer or length!");
    }

    // Sending the data was successful, so we can return the number of bytes sent safely
    return length;
}