use std::sync::Arc;


use crate::{
    drivers::{
        adau1467::ADAU1467,
        adau1962a::{self, ADAU1962A},
        pcm1865::{self, PCM1865},
        tpa3116d2::TPA3116D2,
    },
    hardware_context::{HardwareContext},
};

pub fn hardware_init(hardware_context: Arc<HardwareContext<'_>>) -> anyhow::Result<()> {
    setup_pcm1865(
        &mut hardware_context
            .pcm1865
            .lock()
            .expect("Could not lock PCM1865 driver"),
    )?;
    setup_adau1962a(
        &mut hardware_context
            .adau1962a
            .lock()
            .expect("Could not lock ADAU1962a driver"),
    )?;
    setup_adau1467(
        &mut hardware_context
            .adau1467
            .lock()
            .expect("Could not lock ADAU1467 driver"),
    )?;
    setup_tpa3116d2(
        &mut hardware_context
            .tpa3116d2
            .lock()
            .expect("Could not lock TPA3116d2 driver"),
    )?;

    Ok(())
}

fn setup_pcm1865(pcm1865: &mut PCM1865) -> Result<(), anyhow::Error> {
    log::info!("Setting up PCM1865");

    pcm1865.set_sck_xtal_selection(pcm1865::SckXtalSelection::Xtal)?;
    pcm1865.select_mode(true)?;
    pcm1865.master_clock_to_bit_clock_divider_value(2)?; // 12.228 Mhz BCLK
    pcm1865.bit_clock_to_left_right_clock_divider_value(64)?;
    pcm1865.auto_clock_detector_configuration(true)?;
    Ok(())
}

fn setup_adau1962a(adau1962a: &mut ADAU1962A) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1962a");

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
    adau1962a.set_master_volume(66)?;
    adau1962a.set_master_mute(false)?;

    Ok(())
}

fn setup_adau1467(adau1467: &mut ADAU1467) -> Result<(), anyhow::Error> {
    log::info!("Setting up ADAU1467");

    adau1467.set_reset(true)?;
    adau1467.load_dsp_program()?;
    adau1467.set_subwoofer_gain(6.0)?;

    Ok(())
}

fn setup_tpa3116d2(tpa3116d2: &mut TPA3116D2) -> Result<(), anyhow::Error> {
    log::info!("Setting up TPA3116D2");

    tpa3116d2.enable_speaker_outputs(true)?;

    Ok(())
}
