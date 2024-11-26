pub enum SystemCommand {
    SetVolume { channel: u8, level: u8 },
    MuteChannel { channel: u8 },
    UnmuteChannel { channel: u8 },
    SetInputSource { channel: u8, source_id: u8 },
}