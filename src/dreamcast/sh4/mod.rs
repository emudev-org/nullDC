use crate::dreamcast::sh4::backend_fns::{ dec_start, dec_finalize_shrink, sh4_dec_call_decode, sh4_store32i, sh4_dec_branch_cond, dec_reserve_dispatcher, dec_patch_dispatcher, dec_run_block, dec_free };
use std::ptr;
use std::ptr::{ addr_of, addr_of_mut };
use crate::dreamcast::sh4::sh4dec::format_disas;

#[repr(C)]
pub union FRBank {
    pub f32s: [f32; 32],
    pub u32s: [u32; 32],
    pub u64s: [u64; 16],
}

#[repr(C)]
pub struct Sh4Ctx {
    pub r: [u32; 16],
    pub remaining_cycles: i32,
    pub pc0: u32,
    pub pc1: u32,
    pub pc2: u32,
    pub is_delayslot0: u32,
    pub is_delayslot1: u32,

    pub fr: FRBank,
    pub xf: FRBank,

    pub sr_T: u32,
    pub sr: u32,
    pub macl: u32,
    pub mach: u32,
    pub fpul: u32,
    pub fpscr_PR: u32,
    pub fpscr_SZ: u32,
    pub virt_jdyn: u32,

    pub fns_entrypoint: *const u8,

    // Decoder state (for fns/recompiler)
    pub dec_branch: u32,
    pub dec_branch_cond: u32,
    pub dec_branch_next: u32,
    pub dec_branch_target: u32,
    pub dec_branch_dslot: u32,

    // for fns dispatcher
    pub ptrs: Vec<*const u8>
}

impl Default for Sh4Ctx {
    fn default() -> Self {
        Self {
            r: [0; 16],
            remaining_cycles: 0,
            pc0: 0,
            pc1: 2,
            pc2: 4,

            is_delayslot0: 0,
            is_delayslot1: 0,

            fr: FRBank { u32s: [0; 32] },
            xf: FRBank { u32s: [0; 32] },

            sr_T: 0,
            sr: 0,
            macl: 0,
            mach: 0,
            fpul: 0,
            fpscr_PR: 0,
            fpscr_SZ: 0,
            virt_jdyn: 0,

            fns_entrypoint: ptr::null(),

            ptrs: vec![ptr::null(); 0],

            // dec_pc: 0, // TODO, using pc0 for now and restoring it after
            dec_branch: 0,
            dec_branch_cond: 0,
            dec_branch_next: 0,
            dec_branch_target: 0,
            dec_branch_dslot: 0,
        }
    }
}

mod sh4mem;
use sh4mem::read_mem;

mod sh4dec;
use sh4dec::{SH4_OP_PTR, SH4_OP_DESC};

use crate::dreamcast::Dreamcast;

mod backend_ipr;
mod backend_fns;

pub fn sh4_ipr_dispatcher(dc: *mut Dreamcast) {
    unsafe {
        loop {
            let mut instr: u16 = 0;

            // Equivalent of: read_mem(dc, dc->ctx.pc, instr);
            read_mem(dc, (*dc).ctx.pc0, &mut instr);

            // Call the opcode handler
            let handler = *SH4_OP_PTR.get_unchecked(instr as usize);
            handler(dc, instr);

            (*dc).ctx.pc0 = (*dc).ctx.pc1;
            (*dc).ctx.pc1 = (*dc).ctx.pc2;
            (*dc).ctx.pc2 = (*dc).ctx.pc2.wrapping_add(2);

            (*dc).ctx.is_delayslot0 = (*dc).ctx.is_delayslot1;
            (*dc).ctx.is_delayslot1 = 0;

            // Break when remaining_cycles reaches 0
            (*dc).ctx.remaining_cycles = (*dc).ctx.remaining_cycles.wrapping_sub(1);
            if (*dc).ctx.remaining_cycles <= 0 {
                break;
            }
        }
    }
}

