use backend_fns::{
    dec_finalize_shrink, dec_free, dec_patch_dispatcher, dec_reserve_dispatcher, dec_run_block,
    dec_start, sh4_dec_branch_cond, sh4_dec_call_decode, sh4_store32, sh4_store32i,
};

use bitfield::bitfield;
use std::ptr;
use std::ptr::{addr_of, addr_of_mut};

use crate::sh4mem::MAX_MEMHANDLERS;

mod sh4p4;
pub use sh4p4::{
    InterruptSourceId, intc_clear_interrupt, intc_raise_interrupt, register_peripheral_hook,
};

#[repr(C)]
pub union FRBank {
    pub f32s: [f32; 32],
    pub u32s: [u32; 32],
    pub u64s: [u64; 16],
    pub f64s: [f64; 16],
}

#[repr(C)]
pub union MacReg {
    pub full: u64,
    pub parts: MacRegParts,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MacRegParts {
    pub l: u32,
    pub h: u32,
}

bitfield! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct SrStatus(u32);
    impl Debug;

    pub u32, full, set_full: 31, 0;
    pub t, set_t: 0;
    pub s, set_s: 1;
    // bits 2-3 reserved
    pub imask, set_imask: 7, 4;
    pub q, set_q: 8;
    pub m, set_m: 9;
    // bits 10-14 reserved
    pub fd, set_fd: 15;
    // bits 16-27 reserved
    pub bl, set_bl: 28;
    pub rb, set_rb: 29;
    pub md, set_md: 30;
    // bit 31 reserved
}

bitfield! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FpscrReg(u32);
    impl Debug;

    pub u32, full, set_full: 31, 0;
    pub rm, set_rm: 1, 0;
    pub finexact, set_finexact: 2;
    pub funderflow, set_funderflow: 3;
    pub foverflow, set_foverflow: 4;
    pub fdivbyzero, set_fdivbyzero: 5;
    pub finvalidop, set_finvalidop: 6;
    pub einexact, set_einexact: 7;
    pub eunderflow, set_eunderflow: 8;
    pub eoverflow, set_eoverflow: 9;
    pub edivbyzero, set_edivbyzero: 10;
    pub einvalidop, set_einvalidop: 11;
    pub cinexact, set_cinexact: 12;
    pub cunderflow, set_cunderflow: 13;
    pub coverflow, set_coverflow: 14;
    pub cdivbyzero, set_cdivbyzero: 15;
    pub cinvalid, set_cinvalid: 16;
    pub cfpuerr, set_cfpuerr: 17;
    pub dn, set_dn: 18;
    pub pr, set_pr: 19;
    pub sz, set_sz: 20;
    pub fr, set_fr: 21;
    // bits 22-31 reserved (pad)
}

fn dummy_read<T: Copy + std::fmt::LowerHex>(_ctx: *mut u8, offset: u32) -> T {
    panic!(
        "dummy_read: Attempted read::<u{}> {:x}",
        std::mem::size_of::<T>(),
        offset
    );
}

fn dummy_write<T: Copy + std::fmt::LowerHex>(_ctx: *mut u8, addr: u32, value: T) {
    panic!(
        "dummy_write: Attempted write::<u{}> {:x} data = {:x}",
        std::mem::size_of::<T>(),
        addr,
        value
    );
}

#[derive(Copy, Clone)]
pub struct MemHandlers {
    pub read8: fn(ctx: *mut u8, addr: u32) -> u8,
    pub read16: fn(ctx: *mut u8, addr: u32) -> u16,
    pub read32: fn(ctx: *mut u8, addr: u32) -> u32,
    pub read64: fn(ctx: *mut u8, addr: u32) -> u64,
    pub write8: fn(ctx: *mut u8, addr: u32, value: u8),
    pub write16: fn(ctx: *mut u8, addr: u32, value: u16),
    pub write32: fn(ctx: *mut u8, addr: u32, value: u32),
    pub write64: fn(ctx: *mut u8, addr: u32, value: u64),
}

