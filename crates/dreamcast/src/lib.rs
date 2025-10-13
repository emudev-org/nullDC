//! dreamcast_sh4.rs — 1:1 Rust translation of the provided C++/C code snippet.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use sh4_core::{
    sh4_init_ctx, sh4_ipr_dispatcher,
    sh4dec::{format_disas, SH4DecoderState},
    sh4mem::read_mem,
    Sh4Ctx,
};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;

mod area0;
pub use area0::AREA0_HANDLERS;

mod aica;
mod arm7_disasm;
pub mod arm7di;
mod asic;
mod gdrom;
mod pvr;
pub mod refsw2;
mod sgc;
mod spg;
mod system_bus;
pub mod ta;

use arm7_disasm::{format_arm_instruction, Arm7DecoderState};
use arm7di::{ArmPsr, R13_IRQ, R13_SVC, R15_ARM_NEXT, RN_CPSR, RN_PSR_FLAGS, RN_SPSR};

pub use pvr::present_for_texture;

static DREAMCAST_PTR: AtomicPtr<Dreamcast> = AtomicPtr::new(std::ptr::null_mut());

pub(crate) fn dreamcast_ptr() -> *mut Dreamcast {
    DREAMCAST_PTR.load(Ordering::SeqCst)
}

fn peripheral_hook(_ctx: *mut sh4_core::Sh4Ctx, cycles: u32) {
    spg::tick(cycles);

    if cycles == 0 {
        return;
    }

    let dc_ptr = DREAMCAST_PTR.load(Ordering::Relaxed);
    if dc_ptr.is_null() {
        return;
    }

    unsafe {
        let dc = &mut *dc_ptr;
        if !dc.arm_enabled {
            return;
        }

        if !dc.arm_ctx.enabled {
            return;
        }

        dc.arm_cycle_accumulator = dc.arm_cycle_accumulator.wrapping_add(cycles);

        while dc.arm_cycle_accumulator >= 20 {
            dc.arm_cycle_accumulator -= 20;
            let mut arm = arm7di::Arm7Di::new(&mut dc.arm_ctx);
            arm.update_interrupts();
            arm.step();
        }
    }
}

const BIOS_ROM_SIZE: u32 = 2 * 1024 * 1024;
const BIOS_FLASH_SIZE: u32 = 128 * 1024;

const BIOS_ROM_MASK: u32 = BIOS_ROM_SIZE - 1;
const BIOS_FLASH_MASK: u32 = BIOS_FLASH_SIZE - 1;

const SYSRAM_SIZE: u32 = 16 * 1024 * 1024;
const VIDEORAM_SIZE: u32 = 8 * 1024 * 1024;
const AUDIORAM_SIZE: u32 = 2 * 1024 * 1024;
const OCRAM_SIZE: u32 = 8 * 1024;

const SYSRAM_MASK: u32 = SYSRAM_SIZE - 1;
const VIDEORAM_MASK: u32 = VIDEORAM_SIZE - 1;
const AUDIORAM_MASK: u32 = AUDIORAM_SIZE - 1;
const OCRAM_MASK: u32 = OCRAM_SIZE - 1;

pub struct Dreamcast {
    pub ctx: Sh4Ctx,
    pub memmap: [*mut u8; 256],
    pub memmask: [u32; 256],

    pub bios_rom: Box<[u8; BIOS_ROM_SIZE as usize]>,
    pub bios_flash: Box<[u8; BIOS_FLASH_SIZE as usize]>,

    pub sys_ram: Box<[u8; SYSRAM_SIZE as usize]>,
    pub video_ram: Box<[u8; VIDEORAM_SIZE as usize]>,
    pub audio_ram: Box<[u8; AUDIORAM_SIZE as usize]>,
    pub oc_ram: Box<[u8; OCRAM_SIZE as usize]>,

    pub running: bool,
    pub running_mtx: Mutex<()>,
    pub arm_ctx: arm7di::Arm7Context,
    pub arm_enabled: bool,
    pub arm_cycle_accumulator: u32,
}

