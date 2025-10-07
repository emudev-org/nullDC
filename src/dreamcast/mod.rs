//! dreamcast_sh4.rs — 1:1 Rust translation of the provided C++/C code snippet.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ptr;

use sh4_core::{Sh4Ctx, sh4_fns_dispatcher, sh4_init_ctx};

const SYSRAM_SIZE: u32 = 16 * 1024 * 1024;
const VIDEORAM_SIZE: u32 = 8 * 1024 * 1024;

const SYSRAM_MASK: u32 = SYSRAM_SIZE - 1;
const VIDEORAM_MASK: u32 = VIDEORAM_SIZE - 1;

pub struct Dreamcast {
    pub ctx: Sh4Ctx,
    pub memmap: [*mut u8; 256],
    pub memmask: [u32; 256],

    pub sys_ram: Box<[u8; SYSRAM_SIZE as usize]>,
    pub video_ram: Box<[u8; VIDEORAM_SIZE as usize]>,
}

impl Default for Dreamcast {
    fn default() -> Self {

        let sys_ram = {
            let v = vec![0u8; SYSRAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };
        let video_ram = {
            let v = vec![0u8; VIDEORAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        Self {
            ctx: Sh4Ctx::default(),
            memmap: [ptr::null_mut(); 256],
            memmask: [0; 256],
            sys_ram,
            video_ram,
        }
    }
}


pub static ROTO_BIN: &[u8] = include_bytes!("../../roto.bin");

pub fn init_dreamcast(dc: *mut Dreamcast) {
    unsafe {
        // Zero entire struct (like memset). In Rust, usually you'd implement Default.
        *dc = Dreamcast::default();

        sh4_init_ctx(&mut (*dc).ctx);

        // Build opcode tables
        // build_opcode_tables(dc);

        // Setup memory map
        (*dc).ctx.memmap[0x0C] = (*dc).sys_ram.as_mut_ptr();
        (*dc).ctx.memmask[0x0C] = SYSRAM_MASK;
        (*dc).ctx.memmap[0x8C] = (*dc).sys_ram.as_mut_ptr();
        (*dc).ctx.memmask[0x8C] = SYSRAM_MASK;
        (*dc).ctx.memmap[0xA5] = (*dc).video_ram.as_mut_ptr();
        (*dc).ctx.memmask[0xA5] = VIDEORAM_MASK;

        // Set initial PC
        (*dc).ctx.pc0 = 0x8C01_0000;
        (*dc).ctx.pc1 = 0x8C01_0000 + 2;
        (*dc).ctx.pc2 = 0x8C01_0000 + 4;

        // Copy roto.bin from embedded ROTO_BIN
        let dst = (*dc).sys_ram.as_mut_ptr().add(0x10000);
        let src = ROTO_BIN.as_ptr();

        ptr::copy_nonoverlapping(src, dst, ROTO_BIN.len())
    }
}


pub fn run_dreamcast(dc: *mut Dreamcast) {
    unsafe {
        //sh4_ipr_dispatcher(&mut (*dc).ctx);
        sh4_fns_dispatcher(&mut (*dc).ctx);
    }
}


pub fn rgb565_to_color32(buf: &[u16], w: usize, h: usize) -> egui::ColorImage {
    let mut pixels = Vec::with_capacity(w * h);
    for &px in buf {
        let r = ((px >> 11) & 0x1F) as u8;
        let g = ((px >> 5) & 0x3F) as u8;
        let b = (px & 0x1F) as u8;
        // Expand to 8-bit
        let r = (r << 3) | (r >> 2);
        let g = (g << 2) | (g >> 4);
        let b = (b << 3) | (b >> 2);
        pixels.push(egui::Color32::from_rgb(r, g, b));
    }
    egui::ColorImage { size: [w, h], pixels, source_size: egui::vec2(w as f32, h as f32) }
}
