use nulldc::dreamcast::{Dreamcast, init_dreamcast};

#[cfg(not(target_arch = "wasm32"))]
use nulldc::start_debugger_server;
use std::fs;

fn load_bios_files() -> (Vec<u8>, Vec<u8>) {
    let mut path = std::env::current_dir().expect("failed to get current directory");

    // Load BIOS ROM (2MB)
    path.push("data");
    path.push("dc_boot.bin");
    let bios_rom = fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to load BIOS ROM from {}: {}", path.display(), e));

    // Load BIOS Flash (128KB)
    let mut path = std::env::current_dir().expect("failed to get current directory");
    path.push("data");
    path.push("dc_flash.bin");
    let bios_flash = fs::read(&path)
        .unwrap_or_else(|e| panic!("Failed to load BIOS Flash from {}: {}", path.display(), e));

    (bios_rom, bios_flash)
}

fn main() {
    let (bios_rom, bios_flash) = load_bios_files();

    let dreamcast = Box::into_raw(Box::new(Dreamcast::default()));

    init_dreamcast(dreamcast, &bios_rom, &bios_flash);
    #[cfg(not(target_arch = "wasm32"))]
    start_debugger_server(dreamcast);

    pollster::block_on(nulldc::run(Some(dreamcast)));
}