unsafe fn sh4_build_block(dc: &mut Dreamcast, start_pc: u32) -> *const u8 {
    let mut remaining_line = 32 - (start_pc & 31);

    let mut current_pc = start_pc;

    println!("NEW BLOCK {:x} - remaining: {}", current_pc, remaining_line);
    unsafe { dec_start(1024); }

    dec_reserve_dispatcher();

    dc.ctx.dec_branch = 0;
    dc.ctx.dec_branch_cond = 0;
    dc.ctx.dec_branch_next = 0;
    dc.ctx.dec_branch_target = 0;
    dc.ctx.dec_branch_dslot = 0;

    loop {
        let mut instr: u16 = 0;

        // Equivalent of: read_mem(dc, dc->ctx.pc, instr);
        read_mem(dc, current_pc, &mut instr);

        println!("{:x}: {}", current_pc, format_disas(current_pc, instr));

        // Call the opcode handler
        dc.ctx.pc0 = current_pc;
        let handler = unsafe { (*SH4_OP_DESC.get_unchecked(instr as usize)).dech };
        let was_branch_dslot = dc.ctx.dec_branch_dslot;
        handler(dc, instr);
        if was_branch_dslot != 0 {
            dc.ctx.dec_branch_dslot = 0;
        }

        if dc.ctx.dec_branch != 0 && dc.ctx.dec_branch_dslot == 0 {
            if dc.ctx.dec_branch == 1 {
                if was_branch_dslot != 0 {
                    sh4_dec_branch_cond(addr_of_mut!(dc.ctx.pc0), addr_of!(dc.ctx.virt_jdyn), dc.ctx.dec_branch_cond, dc.ctx.dec_branch_next, dc.ctx.dec_branch_target);    
                } else {
                    sh4_dec_branch_cond(addr_of_mut!(dc.ctx.pc0), addr_of!(dc.ctx.sr_T), dc.ctx.dec_branch_cond, dc.ctx.dec_branch_next, dc.ctx.dec_branch_target);
                }
            } else if dc.ctx.dec_branch == 2 {
                sh4_store32i(addr_of_mut!(dc.ctx.pc0), dc.ctx.dec_branch_target);
            }
            break;
        }

        if remaining_line == 1 {
            if dc.ctx.dec_branch != 0 {
                assert!(dc.ctx.dec_branch_dslot != 0);
                println!("Warning: branch dslot on different line PC {:08X}", current_pc);
            } else {
                // TODO: insert synthetic opcode for static branching here
                sh4_store32i(addr_of_mut!(dc.ctx.pc0), current_pc.wrapping_add(2));
                break;
            }
        }

        current_pc = current_pc.wrapping_add(2);
        remaining_line -= 1;
    }

    // TODO: ugly hack but gets the job done
    dc.ctx.pc0 = start_pc;

    dec_patch_dispatcher();
    unsafe { dec_finalize_shrink().0 }
}

pub fn sh4_fns_decode_on_demand(dc: &mut Dreamcast) {
    unsafe { 
        let new_block = sh4_build_block(dc, dc.ctx.pc0);
        dc.ctx.ptrs[((dc.ctx.pc0 & 0xFF_FFFF) / 2) as usize] = new_block;
    }
}

pub fn sh4_fns_dispatcher(dc: *mut Dreamcast) {
    unsafe {
        loop {
            let pc0 = (*dc).ctx.pc0;
            let idx = ((pc0 & 0xFF_FFFF) >> 1) as usize;

            // ptrs: Vec<BlockFn>
            let block = *(*dc).ctx.ptrs.as_ptr().add(idx); // copy the fn ptr

            let block_cycles = dec_run_block(block);
    
            (*dc).ctx.remaining_cycles = (*dc).ctx.remaining_cycles.wrapping_sub(block_cycles as i32);
            if (*dc).ctx.remaining_cycles <= 0 {
                break;
            }
        }
    }
}

pub fn sh4_init_ctx(dc: *mut Dreamcast) {
    let default_fns_entrypoint: *const u8;

    unsafe {
        dec_start(1024);

        sh4_dec_call_decode(dc);

        default_fns_entrypoint = dec_finalize_shrink().0;
    

        (*dc).ctx.fns_entrypoint = default_fns_entrypoint;
        (*dc).ctx.ptrs = vec![default_fns_entrypoint; 8192 * 1024];
    }
}

pub fn sh4_term_ctx(dc: &mut Dreamcast) {
    // TODO: Free also dc.ctx.ptrs here
    unsafe { dec_free(dc.ctx.fns_entrypoint); }
}