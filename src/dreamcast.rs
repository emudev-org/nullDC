//! dreamcast_sh4.rs — 1:1 Rust translation of the provided C++/C code snippet.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ptr;

use std::ptr::{addr_of, addr_of_mut};

const SYSRAM_SIZE: u32 = 16 * 1024 * 1024;
const VIDEORAM_SIZE: u32 = 8 * 1024 * 1024;

const SYSRAM_MASK: u32 = SYSRAM_SIZE - 1;
const VIDEORAM_MASK: u32 = VIDEORAM_SIZE - 1;

#[derive(Copy, Clone)]
pub struct sh4_opcodelistentry {
    pub oph: fn(&mut Dreamcast, u16),
    pub handler_name: &'static str,
    pub mask: u16,
    pub key: u16,
    pub diss: &'static str,
}

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


// -----------------------------------------------------------------------------
// mem.h — generic read/write declarations; we keep them as stubs, matching
// your snippet (only declarations there). Handlers that depend on memory will
// compile; the stub can be replaced with a real bus implementation later.
// -----------------------------------------------------------------------------
pub fn read_mem<T: Copy>(dc: &mut Dreamcast, addr: u32, out: &mut T) -> bool {
    let region = (addr >> 24) as usize;
    let offset = (addr & dc.memmask[region]) as usize;

    unsafe {
        let base = dc.memmap[region];
        if base.is_null() {
            return false;
        }
        // pointer to T
        let ptr = base.add(offset) as *const T;
        *out = *ptr;
    }

    true
}

pub fn write_mem<T: Copy>(dc: &mut Dreamcast, addr: u32, data: T) -> bool {
    let region = (addr >> 24) as usize;
    let offset = (addr & dc.memmask[region]) as usize;

    unsafe {
        let base = dc.memmap[region];
        if base.is_null() {
            return false;
        }
        let ptr = base.add(offset) as *mut T;
        *ptr = data;
    }

    true
}


// -----------------------------------------------------------------------------
// oplist.inl helpers/macros translated to consts and inline fns
// -----------------------------------------------------------------------------

const MASK_N_M: u16 = 0xF00F;
const MASK_N_M_IMM4: u16 = 0xF000;
const MASK_N: u16 = 0xF0FF;
const MASK_NONE: u16 = 0xFFFF;
const MASK_IMM8: u16 = 0xFF00;
const MASK_IMM12: u16 = 0xF000;
const MASK_N_IMM8: u16 = 0xF000;
const MASK_N_ML3BIT: u16 = 0xF08F;
const MASK_NH3BIT: u16 = 0xF1FF;
const MASK_NH2BIT: u16 = 0xF3FF;

#[inline(always)]
fn GetN(str_: u16) -> usize { ((str_ >> 8) & 0xF) as usize }
#[inline(always)]
fn GetM(str_: u16) -> usize { ((str_ >> 4) & 0xF) as usize }
#[inline(always)]
fn GetImm4(str_: u16) -> u32 { (str_ & 0xF) as u32 }
#[inline(always)]
fn GetImm8(str_: u16) -> u32 { (str_ & 0xFF) as u32 }
#[inline(always)]
fn GetSImm8(str_: u16) -> i32 { (str_ & 0xFF) as i8 as i32 }
#[inline(always)]
fn GetImm12(str_: u16) -> u32 { (str_ & 0xFFF) as u32 }
#[inline(always)]
fn GetSImm12(str_: u16) -> i32 { ((((GetImm12(str_) as u16) << 4) as i16) >> 4) as i32 }

// -----------------------------------------------------------------------------
// sh4impl / sh4op
// -----------------------------------------------------------------------------

fn i_not_implemented(dc: &mut Dreamcast, pc: u32, instr: u16) {
    let desc_ptr: *const sh4_opcodelistentry = &SH4_OP_DESC[instr as usize];
    let diss = unsafe {
        if desc_ptr.is_null() {
            "missing"
        } else {
            let d = &*desc_ptr;
            if d.diss.is_empty() { "missing" } else { d.diss }
        }
    };
    println!("{:08X}: {:04X} {} [i_not_implemented]", pc, instr, diss);
}

fn i_not_known(dc: &mut Dreamcast, instr: u16) {
    let pc = dc.ctx.pc0;
    let desc_ptr = &SH4_OP_DESC[instr as usize];
    println!("{:08X}: {:04X} {} [i_not_known]", pc, instr, desc_ptr.diss);
}

// Helper macro to declare SH-4 opcode handlers with the correct signature.
// Replace your current `sh4op!` with this version.
// Usage:
// sh4op! {
//     /* implemented ops ... */
//
//     stubs! { i0000_nnnn_0010_0011, /* ... */ }
// }

pub const fn parse_opcode(pattern: &str) -> (u16, u16) {
    let bytes = pattern.as_bytes();
    let mut i = 1; // skip the leading 'i'
    let mut mask: u16 = 0;
    let mut key: u16 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'0' || c == b'1' {
            mask = (mask << 1) | 1;
            if c == b'1' {
                key = (key << 1) | 1;
            } else {
                key = key << 1;
            }
        } else if c == b'_' {
            // skip
        } else {
            // wildcard (n, m, etc.)
            mask = mask << 1;
            key = key << 1;
        }
        i += 1;
    }
    (mask, key)
}
macro_rules! sh4op {
    (
        $( (disas = $diss:literal)
           $name:ident ( $($params:tt)* ) { $($body:tt)* }
        )*
    ) => {
        pub(crate) mod exec {
            use super::*;
            $(
                sh4op!(@emit $name ( $($params)* ) { $($body)* } ; backend = backend_exec);
            )*
        }
        pub(crate) mod dec {
            use super::*;
            $(
                sh4op!(@emit $name ( $($params)* ) { $($body)* } ; backend = backend_dec);
            )*
        }

        static OPCODES: &[sh4_opcodelistentry] = &[
            $(
                {
                    const MASK_KEY: (u16,u16) = parse_opcode(stringify!($name));
                    sh4_opcodelistentry {
                        oph: exec::$name,
                        handler_name: stringify!($name),
                        mask: MASK_KEY.0,
                        key: MASK_KEY.1,
                        diss: $diss,
                    }
                }
            ),*,
            sh4_opcodelistentry { oph: i_not_known, handler_name: "unknown_opcode", mask: MASK_NONE, key: 0, diss: "unknown opcode" },
        ];
    };

    (@emit
        $name:ident ( $dc:ident , $instr:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: &mut Dreamcast, $instr: u16) {
            #[allow(unused_imports)]
            use $backend as backend;
            { $($body)* }
        }
    };

    (@emit
        $name:ident ( $dc:ident , $pc:ident , $instr:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: &mut Dreamcast, $instr: u16) {
            #[allow(unused_imports)]
            use $backend as backend;
            let $pc: u32 = $dc.ctx.pc0;
            { $($body)* }
        }
    };
}



// -----------------------------------------------------------------------------
// Implemented handlers (as per your snippet). Unimplemented ones are stubbed.
// -----------------------------------------------------------------------------


fn data_target_s8(pc: u32, disp8: i32) -> u32 {
    ((pc.wrapping_add(4)) & 0xFFFFFFFC).wrapping_add((disp8 << 2) as u32)
}
fn branch_target_s8(pc: u32, disp8: i32) -> u32 {
    (disp8 as i64 * 2 + 4 + pc as i64) as u32
}
fn branch_target_s12(pc: u32, disp12: i32) -> u32 {
    (disp12 as i64 * 2 + 4 + pc as i64) as u32
}

