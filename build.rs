use std::env;
use std::path::{Path, PathBuf};

fn build_sigmastudio_code() {
    let mut build = cc::Build::new();

    let esp_install_dir = embuild::espidf::sysenv::idf_path().unwrap();

    let esp_install_path = Path::new(&esp_install_dir);

    let toolchain_bin = esp_install_path.join("../../tools/riscv32-esp-elf/esp-13.2.0_20230928/riscv32-esp-elf/bin");
    let compiler = toolchain_bin.join("riscv32-esp-elf-gcc");

    build.compiler(compiler);

    // Füge die C-Dateien hinzu
    build.file("src/sigmastudio/ADAU1467.c");
    build.file("src/sigmastudio/SigmaStudioFW.c");

    // Optionale Compiler-Flags
    build.flag("-Wno-unused-parameter");

    let include_dir = esp_install_path.join("esp-idf/components");
    build.include(&include_dir);

    // Kompiliere die C-Dateien
    build.compile("sigmastudio");

    // Stelle sicher, dass das Header-Verzeichnis gefunden wird
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    println!("cargo:include={}", out_dir.display());
}

fn main() {
    embuild::espidf::sysenv::output();

    build_sigmastudio_code();
}
