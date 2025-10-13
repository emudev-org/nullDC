/*
    refsw2 backend switcher

    Feature flags:
    - "refsw2-cpp": Use C++ FFI backend (refsw2 crate)
    - "refsw2-rust": Use pure Rust backend (refsw2r crate) - default
    - Neither: Stub implementation (no rendering)
*/

// C++ backend (via FFI)
#[cfg(feature = "refsw2-cpp")]
mod cpp_backend {
    pub fn refsw2_render(vram: *mut u8, regs: *const u32) {
        unsafe {
            refsw2::render(vram, regs);
        }
    }

    pub fn refsw2_init() {
        unsafe {
            refsw2::init();
        }
    }
}

// Rust backend (pure Rust implementation)
#[cfg(feature = "refsw2-rust")]
mod rust_backend {
    pub fn refsw2_render(vram: *mut u8, regs: *const u32) {
        unsafe {
            refsw2r::render(vram, regs);
        }
    }

    pub fn refsw2_init() {
        unsafe {
            refsw2r::init();
        }
    }
}

// Stub implementation (no rendering)
#[cfg(not(any(feature = "refsw2-cpp", feature = "refsw2-rust")))]
mod stub_backend {
    pub fn refsw2_render(_vram: *mut u8, _regs: *const u32) {
        // No-op
    }

    pub fn refsw2_init() {
        // No-op
    }
}

// Export the selected backend
#[cfg(feature = "refsw2-cpp")]
pub use cpp_backend::{refsw2_init, refsw2_render};

#[cfg(feature = "refsw2-rust")]
pub use rust_backend::{refsw2_init, refsw2_render};

#[cfg(not(any(feature = "refsw2-cpp", feature = "refsw2-rust")))]
pub use stub_backend::{refsw2_init, refsw2_render};
