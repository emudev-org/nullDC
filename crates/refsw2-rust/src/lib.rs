/*
    This file is part of libswirl - Rust port
*/
// #include "license/bsd"

#![allow(dead_code)]
#![allow(static_mut_refs)]

pub mod types;
pub mod pvr_mem;
pub mod pvr_regs;
pub mod tex_utils;
pub mod lists_types;
pub mod lists;
pub mod tile;
// mod gentable;

// Public API
pub unsafe fn init() {
    unsafe {
        tex_utils::init_tex_utils();
    }
}

pub unsafe fn render(vram: *mut u8, regs: *const u32) {
    unsafe {
        pvr_mem::EMU_VRAM = vram;
        pvr_regs::EMU_REGS = regs;
        lists::render_core();
    }
}
