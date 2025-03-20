#![allow(unused)]

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::{io, thread};
use std::time::{Duration, Instant};

use drivers::{adau1467, adau1962a, tpa3116d2};
use embedded_svc::http::Headers;
use encoder::Encoder;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{Pin, PinDriver, Pull};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::i2s::config::{DataBitWidth, StdConfig};
use esp_idf_svc::hal::i2s::I2sDriver;
use esp_idf_svc::hal::i2s::config::ClockSource;
use esp_idf_svc::hal::i2s::config::Config;
use esp_idf_svc::hal::ledc::{config, LedcDriver, LedcTimerDriver};
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::{peripheral, prelude::*};
use esp_idf_svc::http::Method;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::{TickType_t, MALLOC_CAP_INTERNAL};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::http::server::{Configuration as HttpConfig, EspHttpServer};
use rtp_rs::RtpReader;

mod drivers;
mod i2c_helper;
mod api;
mod control;
mod web;
mod sigmastudio;
mod hardware_init;

use crate::api::commands::SystemCommand;
use crate::drivers::{pcm1865::{self, PCM1865}, adau1467::ADAU1467, adau1962a::ADAU1962A, tpa3116d2::TPA3116D2};
use control::handle_command;

const HARDWARE_CONNECTED: bool = false;

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
    let sda = peripherals.pins.gpio5;
    let scl = peripherals.pins.gpio6;

    let config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;

    let shared_i2c = Arc::new(Mutex::new(i2c));

    if HARDWARE_CONNECTED {
        hardware_init::hardware_init(&shared_i2c)?;

        log::info!("Hardware init complete");
    }

    // web::handler::setup_webserver(peripherals.modem, sys_loop, nvs)?;

    // Your code here:

    // Pins für Encoder (z.B. GPIO 18 für A und GPIO 19 für B)
    //let mut pin_a = PinDriver::input(peripherals.pins.gpio2)?;
    let mut pin_a = peripherals.pins.gpio2;
    let mut pin_b = peripherals.pins.gpio3;

     // Pins für Taster
     let mut button = PinDriver::input(peripherals.pins.gpio11)?;
     button.set_pull(Pull::Down)?;
     //Pins fuer RGB-LED
     let mut channel = LedcDriver::new(
        peripherals.ledc.channel0,
        LedcTimerDriver::new(
            peripherals.ledc.timer0,
            &config::TimerConfig::new().frequency(25.kHz().into()),
        )?,
        peripherals.pins.gpio4,
    )?;

    println!("Starting duty-cycle loop");

    let max_duty = channel.get_max_duty();
    for numerator in [0, 1, 2, 3, 4, 5].iter().cycle() {
        println!("Duty {numerator}/5");
        channel.set_duty(max_duty * numerator / 5)?;
        std::thread::sleep(Duration::from_millis(1000));
    }

    // pin_a.set_pull(Pull::Up);

    let encoder = Encoder::new(peripherals.pcnt0, &mut pin_a, &mut pin_b)?;

    let mut last_value = 0i32;
    loop {
        let value = encoder.get_value()?;
        if value != last_value {
            println!("value: {value}");
            last_value = value;
        }
        if button.is_high(){
            log::info!("high");

        }
        std::thread::sleep(Duration::from_millis(1000));
    }
    
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