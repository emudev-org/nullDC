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
use goblin::elf::Elf;

mod area0;
pub use area0::AREA0_HANDLERS;

mod aica;
mod asic;
mod gdrom;
mod pvr;
pub mod refsw2;
mod reios_impl;
mod sgc;
mod spg;
mod system_bus;
pub mod ta;

use arm7di_core::{
    arm7di_disasm::{format_arm_instruction, Arm7DecoderState},
    ArmPsr, R13_IRQ, R13_SVC, R15_ARM_NEXT, RN_CPSR, RN_PSR_FLAGS, RN_SPSR,
};

pub use pvr::present_for_texture;

use crate::sgc::Sgc;

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

        if !dc.arm_ctx.is_running {
            return;
        }

        dc.arm_cycle_accumulator = dc.arm_cycle_accumulator.wrapping_add(cycles);

        while dc.arm_cycle_accumulator >= 20 {
            dc.arm_cycle_accumulator -= 20;
            let mut arm = arm7di_core::Arm7Di::new(&mut dc.arm_ctx);
            arm.update_interrupts();
            arm.step();
        }

        dc.sgc_cycle_accumulator = dc.sgc_cycle_accumulator.wrapping_add(cycles);
        while dc.sgc_cycle_accumulator > (200 * 1000 * 1000 / 44100) {
            dc.sgc_cycle_accumulator -= 200 * 1000 * 1000 / 44100;
            // Step AICA timers before processing audio sample
            aica::step(&mut dc.arm_ctx, 1);

            dc.sgc.aica_sample();
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
    pub arm_ctx: arm7di_core::Arm7Context,
    pub arm_enabled: bool,
    pub arm_cycle_accumulator: u32,
    pub sgc_cycle_accumulator: u32,

    // AICA/SGC state
    pub aica_reg: Box<[u8; 0x8000]>,
    pub dsp: Box<sgc::DspContext>,

    pub sgc: sgc::Sgc,
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
        let mut audio_ram: Box<[u8; AUDIORAM_SIZE as usize]> = {
            let v = vec![0u8; AUDIORAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let oc_ram = {
            let v = vec![0u8; OCRAM_SIZE as usize];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let mut aica_reg: Box<[u8; 0x8000]> = {
            let v = vec![0u8; 0x8000];
            v.into_boxed_slice().try_into().expect("len matches")
        };

        let dsp = Box::new(sgc::DspContext::default());

        // Create SGC with proper references
        let audio_stream = Box::new(sgc::NilAudioStream);
        let aica_reg_ptr = std::ptr::NonNull::new(aica_reg.as_mut_ptr()).unwrap();
        let dsp_ptr = Box::new(sgc::DspContext::default());
        let audio_ram_ptr = std::ptr::NonNull::new(audio_ram.as_mut_ptr()).unwrap();

        let sgc = sgc::Sgc::new(audio_stream, aica_reg_ptr, dsp_ptr, audio_ram_ptr, AUDIORAM_SIZE);

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
            arm_ctx: arm7di_core::Arm7Context::new(),
            arm_enabled: false,
            arm_cycle_accumulator: 0,
            sgc_cycle_accumulator: 0,
            aica_reg,
            dsp,
            sgc,
        }
    }
}

fn reset_arm7(dc: &mut Dreamcast) {
    let mut arm_ctx = arm7di_core::Arm7Context::new();
    
    arm_ctx.aica_ram = NonNull::new(dc.audio_ram.as_mut_ptr());
    arm_ctx.aram_mask = AUDIORAM_MASK;

    arm_ctx.read8 = Some(aica::arm_read8);
    arm_ctx.read32 = Some(aica::arm_read32);
    arm_ctx.write8 = Some(aica::arm_write8);
    arm_ctx.write32 = Some(aica::arm_write32);

    arm7di_core::reset_arm7_ctx(&mut arm_ctx);

    dc.arm_ctx = arm_ctx;
    dc.arm_enabled = true;
    dc.arm_cycle_accumulator = 0;

    let mut arm = arm7di_core::Arm7Di::new(&mut dc.arm_ctx);
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

// pub static ROTO_BIN: &[u8] = include_bytes!("../../../roto.bin");
// pub static IP_BIN: &[u8] = include_bytes!("../../../data/IP.BIN");
// pub static SYS_BIN: &[u8] = include_bytes!("../../../data/syscalls.bin");
// pub static HELLO_BIN: &[u8] = include_bytes!("../../../data/hello.elf.bin");
// pub static ARM7W_BIN: &[u8] = include_bytes!("../../../data/arm7wrestler.bin");

fn reset_dreamcast_to_defaults(dc: &mut Dreamcast) {
    // Zero entire struct (like memset). In Rust, usually you'd implement Default.
    *dc = Dreamcast::default();

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
        0x0000_0000,
        0x03FF_FFFF,
        0xFFFF_FFFF,
        AREA0_HANDLERS,
        dc as *mut _ as *mut u8,
    );
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

    // TA
    sh4_core::sh4_register_mem_handler(
        &mut dc.ctx,
        0x1000_0000,
        0x13FF_FFFF,
        0xFFFF_FFFF,
        ta::TA_HANDLERS,
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
}

pub fn init_dreamcast(dc_: *mut Dreamcast, bios_rom: &[u8], bios_flash: &[u8]) {
    let dc: &mut Dreamcast;
    unsafe {
        dc = &mut *dc_;
    }

    DREAMCAST_PTR.store(dc as *mut Dreamcast, Ordering::SeqCst);

    reset_dreamcast_to_defaults(dc);

    
    // Copy BIOS ROM and Flash from provided slices
    assert_eq!(bios_rom.len(), BIOS_ROM_SIZE as usize, "BIOS ROM must be exactly 2MB");
    assert_eq!(bios_flash.len(), BIOS_FLASH_SIZE as usize, "BIOS Flash must be exactly 128KB");

    dc.bios_rom[..].copy_from_slice(bios_rom);
    dc.bios_flash[..].copy_from_slice(bios_flash);

    // ROTO test program at 0x8C010000
    // dc.ctx.pc0 = 0x8C01_0000;
    // dc.ctx.pc1 = 0x8C01_0000 + 2;
    // dc.ctx.pc2 = 0x8C01_0000 + 4;

    // IP.BIN boot
    // dc.ctx.pc0 = 0x8C00_8300;
    // dc.ctx.pc1 = 0x8C00_8300 + 2;
    // dc.ctx.pc2 = 0x8C00_8300 + 4;

    // unsafe {
    //     let dst = dc.sys_ram.as_mut_ptr().add(0);
    //     let src = SYS_BIN.as_ptr();

    //     ptr::copy_nonoverlapping(src, dst, SYS_BIN.len())
    // }

    // unsafe {
    //     let dst = dc.sys_ram.as_mut_ptr().add(0x8000);
    //     let src = IP_BIN.as_ptr();

    //     ptr::copy_nonoverlapping(src, dst, IP_BIN.len())
    // }

    // unsafe {
    //     // Copy roto.bin from embedded ROTO_BIN
    //     let dst = dc.sys_ram.as_mut_ptr().add(0x10000);
    //     let src = ROTO_BIN.as_ptr();

    //     ptr::copy_nonoverlapping(src, dst, ROTO_BIN.len())
    // }

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
            (*dc).ctx.remaining_cycles += 3_333_333; // ~16.67ms at 200MHz
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

pub fn init_dreamcast_with_elf(dc: *mut Dreamcast, elf_bytes: &[u8]) -> Result<(), String> {
    // Parse the ELF file
    let elf = Elf::parse(elf_bytes)
        .map_err(|e| format!("Failed to parse ELF file: {}", e))?;

    unsafe {
        let dc_ref = &mut *dc;

        reset_dreamcast_to_defaults(dc_ref);

        // Iterate through program headers and load PT_LOAD segments
        for ph in &elf.program_headers {
            // Only load PT_LOAD segments
            if ph.p_type != goblin::elf::program_header::PT_LOAD {
                continue;
            }

            let vaddr = ph.p_vaddr as u32;
            let memsz = ph.p_memsz as usize;
            let filesz = ph.p_filesz as usize;
            let offset = ph.p_offset as usize;

            println!("Loading ELF segment: vaddr=0x{:08X}, memsz=0x{:X}, filesz=0x{:X}",
                     vaddr, memsz, filesz);

            // Determine which memory region this address belongs to
            let (dest_ptr, mask) = match vaddr {
                // System RAM (mirrored across different regions)
                0x0C00_0000..=0x0FFF_FFFF |
                0x8C00_0000..=0x8FFF_FFFF |
                0xAC00_0000..=0xAFFF_FFFF => {
                    (dc_ref.sys_ram.as_mut_ptr(), SYSRAM_MASK)
                }
                // Video RAM
                0x0400_0000..=0x04FF_FFFF |
                0xA400_0000..=0xA4FF_FFFF |
                0x0500_0000..=0x05FF_FFFF |
                0xA500_0000..=0xA5FF_FFFF |
                0x0600_0000..=0x06FF_FFFF |
                0xA600_0000..=0xA6FF_FFFF |
                0x0700_0000..=0x07FF_FFFF |
                0xA700_0000..=0xA7FF_FFFF => {
                    (dc_ref.video_ram.as_mut_ptr(), VIDEORAM_MASK)
                }
                // Audio RAM
                0x0080_0000..=0x009F_FFFF |
                0x8080_0000..=0x809F_FFFF |
                0xA080_0000..=0xA09F_FFFF => {
                    (dc_ref.audio_ram.as_mut_ptr(), AUDIORAM_MASK)
                }
                // On-chip RAM
                0x7C00_0000..=0x7FFF_FFFF => {
                    (dc_ref.oc_ram.as_mut_ptr(), OCRAM_MASK)
                }
                _ => {
                    return Err(format!("ELF segment virtual address 0x{:08X} is not in a valid memory region", vaddr));
                }
            };

            // Calculate the offset into the destination memory
            let dest_offset = (vaddr & mask) as usize;

            // Check if the segment fits in the memory region
            if dest_offset + memsz > (mask as usize + 1) {
                return Err(format!("ELF segment at 0x{:08X} with size 0x{:X} exceeds memory bounds",
                                   vaddr, memsz));
            }

            // Copy the file data
            if filesz > 0 {
                // Validate that we have enough data in the ELF file
                if offset + filesz > elf_bytes.len() {
                    return Err(format!("ELF segment data at offset 0x{:X} with size 0x{:X} exceeds file bounds",
                                       offset, filesz));
                }

                let src_slice = &elf_bytes[offset..offset + filesz];

                ptr::copy_nonoverlapping(
                    src_slice.as_ptr(),
                    dest_ptr.add(dest_offset),
                    filesz
                );
            }

            // Zero out remaining bytes (BSS sections)
            if memsz > filesz {
                let bss_size = memsz - filesz;
                ptr::write_bytes(
                    dest_ptr.add(dest_offset + filesz),
                    0,
                    bss_size
                );
            }
        }

        DREAMCAST_PTR.store(dc, Ordering::SeqCst);
        

        // Set PC to entry point if valid
        if elf.entry > 0 {
            let entry = elf.entry as u32;
            println!("Setting entry point to 0x{:08X}", entry);
            dc_ref.ctx.pc0 = entry;
            dc_ref.ctx.pc1 = entry + 2;
            dc_ref.ctx.pc2 = entry + 4;
        }

        // Initialize REIOS for syscall support
        init_reios_for_elf(dc_ref);

        Ok(())
    }
}

/// Initialize REIOS for ELF execution (provides syscall support)
fn init_reios_for_elf(dc: &mut Dreamcast) {
    use crate::reios_impl::{Sh4ContextWrapper, DUMMY_DISC_INSTANCE};

    // Use unsafe to split borrows for REIOS boot and create context pointers
    let dc_ptr = dc as *mut Dreamcast;
    let mut reios_ctx = unsafe {
        // Allocate ctx_wrapper on heap so pointers remain valid
        let ctx_wrapper = Box::new(Sh4ContextWrapper {
            ctx: &mut (*dc_ptr).ctx,
            running: &mut (*dc_ptr).running,
            dreamcast: &mut *dc_ptr,
        });

        // Leak the box to get a stable pointer with 'static lifetime
        let wrapper_ptr = Box::leak(ctx_wrapper) as *mut Sh4ContextWrapper;

        // Create trait object pointers from wrapper
        let mem_ptr: *mut dyn reios::ReiosSh4Memory = wrapper_ptr;
        let ctx_ptr: *mut dyn reios::ReiosSh4Context = wrapper_ptr;
        let disc_ptr: *const dyn reios::ReiosDisc = &DUMMY_DISC_INSTANCE;

        // Initialize REIOS with pointers
        let mut reios_ctx = reios::ReiosContext::new(mem_ptr, ctx_ptr, disc_ptr);

        let mem = &mut *dc_ptr;
        reios_ctx.init(mem);

        // Boot REIOS (this sets up syscalls but doesn't change PC)
        let mem_ref = &mut *wrapper_ptr as &mut dyn reios::ReiosSh4Memory;
        let ctx_ref = &mut *wrapper_ptr as &mut dyn reios::ReiosSh4Context;

        reios_ctx.boot(mem_ref, ctx_ref, &DUMMY_DISC_INSTANCE);

        reios_ctx
    };

    // Store REIOS context in SH4 context for trap handling
    dc.ctx.reios_ctx = Some(reios_ctx);

    println!("REIOS initialized for ELF execution");
}
