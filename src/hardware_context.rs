use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::i2c::I2cDriver;

use crate::drivers::{
    adau1467::ADAU1467,
    adau1962a::{self, ADAU1962A},
    pcm1865::{self, PCM1865},
    tpa3116d2::TPA3116D2,
};

pub struct HardwareContext<'a> {
    pub i2c: Arc<Mutex<I2cDriver<'a>>>,
    pub pcm1865: Mutex<PCM1865<'a>>,
    pub adau1467: Mutex<ADAU1467<'a>>,
    pub adau1962a: Mutex<ADAU1962A<'a>>,
    pub tpa3116d2: Mutex<TPA3116D2<'a>>,
}

impl<'a> HardwareContext<'a> {
    //pub fn new(i2c: I2cDriver<'a>, pcm1865: PCM1865<'a>, adau1467: ADAU1467<'a>, adau1962a: ADAU1962A<'a>, tpa3116d2: TPA3116D2<'a>) -> HardwareContext<'a> {
    pub fn new(i2c: Arc<Mutex<I2cDriver<'a>>>) -> HardwareContext<'a> {
        let mut pcm1865 = Mutex::new(PCM1865::new(i2c.clone(), 0x4a));
        let mut adau1962a = Mutex::new(ADAU1962A::new(i2c.clone(), 0x04));
        let mut adau1467 = Mutex::new(ADAU1467::new(i2c.clone(), 0x38));
        let mut tpa3116d2 = Mutex::new(TPA3116D2::new(i2c.clone()));

        HardwareContext {
            i2c,
            pcm1865,
            adau1467,
            adau1962a,
            tpa3116d2,
        }
    }
}