pub const DEFAULT_HANDLERS: MemHandlers = MemHandlers {
    read8: dummy_read::<u8>,
    read16: dummy_read::<u16>,
    read32: dummy_read::<u32>,
    read64: dummy_read::<u64>,

    write8: dummy_write::<u8>,
    write16: dummy_write::<u16>,
    write32: dummy_write::<u32>,
    write64: dummy_write::<u64>,
};

#[repr(C)]
pub struct Sh4Ctx {
    pub r: [u32; 16],
    pub r_bank: [u32; 8],
    pub remaining_cycles: i32,
    pub pc0: u32,
    pub pc1: u32,
    pub pc2: u32,
    pub is_delayslot0: u32,
    pub is_delayslot1: u32,

    pub fr: FRBank,
    pub xf: FRBank,

    pub sr_t: u32,
    pub sr: SrStatus,
    pub mac: MacReg,
    pub fpul: u32,
    pub fpscr: FpscrReg,

    // Additional control registers
    pub gbr: u32,
    pub ssr: u32,
    pub spc: u32,
    pub sgr: u32,
    pub dbr: u32,
    pub vbr: u32,
    pub pr: u32,

    pub virt_jdyn: u32,

    pub temp: [u32; 8],

    pub sq_both: [u32; 16],

    pub qacr0_base: u32,
    pub qacr1_base: u32,

    pub fns_entrypoint: *const u8,

    // Memory map (moved from Dreamcast)
    pub memmap: [*mut u8; 256],
    pub memmask: [u32; 256],
    pub memhandlers: [MemHandlers; 256],
    pub memcontexts: [*mut u8; 256],

    pub memhandler_idx: u32,

    // Decoder state (for fns/recompiler)
    pub dec_branch: u32,
    pub dec_branch_cond: u32,
    pub dec_branch_next: u32,
    pub dec_branch_target: u32,
    pub dec_branch_target_dynamic: *const u32,
    pub dec_branch_dslot: u32,

    // for fns dispatcher
    pub ptrs: Vec<*const u8>,
}

impl Default for Sh4Ctx {
    fn default() -> Self {
        Self {
            r: [0; 16],
            r_bank: [0; 8],
            remaining_cycles: 0,
            pc0: 0,
            pc1: 2,
            pc2: 4,

            is_delayslot0: 0,
            is_delayslot1: 0,

            fr: FRBank { u32s: [0; 32] },
            xf: FRBank { u32s: [0; 32] },

            sr_t: 0,
            sr: SrStatus(0),
            mac: MacReg { full: 0 },
            fpul: 0,
            fpscr: FpscrReg(0),

            gbr: 0,
            ssr: 0,
            spc: 0,
            sgr: 0,
            dbr: 0,
            vbr: 0,
            pr: 0,

            virt_jdyn: 0,

            temp: [0; 8],

            sq_both:[0; 16],
            qacr0_base: 0,
            qacr1_base: 0,

            fns_entrypoint: ptr::null(),

            memmap: [ptr::null_mut(); 256],
            memmask: [0xFFFF_FFFF; 256],
            memhandlers: [DEFAULT_HANDLERS; 256],
            memcontexts: [ptr::null_mut(); 256],
            memhandler_idx: 1,

            ptrs: vec![ptr::null(); 0],

            // dec_pc: 0, // TODO, using pc0 for now and restoring it after
            dec_branch: 0,
            dec_branch_cond: 0,
            dec_branch_next: 0,
            dec_branch_target: 0,
            dec_branch_target_dynamic: ptr::null(),
            dec_branch_dslot: 0,
        }
    }
}

pub mod sh4mem;
use sh4mem::read_mem;

pub mod sh4dec;
use sh4dec::{SH4_OP_DESC, SH4_OP_PTR};

pub mod backend_fns;
pub mod backend_ipr;

