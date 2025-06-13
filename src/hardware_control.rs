use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use esp_idf_svc::hal::gpio::{AnyIOPin, AnyOutputPin, Input, InterruptType, PinDriver, Pull};
use esp_idf_svc::hal::ledc::config::TimerConfig;
use esp_idf_svc::hal::ledc::{LedcDriver, LedcTimerDriver, LEDC};
use esp_idf_svc::hal::pcnt::Pcnt;
use esp_idf_svc::hal::peripheral::Peripheral;

use crate::encoder::Encoder;
use crate::hardware_context::HardwareContext;
use crate::sticky_limiter::StickyLimiter;

fn setup_button(
    pin: AnyIOPin,
    pull: Pull,
    interrupt_type: InterruptType,
    pressed_flag: Arc<AtomicBool>,
) -> anyhow::Result<Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>> {
    let button = Arc::new(Mutex::new(PinDriver::input(pin)?));
    button.lock().unwrap().set_pull(pull)?;

    let pressed_flag_clone = Arc::clone(&pressed_flag);
    unsafe {
        button.lock().unwrap().subscribe(move || {
            pressed_flag_clone.store(true, Ordering::SeqCst);
        })?;
    }

    button.lock().unwrap().set_interrupt_type(interrupt_type)?;
    button.lock().unwrap().enable_interrupt()?;

    Ok(button)
}

fn setup_leds<'a>(
    ledc: LEDC,
    red_pin: AnyOutputPin,
    green_pin: AnyOutputPin,
    blue_pin: AnyOutputPin,
) -> anyhow::Result<(
    LedcDriver<'a>,
    LedcDriver<'a>,
    LedcDriver<'a>,
)> {
    let timer = LedcTimerDriver::new(ledc.timer0, &TimerConfig::default())?;

    let red = LedcDriver::new(ledc.channel0, &timer, red_pin)?;
    let green = LedcDriver::new(ledc.channel1, &timer, green_pin)?;
    let blue = LedcDriver::new(ledc.channel2, &timer, blue_pin)?;

    Ok((red, green, blue))
}

fn main_loop(
    encoder: Encoder,
    hardware_context: Arc<HardwareContext<'static>>,
    mute_pressed: Arc<AtomicBool>,
    bassboost_pressed: Arc<AtomicBool>,
    standby_pressed: Arc<AtomicBool>,
    button_mute: Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>,
    button_bassboost: Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>,
    button_standby: Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>,
) -> anyhow::Result<()> {
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
    hardware_context: Arc<HardwareContext<'static>>,
) -> anyhow::Result<()> {
    log::info!("Hardware control thread started");

    let mute_pressed = Arc::new(AtomicBool::new(false));
    let bassboost_pressed = Arc::new(AtomicBool::new(false));
    let standby_pressed = Arc::new(AtomicBool::new(false));

    let button_mute = setup_button(button_pin_1, Pull::Down, InterruptType::PosEdge, Arc::clone(&mute_pressed))?;
    let button_bassboost = setup_button(button_pin_2, Pull::Down, InterruptType::PosEdge, Arc::clone(&bassboost_pressed))?;
    let button_standby = setup_button(button_pin_3, Pull::Down, InterruptType::PosEdge, Arc::clone(&standby_pressed))?;

    let (mut red, mut green, mut blue) = setup_leds(ledc, led_pin_red, led_pin_green, led_pin_blue)?;
    set_rgb(&mut red, &mut green, &mut blue, 150, 255, 200)?;

    let encoder = Encoder::new(pcnt, encoder_pin_a, encoder_pin_b).unwrap();

    main_loop(
        encoder,
        hardware_context.clone(),
        mute_pressed.clone(),
        bassboost_pressed.clone(),
        standby_pressed.clone(),
        button_mute.clone(),
        button_bassboost.clone(),
        button_standby.clone(),
    )
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