impl Default for Dreamcast {
    fn default() -> Self {
        let bios_rom = {
            let v = vec![0u8; BIOS_ROM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let bios_flash = {
            let v = vec![0u8; BIOS_FLASH_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let sys_ram = {
            let v = vec![0u8; SYSRAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };
        let video_ram = {
            let v = vec![0u8; VIDEORAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };
        let audio_ram = {
            let v = vec![0u8; AUDIORAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let oc_ram = {
            let v = vec![0u8; OCRAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        Self {
            ctx: Sh4Ctx::default(),
            memmap: [ptr::null_mut(); 256],
            memmask: [0; 256],
            bios_rom,
            bios_flash,
            sys_ram,
            video_ram,
            audio_ram,
            oc_ram,
            running: true,
            running_mtx: Mutex::new(()),
            arm_ctx: arm7di::Arm7Context::new(),
            arm_enabled: false,
            arm_cycle_accumulator: 0,
        }
    }
}

fn reset_arm7(dc: &mut Dreamcast) {
    let mut arm_ctx = arm7di::Arm7Context::new();
    arm_ctx.aica_ram = NonNull::new(dc.audio_ram.as_mut_ptr());
    arm_ctx.aram_mask = AUDIORAM_MASK;

    arm_ctx.arm_mode = 0x13; // SVC
    arm_ctx.arm_irq_enable = true;
    arm_ctx.arm_fiq_enable = false;
    arm_ctx.enabled = true;
    arm_ctx.read8 = Some(aica::arm_read8);
    arm_ctx.read32 = Some(aica::arm_read32);
    arm_ctx.write8 = Some(aica::arm_write8);
    arm_ctx.write32 = Some(aica::arm_write32);

    // Initialise general-purpose registers and banked stacks
    arm_ctx.regs[13].set(0x0300_7F00);
    arm_ctx.regs[R13_IRQ].set(0x0300_7FA0);
    arm_ctx.regs[R13_SVC].set(0x0300_7FE0);
    arm_ctx.regs[R15_ARM_NEXT].set(0);

    let mut cpsr = ArmPsr::new(0);
    cpsr.set_mode(0x13);
    cpsr.set_fiq_mask(true);
    arm_ctx.regs[RN_CPSR].set_psr(cpsr);
    arm_ctx.regs[RN_SPSR].set_psr(cpsr);
    arm_ctx.regs[RN_PSR_FLAGS].set_psr(cpsr);

    // ARM pipeline prefetch: PC visible as PC+8
    arm_ctx.regs[15].set(4);

    dc.arm_ctx = arm_ctx;
    dc.arm_enabled = true;
    dc.arm_cycle_accumulator = 0;

    let mut arm = arm7di::Arm7Di::new(&mut dc.arm_ctx);
    arm.cpu_update_flags();
    arm.update_interrupts();
}

fn load_file_into_slice<P: AsRef<Path>>(path: P, buf: &mut [u8]) -> io::Result<()> {
    let path_ref = path.as_ref();
    let mut file = File::open(path_ref)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", path_ref.display(), e));

    // Read entire file
    let bytes_read = file
        .read(buf)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path_ref.display(), e));

    // Validate file size
    if bytes_read != buf.len() {
        panic!(
            "File size mismatch for {}: expected {} bytes, got {} bytes",
            path_ref.display(),
            buf.len(),
            bytes_read
        );
    }

    Ok(())
}

pub static ROTO_BIN: &[u8] = include_bytes!("../../../roto.bin");
pub static IP_BIN: &[u8] = include_bytes!("../../../data/IP.BIN");
pub static SYS_BIN: &[u8] = include_bytes!("../../../data/syscalls.bin");
pub static HELLO_BIN: &[u8] = include_bytes!("../../../data/hello.elf.bin");
// pub static ARM7W_BIN: &[u8] = include_bytes!("../../../data/arm7wrestler.bin");

pub fn init_dreamcast(dc_: *mut Dreamcast) {
    let dc: &mut Dreamcast;
    unsafe {
        dc = &mut *dc_;
    }

    DREAMCAST_PTR.store(dc as *mut Dreamcast, Ordering::SeqCst);

    // Zero entire struct (like memset). In Rust, usually you'd implement Default.
    *dc = Dreamcast::default();

    // Load BIOS ROM + Flash from file
    let mut path = std::env::current_dir().expect("failed to get current directory");
    path.push("data");
    path.push("dc_boot.bin");
    load_file_into_slice(&path, &mut dc.bios_rom[..]).unwrap();
    let mut path = std::env::current_dir().expect("failed to get current directory");
    path.push("data");
    path.push("dc_flash.bin");
    load_file_into_slice(&path, &mut dc.bios_flash[..]).unwrap();

    sh4_init_ctx(&mut dc.ctx);

    refsw2::refsw2_init();
    gdrom::reset();
    asic::reset();
    aica::reset();
    area0::reset();
    spg::reset();
    ta::reset();
    sh4_core::register_peripheral_hook(Some(peripheral_hook));

    // Build opcode tables
    // build_opcode_tables(dc);

    // Setup memory map
    // SYSRAM
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0x0C00_0000,
        0x0FFF_FFFF,
        SYSRAM_MASK,
        dc.sys_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0x8C00_0000,
        0x8FFF_FFFF,
        SYSRAM_MASK,
        dc.sys_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0xAC00_0000,
        0xAFFF_FFFF,
        SYSRAM_MASK,
        dc.sys_ram.as_mut_ptr(),
    );

    // VRAM
    // Gotta handle 32/64 bit vram mirroring at some point
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0x0400_0000,
        0x04FF_FFFF,
        VIDEORAM_MASK,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0xA400_0000,
        0xA4FF_FFFF,
        VIDEORAM_MASK,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0x0500_0000,
        0x05FF_FFFF,
        VIDEORAM_MASK,
        pvr::PVR_32BIT_HANDLERS,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0xA500_0000,
        0xA5FF_FFFF,
        VIDEORAM_MASK,
        pvr::PVR_32BIT_HANDLERS,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0x0600_0000,
        0x06FF_FFFF,
        VIDEORAM_MASK,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0xA600_0000,
        0xA6FF_FFFF,
        VIDEORAM_MASK,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0x0700_0000,
        0x07FF_FFFF,
        VIDEORAM_MASK,
        pvr::PVR_32BIT_HANDLERS,
        dc.video_ram.as_mut_ptr(),
    );
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0xA700_0000,
        0xA7FF_FFFF,
        VIDEORAM_MASK,
        pvr::PVR_32BIT_HANDLERS,
        dc.video_ram.as_mut_ptr(),
    );
    ta::init(dc.video_ram.as_mut_ptr());

    // OCRAM
    sh4_core::sh4_register_mem_buffer(
        &mut dc.ctx,
        0x7C00_0000,
        0x7FFF_FFFF,
        OCRAM_MASK,
        dc.oc_ram.as_mut_ptr(),
    );

    // AREA 0 (BIOS, Flash, System Bus)
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0x8000_0000,
        0x83FF_FFFF,
        0xFFFF_FFFF,
        AREA0_HANDLERS,
        dc as *mut _ as *mut u8,
    );
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0xA000_0000,
        0xA3FF_FFFF,
        0xFFFF_FFFF,
        AREA0_HANDLERS,
        dc as *mut _ as *mut u8,
    );

    // Set initial PC
    dc.ctx.pc0 = 0xA000_0000;
    dc.ctx.pc1 = 0xA000_0000 + 2;
    dc.ctx.pc2 = 0xA000_0000 + 4;

    reset_arm7(dc);

    dc.ctx.r[15] = 0x8d000000;

    dc.ctx.gbr = 0x8c000000;
    dc.ctx.ssr = 0x40000001;
    dc.ctx.spc = 0x8c000776;
    dc.ctx.sgr = 0x8d000000;
    dc.ctx.dbr = 0x8c000010;
    dc.ctx.vbr = 0x8c000000;
    dc.ctx.pr = 0xac00043c;
    dc.ctx.fpul = 0x00000000;

    dc.ctx.sr.0 = 0x400000f0;
    dc.ctx.sr_t = 1;

    dc.ctx.fpscr.0 = 0x00040001;

    // ROTO test program at 0x8C010000
    // dc.ctx.pc0 = 0x8C01_0000;
    // dc.ctx.pc1 = 0x8C01_0000 + 2;
    // dc.ctx.pc2 = 0x8C01_0000 + 4;

    // IP.BIN boot
    // dc.ctx.pc0 = 0x8C00_8300;
    // dc.ctx.pc1 = 0x8C00_8300 + 2;
    // dc.ctx.pc2 = 0x8C00_8300 + 4;

    unsafe {
        let dst = dc.sys_ram.as_mut_ptr().add(0);
        let src = SYS_BIN.as_ptr();

        ptr::copy_nonoverlapping(src, dst, SYS_BIN.len())
    }

    unsafe {
        let dst = dc.sys_ram.as_mut_ptr().add(0x8000);
        let src = IP_BIN.as_ptr();

        ptr::copy_nonoverlapping(src, dst, IP_BIN.len())
    }

    unsafe {
        // Copy roto.bin from embedded ROTO_BIN
        let dst = dc.sys_ram.as_mut_ptr().add(0x10000);
        let src = ROTO_BIN.as_ptr();

        ptr::copy_nonoverlapping(src, dst, ROTO_BIN.len())
    }

    // unsafe {
    //     let dst = dc.sys_ram.as_mut_ptr().add(0x10000);
    //     let src = HELLO_BIN.as_ptr();

    //     ptr::copy_nonoverlapping(src, dst, HELLO_BIN.len())
    // }

    // unsafe {
    //     let dst = dc.sys_ram.as_mut_ptr().add(0x10000);
    //     let src = ARM7W_BIN.as_ptr();

    //     ptr::copy_nonoverlapping(src, dst, ARM7W_BIN.len())
    // }
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

pub fn read_arm_memory_slice(dc: *mut Dreamcast, base_address: u64, length: usize) -> Vec<u8> {
    let mut result = Vec::with_capacity(length);
    unsafe {
        let ctx = &mut (*dc).arm_ctx;
        if !ctx.enabled {
            result.resize(length, 0);
            return result;
        }

        let mut cached_aligned = u32::MAX;
        let mut cached_word = 0u32;

        for i in 0..length {
            let addr = (base_address as u32).wrapping_add(i as u32);
            let aligned = addr & !3;
            if aligned != cached_aligned {
                cached_word = ctx.read32(aligned);
                cached_aligned = aligned;
            }
            let shift = (addr & 3) * 8;
            result.push(((cached_word >> shift) & 0xFF) as u8);
        }
    }
    result
}

pub struct DisassemblyLine {
    pub address: u64,
    pub bytes: String,
    pub disassembly: String,
}

pub fn disassemble_sh4(
    dc: *mut Dreamcast,
    base_address: u64,
    count: usize,
) -> Vec<DisassemblyLine> {
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

pub fn disassemble_arm7(
    dc: *mut Dreamcast,
    base_address: u64,
    count: usize,
) -> Vec<DisassemblyLine> {
    let mut result = Vec::with_capacity(count);
    let mut addr = base_address as u32;

    unsafe {
        let ctx = &mut (*dc).arm_ctx;
        for _ in 0..count {
            let opcode = ctx.read32(addr & !3);
            let disassembly = format_arm_instruction(Arm7DecoderState { pc: addr }, opcode);
            let bytes = format!("{:08X}", opcode);

            result.push(DisassemblyLine {
                address: addr as u64,
                bytes,
                disassembly,
            });

            addr = addr.wrapping_add(4);
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
            sh4_ipr_dispatcher(&mut (*dc).ctx);
            //sh4_fns_dispatcher(&mut (*dc).ctx);
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
                if let Some(rest) = register_name
                    .strip_prefix('R')
                    .or_else(|| register_name.strip_prefix('r'))
                {
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

pub fn get_arm_register(dc: *mut Dreamcast, register_name: &str) -> Option<u32> {
    unsafe {
        let ctx = &(*dc).arm_ctx;
        match register_name.to_uppercase().as_str() {
            "PC" => Some(ctx.regs[R15_ARM_NEXT].get()),
            _ => {
                // Check if it's a general purpose register (R0-R15)
                if let Some(rest) = register_name
                    .strip_prefix('R')
                    .or_else(|| register_name.strip_prefix('r'))
                {
                    if let Ok(idx) = rest.parse::<usize>() {
                        if idx < 16 {
                            return Some(ctx.regs[idx].get());
                        }
                    }
                }
                None
            }
        }
    }
}
