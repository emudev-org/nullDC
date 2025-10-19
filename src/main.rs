use nulldc::dreamcast::{Dreamcast, init_dreamcast, init_dreamcast_with_elf};

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
    let args: Vec<String> = std::env::args().collect();

    let dreamcast = Box::into_raw(Box::new(Dreamcast::default()));

    // Load ELF if provided as command line argument
    if args.len() > 1 {
        let elf_path = &args[1];
        println!("Loading ELF file: {}", elf_path);

        let elf_data = fs::read(elf_path)
            .unwrap_or_else(|e| panic!("Failed to load ELF file from {}: {}", elf_path, e));

        init_dreamcast_with_elf(dreamcast, &elf_data)
            .unwrap_or_else(|e| panic!("Failed to load ELF: {}", e));

        println!("ELF file loaded successfully");
    } else {
        let (bios_rom, bios_flash) = load_bios_files();
        init_dreamcast(dreamcast, &bios_rom, &bios_flash);

        println!("BIOS loaded successfully");
    }

    #[cfg(not(target_arch = "wasm32"))]
    start_debugger_server(dreamcast);

    pollster::block_on(nulldc::run(Some(dreamcast)));
}
