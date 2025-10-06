use backend_fns::{ dec_start, dec_finalize_shrink, sh4_dec_call_decode, sh4_store32, sh4_store32i, sh4_dec_branch_cond, dec_reserve_dispatcher, dec_patch_dispatcher, dec_run_block, dec_free };
use std::ptr;
use std::ptr::{ addr_of, addr_of_mut };
use bitfield::bitfield;

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
    pub T, set_T: 0;
    pub S, set_S: 1;
    // bits 2-3 reserved
    pub IMASK, set_IMASK: 7, 4;
    pub Q, set_Q: 8;
    pub M, set_M: 9;
    // bits 10-14 reserved
    pub FD, set_FD: 15;
    // bits 16-27 reserved
    pub BL, set_BL: 28;
    pub RB, set_RB: 29;
    pub MD, set_MD: 30;
    // bit 31 reserved
}

bitfield! {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct FpscrReg(u32);
    impl Debug;

    pub u32, full, set_full: 31, 0;
    pub RM, set_RM: 1, 0;
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
    pub DN, set_DN: 18;
    pub PR, set_PR: 19;
    pub SZ, set_SZ: 20;
    pub FR, set_FR: 21;
    // bits 22-31 reserved (pad)
}

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

    pub sr_T: u32,
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

    pub fns_entrypoint: *const u8,

    // Memory map (moved from Dreamcast)
    pub memmap: [*mut u8; 256],
    pub memmask: [u32; 256],

    // Decoder state (for fns/recompiler)
    pub dec_branch: u32,
    pub dec_branch_cond: u32,
    pub dec_branch_next: u32,
    pub dec_branch_target: u32,
    pub dec_branch_target_dynamic: *const u32,
    pub dec_branch_ssr: *const u32,
    pub dec_branch_dslot: u32,

    // for fns dispatcher
    pub ptrs: Vec<*const u8>
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

            sr_T: 0,
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

            fns_entrypoint: ptr::null(),

            memmap: [ptr::null_mut(); 256],
            memmask: [0; 256],

            ptrs: vec![ptr::null(); 0],

            // dec_pc: 0, // TODO, using pc0 for now and restoring it after
            dec_branch: 0,
            dec_branch_cond: 0,
            dec_branch_next: 0,
            dec_branch_target: 0,
            dec_branch_target_dynamic: ptr::null(),
            dec_branch_ssr: ptr::null(),
            dec_branch_dslot: 0,
        }
    }
}

pub mod sh4mem;
use sh4mem::read_mem;

pub mod sh4dec;
use sh4dec::{SH4_OP_PTR, SH4_OP_DESC};

pub mod backend_ipr;
pub mod backend_fns;

pub fn sh4_ipr_dispatcher(ctx: *mut Sh4Ctx) {
    unsafe {
        loop {
            let mut opcode: u16 = 0;

            // Equivalent of: read_mem(ctx, ctx->pc, opcode);
            read_mem(ctx, (*ctx).pc0, &mut opcode);

            // Call the opcode handler
            let handler = *SH4_OP_PTR.get_unchecked(opcode as usize);
            handler(ctx, opcode);

            (*ctx).pc0 = (*ctx).pc1;
            (*ctx).pc1 = (*ctx).pc2;
            (*ctx).pc2 = (*ctx).pc2.wrapping_add(2);

            (*ctx).is_delayslot0 = (*ctx).is_delayslot1;
            (*ctx).is_delayslot1 = 0;

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
    unsafe { dec_start(1024); }

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
                    if was_branch_dslot != 0 {
                        sh4_dec_branch_cond(addr_of_mut!(ctx.pc0), addr_of!(ctx.virt_jdyn), ctx.dec_branch_cond, ctx.dec_branch_next, ctx.dec_branch_target);
                    } else {
                        sh4_dec_branch_cond(addr_of_mut!(ctx.pc0), addr_of!(ctx.sr_T), ctx.dec_branch_cond, ctx.dec_branch_next, ctx.dec_branch_target);
                    }
                }
                2 => {
                    // Static branch with immediate target
                    sh4_store32i(addr_of_mut!(ctx.pc0), ctx.dec_branch_target);
                }
                3 => {
                    // Dynamic branch with pointer to target
                    sh4_store32(addr_of_mut!(ctx.pc0), ctx.dec_branch_target_dynamic);
                }
                4 => {
                    // Special case for rte
                    sh4_store32(addr_of_mut!(ctx.pc0), ctx.dec_branch_target_dynamic);
                    sh4_store32(addr_of_mut!(ctx.sr.0), ctx.dec_branch_ssr);
                }
                _ => panic!("invalid dec_branch value")
            }
            break;
        }

        if remaining_line == 1 {
            if ctx.dec_branch != 0 {
                assert!(ctx.dec_branch_dslot != 0);
                println!("Warning: branch dslot on different line PC {:08X}", current_pc);
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
}

pub fn sh4_term_ctx(ctx: &mut Sh4Ctx) {
    // TODO: Free also ctx.ptrs here
    unsafe { dec_free(ctx.fns_entrypoint); }
}