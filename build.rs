use std::env;
use std::path::PathBuf;

fn build_sigmastudio_code() {
    let target = env::var("TARGET").unwrap();

    let target_is_xtensa = target.starts_with("xtensa-esp32");

    // Try to detect ESP-IDF toolchain via IDF_PATH or fallback
    let compiler = if let Ok(compiler) = env::var("CC") {
        compiler
    } else {
        // Try to guess toolchain path from IDF_PATH and target
        let toolchain = if target.starts_with("xtensa-esp32") {
            "xtensa-esp32-elf-gcc"
        } else if target.starts_with("riscv32imac-esp-espidf") {
            "riscv32-esp-elf-gcc"
        } else {
            panic!("Unsupported target: {}", target);
        };

        // Try to find the toolchain in $PATH (assumes idf-env is activated)
        which::which(toolchain)
            .unwrap_or_else(|_| PathBuf::from(toolchain))
            .to_str()
            .unwrap()
            .to_owned()
    };

    let mut build = cc::Build::new();

    build.file("src/sigmastudio/ADAU1467.c");
    build.file("src/sigmastudio/SigmaStudioFW.c");

    build.compiler(compiler);

    build.flag("-Wno-unused-parameter");
    if target_is_xtensa {
        build.flag("-mlongcalls");
    }

    // Kompiliere die C-Dateien
    build.compile("sigmastudio");
}

fn main() {
    embuild::espidf::sysenv::output();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/sigmastudio/ADAU1467.c");
    println!("cargo:rerun-if-changed=src/sigmastudio/systemfiles_IC_1.h");

    build_sigmastudio_code();
}
