use dreamcast::{self};
use minifb::{Key, Window, WindowOptions};
use std::fs;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

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

    let (bios_rom, bios_flash) = load_bios_files();

    // Create window
    let mut window = Window::new(
        "nullDC Minimal - Dreamcast Emulator",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| {
        panic!("Failed to create window: {}", e);
    });

    // Limit to ~60 FPS
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // Initialize Dreamcast
    let dc = Box::into_raw(Box::new(dreamcast::Dreamcast::default()));
    dreamcast::init_dreamcast(dc, &bios_rom, &bios_flash);

    // Load ELF if provided as command line argument
    if args.len() > 1 {
        let elf_path = &args[1];
        println!("Loading ELF file: {}", elf_path);

        let elf_data = fs::read(elf_path)
            .unwrap_or_else(|e| panic!("Failed to load ELF file from {}: {}", elf_path, e));

        dreamcast::init_dreamcast_with_elf(dc, &elf_data)
            .unwrap_or_else(|e| panic!("Failed to load ELF: {}", e));

        println!("ELF file loaded successfully");
    }

    // Framebuffer for minifb (ARGB format)
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // Main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Run emulator slice
        dreamcast::run_slice_dreamcast(dc);

        // Get framebuffer from emulator
        if let Some((rgba, width, height)) = dreamcast::present_for_texture() {
            // Convert RGBA to minifb's 0RGB format
            convert_rgba_to_buffer(&rgba, &mut buffer, width, height);
        }

        // Update window with buffer
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap_or_else(|e| {
                eprintln!("Failed to update window: {}", e);
            });
    }

    // Cleanup
    unsafe {
        let _ = Box::from_raw(dc);
    }
}

/// Convert RGBA8 buffer to minifb's 0RGB format (0xAARRGGBB)
fn convert_rgba_to_buffer(rgba: &[u8], buffer: &mut [u32], width: usize, height: usize) {
    let pixel_count = width.min(WIDTH) * height.min(HEIGHT);

    for i in 0..pixel_count {
        let rgba_idx = i * 4;
        if rgba_idx + 3 < rgba.len() {
            let r = rgba[rgba_idx] as u32;
            let g = rgba[rgba_idx + 1] as u32;
            let b = rgba[rgba_idx + 2] as u32;
            // minifb uses 0RGB format (ignore alpha)
            buffer[i] = (r << 16) | (g << 8) | b;
        }
    }
}
