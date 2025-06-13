use std::time::Duration;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, PinDriver, Pull};
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LEDC};
use esp_idf_svc::hal::pcnt::Pcnt;
use esp_idf_svc::hal::peripheral::Peripheral;

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

    button_mute.set_pull(Pull::Down)?;
    button_bassboost.set_pull(Pull::Down)?;
    button_standby.set_pull(Pull::Down)?;

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
