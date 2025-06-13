#![allow(unused)]

use std::collections::VecDeque;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, thread};

use drivers::{adau1467, adau1962a, tpa3116d2};
use embedded_svc::http::Headers;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyInputPin, AnyOutputPin, InputPin, PinDriver, Pull};
use esp_idf_svc::hal::gpio::Output;
use esp_idf_svc::hal::gpio::Input;
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::i2s::config::ClockSource;
use esp_idf_svc::hal::i2s::config::Config;
use esp_idf_svc::hal::i2s::config::{DataBitWidth, StdConfig};
use esp_idf_svc::hal::i2s::I2sDriver;
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimer, LedcTimerDriver, LEDC};
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::hal::pcnt::Pcnt;
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::timer::Timer;
use esp_idf_svc::hal::{pcnt, peripheral, prelude::*};
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use esp_idf_svc::http::Method;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::netif::EspNetif;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{TickType_t, MALLOC_CAP_INTERNAL};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use rtp_rs::RtpReader;

mod api;
mod drivers;
mod hardware_context;
mod hardware_init;
mod i2c_helper;
mod linkwitz_riley_coeffs;
mod sigmastudio;
mod web;

use crate::drivers::{
    adau1467::ADAU1467,
    adau1962a::ADAU1962A,
    pcm1865::{self, PCM1865},
    tpa3116d2::TPA3116D2,
};
use crate::encoder::Encoder;

const HARDWARE_CONNECTED: bool = true;
const ENABLE_WEB: bool = true;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio12;
    let scl = peripherals.pins.gpio13;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));

    let hardware_context = Arc::new(hardware_context::HardwareContext::new(shared_i2c));

    let handle;

    if HARDWARE_CONNECTED {
        hardware_init::hardware_init(hardware_context.clone())?;

        handle = std::thread::spawn(|| hardware_control(
            peripherals.pins.gpio35.into(),
            peripherals.pins.gpio36.into(),
            peripherals.pins.gpio37.into(),
            peripherals.pins.gpio38.into(),
            peripherals.pins.gpio39.into(),
            peripherals.pins.gpio40.into(),
            peripherals.pins.gpio41.into(),
            peripherals.pins.gpio42.into(),
            peripherals.ledc.into(),
            peripherals.pcnt0,
        ));

        log::info!("Hardware init complete");
    }

    if ENABLE_WEB {
        let wifi = web::wifi::setup_wifi(peripherals.modem, sys_loop, nvs)?;

        let web_server = web::server::start_server(hardware_context)?;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    Ok(())
}

fn hardware_control(
    encoder_pin_a: AnyIOPin,
    encoder_pin_b: AnyIOPin,
    button_pin_1: AnyIOPin,
    button_pin_2: AnyIOPin,
    button_pin_3: AnyIOPin,
    led_pin_red: AnyOutputPin,
    led_pin_green: AnyOutputPin,
    led_pin_blue: AnyOutputPin,
    ledc: LEDC,
    pcnt: impl Peripheral<P = impl Pcnt>,
) -> anyhow::Result<()> {

    log::info!("Hardware control thread started");

    let mut button_mute = PinDriver::input(button_pin_1)?;
    let mut button_bassboost = PinDriver::input(button_pin_2)?;
    let mut button_standby = PinDriver::input(button_pin_3)?;

    button_mute.set_pull(Pull::Down);
    button_bassboost.set_pull(Pull::Down);
    button_standby.set_pull(Pull::Down);

    let timer = LedcTimerDriver::new(
            ledc.timer0,
            &TimerConfig::default()
        )?;

    let mut red = LedcDriver::new(ledc.channel0, &timer, led_pin_red)?;
    let mut green = LedcDriver::new(ledc.channel1, &timer, led_pin_green)?;
    let mut blue = LedcDriver::new(ledc.channel2, &timer, led_pin_blue)?;

    set_rgb(&mut red, &mut green, &mut blue, 0, 100, 0)?;

    let encoder = Encoder::new(pcnt, encoder_pin_a, encoder_pin_b).unwrap();

    let mut last_value = 0i32;
    loop {
        let value = encoder.get_value()?;
        if value != last_value {
            log::info!("value: {value}");
            last_value = value;
        }
        if button_mute.is_high(){
            log::info!("mute high");

        }
        if button_bassboost.is_high(){
            log::info!("bassboost high");
        }
        if button_standby.is_high(){
            log::info!("standby high");
        }
        std::thread::sleep(Duration::from_millis(1000));
    }


    Ok(())
}