sh4op! {
    (disas = "mul.l <REG_M>,<REG_N>")
    i0000_nnnn_mmmm_0111(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_muls32(addr_of_mut!(dc.ctx.macl), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "nop")
    i0000_0000_0000_1001(dc, instr) {
        // no-op
    }

    (disas = "sts FPUL,<REG_N>")
    i0000_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.fpul));
    }

    (disas = "sts MACL,<REG_N>")
    i0000_nnnn_0001_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.macl));
    }

    (disas = "mov.b <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem8(addr_of_mut!(*dc), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "mov.w <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0001(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem16(addr_of_mut!(*dc), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "mov.l <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0010(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem32(addr_of_mut!(*dc), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "and <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1001(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_and(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "xor <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1010(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_xor(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "sub <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_sub(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "add <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_add(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "dt <REG_N>")
    i0100_nnnn_0001_0000(dc, instr) {
        let n = GetN(instr);
        backend::sh4_dt(addr_of_mut!(dc.ctx.sr_T), addr_of_mut!(dc.ctx.r[n]));
    }

    (disas = "shlr <REG_N>")
    i0100_nnnn_0000_0001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlr(addr_of_mut!(dc.ctx.sr_T), addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]));
    }

    (disas = "shll8 <REG_N>")
    i0100_nnnn_0001_1000(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shllf(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), 8);
    }

    (disas = "shlr2 <REG_N>")
    i0100_nnnn_0000_1001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlrf(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), 2);
    }

    (disas = "shlr16 <REG_N>")
    i0100_nnnn_0010_1001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlrf(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), 16);
    }

    (disas = "mov.b @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);

        backend::sh4_read_mems8(addr_of_mut!(*dc), addr_of!(dc.ctx.r[m]), addr_of_mut!(dc.ctx.r[n]));
    }

    (disas = "mov <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0011(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_store32(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "neg <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1011(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_neg(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "extu.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_extub(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[m]));
    }

    (disas = "add #<simm8>,<REG_N>")
    i0111_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr);
        let stmp1 = GetSImm8(instr);
        backend::sh4_addi(addr_of_mut!(dc.ctx.r[n]), addr_of!(dc.ctx.r[n]), stmp1 as u32);
    }

    (disas = "bf <bdisp8>")
    i1000_1011_iiii_iiii(dc, pc, instr) {
        let disp8 = GetSImm8(instr);
        let next = pc.wrapping_add(2);
        let target = branch_target_s8(pc, disp8);
        backend::sh4_branch_cond(addr_of_mut!(*dc), addr_of!(dc.ctx.sr_T), 0, next, target);
    }

    (disas = "bf/s <bdisp8>")
    i1000_1111_iiii_iiii(dc, pc, instr) {
        let disp8 = GetSImm8(instr);
        let next = pc.wrapping_add(4);
        let target = branch_target_s8(pc, disp8);
        backend::sh4_branch_cond_delay(addr_of_mut!(*dc), addr_of!(dc.ctx.sr_T), 0, next, target);
    }

    (disas = "bra <bdisp12>")
    i1010_iiii_iiii_iiii(dc, pc, instr) {
        let disp12 = GetSImm12(instr);
        let target = branch_target_s12(pc, disp12);
        backend::sh4_branch_delay(addr_of_mut!(*dc), target);
    }

    (disas = "mova @(<PCdisp8d>),R0")
    i1100_0111_iiii_iiii(dc, instr) {
        let disp8 = GetImm8(instr) as i32;
        let addr = data_target_s8(dc.ctx.pc0, disp8);
        backend::sh4_store32i(addr_of_mut!(dc.ctx.r[0]), addr);
    }
    (disas = "mov.b R0,@(<disp4b>,<REG_M>)")
    i1000_0000_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w R0,@(<disp4w>,<REG_M>)")
    i1000_0001_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b @(<disp4b>,<REG_M>),R0")
    i1000_0100_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w @(<disp4w>,<REG_M>),R0")
    i1000_0101_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/eq #<simm8hex>,R0")
    i1000_1000_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }


    (disas = "mov.l @(<PCdisp8d>),<REG_N>")
    i1101_nnnn_iiii_iiii(dc, pc, instr) {
        let n = GetN(instr);
        let disp8 = GetImm8(instr) as i32;
        let addr = data_target_s8(pc, disp8);

        backend::sh4_read_mem32i(dc, addr, addr_of_mut!(dc.ctx.r[n]));
    }

    (disas = "mov #<simm8hex>,<REG_N>")
    i1110_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr);
        let imm = GetSImm8(instr);
        backend::sh4_store32i(addr_of_mut!(dc.ctx.r[n]), imm as u32);
    }

    (disas = "fadd <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0000(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fadd(addr_of_mut!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fsub <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmul <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0010(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fmul(addr_of_mut!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fdiv <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0011(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fdiv(addr_of_mut!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fmov.s @<REG_M>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1000(dc, instr) {
        if dc.ctx.fpscr_SZ == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_read_mem32(dc, addr_of!(dc.ctx.r[m]), addr_of_mut!(dc.ctx.fr.u32s[n])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fmov <FREG_M_SD_A>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1100(dc, instr) {
        if dc.ctx.fpscr_SZ == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_store32(addr_of_mut!(dc.ctx.fr.u32s[n]), addr_of!(dc.ctx.fr.u32s[m])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fsca FPUL,<DR_N>")
    i1111_nnn0_1111_1101(dc, instr) {
        let n = (GetN(instr) & 0xE) as usize;
        if dc.ctx.fpscr_PR == 0 {
            unsafe { backend::sh4_fsca(addr_of_mut!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fpul)); }
            
        } else {
            debug_assert!(false);
        }
    }

    (disas = "float FPUL,<FREG_N_SD_F>")
    i1111_nnnn_0010_1101(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            unsafe { backend::sh4_float(addr_of_mut!(dc.ctx.fr.f32s[n]), addr_of!(dc.ctx.fpul)); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "ftrc <FREG_N>,FPUL")
    i1111_nnnn_0011_1101(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            unsafe { backend::sh4_ftrc(addr_of_mut!(dc.ctx.fpul), addr_of!(dc.ctx.fr.f32s[n])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "lds <REG_N>,FPUL")
    i0100_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!(dc.ctx.fpul), addr_of!(dc.ctx.r[n]));
    }

    (disas = "stc SR,<REG_N>")
    i0000_nnnn_0000_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc GBR,<REG_N>")
    i0000_nnnn_0001_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc VBR,<REG_N>")
    i0000_nnnn_0010_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc SSR,<REG_N>")
    i0000_nnnn_0011_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts SGR,<REG_N>")
    i0000_nnnn_0011_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc SPC,<REG_N>")
    i0000_nnnn_0100_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc RM_BANK,<REG_N>")
    i0000_nnnn_1mmm_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "braf <REG_N>")
    i0000_nnnn_0010_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "bsrf <REG_N>")
    i0000_nnnn_0000_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "movca.l R0,@<REG_N>")
    i0000_nnnn_1100_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ocbi @<REG_N>")
    i0000_nnnn_1001_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ocbp @<REG_N>")
    i0000_nnnn_1010_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ocbwb @<REG_N>")
    i0000_nnnn_1011_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "pref @<REG_N>")
    i0000_nnnn_1000_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "clrmac")
    i0000_0000_0010_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "clrs")
    i0000_0000_0100_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "clrt")
    i0000_0000_0000_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldtlb")
    i0000_0000_0011_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sets")
    i0000_0000_0101_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sett")
    i0000_0000_0001_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "div0u")
    i0000_0000_0001_1001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "movt <REG_N>")
    i0000_nnnn_0010_1001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts FPSCR,<REG_N>")
    i0000_nnnn_0110_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts DBR,<REG_N>")
    i0000_nnnn_1111_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts MACH,<REG_N>")
    i0000_nnnn_0000_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rte")
    i0000_0000_0010_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rts")
    i0000_0000_0000_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sleep")
    i0000_0000_0001_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mac.l @<REG_M>+,@<REG_N>+")
    i0000_nnnn_mmmm_1111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l <REG_M>,@(<disp4dw>,<REG_N>)")
    i0001_nnnn_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "div0s <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "tst <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "or <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/str <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "xtrct <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mulu.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "muls.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/eq <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/hs <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/ge <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "div1 <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "dmulu.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/hi <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/gt <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "subc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "subv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "dmuls.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "addc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "addv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts.l FPUL,@-<REG_N>")
    i0100_nnnn_0101_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts.l FPSCR,@-<REG_N>")
    i0100_nnnn_0110_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts.l MACH,@-<REG_N>")
    i0100_nnnn_0000_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts.l MACL,@-<REG_N>")
    i0100_nnnn_0001_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "sts.l PR,@-<REG_N>")
    i0100_nnnn_0010_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l DBR,@-<REG_N>")
    i0100_nnnn_1111_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l SR,@-<REG_N>")
    i0100_nnnn_0000_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l GBR,@-<REG_N>")
    i0100_nnnn_0001_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l VBR,@-<REG_N>")
    i0100_nnnn_0010_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l SSR,@-<REG_N>")
    i0100_nnnn_0011_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l SPC,@-<REG_N>")
    i0100_nnnn_0100_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc <RM_BANK>,@-<REG_N>")
    i0100_nnnn_1mmm_0011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds.l @<REG_N>+,MACH")
    i0100_nnnn_0000_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds.l @<REG_N>+,MACL")
    i0100_nnnn_0001_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds.l @<REG_N>+,PR")
    i0100_nnnn_0010_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,SGR")
    i0100_nnnn_0011_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds.l @<REG_N>+,FPUL")
    i0100_nnnn_0101_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds.l @<REG_N>+,FPSCR")
    i0100_nnnn_0110_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,DBR")
    i0100_nnnn_1111_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,SR")
    i0100_nnnn_0000_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,GBR")
    i0100_nnnn_0001_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,VBR")
    i0100_nnnn_0010_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,SSR")
    i0100_nnnn_0011_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,SPC")
    i0100_nnnn_0100_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc.l @<REG_N>+,RM_BANK")
    i0100_nnnn_1mmm_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds <REG_N>,MACH")
    i0100_nnnn_0000_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds <REG_N>,MACL")
    i0100_nnnn_0001_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds <REG_N>,PR")
    i0100_nnnn_0010_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "lds <REG_N>,FPSCR")
    i0100_nnnn_0110_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,DBR")
    i0100_nnnn_1111_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,SR")
    i0100_nnnn_0000_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,GBR")
    i0100_nnnn_0001_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,VBR")
    i0100_nnnn_0010_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,SSR")
    i0100_nnnn_0011_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,SPC")
    i0100_nnnn_0100_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ldc <REG_N>,<RM_BANK>")
    i0100_nnnn_1mmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shll <REG_N>")
    i0100_nnnn_0000_0000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shal <REG_N>")
    i0100_nnnn_0010_0000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/pz <REG_N>")
    i0100_nnnn_0001_0001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shar <REG_N>")
    i0100_nnnn_0010_0001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rotcl <REG_N>")
    i0100_nnnn_0010_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rotl <REG_N>")
    i0100_nnnn_0000_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "cmp/pl <REG_N>")
    i0100_nnnn_0001_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rotcr <REG_N>")
    i0100_nnnn_0010_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "rotr <REG_N>")
    i0100_nnnn_0000_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shll2 <REG_N>")
    i0100_nnnn_0000_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shll16 <REG_N>")
    i0100_nnnn_0010_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shlr8 <REG_N>")
    i0100_nnnn_0001_1001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "jmp @<REG_N>")
    i0100_nnnn_0010_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "jsr @<REG_N>")
    i0100_nnnn_0000_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "tas.b @<REG_N>")
    i0100_nnnn_0001_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shad <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "shld <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mac.w @<REG_M>+,@<REG_N>+")
    i0100_nnnn_mmmm_1111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    // 5xxx
    (disas = "mov.l @(<disp4dw>,<REG_M>),<REG_N>")
    i0101_nnnn_mmmm_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    // 6xxx
    (disas = "mov.w @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "not <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "swap.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1000(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "swap.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "negc <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "extu.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "exts.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "exts.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    //8xxx

    (disas = "mov.w @(<PCdisp8w>),<REG_N>")
    i1001_nnnn_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }


    (disas = "bt <bdisp8>")
    i1000_1001_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "bt/s <bdisp8>")
    i1000_1101_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    
    //bxxx
    (disas = "bsr <bdisp12>")
    i1011_iiii_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }


    //Cxxx
    (disas = "mov.b R0,@(<disp8b>,GBR)")
    i1100_0000_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w R0,@(<disp8w>,GBR)")
    i1100_0001_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l R0,@(<disp8dw>,GBR)")
    i1100_0010_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "trapa #<imm8>")
    i1100_0011_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.b @(<GBRdisp8b>),R0")
    i1100_0100_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.w @(<GBRdisp8w>),R0")
    i1100_0101_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "mov.l @(<GBRdisp8dw>),R0")
    i1100_0110_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "tst #<imm8>,R0")
    i1100_1000_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "and #<imm8>,R0")
    i1100_1001_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "xor #<imm8>,R0")
    i1100_1010_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "or #<imm8>,R0")
    i1100_1011_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "tst.b #<imm8>,@(R0,GBR)")
    i1100_1100_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "and.b #<imm8>,@(R0,GBR)")
    i1100_1101_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "xor.b #<imm8>,@(R0,GBR)")
    i1100_1110_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "or.b #<imm8>,@(R0,GBR)")
    i1100_1111_iiii_iiii(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    //Fxxx
    (disas = "flds <FREG_N>,FPUL")
    i1111_nnnn_0001_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fneg <FREG_N_SD_F>")
    i1111_nnnn_0100_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fabs <FREG_N_SD_F>")
    i1111_nnnn_0101_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fsqrt <FREG_N>")
    i1111_nnnn_0110_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fldi0 <FREG_N>")
    i1111_nnnn_1000_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fldi1 <FREG_N>")
    i1111_nnnn_1001_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "ftrv xmtrx,<FV_N>")
    i1111_nn01_1111_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fcmp/eq <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0100(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmov.s @(R0,<REG_M>),<FREG_N_SD_A>")
    i1111_nnnn_mmmm_0110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)")
    i1111_nnnn_mmmm_0111(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmov.s @<REG_M>+,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1001(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@<REG_N>")
    i1111_nnnn_mmmm_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@-<REG_N>")
    i1111_nnnn_mmmm_1011(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }


    (disas = "fcnvds <DR_N>,FPUL")
    i1111_nnnn_1011_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fcnvsd FPUL,<DR_N>")
    i1111_nnnn_1010_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fipr <FV_M>,<FV_N>")
    i1111_nnmm_1110_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "frchg")
    i1111_1011_1111_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fschg")
    i1111_0011_1111_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fsts FPUL,<FREG_N>")
    i1111_nnnn_0000_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fsrra <FREG_N>")
    i1111_nnnn_0111_1101(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "fmac <FREG_0>,<FREG_M>,<FREG_N>")
    i1111_nnnn_mmmm_1110(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }


    (disas = "sts PR,<REG_N>")
    i0000_nnnn_0010_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }
    (disas = "ldc <REG_N>,SGR")
    i0100_nnnn_0011_1010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }

    (disas = "stc.l SGR,@-<REG_N>")
    i0100_nnnn_0011_0010(dc, pc, instr) {
        i_not_implemented(dc, pc, instr);
    }
}

