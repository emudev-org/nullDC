//! dreamcast_sh4.rs — 1:1 Rust translation of the provided C++/C code snippet.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ptr;

use egui::mutex::Mutex;
use sh4_core::{Sh4Ctx, sh4_ipr_dispatcher, sh4_fns_dispatcher, sh4_init_ctx, sh4mem::read_mem, sh4dec::{format_disas, SH4DecoderState}};

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
    pub running: bool,
    pub running_mtx: Mutex<()>,
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
            running: true,
            running_mtx: Mutex::new(()),
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

pub fn readbyte_sh4_dreamcast(dc: *mut Dreamcast, addr: u32) -> u8 {
    unsafe {
        let mut byte: u8 = 0;
        read_mem(&mut (*dc).ctx, addr, &mut byte);
        byte
    }
}

pub fn read_memory_slice(dc: *mut Dreamcast, base_address: u64, length: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(length);
    unsafe {
        let ctx = &mut (*dc).ctx;
        for i in 0..length {
            let addr = (base_address as u32).wrapping_add(i as u32);
            let mut byte: u8 = 0;
            read_mem(ctx, addr, &mut byte);
            result.push(byte);
        }
    }
    result
}

pub struct DisassemblyLine {
    pub address: u64,
    pub bytes: String,
    pub disassembly: String,
}

pub fn disassemble_sh4(dc: *mut Dreamcast, base_address: u64, count: usize) -> Vec<DisassemblyLine> {
    let mut result = Vec::with_capacity(count);
    let mut addr = base_address as u32;

    unsafe {
        let ctx = &mut (*dc).ctx;

        // Get decoder state from context
        let state = SH4DecoderState {
            pc: addr,
            fpscr_PR: false, // TODO: Get from actual FPSCR register
            fpscr_SZ: false, // TODO: Get from actual FPSCR register
        };

        for _ in 0..count {
            // Read instruction word (SH4 instructions are 16-bit)
            let mut opcode: u16 = 0;
            read_mem(ctx, addr, &mut opcode);

            // Disassemble
            let disassembly = format_disas(state, opcode);

            // Format bytes as hex string
            let bytes = format!("{:04X}", opcode);

            result.push(DisassemblyLine {
                address: addr as u64,
                bytes,
                disassembly,
            });

            addr += 2; // SH4 instructions are 2 bytes
        }
    }

    result
}

pub fn step_dreamcast(dc: *mut Dreamcast) {
    unsafe {
        let _lock = (*dc).running_mtx.lock();
        let old_cycles = (*dc).ctx.remaining_cycles;
        (*dc).ctx.remaining_cycles = 1;
        sh4_ipr_dispatcher(&mut (*dc).ctx);
        //sh4_fns_dispatcher(&mut (*dc).ctx);
        (*dc).ctx.remaining_cycles = old_cycles - 1;
    }
}

pub fn run_slice_dreamcast(dc: *mut Dreamcast) {
    unsafe {
        let _lock = (*dc).running_mtx.lock();
        if (*dc).running {
            (*dc).ctx.remaining_cycles += 2_000_000;
            //sh4_ipr_dispatcher(&mut (*dc).ctx);
            sh4_fns_dispatcher(&mut (*dc).ctx);
        }
    }
}

pub fn is_dreamcast_running(dc: *mut Dreamcast) -> bool {
    unsafe {
        let _lock = (*dc).running_mtx.lock();
        (*dc).running
    }
}

pub fn set_dreamcast_running(dc: *mut Dreamcast, newstate: bool) {
    unsafe {
        let _lock = (*dc).running_mtx.lock();
        (*dc).running = newstate;
    }
}

pub fn get_sh4_register(dc: *mut Dreamcast, register_name: &str) -> Option<u32> {
    unsafe {
        let ctx = &(*dc).ctx;
        match register_name.to_uppercase().as_str() {
            "PC" => Some(ctx.pc0),
            "PR" => Some(ctx.pr),
            "SR" => Some(ctx.sr.full()),
            "GBR" => Some(ctx.gbr),
            "VBR" => Some(ctx.vbr),
            "MACH" => Some(ctx.mac.parts.h),
            "MACL" => Some(ctx.mac.parts.l),
            "FPSCR" => Some(ctx.fpscr.full()),
            "FPUL" => Some(ctx.fpul),
            _ => {
                // Check if it's a general purpose register (R0-R15)
                if let Some(rest) = register_name.strip_prefix('R').or_else(|| register_name.strip_prefix('r')) {
                    if let Ok(idx) = rest.parse::<usize>() {
                        if idx < 16 {
                            return Some(ctx.r[idx]);
                        }
                    }
                }
                None
            }
        }
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