pub fn sh4_ipr_dispatcher(ctx: *mut Sh4Ctx) {
    unsafe {
        loop {
            if (*ctx).is_delayslot0 == 0 {
                sh4p4::intc_try_service(ctx);
            }

            let mut opcode: u16 = 0;

            read_mem(ctx, (*ctx).pc0, &mut opcode);
            // println!("PC: {:08X} Opcode: {:04X}", (*ctx).pc0, opcode);

            // Call the opcode handler
            let handler = *SH4_OP_PTR.get_unchecked(opcode as usize);

            handler(ctx, opcode);

            (*ctx).pc0 = (*ctx).pc1;
            (*ctx).pc1 = (*ctx).pc2;
            (*ctx).pc2 = (*ctx).pc2.wrapping_add(2);

            (*ctx).is_delayslot0 = (*ctx).is_delayslot1;
            (*ctx).is_delayslot1 = 0;
            sh4p4::peripherals_step(ctx, 1);

            // Break when remaining_cycles reaches 0
            (*ctx).remaining_cycles = (*ctx).remaining_cycles.wrapping_sub(1);
            if (*ctx).remaining_cycles <= 0 {
                break;
            }
        }
    }
}

unsafe fn sh4_build_block(ctx: &mut Sh4Ctx, start_pc: u32) -> *const u8 {
    let mut remaining_line = (32 - (start_pc & 31)) / 2;

    let mut current_pc = start_pc;

    // println!("NEW BLOCK {:x} - remaining: {}", current_pc, remaining_line);
    unsafe {
        dec_start(1024);
    }

    dec_reserve_dispatcher();

    ctx.dec_branch = 0;
    ctx.dec_branch_cond = 0;
    ctx.dec_branch_next = 0;
    ctx.dec_branch_target = 0;
    ctx.dec_branch_dslot = 0;

    loop {
        let mut opcode: u16 = 0;

        // Equivalent of: read_mem(ctx, ctx->pc, opcode);
        read_mem(ctx, current_pc, &mut opcode);

        // println!("{:x}: {}", current_pc, format_disas(SH4DecoderState{pc: current_pc, fpscr_PR: (*ctx).fpscr_PR, fpscr_SZ: (*ctx).fpscr_SZ}, opcode));

        // Call the opcode handler
        ctx.pc0 = current_pc;
        let handler = unsafe { (*SH4_OP_DESC.get_unchecked(opcode as usize)).dech };
        let was_branch_dslot = ctx.dec_branch_dslot;
        handler(ctx, opcode);
        if was_branch_dslot != 0 {
            ctx.dec_branch_dslot = 0;
        }

        if ctx.dec_branch != 0 && ctx.dec_branch_dslot == 0 {
            match ctx.dec_branch {
                1 => {
                    // Conditional branch
                    sh4_dec_branch_cond(
                        addr_of_mut!(ctx.pc0),
                        addr_of!(ctx.virt_jdyn),
                        ctx.dec_branch_cond,
                        ctx.dec_branch_next,
                        ctx.dec_branch_target,
                    );
                }
                2 => {
                    // Static branch with immediate target
                    sh4_store32i(addr_of_mut!(ctx.pc0), ctx.dec_branch_target);
                }
                3 => {
                    // Dynamic branch with pointer to target
                    sh4_store32(addr_of_mut!(ctx.pc0), ctx.dec_branch_target_dynamic);
                }
                _ => panic!("invalid dec_branch value"),
            }
            break;
        }

        if remaining_line == 1 {
            if ctx.dec_branch != 0 {
                assert!(ctx.dec_branch_dslot != 0);
                println!(
                    "Warning: branch dslot on different line PC {:08X}",
                    current_pc
                );
            } else {
                sh4_store32i(addr_of_mut!(ctx.pc0), current_pc.wrapping_add(2));
                break;
            }
        }

        current_pc = current_pc.wrapping_add(2);
        remaining_line -= 1;
    }

    // TODO: ugly hack but gets the job done
    ctx.pc0 = start_pc;

    dec_patch_dispatcher();
    let rv = unsafe { dec_finalize_shrink() };
    // println!("BLOCK done {:?} {}", rv.0, rv.1);
    rv.0
}

