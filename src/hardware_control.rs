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

const INITIAL_VOLUME_OFFSET: i32 = 66;
const BUTTON_DEBOUNCE_DELAY_MS: u64 = 500;
const MAIN_LOOP_DELAY_MS: u64 = 20;

struct Button {
    driver: Arc<Mutex<PinDriver<'static, AnyIOPin, Input>>>,
    pressed_flag: Arc<AtomicBool>,
}

impl Button {
    fn new(
        pin: AnyIOPin,
        pull: Pull,
        interrupt_type: InterruptType,
        pressed_flag: Arc<AtomicBool>,
    ) -> anyhow::Result<Self> {
        let driver = Arc::new(Mutex::new(PinDriver::input(pin)?));
        driver.lock().unwrap().set_pull(pull)?;

        let pressed_flag_clone = Arc::clone(&pressed_flag);
        unsafe {
            driver.lock().unwrap().subscribe(move || {
                pressed_flag_clone.store(true, Ordering::SeqCst);
            })?;
        }

        driver.lock().unwrap().set_interrupt_type(interrupt_type)?;
        driver.lock().unwrap().enable_interrupt()?;

        Ok(Self {
            driver,
            pressed_flag,
        })
    }

    fn handle_press<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if self.pressed_flag.load(Ordering::SeqCst) {
            self.pressed_flag.store(false, Ordering::SeqCst);
            callback();

            let driver = Arc::clone(&self.driver);
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(BUTTON_DEBOUNCE_DELAY_MS));
                driver.lock().unwrap().enable_interrupt().ok();
            });
        }
    }
}

struct LedController<'a> {
    red: LedcDriver<'a>,
    green: LedcDriver<'a>,
    blue: LedcDriver<'a>,
}

impl<'a> LedController<'a> {
    fn new(
        ledc: LEDC,
        red_pin: AnyOutputPin,
        green_pin: AnyOutputPin,
        blue_pin: AnyOutputPin,
    ) -> anyhow::Result<Self> {
        let timer = LedcTimerDriver::new(ledc.timer0, &TimerConfig::default())?;

        let red = LedcDriver::new(ledc.channel0, &timer, red_pin)?;
        let green = LedcDriver::new(ledc.channel1, &timer, green_pin)?;
        let blue = LedcDriver::new(ledc.channel2, &timer, blue_pin)?;

        Ok(Self { red, green, blue })
    }

    fn set_rgb(&mut self, r: u8, g: u8, b: u8) -> anyhow::Result<()> {
        self.red.set_duty(self.red.get_max_duty() * r as u32 / 255)?;
        self.green.set_duty(self.green.get_max_duty() * g as u32 / 255)?;
        self.blue.set_duty(self.blue.get_max_duty() * b as u32 / 255)?;
        Ok(())
    }
}

fn main_loop(
    encoder: Encoder,
    hardware_context: Arc<HardwareContext<'static>>,
    button_mute: Button,
    button_bassboost: Button,
    button_standby: Button,
) -> anyhow::Result<()> {
    let mut last_value = INITIAL_VOLUME_OFFSET;
    let mut volume = StickyLimiter::new(0, 100);

    loop {
        let value = encoder.get_value()? + INITIAL_VOLUME_OFFSET;
        if value != last_value {
            last_value = value;
            let volume = volume.update(value);
            log::info!("value: {value} volume: {volume}");
            hardware_context.adau1962a.lock().unwrap().set_master_volume(volume as u8)?;
        }

        let hardware_context = hardware_context.clone();
        button_mute.handle_press(move || {
            log::info!("mute pressed");
            let tpa3116d2 = hardware_context.tpa3116d2.lock().unwrap();
            let speakers_muted = tpa3116d2.speakers_muted().unwrap();
            tpa3116d2.mute_speaker_outputs(!speakers_muted).unwrap();
        });

        button_bassboost.handle_press(|| {
            log::info!("bassboost pressed");
        });

        button_standby.handle_press(|| {
            log::info!("standby pressed");
        });

        std::thread::sleep(Duration::from_millis(MAIN_LOOP_DELAY_MS));
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

    let button_mute = Button::new(
        button_pin_1,
        Pull::Down,
        InterruptType::PosEdge,
        Arc::new(AtomicBool::new(false)),
    )?;
    let button_bassboost = Button::new(
        button_pin_2,
        Pull::Down,
        InterruptType::PosEdge,
        Arc::new(AtomicBool::new(false)),
    )?;
    let button_standby = Button::new(
        button_pin_3,
        Pull::Down,
        InterruptType::PosEdge,
        Arc::new(AtomicBool::new(false)),
    )?;

    let mut led_controller = LedController::new(ledc, led_pin_red, led_pin_green, led_pin_blue)?;
    led_controller.set_rgb(150, 255, 200)?;

    let encoder = Encoder::new(pcnt, encoder_pin_a, encoder_pin_b).unwrap();

    main_loop(
        encoder,
        hardware_context.clone(),
        button_mute,
        button_bassboost,
        button_standby,
    )
}