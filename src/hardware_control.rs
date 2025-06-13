use std::collections::VecDeque;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, thread};
use embedded_svc::http::Headers;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::gpio::Input;
use esp_idf_svc::hal::gpio::Output;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyInputPin, AnyOutputPin, InputPin, PinDriver, Pull};
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

use crate::encoder::Encoder;

pub fn hardware_control(
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

    let timer = LedcTimerDriver::new(ledc.timer0, &TimerConfig::default())?;

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
        if button_mute.is_high() {
            log::info!("mute high");
        }
        if button_bassboost.is_high() {
            log::info!("bassboost high");
        }
        if button_standby.is_high() {
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
