use dreamcast::{self};
use minifb::{Key, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 480;

fn main() {
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
    dreamcast::init_dreamcast(dc);

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
