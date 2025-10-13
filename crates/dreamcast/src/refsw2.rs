#[cfg(feature = "refsw2-native")]
mod native {
    #[link(name = "refsw2")]
    extern "C" {
        fn ffi_refsw2_render(vram: *mut u8, regs: *const u32);
    }

    pub fn refsw2_render(vram: *mut u8, regs: *const u32) {
        unsafe {
            ffi_refsw2_render(vram, regs);
        }
    }
}

#[cfg(feature = "refsw2-native")]
pub use native::refsw2_render;

#[cfg(not(feature = "refsw2-native"))]
pub fn refsw2_render(vram: *mut u8, regs: *const u32) {}