fn set_rgb(
    red: &mut LedcDriver<'_>,
    green: &mut LedcDriver<'_>,
    blue: &mut LedcDriver<'_>,
    r: u8,
    g: u8,
    b: u8,
) -> anyhow::Result<()> {
    red.set_duty(red.get_max_duty() * r as u32 / 255)?;
    green.set_duty(green.get_max_duty() * g as u32 / 255)?;
    blue.set_duty(blue.get_max_duty() * b as u32 / 255)?;
    Ok(())
}

mod encoder {
    use std::cmp::min;
    use std::sync::atomic::AtomicI32;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;

    use esp_idf_svc::hal::gpio::AnyInputPin;
    use esp_idf_svc::hal::gpio::InputPin;
    use esp_idf_svc::hal::pcnt::*;
    use esp_idf_svc::hal::peripheral::Peripheral;
    use esp_idf_svc::sys::EspError;

    const LOW_LIMIT: i16 = -100;
    const HIGH_LIMIT: i16 = 100;

    pub struct Encoder<'d> {
        unit: PcntDriver<'d>,
        approx_value: Arc<AtomicI32>,
    }

    impl<'d> Encoder<'d> {
        pub fn new<PCNT: Pcnt>(
            pcnt: impl Peripheral<P = PCNT> + 'd,
            pin_a: impl Peripheral<P = impl InputPin> + 'd,
            pin_b: impl Peripheral<P = impl InputPin> + 'd,
        ) -> Result<Self, EspError> {
            let mut unit = PcntDriver::new(
                pcnt,
                Some(pin_a),
                Some(pin_b),
                Option::<AnyInputPin>::None,
                Option::<AnyInputPin>::None,
            )?;
            unit.channel_config(
                PcntChannel::Channel0,
                PinIndex::Pin0,
                PinIndex::Pin1,
                &PcntChannelConfig {
                    lctrl_mode: PcntControlMode::Reverse,
                    hctrl_mode: PcntControlMode::Keep,
                    pos_mode: PcntCountMode::Decrement,
                    neg_mode: PcntCountMode::Increment,
                    counter_h_lim: HIGH_LIMIT,
                    counter_l_lim: LOW_LIMIT,
                },
            )?;
            unit.channel_config(
                PcntChannel::Channel1,
                PinIndex::Pin1,
                PinIndex::Pin0,
                &PcntChannelConfig {
                    lctrl_mode: PcntControlMode::Reverse,
                    hctrl_mode: PcntControlMode::Keep,
                    pos_mode: PcntCountMode::Increment,
                    neg_mode: PcntCountMode::Decrement,
                    counter_h_lim: HIGH_LIMIT,
                    counter_l_lim: LOW_LIMIT,
                },
            )?;

            unit.set_filter_value(min(10 * 80, 1023))?;
            unit.filter_enable()?;

            let approx_value = Arc::new(AtomicI32::new(0));
            // unsafe interrupt code to catch the upper and lower limits from the encoder
            // and track the overflow in `value: Arc<AtomicI32>` - I plan to use this for
            // a wheeled robot's odomerty
            unsafe {
                let approx_value = approx_value.clone();
                unit.subscribe(move |status| {
                    let status = PcntEventType::from_repr_truncated(status);
                    if status.contains(PcntEvent::HighLimit) {
                        approx_value.fetch_add(HIGH_LIMIT as i32, Ordering::SeqCst);
                    }
                    if status.contains(PcntEvent::LowLimit) {
                        approx_value.fetch_add(LOW_LIMIT as i32, Ordering::SeqCst);
                    }
                })?;
            }
            unit.event_enable(PcntEvent::HighLimit)?;
            unit.event_enable(PcntEvent::LowLimit)?;
            unit.counter_pause()?;
            unit.counter_clear()?;
            unit.counter_resume()?;

            Ok(Self { unit, approx_value })
        }

        pub fn get_value(&self) -> Result<i32, EspError> {
            let value =
                self.approx_value.load(Ordering::Relaxed) + self.unit.get_counter_value()? as i32;
            Ok(value)
        }
    }
}