// -----------------------------------------------------------------------------
// Opcode list (array) — translated 1:1 from your snippet
// -----------------------------------------------------------------------------

pub const fn build_opcode_tables(
    opcodes: &[sh4_opcodelistentry]
) -> ([fn(&mut Dreamcast, u16); 0x10000],
      [sh4_opcodelistentry; 0x10000])
{
    // The sentinel is always the last element of OPCODES
    let sentinel = opcodes[opcodes.len() - 1];

    let mut ptrs: [fn(&mut Dreamcast, u16); 0x10000] = [sentinel.oph; 0x10000];
    let mut descs: [sh4_opcodelistentry; 0x10000] = [sentinel; 0x10000];

    let mut i = 0;
    while i < opcodes.len() {
        let op = opcodes[i];
        if op.key == 0 {
            break; // stop at sentinel
        }

        let (count, shft) = match op.mask {
            MASK_NONE       => (1, 0),
            MASK_N          => (16, 8),
            MASK_N_M        => (256, 4),
            MASK_N_M_IMM4   => (256*16, 0),
            MASK_IMM8       => (256, 0),
            MASK_N_ML3BIT   => (256, 4),
            MASK_NH3BIT     => (8, 9),
            MASK_NH2BIT     => (4, 10),
            _               => (0, 0), // invalid mask -> no expansion
        };

        let mask = !(op.mask as u32);
        let base = op.key as u32;

        let mut j = 0;
        while j < count {
            let idx = ((j << shft) & mask) + base;
            ptrs[idx as usize] = op.oph;
            descs[idx as usize] = op;
            j += 1;
        }

        i += 1;
    }

    (ptrs, descs)
}

pub const SH4_OP_TABLES: (
    [fn(&mut Dreamcast, u16); 0x10000],
    [sh4_opcodelistentry; 0x10000]
) = build_opcode_tables(OPCODES);

pub const SH4_OP_PTR: [fn(&mut Dreamcast, u16); 0x10000] = SH4_OP_TABLES.0;
pub const SH4_OP_DESC: [sh4_opcodelistentry; 0x10000] = SH4_OP_TABLES.1;

