use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, InterruptType, PinDriver, Pull};
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LEDC};
use esp_idf_svc::hal::pcnt::Pcnt;
use esp_idf_svc::hal::peripheral::Peripheral;

use crate::encoder::Encoder;
use crate::hardware_context::HardwareContext;
use crate::sticky_limiter::StickyLimiter;

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
    hardware_context: Arc<HardwareContext<'_>>,
) -> anyhow::Result<()> {
    log::info!("Hardware control thread started");

    let button_mute = Arc::new(Mutex::new(PinDriver::input(button_pin_1)?));
    let button_bassboost = Arc::new(Mutex::new(PinDriver::input(button_pin_2)?));
    let button_standby = Arc::new(Mutex::new(PinDriver::input(button_pin_3)?));

    button_mute.lock().unwrap().set_pull(Pull::Down)?;
    button_bassboost.lock().unwrap().set_pull(Pull::Down)?;
    button_standby.lock().unwrap().set_pull(Pull::Down)?;

    let mute_pressed = Arc::new(AtomicBool::new(false));
    let bassboost_pressed = Arc::new(AtomicBool::new(false));
    let standby_pressed = Arc::new(AtomicBool::new(false));

    let mute_pressed_clone = Arc::clone(&mute_pressed);
    unsafe {
        button_mute.lock().unwrap().subscribe(move || {
            mute_pressed_clone.store(true, Ordering::SeqCst);
        })?;
    }

    let bassboost_pressed_clone = Arc::clone(&bassboost_pressed);
    unsafe {
        button_bassboost.lock().unwrap().subscribe(move || {
            bassboost_pressed_clone.store(true, Ordering::SeqCst);
        })?;
    }

    let standby_pressed_clone = Arc::clone(&standby_pressed);
    unsafe {
        button_standby.lock().unwrap().subscribe(move || {
            standby_pressed_clone.store(true, Ordering::SeqCst);
        })?;
    }

    button_mute.lock().unwrap().set_interrupt_type(InterruptType::PosEdge)?;
    button_bassboost.lock().unwrap().set_interrupt_type(InterruptType::PosEdge)?;
    button_standby.lock().unwrap().set_interrupt_type(InterruptType::PosEdge)?;

    button_mute.lock().unwrap().enable_interrupt()?;
    button_bassboost.lock().unwrap().enable_interrupt()?;
    button_standby.lock().unwrap().enable_interrupt()?;

    let timer = LedcTimerDriver::new(ledc.timer0, &TimerConfig::default())?;

    let mut red = LedcDriver::new(ledc.channel0, &timer, led_pin_red)?;
    let mut green = LedcDriver::new(ledc.channel1, &timer, led_pin_green)?;
    let mut blue = LedcDriver::new(ledc.channel2, &timer, led_pin_blue)?;

    set_rgb(&mut red, &mut green, &mut blue, 150, 255, 200)?;

    let encoder = Encoder::new(pcnt, encoder_pin_a, encoder_pin_b).unwrap();

    let mut last_value = 66i32;
    let mut volume = StickyLimiter::new(0, 100);
    loop {
        let value = encoder.get_value()? + 66;
        if value != last_value {
            last_value = value;
            
            let volume = volume.update(value);

            log::info!("value: {value} volume: {volume}");

            hardware_context.adau1962a.lock().unwrap().set_master_volume(volume as u8)?;
        }
        if mute_pressed.load(Ordering::SeqCst) {
            log::info!("mute pressed");
            mute_pressed.store(false, Ordering::SeqCst);
            let tpa3116d2 = hardware_context.tpa3116d2.lock().unwrap();
            let speakers_muted = tpa3116d2.speakers_muted()?;
            log::info!("speakers_muted: {}", speakers_muted);
            tpa3116d2.mute_speaker_outputs(!speakers_muted)?;

            let button_mute = Arc::clone(&button_mute);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                button_mute.lock().unwrap().enable_interrupt().ok();
            });
        }
        if bassboost_pressed.load(Ordering::SeqCst) {
            log::info!("bassboost pressed");
            bassboost_pressed.store(false, Ordering::SeqCst);

            let button_bassboost = Arc::clone(&button_bassboost);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                button_bassboost.lock().unwrap().enable_interrupt().ok();
            });
        }
        if standby_pressed.load(Ordering::SeqCst) {
            log::info!("standby pressed");
            standby_pressed.store(false, Ordering::SeqCst);

            let button_standby = Arc::clone(&button_standby);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(500));
                button_standby.lock().unwrap().enable_interrupt().ok();
            });
        }
        std::thread::sleep(Duration::from_millis(20));
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