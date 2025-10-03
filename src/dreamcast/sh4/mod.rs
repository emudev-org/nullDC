
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
        }
    }
}

mod sh4mem;
use sh4mem::read_mem;

mod sh4dec;
use sh4dec::SH4_OP_PTR;

use crate::dreamcast::Dreamcast;

mod backend_ipr;
mod backend_fns;

pub fn sh4_ipr_dispatcher(dc: &mut Dreamcast) {
    loop {
        let mut instr: u16 = 0;

        // Equivalent of: read_mem(dc, dc->ctx.pc, instr);
        read_mem(dc, dc.ctx.pc0, &mut instr);

        // Call the opcode handler
        let handler = unsafe { *SH4_OP_PTR.get_unchecked(instr as usize) };
        handler(dc, instr);

        dc.ctx.pc0 = dc.ctx.pc1;
        dc.ctx.pc1 = dc.ctx.pc2;
        dc.ctx.pc2 = dc.ctx.pc2.wrapping_add(2);

        dc.ctx.is_delayslot0 = dc.ctx.is_delayslot1;
        dc.ctx.is_delayslot1 = 0;

        // Break when remaining_cycles reaches 0
        dc.ctx.remaining_cycles = dc.ctx.remaining_cycles.wrapping_sub(1);
        if dc.ctx.remaining_cycles <= 0 {
            break;
        }
    }
}