pub fn sh4_fns_decode_on_demand(ctx: &mut Sh4Ctx) {
    unsafe {
        let new_block = sh4_build_block(ctx, ctx.pc0);
        ctx.ptrs[((ctx.pc0 & 0xFF_FFFF) / 2) as usize] = new_block;
    }
}

pub fn sh4_fns_dispatcher(ctx: *mut Sh4Ctx) {
    unsafe {
        loop {
            let pc0 = (*ctx).pc0;
            let idx = ((pc0 & 0xFF_FFFF) >> 1) as usize;

            // ptrs: Vec<BlockFn>
            let block = *(*ctx).ptrs.as_ptr().add(idx); // copy the fn ptr

            let block_cycles = dec_run_block(block);

            sh4p4::peripherals_step(ctx, block_cycles as u32);

            let old_pc0 = (*ctx).pc0;
            if sh4p4::intc_try_service(ctx) {
                println!(
                    "Interrupt taken at PC {:08X} -> {:08X}",
                    old_pc0,
                    (*ctx).pc0
                );
            }

            (*ctx).remaining_cycles = (*ctx).remaining_cycles.wrapping_sub(block_cycles as i32);
            if (*ctx).remaining_cycles <= 0 {
                break;
            }
        }
    }
}

pub fn sh4_init_ctx(ctx: *mut Sh4Ctx) {
    let default_fns_entrypoint: *const u8;

    unsafe {
        dec_start(1024);

        sh4_dec_call_decode(ctx);

        default_fns_entrypoint = dec_finalize_shrink().0;

        (*ctx).fns_entrypoint = default_fns_entrypoint;
        (*ctx).ptrs = vec![default_fns_entrypoint; 8192 * 1024];
    }

    sh4_register_mem_handler(
        ctx,
        0xF000_0000,
        0xFFFF_FFFF,
        0xFFFF_FFFF,
        sh4p4::P4_HANDLERS,
        ctx as *mut _ as *mut u8,
    );

    sh4_register_mem_handler(
        ctx,
        0xE000_0000,
        0xE3FF_FFFF,
        63,
        sh4p4::SQ_HANDLERS,
        unsafe { (*ctx).sq_both.as_mut_ptr() as *mut u8 }
    );

    sh4p4::p4_init(unsafe { &mut (*ctx) });
}

pub fn sh4_term_ctx(ctx: &mut Sh4Ctx) {
    // TODO: Free also ctx.ptrs here
    unsafe {
        dec_free(ctx.fns_entrypoint);
    }
}

pub fn sh4_register_mem_handler(
    ctx: *mut Sh4Ctx,
    base: u32,
    end: u32,
    mask: u32,
    handler: MemHandlers,
    memctx: *mut u8,
) {
    assert!(base <= end);

    unsafe {
        assert!((*ctx).memhandler_idx < (MAX_MEMHANDLERS - 1) as u32);

        let registered_memhandler = (*ctx).memhandler_idx as usize;
        (*ctx).memhandler_idx += 1;

        (*ctx).memhandlers[registered_memhandler] = handler;
        (*ctx).memcontexts[registered_memhandler] = memctx;

        let start_index = (base >> 24) as usize;
        let end_index = (end >> 24) as usize;

        for i in start_index..=end_index {
            (*ctx).memmap[i] = registered_memhandler as *mut u8;
            (*ctx).memmask[i] = mask;
        }
    }
}

pub fn sh4_register_mem_buffer(ctx: *mut Sh4Ctx, base: u32, end: u32, mask: u32, buffer: *mut u8) {
    assert!(base <= end);

    unsafe {
        let start_index = (base >> 24) as usize;
        let end_index = (end >> 24) as usize;

        for i in start_index..=end_index {
            (*ctx).memmap[i] = buffer.wrapping_add((i << 24) & mask as usize);
            (*ctx).memmask[i] = mask;
            (*ctx).memhandlers[i] = DEFAULT_HANDLERS;
            (*ctx).memcontexts[i] = ptr::null_mut();
        }
    }
}
