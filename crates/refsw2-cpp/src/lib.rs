/*
    This file is part of libswirl - C++ FFI backend for refsw2

    This crate wraps the C++ refsw2 reference renderer implementation.
*/

#![allow(dead_code)]

// C++ FFI functions
unsafe extern "C" {
    fn ffi_refsw2_init();
    fn ffi_refsw2_render(vram: *mut u8, regs: *const u32);
}

/// Initialize the C++ renderer backend
pub unsafe fn init() {
    unsafe {
        ffi_refsw2_init();
    }
}

/// Render a frame using the C++ backend
///
/// # Arguments
/// * `vram` - Pointer to emulated VRAM (8MB)
/// * `regs` - Pointer to emulated PVR registers
pub unsafe fn render(vram: *mut u8, regs: *const u32) {
    unsafe {
        ffi_refsw2_render(vram, regs);
    }
}