static OPCODES2: &[sh4_opcodelistentry] = &[
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0010_0011, handler_name: "i0000_nnnn_0010_0011", mask: MASK_N, key: 0x0023, diss: "braf <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0000_0011, handler_name: "i0000_nnnn_0000_0011", mask: MASK_N, key: 0x0003, diss: "bsrf <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1100_0011, handler_name: "i0000_nnnn_1100_0011", mask: MASK_N, key: 0x00C3, diss: "movca.l R0,@<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1001_0011, handler_name: "i0000_nnnn_1001_0011", mask: MASK_N, key: 0x0093, diss: "ocbi @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1010_0011, handler_name: "i0000_nnnn_1010_0011", mask: MASK_N, key: 0x00A3, diss: "ocbp @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1011_0011, handler_name: "i0000_nnnn_1011_0011", mask: MASK_N, key: 0x00B3, diss: "ocbwb @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1000_0011, handler_name: "i0000_nnnn_1000_0011", mask: MASK_N, key: 0x0083, diss: "pref @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_0111, handler_name: "i0000_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x0007, diss: "mul.l <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0010_1000, handler_name: "i0000_0000_0010_1000", mask: MASK_NONE, key: 0x0028, diss: "clrmac" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0100_1000, handler_name: "i0000_0000_0100_1000", mask: MASK_NONE, key: 0x0048, diss: "clrs" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0000_1000, handler_name: "i0000_0000_0000_1000", mask: MASK_NONE, key: 0x0008, diss: "clrt" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0011_1000, handler_name: "i0000_0000_0011_1000", mask: MASK_NONE, key: 0x0038, diss: "ldtlb" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0101_1000, handler_name: "i0000_0000_0101_1000", mask: MASK_NONE, key: 0x0058, diss: "sets" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0001_1000, handler_name: "i0000_0000_0001_1000", mask: MASK_NONE, key: 0x0018, diss: "sett" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0001_1001, handler_name: "i0000_0000_0001_1001", mask: MASK_NONE, key: 0x0019, diss: "div0u" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0010_1001, handler_name: "i0000_nnnn_0010_1001", mask: MASK_N, key: 0x0029, diss: "movt <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0000_1001, handler_name: "i0000_0000_0000_1001", mask: MASK_NONE, key: 0x0009, diss: "nop" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0010_1011, handler_name: "i0000_0000_0010_1011", mask: MASK_NONE, key: 0x002B, diss: "rte" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0000_1011, handler_name: "i0000_0000_0000_1011", mask: MASK_NONE, key: 0x000B, diss: "rts" },
    sh4_opcodelistentry { oph: exec::i0000_0000_0001_1011, handler_name: "i0000_0000_0001_1011", mask: MASK_NONE, key: 0x001B, diss: "sleep" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_1111, handler_name: "i0000_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x000F, diss: "mac.l @<REG_M>+,@<REG_N>+" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0111, handler_name: "i0010_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x2007, diss: "div0s <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1000, handler_name: "i0010_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x2008, diss: "tst <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1001, handler_name: "i0010_nnnn_mmmm_1001", mask: MASK_N_M, key: 0x2009, diss: "and <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1010, handler_name: "i0010_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x200A, diss: "xor <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1011, handler_name: "i0010_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x200B, diss: "or <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1100, handler_name: "i0010_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x200C, diss: "cmp/str <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1101, handler_name: "i0010_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x200D, diss: "xtrct <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1110, handler_name: "i0010_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x200E, diss: "mulu.w <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_1111, handler_name: "i0010_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x200F, diss: "muls.w <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0000, handler_name: "i0011_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x3000, diss: "cmp/eq <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0010, handler_name: "i0011_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x3002, diss: "cmp/hs <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0011, handler_name: "i0011_nnnn_mmmm_0011", mask: MASK_N_M, key: 0x3003, diss: "cmp/ge <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0100, handler_name: "i0011_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x3004, diss: "div1 <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0101, handler_name: "i0011_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x3005, diss: "dmulu.l <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0110, handler_name: "i0011_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x3006, diss: "cmp/hi <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_0111, handler_name: "i0011_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x3007, diss: "cmp/gt <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1000, handler_name: "i0011_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x3008, diss: "sub <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1010, handler_name: "i0011_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x300A, diss: "subc <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1011, handler_name: "i0011_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x300B, diss: "subv <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1100, handler_name: "i0011_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x300C, diss: "add <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1101, handler_name: "i0011_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x300D, diss: "dmuls.l <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1110, handler_name: "i0011_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x300E, diss: "addc <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0011_nnnn_mmmm_1111, handler_name: "i0011_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x300F, diss: "addv <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_0100, handler_name: "i0000_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x0004, diss: "mov.b <REG_M>,@(R0,<REG_N>)" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_0101, handler_name: "i0000_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x0005, diss: "mov.w <REG_M>,@(R0,<REG_N>)" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_0110, handler_name: "i0000_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x0006, diss: "mov.l <REG_M>,@(R0,<REG_N>)" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_1100, handler_name: "i0000_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x000C, diss: "mov.b @(R0,<REG_M>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_1101, handler_name: "i0000_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x000D, diss: "mov.w @(R0,<REG_M>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_mmmm_1110, handler_name: "i0000_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x000E, diss: "mov.l @(R0,<REG_M>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0001_nnnn_mmmm_iiii, handler_name: "i0001_nnnn_mmmm_iiii", mask: MASK_N_IMM8, key: 0x1000, diss: "mov.l <REG_M>,@(<disp4dw>,<REG_N>)" },
    sh4_opcodelistentry { oph: exec::i0101_nnnn_mmmm_iiii, handler_name: "i0101_nnnn_mmmm_iiii", mask: MASK_N_M_IMM4, key: 0x5000, diss: "mov.l @(<disp4dw>,<REG_M>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0000, handler_name: "i0010_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x2000, diss: "mov.b <REG_M>,@<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0001, handler_name: "i0010_nnnn_mmmm_0001", mask: MASK_N_M, key: 0x2001, diss: "mov.w <REG_M>,@<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0010, handler_name: "i0010_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x2002, diss: "mov.l <REG_M>,@<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0000, handler_name: "i0110_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x6000, diss: "mov.b @<REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0001, handler_name: "i0110_nnnn_mmmm_0001", mask: MASK_N_M, key: 0x6001, diss: "mov.w @<REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0010, handler_name: "i0110_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x6002, diss: "mov.l @<REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0100, handler_name: "i0010_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x2004, diss: "mov.b <REG_M>,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0101, handler_name: "i0010_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x2005, diss: "mov.w <REG_M>,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0010_nnnn_mmmm_0110, handler_name: "i0010_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x2006, diss: "mov.l <REG_M>,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0100, handler_name: "i0110_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x6004, diss: "mov.b @<REG_M>+,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0101, handler_name: "i0110_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x6005, diss: "mov.w @<REG_M>+,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0110, handler_name: "i0110_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x6006, diss: "mov.l @<REG_M>+,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i1000_0000_mmmm_iiii, handler_name: "i1000_0000_mmmm_iiii", mask: MASK_IMM8, key: 0x8000, diss: "mov.b R0,@(<disp4b>,<REG_M>)" },
    sh4_opcodelistentry { oph: exec::i1000_0001_mmmm_iiii, handler_name: "i1000_0001_mmmm_iiii", mask: MASK_IMM8, key: 0x8100, diss: "mov.w R0,@(<disp4w>,<REG_M>)" },
    sh4_opcodelistentry { oph: exec::i1000_0100_mmmm_iiii, handler_name: "i1000_0100_mmmm_iiii", mask: MASK_IMM8, key: 0x8400, diss: "mov.b @(<disp4b>,<REG_M>),R0" },
    sh4_opcodelistentry { oph: exec::i1000_0101_mmmm_iiii, handler_name: "i1000_0101_mmmm_iiii", mask: MASK_IMM8, key: 0x8500, diss: "mov.w @(<disp4w>,<REG_M>),R0" },
    sh4_opcodelistentry { oph: exec::i1001_nnnn_iiii_iiii, handler_name: "i1001_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0x9000, diss: "mov.w @(<PCdisp8w>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i1100_0000_iiii_iiii, handler_name: "i1100_0000_iiii_iiii", mask: MASK_IMM8, key: 0xC000, diss: "mov.b R0,@(<disp8b>,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_0001_iiii_iiii, handler_name: "i1100_0001_iiii_iiii", mask: MASK_IMM8, key: 0xC100, diss: "mov.w R0,@(<disp8w>,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_0010_iiii_iiii, handler_name: "i1100_0010_iiii_iiii", mask: MASK_IMM8, key: 0xC200, diss: "mov.l R0,@(<disp8dw>,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_0100_iiii_iiii, handler_name: "i1100_0100_iiii_iiii", mask: MASK_IMM8, key: 0xC400, diss: "mov.b @(<GBRdisp8b>),R0" },
    sh4_opcodelistentry { oph: exec::i1100_0101_iiii_iiii, handler_name: "i1100_0101_iiii_iiii", mask: MASK_IMM8, key: 0xC500, diss: "mov.w @(<GBRdisp8w>),R0" },
    sh4_opcodelistentry { oph: exec::i1100_0110_iiii_iiii, handler_name: "i1100_0110_iiii_iiii", mask: MASK_IMM8, key: 0xC600, diss: "mov.l @(<GBRdisp8dw>),R0" },
    sh4_opcodelistentry { oph: exec::i1101_nnnn_iiii_iiii, handler_name: "i1101_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0xD000, diss: "mov.l @(<PCdisp8d>),<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0011, handler_name: "i0110_nnnn_mmmm_0011", mask: MASK_N_M, key: 0x6003, diss: "mov <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i1100_0111_iiii_iiii, handler_name: "i1100_0111_iiii_iiii", mask: MASK_IMM8, key: 0xC700, diss: "mova @(<PCdisp8d>),R0" },
    sh4_opcodelistentry { oph: exec::i1110_nnnn_iiii_iiii, handler_name: "i1110_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0xE000, diss: "mov #<simm8hex>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0101_0010, handler_name: "i0100_nnnn_0101_0010", mask: MASK_N, key: 0x4052, diss: "sts.l FPUL,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0110_0010, handler_name: "i0100_nnnn_0110_0010", mask: MASK_N, key: 0x4062, diss: "sts.l FPSCR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0010, handler_name: "i0100_nnnn_0000_0010", mask: MASK_N, key: 0x4002, diss: "sts.l MACH,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0010, handler_name: "i0100_nnnn_0001_0010", mask: MASK_N, key: 0x4012, diss: "sts.l MACL,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0010, handler_name: "i0100_nnnn_0010_0010", mask: MASK_N, key: 0x4022, diss: "sts.l PR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1111_0010, handler_name: "i0100_nnnn_1111_0010", mask: MASK_N, key: 0x40F2, diss: "stc.l DBR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_0010, handler_name: "i0100_nnnn_0011_0010", mask: MASK_N, key: 0x4032, diss: "stc.l SGR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0011, handler_name: "i0100_nnnn_0000_0011", mask: MASK_N, key: 0x4003, diss: "stc.l SR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0011, handler_name: "i0100_nnnn_0001_0011", mask: MASK_N, key: 0x4013, diss: "stc.l GBR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0011, handler_name: "i0100_nnnn_0010_0011", mask: MASK_N, key: 0x4023, diss: "stc.l VBR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_0011, handler_name: "i0100_nnnn_0011_0011", mask: MASK_N, key: 0x4033, diss: "stc.l SSR,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0100_0011, handler_name: "i0100_nnnn_0100_0011", mask: MASK_N, key: 0x4043, diss: "stc.l SPC,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1mmm_0011, handler_name: "i0100_nnnn_1mmm_0011", mask: MASK_N_ML3BIT, key: 0x4083, diss: "stc <RM_BANK>,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0110, handler_name: "i0100_nnnn_0000_0110", mask: MASK_N, key: 0x4006, diss: "lds.l @<REG_N>+,MACH" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0110, handler_name: "i0100_nnnn_0001_0110", mask: MASK_N, key: 0x4016, diss: "lds.l @<REG_N>+,MACL" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0110, handler_name: "i0100_nnnn_0010_0110", mask: MASK_N, key: 0x4026, diss: "lds.l @<REG_N>+,PR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_0110, handler_name: "i0100_nnnn_0011_0110", mask: MASK_N, key: 0x4036, diss: "ldc.l @<REG_N>+,SGR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0101_0110, handler_name: "i0100_nnnn_0101_0110", mask: MASK_N, key: 0x4056, diss: "lds.l @<REG_N>+,FPUL" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0110_0110, handler_name: "i0100_nnnn_0110_0110", mask: MASK_N, key: 0x4066, diss: "lds.l @<REG_N>+,FPSCR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1111_0110, handler_name: "i0100_nnnn_1111_0110", mask: MASK_N, key: 0x40F6, diss: "ldc.l @<REG_N>+,DBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0111, handler_name: "i0100_nnnn_0000_0111", mask: MASK_N, key: 0x4007, diss: "ldc.l @<REG_N>+,SR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0111, handler_name: "i0100_nnnn_0001_0111", mask: MASK_N, key: 0x4017, diss: "ldc.l @<REG_N>+,GBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0111, handler_name: "i0100_nnnn_0010_0111", mask: MASK_N, key: 0x4027, diss: "ldc.l @<REG_N>+,VBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_0111, handler_name: "i0100_nnnn_0011_0111", mask: MASK_N, key: 0x4037, diss: "ldc.l @<REG_N>+,SSR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0100_0111, handler_name: "i0100_nnnn_0100_0111", mask: MASK_N, key: 0x4047, diss: "ldc.l @<REG_N>+,SPC" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1mmm_0111, handler_name: "i0100_nnnn_1mmm_0111", mask: MASK_N_ML3BIT, key: 0x4087, diss: "ldc.l @<REG_N>+,RM_BANK" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0000_0010, handler_name: "i0000_nnnn_0000_0010", mask: MASK_N, key: 0x0002, diss: "stc SR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0001_0010, handler_name: "i0000_nnnn_0001_0010", mask: MASK_N, key: 0x0012, diss: "stc GBR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0010_0010, handler_name: "i0000_nnnn_0010_0010", mask: MASK_N, key: 0x0022, diss: "stc VBR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0011_0010, handler_name: "i0000_nnnn_0011_0010", mask: MASK_N, key: 0x0032, diss: "stc SSR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0100_0010, handler_name: "i0000_nnnn_0100_0010", mask: MASK_N, key: 0x0042, diss: "stc SPC,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1mmm_0010, handler_name: "i0000_nnnn_1mmm_0010", mask: MASK_N_ML3BIT, key: 0x0082, diss: "stc RM_BANK,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0000_1010, handler_name: "i0000_nnnn_0000_1010", mask: MASK_N, key: 0x000A, diss: "sts MACH,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0001_1010, handler_name: "i0000_nnnn_0001_1010", mask: MASK_N, key: 0x001A, diss: "sts MACL,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0010_1010, handler_name: "i0000_nnnn_0010_1010", mask: MASK_N, key: 0x002A, diss: "sts PR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0011_1010, handler_name: "i0000_nnnn_0011_1010", mask: MASK_N, key: 0x003A, diss: "sts SGR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0101_1010, handler_name: "i0000_nnnn_0101_1010", mask: MASK_N, key: 0x005A, diss: "sts FPUL,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_0110_1010, handler_name: "i0000_nnnn_0110_1010", mask: MASK_N, key: 0x006A, diss: "sts FPSCR,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0000_nnnn_1111_1010, handler_name: "i0000_nnnn_1111_1010", mask: MASK_N, key: 0x00FA, diss: "sts DBR,<REG_N>" },

    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_1010, handler_name: "i0100_nnnn_0000_1010", mask: MASK_N, key: 0x400A, diss: "lds <REG_N>,MACH" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_1010, handler_name: "i0100_nnnn_0001_1010", mask: MASK_N, key: 0x401A, diss: "lds <REG_N>,MACL" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_1010, handler_name: "i0100_nnnn_0010_1010", mask: MASK_N, key: 0x402A, diss: "lds <REG_N>,PR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_1010, handler_name: "i0100_nnnn_0011_1010", mask: MASK_N, key: 0x403A, diss: "ldc <REG_N>,SGR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0101_1010, handler_name: "i0100_nnnn_0101_1010", mask: MASK_N, key: 0x405A, diss: "lds <REG_N>,FPUL" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0110_1010, handler_name: "i0100_nnnn_0110_1010", mask: MASK_N, key: 0x406A, diss: "lds <REG_N>,FPSCR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1111_1010, handler_name: "i0100_nnnn_1111_1010", mask: MASK_N, key: 0x40FA, diss: "ldc <REG_N>,DBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_1110, handler_name: "i0100_nnnn_0000_1110", mask: MASK_N, key: 0x400E, diss: "ldc <REG_N>,SR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_1110, handler_name: "i0100_nnnn_0001_1110", mask: MASK_N, key: 0x401E, diss: "ldc <REG_N>,GBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_1110, handler_name: "i0100_nnnn_0010_1110", mask: MASK_N, key: 0x402E, diss: "ldc <REG_N>,VBR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0011_1110, handler_name: "i0100_nnnn_0011_1110", mask: MASK_N, key: 0x403E, diss: "ldc <REG_N>,SSR" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0100_1110, handler_name: "i0100_nnnn_0100_1110", mask: MASK_N, key: 0x404E, diss: "ldc <REG_N>,SPC" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_1mmm_1110, handler_name: "i0100_nnnn_1mmm_1110", mask: MASK_N_ML3BIT, key: 0x408E, diss: "ldc <REG_N>,<RM_BANK>" },

    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0000, handler_name: "i0100_nnnn_0000_0000", mask: MASK_N, key: 0x4000, diss: "shll <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0000, handler_name: "i0100_nnnn_0001_0000", mask: MASK_N, key: 0x4010, diss: "dt <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0000, handler_name: "i0100_nnnn_0010_0000", mask: MASK_N, key: 0x4020, diss: "shal <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0001, handler_name: "i0100_nnnn_0000_0001", mask: MASK_N, key: 0x4001, diss: "shlr <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0001, handler_name: "i0100_nnnn_0001_0001", mask: MASK_N, key: 0x4011, diss: "cmp/pz <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0001, handler_name: "i0100_nnnn_0010_0001", mask: MASK_N, key: 0x4021, diss: "shar <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0100, handler_name: "i0100_nnnn_0010_0100", mask: MASK_N, key: 0x4024, diss: "rotcl <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0100, handler_name: "i0100_nnnn_0000_0100", mask: MASK_N, key: 0x4004, diss: "rotl <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_0101, handler_name: "i0100_nnnn_0001_0101", mask: MASK_N, key: 0x4015, diss: "cmp/pl <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_0101, handler_name: "i0100_nnnn_0010_0101", mask: MASK_N, key: 0x4025, diss: "rotcr <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_0101, handler_name: "i0100_nnnn_0000_0101", mask: MASK_N, key: 0x4005, diss: "rotr <REG_N>" },

    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_1000, handler_name: "i0100_nnnn_0000_1000", mask: MASK_N, key: 0x4008, diss: "shll2 <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_1000, handler_name: "i0100_nnnn_0001_1000", mask: MASK_N, key: 0x4018, diss: "shll8 <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_1000, handler_name: "i0100_nnnn_0010_1000", mask: MASK_N, key: 0x4028, diss: "shll16 <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_1001, handler_name: "i0100_nnnn_0000_1001", mask: MASK_N, key: 0x4009, diss: "shlr2 <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_1001, handler_name: "i0100_nnnn_0001_1001", mask: MASK_N, key: 0x4019, diss: "shlr8 <REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_1001, handler_name: "i0100_nnnn_0010_1001", mask: MASK_N, key: 0x4029, diss: "shlr16 <REG_N>" },

    sh4_opcodelistentry { oph: exec::i0100_nnnn_0010_1011, handler_name: "i0100_nnnn_0010_1011", mask: MASK_N, key: 0x402B, diss: "jmp @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0000_1011, handler_name: "i0100_nnnn_0000_1011", mask: MASK_N, key: 0x400B, diss: "jsr @<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_0001_1011, handler_name: "i0100_nnnn_0001_1011", mask: MASK_N, key: 0x401B, diss: "tas.b @<REG_N>" },

    sh4_opcodelistentry { oph: exec::i0100_nnnn_mmmm_1100, handler_name: "i0100_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x400C, diss: "shad <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_mmmm_1101, handler_name: "i0100_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x400D, diss: "shld <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0100_nnnn_mmmm_1111, handler_name: "i0100_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x400F, diss: "mac.w @<REG_M>+,@<REG_N>+" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_0111, handler_name: "i0110_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x6007, diss: "not <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1000, handler_name: "i0110_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x6008, diss: "swap.b <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1001, handler_name: "i0110_nnnn_mmmm_1001", mask: MASK_N_M, key: 0x6009, diss: "swap.w <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1010, handler_name: "i0110_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x600A, diss: "negc <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1011, handler_name: "i0110_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x600B, diss: "neg <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1100, handler_name: "i0110_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x600C, diss: "extu.b <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1101, handler_name: "i0110_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x600D, diss: "extu.w <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1110, handler_name: "i0110_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x600E, diss: "exts.b <REG_M>,<REG_N>" },
    sh4_opcodelistentry { oph: exec::i0110_nnnn_mmmm_1111, handler_name: "i0110_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x600F, diss: "exts.w <REG_M>,<REG_N>" },

    sh4_opcodelistentry { oph: exec::i0111_nnnn_iiii_iiii, handler_name: "i0111_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0x7000, diss: "add #<simm8>,<REG_N>" },

    sh4_opcodelistentry { oph: exec::i1000_1011_iiii_iiii, handler_name: "i1000_1011_iiii_iiii", mask: MASK_IMM8, key: 0x8B00, diss: "bf <bdisp8>" },
    sh4_opcodelistentry { oph: exec::i1000_1111_iiii_iiii, handler_name: "i1000_1111_iiii_iiii", mask: MASK_IMM8, key: 0x8F00, diss: "bf/s <bdisp8>" },
    sh4_opcodelistentry { oph: exec::i1000_1001_iiii_iiii, handler_name: "i1000_1001_iiii_iiii", mask: MASK_IMM8, key: 0x8900, diss: "bt <bdisp8>" },
    sh4_opcodelistentry { oph: exec::i1000_1101_iiii_iiii, handler_name: "i1000_1101_iiii_iiii", mask: MASK_IMM8, key: 0x8D00, diss: "bt/s <bdisp8>" },

    sh4_opcodelistentry { oph: exec::i1000_1000_iiii_iiii, handler_name: "i1000_1000_iiii_iiii", mask: MASK_IMM8, key: 0x8800, diss: "cmp/eq #<simm8hex>,R0" },

    sh4_opcodelistentry { oph: exec::i1010_iiii_iiii_iiii, handler_name: "i1010_iiii_iiii_iiii", mask: MASK_N_IMM8, key: 0xA000, diss: "bra <bdisp12>" },
    sh4_opcodelistentry { oph: exec::i1011_iiii_iiii_iiii, handler_name: "i1011_iiii_iiii_iiii", mask: MASK_N_IMM8, key: 0xB000, diss: "bsr <bdisp12>" },

    sh4_opcodelistentry { oph: exec::i1100_0011_iiii_iiii, handler_name: "i1100_0011_iiii_iiii", mask: MASK_IMM8, key: 0xC300, diss: "trapa #<imm8>" },

    sh4_opcodelistentry { oph: exec::i1100_1000_iiii_iiii, handler_name: "i1100_1000_iiii_iiii", mask: MASK_IMM8, key: 0xC800, diss: "tst #<imm8>,R0" },
    sh4_opcodelistentry { oph: exec::i1100_1001_iiii_iiii, handler_name: "i1100_1001_iiii_iiii", mask: MASK_IMM8, key: 0xC900, diss: "and #<imm8>,R0" },
    sh4_opcodelistentry { oph: exec::i1100_1010_iiii_iiii, handler_name: "i1100_1010_iiii_iiii", mask: MASK_IMM8, key: 0xCA00, diss: "xor #<imm8>,R0" },
    sh4_opcodelistentry { oph: exec::i1100_1011_iiii_iiii, handler_name: "i1100_1011_iiii_iiii", mask: MASK_IMM8, key: 0xCB00, diss: "or #<imm8>,R0" },

    sh4_opcodelistentry { oph: exec::i1100_1100_iiii_iiii, handler_name: "i1100_1100_iiii_iiii", mask: MASK_IMM8, key: 0xCC00, diss: "tst.b #<imm8>,@(R0,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_1101_iiii_iiii, handler_name: "i1100_1101_iiii_iiii", mask: MASK_IMM8, key: 0xCD00, diss: "and.b #<imm8>,@(R0,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_1110_iiii_iiii, handler_name: "i1100_1110_iiii_iiii", mask: MASK_IMM8, key: 0xCE00, diss: "xor.b #<imm8>,@(R0,GBR)" },
    sh4_opcodelistentry { oph: exec::i1100_1111_iiii_iiii, handler_name: "i1100_1111_iiii_iiii", mask: MASK_IMM8, key: 0xCF00, diss: "or.b #<imm8>,@(R0,GBR)" },

    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0000, handler_name: "i1111_nnnn_mmmm_0000", mask: MASK_N_M, key: 0xF000, diss: "fadd <FREG_M_SD_F>,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0001, handler_name: "i1111_nnnn_mmmm_0001", mask: MASK_N_M, key: 0xF001, diss: "fsub <FREG_M_SD_F>,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0010, handler_name: "i1111_nnnn_mmmm_0010", mask: MASK_N_M, key: 0xF002, diss: "fmul <FREG_M_SD_F>,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0011, handler_name: "i1111_nnnn_mmmm_0011", mask: MASK_N_M, key: 0xF003, diss: "fdiv <FREG_M_SD_F>,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0100, handler_name: "i1111_nnnn_mmmm_0100", mask: MASK_N_M, key: 0xF004, diss: "fcmp/eq <FREG_M_SD_F>,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0101, handler_name: "i1111_nnnn_mmmm_0101", mask: MASK_N_M, key: 0xF005, diss: "fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>" },

    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0110, handler_name: "i1111_nnnn_mmmm_0110", mask: MASK_N_M, key: 0xF006, diss: "fmov.s @(R0,<REG_M>),<FREG_N_SD_A>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_0111, handler_name: "i1111_nnnn_mmmm_0111", mask: MASK_N_M, key: 0xF007, diss: "fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1000, handler_name: "i1111_nnnn_mmmm_1000", mask: MASK_N_M, key: 0xF008, diss: "fmov.s @<REG_M>,<FREG_N_SD_A>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1001, handler_name: "i1111_nnnn_mmmm_1001", mask: MASK_N_M, key: 0xF009, diss: "fmov.s @<REG_M>+,<FREG_N_SD_A>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1010, handler_name: "i1111_nnnn_mmmm_1010", mask: MASK_N_M, key: 0xF00A, diss: "fmov.s <FREG_M_SD_A>,@<REG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1011, handler_name: "i1111_nnnn_mmmm_1011", mask: MASK_N_M, key: 0xF00B, diss: "fmov.s <FREG_M_SD_A>,@-<REG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1100, handler_name: "i1111_nnnn_mmmm_1100", mask: MASK_N_M, key: 0xF00C, diss: "fmov <FREG_M_SD_A>,<FREG_N_SD_A>" },

    sh4_opcodelistentry { oph: exec::i1111_nnnn_0101_1101, handler_name: "i1111_nnnn_0101_1101", mask: MASK_N, key: 0xF05D, diss: "fabs <FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnn0_1111_1101, handler_name: "i1111_nnn0_1111_1101", mask: MASK_NH3BIT, key: 0xF0FD, diss: "fsca FPUL,<DR_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_1011_1101, handler_name: "i1111_nnnn_1011_1101", mask: MASK_N, key: 0xF0BD, diss: "fcnvds <DR_N>,FPUL" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_1010_1101, handler_name: "i1111_nnnn_1010_1101", mask: MASK_N, key: 0xF0AD, diss: "fcnvsd FPUL,<DR_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnmm_1110_1101, handler_name: "i1111_nnmm_1110_1101", mask: MASK_N, key: 0xF0ED, diss: "fipr <FV_M>,<FV_N>" },

    sh4_opcodelistentry { oph: exec::i1111_nnnn_1000_1101, handler_name: "i1111_nnnn_1000_1101", mask: MASK_N, key: 0xF08D, diss: "fldi0 <FREG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_1001_1101, handler_name: "i1111_nnnn_1001_1101", mask: MASK_N, key: 0xF09D, diss: "fldi1 <FREG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0001_1101, handler_name: "i1111_nnnn_0001_1101", mask: MASK_N, key: 0xF01D, diss: "flds <FREG_N>,FPUL" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0010_1101, handler_name: "i1111_nnnn_0010_1101", mask: MASK_N, key: 0xF02D, diss: "float FPUL,<FREG_N_SD_F>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0100_1101, handler_name: "i1111_nnnn_0100_1101", mask: MASK_N, key: 0xF04D, diss: "fneg <FREG_N_SD_F>" },

    sh4_opcodelistentry { oph: exec::i1111_1011_1111_1101, handler_name: "i1111_1011_1111_1101", mask: MASK_NONE, key: 0xFBFD, diss: "frchg" },
    sh4_opcodelistentry { oph: exec::i1111_0011_1111_1101, handler_name: "i1111_0011_1111_1101", mask: MASK_NONE, key: 0xF3FD, diss: "fschg" },

    sh4_opcodelistentry { oph: exec::i1111_nnnn_0110_1101, handler_name: "i1111_nnnn_0110_1101", mask: MASK_N, key: 0xF06D, diss: "fsqrt <FREG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0011_1101, handler_name: "i1111_nnnn_0011_1101", mask: MASK_N, key: 0xF03D, diss: "ftrc <FREG_N>,FPUL" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0000_1101, handler_name: "i1111_nnnn_0000_1101", mask: MASK_N, key: 0xF00D, diss: "fsts FPUL,<FREG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nn01_1111_1101, handler_name: "i1111_nn01_1111_1101", mask: MASK_NH2BIT, key: 0xF1FD, diss: "ftrv xmtrx,<FV_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_mmmm_1110, handler_name: "i1111_nnnn_mmmm_1110", mask: MASK_N_M, key: 0xF00E, diss: "fmac <FREG_0>,<FREG_M>,<FREG_N>" },
    sh4_opcodelistentry { oph: exec::i1111_nnnn_0111_1101, handler_name: "i1111_nnnn_0111_1101", mask: MASK_N, key: 0xF07D, diss: "fsrra <FREG_N>" },

    sh4_opcodelistentry { oph: i_not_known, handler_name: "unknown_opcode", mask: MASK_NONE, key: 0, diss: "unknown opcode" },
];

// Executes the operation using raw pointers.
// Safe API; unsafety is contained inside.
pub mod backend_exec {
    #[inline(always)]
    pub fn sh4_muls32(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        unsafe {
            let a = (*src_n) as i32;
            let b = (*src_m) as i32;
            *dst = a.wrapping_mul(b) as u32;
        }
    }

    #[inline(always)]
    pub fn sh4_store32(dst: *mut u32, src: *const u32) {
        unsafe {
            *dst = *src;
        }
    }

    #[inline(always)]
    pub fn sh4_store32i(dst: *mut u32, imm: u32) {
        unsafe {
            *dst = imm;
        }
    }

    #[inline(always)]
    pub fn sh4_and(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        unsafe {
            *dst = *src_n & *src_m;
        }
    }

    #[inline(always)]
    pub fn sh4_xor(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        unsafe {
            *dst = *src_n ^ *src_m;
        }
    }
    
    #[inline(always)]
    pub fn sh4_sub(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        unsafe {
            *dst = (*src_n).wrapping_sub(*src_m);
        }
    }

    #[inline(always)]
    pub fn sh4_add(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        unsafe {
            *dst = (*src_n).wrapping_add(*src_m);
        }
    }

    #[inline(always)]
    pub fn sh4_addi(dst: *mut u32, src_n: *const u32, imm: u32) {
        unsafe {
            *dst = (*src_n).wrapping_add(imm);
        }
    }

    #[inline(always)]
    pub fn sh4_neg(dst: *mut u32, src_n: *const u32) {
        unsafe {
            *dst = (*src_n).wrapping_neg();
        }
    }

    #[inline(always)]
    pub fn sh4_extub(dst: *mut u32, src: *const u32) {
        unsafe {
            *dst = *src as u8 as u32;
        }
    }

    #[inline(always)]
    pub fn sh4_dt(sr_T: *mut u32, dst: *mut u32) {
        unsafe {
            *dst = (*dst).wrapping_sub(1);
            *sr_T = if *dst == 0 { 1 } else { 0 };
        }
    }

    #[inline(always)]
    pub fn sh4_shlr(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
        unsafe {
            *sr_T = *src_n & 1; 
            *dst = *src_n >> 1;
        }
    }

    #[inline(always)]
    pub fn sh4_shllf(dst: *mut u32, src_n: *const u32, amt: u32) {
        unsafe {
            *dst = *src_n << amt;
        }
    }

    #[inline(always)]
    pub fn sh4_shlrf(dst: *mut u32, src_n: *const u32, amt: u32) {
        unsafe {
            *dst = *src_n >> amt;
        }
    }

    #[inline(always)]
    pub fn sh4_write_mem8(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        unsafe {
            let _ = super::write_mem::<u8>(&mut *dc, *addr, *data as u8);
        }
    }

    #[inline(always)]
    pub fn sh4_write_mem16(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        unsafe {
            let _ = super::write_mem::<u16>(&mut *dc, *addr, *data as u16);
        }
    }

    #[inline(always)]
    pub fn sh4_write_mem32(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        unsafe {
            let _ = super::write_mem::<u32>(&mut *dc, *addr, *data);
        }
    }

    #[inline(always)]
    pub fn sh4_write_mem64(dc: *mut super::Dreamcast, addr: *const u32, data: *const u64) {
        unsafe {
            let _ = super::write_mem::<u64>(&mut *dc, *addr, *data);
        }
    }

    #[inline(always)]
    pub fn sh4_read_mems8(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        unsafe {
            let mut read: i8 = 0;
            let _ = super::read_mem::<i8>(&mut *dc, *addr, &mut read);
            *data = read as i32 as u32;
        }
    }

    #[inline(always)]
    pub fn sh4_read_mems16(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        unsafe {
            let mut read: i16 = 0;
            let _ = super::read_mem::<i16>(&mut *dc, *addr, &mut read);
            *data = read as i32 as u32;
        }
    }

    #[inline(always)]
    pub fn sh4_read_mem32(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        unsafe {
            let _ = super::read_mem::<u32>(&mut *dc, *addr, &mut *data);
        }
    }

    #[inline(always)]
    pub fn sh4_read_mem64(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u64) {
        unsafe {
            let _ = super::read_mem::<u64>(&mut *dc, *addr, &mut *data);
        }
    }


    #[inline(always)]
    pub fn sh4_read_mem32i(dc: *mut super::Dreamcast, addr: u32, data: *mut u32) {
        unsafe {
            let _ = super::read_mem::<u32>(&mut *dc, addr, &mut *data);
        }
    }

    #[inline(always)]
    pub fn sh4_fadd(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        unsafe {
            *dst = *src_n + *src_m;
        }
    }

    #[inline(always)]
    pub fn sh4_fmul(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        unsafe {
            *dst = *src_n * *src_m;
        }
    }
    
    #[inline(always)]
    pub fn sh4_fdiv(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        unsafe {
            *dst = *src_n / *src_m;
        }
    }

    #[inline(always)]
    pub fn sh4_fsca(dst: *mut f32, index: *const u32) {
        unsafe {
            let pi_index = *index & 0xFFFF;
            // rads = (index / (65536/2)) * pi
            let rads = (pi_index as f32) / (65536.0f32 / 2.0f32) * std::f32::consts::PI;

            *dst.add(0) = rads.sin();
            *dst.add(1) = rads.cos();
        }
    }

    #[inline(always)]
    pub fn sh4_float(dst: *mut f32, src: *const u32) {
        unsafe {
            *dst = *src as i32 as f32;
        }
    }

    #[inline(always)]
    pub fn sh4_ftrc(dst: *mut u32, src: *const f32) {
        unsafe {
            let clamped = (*src).min(0x7FFFFFBF as f32);
            let mut as_i = clamped as i32 as u32;
            if as_i == 0x80000000 {
                if (*src) > 0.0 {
                    as_i = as_i.wrapping_sub(1);
                }
            }
            *dst = as_i;
        }
    }

    #[inline(always)]
    pub fn sh4_branch_cond(dc: *mut super::Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
        unsafe {
            if *T == condition {
                (*dc).ctx.pc1 = target;
                (*dc).ctx.pc2 = target.wrapping_add(2);
            } else {
                // these are calcualted by the pipeline logic in the main loop, no need to do it here
                // but it is done anyway for validation purposes
                (*dc).ctx.pc1 = next;
                (*dc).ctx.pc2 = next.wrapping_add(2);
            }
        }
    }

    #[inline(always)]
    pub fn sh4_branch_cond_delay(dc: *mut super::Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
        unsafe {
            if *T == condition {
                (*dc).ctx.pc2 = target;
            } else {
                // this is calcualted by the pipeline logic in the main loop, no need to do it here
                // but it is done anyway for validation purposes
                (*dc).ctx.pc2 = next;
            }
            (*dc).ctx.is_delayslot1 = 1;
        }
    }

    #[inline(always)]
    pub fn sh4_branch_delay(dc: *mut super::Dreamcast, target: u32) {
        unsafe {
            (*dc).ctx.pc2 = target;
            (*dc).ctx.is_delayslot1 = 1;
        }
    }
}

// Decoder/recording backend: stores stable pointers to records (no pc/a/b).
pub mod backend_dec {
    use std::{cell::RefCell, ptr::NonNull};

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct MulRec {
        pub dst:   *mut u32,
        pub src_n: *const u32,
        pub src_m: *const u32,
    }

    thread_local! {
        // Owns storage so pointers remain valid until `clear`.
        static ARENA: RefCell<Vec<Box<MulRec>>> = RefCell::new(Vec::with_capacity(1 << 16));
        // Compact list of stable pointers into ARENA.
        static PTRS:  RefCell<Vec<NonNull<MulRec>>> = RefCell::new(Vec::with_capacity(1 << 16));
    }

    #[inline(always)]
    pub fn sh4_muls32(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        ARENA.with(|arena| PTRS.with(|ptrs| {
            let mut arena = arena.borrow_mut();
            let mut ptrs  = ptrs.borrow_mut();

            let mut rec = Box::new(MulRec { dst, src_n, src_m });
            let nn = NonNull::from(rec.as_mut());

            arena.push(rec);
            ptrs.push(nn);
        }));
    }

    #[inline(always)]
    pub fn sh4_store32(dst: *mut u32, src: *const u32) {
        panic!("sh4_store32 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_store32i(dst: *mut u32, imm: u32) {
        panic!("sh4_store32i is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_and(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        panic!("sh4_and is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_xor(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        panic!("sh4_xor is not implemented in backend_dec");
    }
    
    #[inline(always)]
    pub fn sh4_sub(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        panic!("sh4_sub is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_add(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
        panic!("sh4_add is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_addi(dst: *mut u32, src_n: *const u32, imm: u32) {
        panic!("sh4_addi is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_neg(dst: *mut u32, src_n: *const u32) {
        panic!("sh4_neg is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_extub(dst: *mut u32, src: *const u32) {
        panic!("sh4_extub is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_dt(sr_T: *mut u32, dst: *mut u32) {
        panic!("sh4_dt is not implemented in backend_dec");
    }
    
    #[inline(always)]
    pub fn sh4_shlr(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
        panic!("sh4_shlr is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_shllf(dst: *mut u32, src_n: *const u32, amt: u32) {
        panic!("sh4_shllf is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_shlrf(dst: *mut u32, src_n: *const u32, amt: u32) {
        panic!("sh4_shlrf is not implemented in backend_dec");
    }

    
    #[inline(always)]
    pub fn sh4_write_mem8(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        panic!("sh4_write_mem8 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_write_mem16(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        panic!("sh4_write_mem16 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_write_mem32(dc: *mut super::Dreamcast, addr: *const u32, data: *const u32) {
        panic!("sh4_write_mem32 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_write_mem64(dc: *mut super::Dreamcast, addr: *const u32, data: *const u64) {
        panic!("sh4_write_mem64 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_read_mems8(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        panic!("sh4_read_mems8 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_read_mems16(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        panic!("sh4_read_mems16 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_read_mem32(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u32) {
        panic!("sh4_read_mems32 is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_read_mem64(dc: *mut super::Dreamcast, addr: *const u32, data: *mut u64) {
        panic!("sh4_read_mems64 is not implemented in backend_dec");
    }

    pub fn sh4_read_mem32i(dc: *mut super::Dreamcast, addr: u32, data: *mut u32) {
        panic!("sh4_read_mem32i is not implemented in backend_dec");
    }
    
    #[inline(always)]
    pub fn sh4_fadd(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        panic!("sh4_fadd is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_fmul(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        panic!("sh4_fmul is not implemented in backend_dec");
    }
    
    #[inline(always)]
    pub fn sh4_fdiv(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
        panic!("sh4_fdiv is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_fsca(dst: *mut f32, index: *const u32) {
        panic!("sh4_fsca is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_float(dst: *mut f32, src: *const u32) {
        panic!("sh4_float is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_ftrc(dst: *mut u32, src: *const f32) {
        panic!("sh4_ftrc is not implemented in backend_dec");
    }

    
    #[inline(always)]
    pub fn sh4_branch_cond(dc: *mut super::Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
        panic!("sh4_branch_cond is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_branch_cond_delay(dc: *mut super::Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
        panic!("sh4_branch_cond_delay is not implemented in backend_dec");
    }

    #[inline(always)]
    pub fn sh4_branch_delay(dc: *mut super::Dreamcast, target: u32) {
        panic!("sh4_branch_delay is not implemented in backend_dec");
    }

    #[inline]
    pub fn ptrs_snapshot() -> Vec<NonNull<MulRec>> {
        PTRS.with(|p| p.borrow().iter().copied().collect())
    }

    #[inline]
    pub fn clear() {
        ARENA.with(|a| a.borrow_mut().clear());
        PTRS.with(|p| p.borrow_mut().clear());
    }
}

fn diff_opcodes(new: &[sh4_opcodelistentry], old: &[sh4_opcodelistentry]) {
    let mut new_idx: Vec<usize> = (0..new.len()).collect();
    let mut old_idx: Vec<usize> = (0..old.len()).collect();

    new_idx.sort_by_key(|&i| new[i].key);
    old_idx.sort_by_key(|&i| old[i].key);

    let (mut i, mut j) = (0, 0);

    while i < new_idx.len() && j < old_idx.len() {
        let a = &new[new_idx[i]];
        let b = &old[old_idx[j]];
        match a.key.cmp(&b.key) {
            std::cmp::Ordering::Equal => {
                if a.mask != b.mask || a.diss != b.diss || a.handler_name != b.handler_name {
                    println!(
                        "Key {:04x} differs:\n  NEW: {:<20} mask={:04x} diss={}\n  OLD: {:<20} mask={:04x} diss={}",
                        a.key, a.handler_name, a.mask, a.diss,
                        b.handler_name, b.mask, b.diss
                    );
                }
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => {
                println!(
                    "Key {:04x} present in NEW only: {:<20} mask={:04x} diss={}",
                    a.key, a.handler_name, a.mask, a.diss
                );
                i += 1;
            }
            std::cmp::Ordering::Greater => {
                println!(
                    "Key {:04x} present in OLD only: {:<20} mask={:04x} diss={}",
                    b.key, b.handler_name, b.mask, b.diss
                );
                j += 1;
            }
        }
    }

    while i < new_idx.len() {
        let a = &new[new_idx[i]];
        println!(
            "Key {:04x} present in NEW only: {:<20} mask={:04x} diss={}",
            a.key, a.handler_name, a.mask, a.diss
        );
        i += 1;
    }
    while j < old_idx.len() {
        let b = &old[old_idx[j]];
        println!(
            "Key {:04x} present in OLD only: {:<20} mask={:04x} diss={}",
            b.key, b.handler_name, b.mask, b.diss
        );
        j += 1;
    }
}


pub static ROTO_BIN: &[u8] = include_bytes!("../roto.bin");


pub fn init_dreamcast(dc: &mut Dreamcast) {
    // Zero entire struct (like memset). In Rust, usually you'd implement Default.
    *dc = Dreamcast::default();

    // Build opcode tables
    // build_opcode_tables(dc);

    // Setup memory map
    dc.memmap[0x0C] = dc.sys_ram.as_mut_ptr();
    dc.memmask[0x0C] = SYSRAM_MASK;
    dc.memmap[0x8C] = dc.sys_ram.as_mut_ptr();
    dc.memmask[0x8C] = SYSRAM_MASK;
    dc.memmap[0xA5] = dc.video_ram.as_mut_ptr();
    dc.memmask[0xA5] = VIDEORAM_MASK;

    // Set initial PC
    dc.ctx.pc0 = 0x8C01_0000;
    dc.ctx.pc1 = 0x8C01_0000 + 2;
    dc.ctx.pc2 = 0x8C01_0000 + 4;

    // Copy roto.bin from embedded ROTO_BIN
    let sysram_slice = &mut dc.sys_ram[0x10000..0x10000 + ROTO_BIN.len()];
    sysram_slice.copy_from_slice(ROTO_BIN);
}


pub fn run_dreamcast(dc: &mut Dreamcast) {
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
