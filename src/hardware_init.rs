use std::sync::{Arc, Mutex};

use esp_idf_svc::hal::i2c::I2cDriver;

use crate::drivers::{adau1467::ADAU1467, adau1962a::{self, ADAU1962A}, pcm1865::{self, PCM1865}, tpa3116d2::TPA3116D2};


pub fn hardware_init(shared_i2c: &Arc<Mutex<I2cDriver>>) -> anyhow::Result<()> {  

    setup_pcm1865(shared_i2c.clone())?;
    setup_adau1962a(shared_i2c.clone())?;
    setup_adau1467(shared_i2c.clone())?;
    setup_tpa3116d2(shared_i2c.clone())?;

    Ok(())
}

fn setup_pcm1865(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up PCM1865");

    let mut pcm1865 = PCM1865::new(i2c, 0x4a);

    pcm1865.set_sck_xtal_selection(pcm1865::SckXtalSelection::Xtal)?;
    pcm1865.select_mode(true)?;
    pcm1865.master_clock_to_bit_clock_divider_value(2)?; // 12.228 Mhz BCLK
    pcm1865.bit_clock_to_left_right_clock_divider_value(64)?;
    pcm1865.auto_clock_detector_configuration(true)?;
    Ok(())
}

fn setup_adau1962a(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1962a");

    let mut adau1962a = ADAU1962A::new(i2c, 0x04);

    adau1962a.set_reset(true)?;
    adau1962a.master_power_up(true)?;
    adau1962a.set_xtal_oscillator_enabled(true)?;
    adau1962a.set_pll_input_source(adau1962a::MclkiXtaliOrDlrclkSelection::MclkiOrXtali)?;
    adau1962a.set_master_clock_fs_ratio(adau1962a::MasterClockFsRatio::Ratio512)?;
    adau1962a.set_dac_clock_select(adau1962a::DacClockSource::PLL)?;
    adau1962a.set_serial_audio_interface(adau1962a::AudioInterface::Stero)?;
    adau1962a.set_sample_rate_selection(adau1962a::SampleRate::FS192)?;
    adau1962a.set_dlrclk_polarity(false)?;
    adau1962a.set_serial_interface_master(true)?;
    adau1962a.set_master_mute(false)?;

    Ok(())
}

fn setup_adau1467(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1467");

    let mut adau1467 = ADAU1467::new(i2c, 0x38);

    adau1467.set_reset(true)?;
    adau1467.load_dsp_program()?;

    Ok(())
}

fn setup_tpa3116d2(i2c: Arc<Mutex<I2cDriver>>) -> Result<(), anyhow::Error> {
    log::info!("Setting up TPA3116D2");

    let mut tpa3116d2 = TPA3116D2::new(i2c);

    tpa3116d2.enable_speaker_outputs(true)?;

    Ok(())
}