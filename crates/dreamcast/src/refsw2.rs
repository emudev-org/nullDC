#[cfg(feature = "refsw2-native")]
mod native {
    #[link(name = "refsw2")]
    extern "C" {
        fn ffi_refsw2_render(vram: *mut u8, regs: *const u32);
        fn ffi_refsw2_init();
    }

    pub fn refsw2_render(vram: *mut u8, regs: *const u32) {
        unsafe {
            ffi_refsw2_render(vram, regs);
        }
    }
    pub fn refsw2_init() {
        unsafe {
            ffi_refsw2_init();
        }
    }
}

#[cfg(feature = "refsw2-native")]
pub use native::refsw2_render;

#[cfg(feature = "refsw2-native")]
pub use native::refsw2_init;

#[cfg(not(feature = "refsw2-native"))]
pub fn refsw2_render(vram: *mut u8, regs: *const u32) {}

#[cfg(not(feature = "refsw2-native"))]
pub fn refsw2_init() {}
