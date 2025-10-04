// src/sh4dec.rs

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ptr::{addr_of, addr_of_mut};

// Pull in the machine types and backends from the parent module.
use crate::dreamcast::Dreamcast;

#[derive(Copy, Clone)]
pub struct sh4_opcodelistentry {
    pub oph: fn(*mut Dreamcast, u16),
    pub dech: fn(*mut Dreamcast, u16),
    pub handler_name: &'static str,
    pub mask: u16,
    pub key: u16,
    pub diss: &'static str,
}

const fn parse_opcode(pattern: &str) -> (u16, u16) {
    let bytes = pattern.as_bytes();
    let mut i = 1; // skip leading 'i'
    let mut mask: u16 = 0;
    let mut key:  u16 = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'0' || c == b'1' {
            mask = (mask << 1) | 1;
            if c == b'1' { key = (key << 1) | 1; } else { key = key << 1; }
        } else if c != b'_' {
            // wildcard
            mask = mask << 1;
            key  = key  << 1;
        }
        i += 1;
    }
    (mask, key)
}

#[derive(Copy, Clone)]
pub struct SH4DecoderState {
    pub pc: u32,
    pub fpscr_PR: u32,
    pub fpscr_SZ: u32,
}

macro_rules! sh4op {
    (
        $( (disas = $diss:literal)
           $name:ident ( $($params:tt)* ) { $($body:tt)* }
        )*
    ) => {
        // Exec expansion
        pub(crate) mod exec {
            use super::*;
            $(
                sh4op!(@emit $name ( $($params)* ) { $($body)* }
                       ; backend = crate::dreamcast::sh4::backend_ipr);
            )*
        }
        // Decoder expansion
        pub(crate) mod dec {
            use super::*;
            $(
                sh4op!(@emit $name ( $($params)* ) { $($body)* }
                       ; backend = crate::dreamcast::sh4::backend_fns);
            )*
        }

        // Opcode descriptor table
        pub(crate) static OPCODES: &[sh4_opcodelistentry] = &[
            $(
                {
                    const MASK_KEY: (u16,u16) = parse_opcode(stringify!($name));
                    sh4_opcodelistentry {
                        oph:  exec::$name,
                        dech: dec::$name,
                        handler_name: stringify!($name),
                        mask: MASK_KEY.0,
                        key:  MASK_KEY.1,
                        diss: $diss,
                    }
                }
            ),*,
            sh4_opcodelistentry {
                oph: i_not_known, dech: i_not_known,
                handler_name: "unknown_opcode", mask: 0xFFFF, key: 0, diss: "unknown opcode"
            },
        ];
    };

    // --- (dc, instr) ---------------------------------------------------------------------------
    (@emit
        $name:ident ( $dc:ident , $instr:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: *mut Dreamcast, $instr: u16) {
            #[allow(unused_unsafe)]
            unsafe {
                #[allow(unused_imports)]
                use $backend as backend;
                { $($body)* }
            }
        }
    };

    // --- (dc, state, instr): inject a local state struct ---------------------------------------
    (@emit
        $name:ident ( $dc:ident , $state:ident , $instr:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: *mut Dreamcast, $instr: u16) {
            #[allow(unused_unsafe)]
            unsafe {
                #[allow(unused_imports)]
                use $backend as backend;

                let $state = SH4DecoderState {
                    pc: (*$dc).ctx.pc0,
                    fpscr_PR: (*$dc).ctx.fpscr_PR,
                    fpscr_SZ: (*$dc).ctx.fpscr_SZ,
                };

                { $($body)* }
            }
        }
    };
}


#[inline(always)] fn GetN(str_: u16) -> usize { ((str_ >> 8) & 0xF) as usize }
#[inline(always)] fn GetM(str_: u16) -> usize { ((str_ >> 4) & 0xF) as usize }
#[inline(always)] fn GetImm4(str_: u16) -> u32 { (str_ & 0xF) as u32 }
#[inline(always)] fn GetImm8(str_: u16) -> u32 { (str_ & 0xFF) as u32 }
#[inline(always)] fn GetSImm8(str_: u16) -> i32 { (str_ & 0xFF) as i8 as i32 }
#[inline(always)] fn GetImm12(str_: u16) -> u32 { (str_ & 0xFFF) as u32 }
#[inline(always)] fn GetSImm12(str_: u16) -> i32 { ((((GetImm12(str_) as u16) << 4) as i16) >> 4) as i32 }

#[inline(always)] fn data_target_s8(pc: u32, disp8: i32) -> u32 { ((pc.wrapping_add(4)) & 0xFFFF_FFFC).wrapping_add((disp8 << 2) as u32) }
#[inline(always)] fn branch_target_s8(pc: u32, disp8: i32) -> u32 { (disp8 as i64 * 2 + 4 + pc as i64) as u32 }
#[inline(always)] fn branch_target_s12(pc: u32, disp12: i32) -> u32 { (disp12 as i64 * 2 + 4 + pc as i64) as u32 }

fn i_not_implemented(dc: *mut Dreamcast, state: SH4DecoderState, instr: u16) {
    let desc_ptr: *const sh4_opcodelistentry = &SH4_OP_DESC[instr as usize];
    let diss = unsafe {
        if desc_ptr.is_null() {
            "missing"
        } else {
            let d = &*desc_ptr;
            if d.diss.is_empty() { "missing" } else { d.diss }
        }
    };
    panic!("{:08X}: {:04X} {} [i_not_implemented]", state.pc, instr, diss);
}

fn i_not_known(dc: *mut Dreamcast, instr: u16) {
    unsafe {
        let pc = (*dc).ctx.pc0;
        let desc_ptr = &SH4_OP_DESC[instr as usize];
        panic!("{:08X}: {:04X} {} [i_not_known]", pc, instr, desc_ptr.diss);
    }
}

sh4op! {
    (disas = "mul.l <REG_M>,<REG_N>")
    i0000_nnnn_mmmm_0111(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_muls32(addr_of_mut!((*dc).ctx.macl), addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "nop")
    i0000_0000_0000_1001(dc, instr) {
        // no-op
    }

    (disas = "sts FPUL,<REG_N>")
    i0000_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.fpul));
    }

    (disas = "sts MACL,<REG_N>")
    i0000_nnnn_0001_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.macl));
    }

    (disas = "mov.b <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem8(dc, addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "mov.w <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0001(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem16(dc, addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "mov.l <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0010(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_write_mem32(dc, addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "and <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1001(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_and(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "xor <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1010(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_xor(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "sub <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_sub(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "add <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_add(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "dt <REG_N>")
    i0100_nnnn_0001_0000(dc, instr) {
        let n = GetN(instr);
        backend::sh4_dt(addr_of_mut!((*dc).ctx.sr_T), addr_of_mut!((*dc).ctx.r[n]));
    }

    (disas = "shlr <REG_N>")
    i0100_nnnn_0000_0001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlr(addr_of_mut!((*dc).ctx.sr_T), addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]));
    }

    (disas = "shll8 <REG_N>")
    i0100_nnnn_0001_1000(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shllf(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), 8);
    }

    (disas = "shlr2 <REG_N>")
    i0100_nnnn_0000_1001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlrf(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), 2);
    }

    (disas = "shlr16 <REG_N>")
    i0100_nnnn_0010_1001(dc, instr) {
        let n = GetN(instr);
        backend::sh4_shlrf(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), 16);
    }

    (disas = "mov.b @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);

        backend::sh4_read_mems8(dc, addr_of!((*dc).ctx.r[m]), addr_of_mut!((*dc).ctx.r[n]));
    }

    (disas = "mov <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0011(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_store32(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "neg <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1011(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_neg(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "extu.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr);
        let m = GetM(instr);
        backend::sh4_extub(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[m]));
    }

    (disas = "add #<simm8>,<REG_N>")
    i0111_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr);
        let stmp1 = GetSImm8(instr);
        backend::sh4_addi(addr_of_mut!((*dc).ctx.r[n]), addr_of!((*dc).ctx.r[n]), stmp1 as u32);
    }

    (disas = "bf <bdisp8>")
    i1000_1011_iiii_iiii(dc, state, instr) {
        let disp8 = GetSImm8(instr);
        let next = state.pc.wrapping_add(2);
        let target = branch_target_s8(state.pc, disp8);
        backend::sh4_branch_cond(dc, addr_of!((*dc).ctx.sr_T), 0, next, target);
    }

    (disas = "bf/s <bdisp8>")
    i1000_1111_iiii_iiii(dc, state, instr) {
        let disp8 = GetSImm8(instr);
        let next = state.pc.wrapping_add(4);
        let target = branch_target_s8(state.pc, disp8);
        backend::sh4_branch_cond_delay(dc, addr_of!((*dc).ctx.sr_T), 0, next, target);
    }

    (disas = "bra <bdisp12>")
    i1010_iiii_iiii_iiii(dc, state, instr) {
        let disp12 = GetSImm12(instr);
        let target = branch_target_s12(state.pc, disp12);
        backend::sh4_branch_delay(dc, target);
    }

    (disas = "mova @(<PCdisp8d>),R0")
    i1100_0111_iiii_iiii(dc, state, instr) {
        let disp8 = GetImm8(instr) as i32;
        let addr = data_target_s8(state.pc, disp8);
        backend::sh4_store32i(addr_of_mut!((*dc).ctx.r[0]), addr);
    }
    (disas = "mov.b R0,@(<disp4b>,<REG_M>)")
    i1000_0000_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w R0,@(<disp4w>,<REG_M>)")
    i1000_0001_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b @(<disp4b>,<REG_M>),R0")
    i1000_0100_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w @(<disp4w>,<REG_M>),R0")
    i1000_0101_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/eq #<simm8hex>,R0")
    i1000_1000_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }


    (disas = "mov.l @(<PCdisp8d>),<REG_N>")
    i1101_nnnn_iiii_iiii(dc, state, instr) {
        let n = GetN(instr);
        let disp8 = GetImm8(instr) as i32;
        let addr = data_target_s8(state.pc, disp8);

        backend::sh4_read_mem32i(dc, addr, addr_of_mut!((*dc).ctx.r[n]));
    }

    (disas = "mov #<simm8hex>,<REG_N>")
    i1110_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr);
        let imm = GetSImm8(instr);
        backend::sh4_store32i(addr_of_mut!((*dc).ctx.r[n]), imm as u32);
    }

    (disas = "fadd <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0000(dc, instr) {
        if (*dc).ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fadd(addr_of_mut!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fsub <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmul <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0010(dc, instr) {
        if (*dc).ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fmul(addr_of_mut!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fdiv <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0011(dc, instr) {
        if (*dc).ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_fdiv(addr_of_mut!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fr.f32s[m])); };
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fmov.s @<REG_M>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1000(dc, instr) {
        if (*dc).ctx.fpscr_SZ == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_read_mem32(dc, addr_of!((*dc).ctx.r[m]), addr_of_mut!((*dc).ctx.fr.u32s[n])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fmov <FREG_M_SD_A>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1100(dc, instr) {
        if (*dc).ctx.fpscr_SZ == 0 {
            let n = GetN(instr);
            let m = GetM(instr);
            unsafe { backend::sh4_store32(addr_of_mut!((*dc).ctx.fr.u32s[n]), addr_of!((*dc).ctx.fr.u32s[m])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "fsca FPUL,<DR_N>")
    i1111_nnn0_1111_1101(dc, instr) {
        let n = (GetN(instr) & 0xE) as usize;
        if (*dc).ctx.fpscr_PR == 0 {
            unsafe { backend::sh4_fsca(addr_of_mut!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fpul)); }
            
        } else {
            debug_assert!(false);
        }
    }

    (disas = "float FPUL,<FREG_N_SD_F>")
    i1111_nnnn_0010_1101(dc, instr) {
        if (*dc).ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            unsafe { backend::sh4_float(addr_of_mut!((*dc).ctx.fr.f32s[n]), addr_of!((*dc).ctx.fpul)); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "ftrc <FREG_N>,FPUL")
    i1111_nnnn_0011_1101(dc, instr) {
        if (*dc).ctx.fpscr_PR == 0 {
            let n = GetN(instr);
            unsafe { backend::sh4_ftrc(addr_of_mut!((*dc).ctx.fpul), addr_of!((*dc).ctx.fr.f32s[n])); }
        } else {
            debug_assert!(false);
        }
    }

    (disas = "lds <REG_N>,FPUL")
    i0100_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr);
        backend::sh4_store32(addr_of_mut!((*dc).ctx.fpul), addr_of!((*dc).ctx.r[n]));
    }

    (disas = "stc SR,<REG_N>")
    i0000_nnnn_0000_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc GBR,<REG_N>")
    i0000_nnnn_0001_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc VBR,<REG_N>")
    i0000_nnnn_0010_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc SSR,<REG_N>")
    i0000_nnnn_0011_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts SGR,<REG_N>")
    i0000_nnnn_0011_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc SPC,<REG_N>")
    i0000_nnnn_0100_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc RM_BANK,<REG_N>")
    i0000_nnnn_1mmm_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "braf <REG_N>")
    i0000_nnnn_0010_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "bsrf <REG_N>")
    i0000_nnnn_0000_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "movca.l R0,@<REG_N>")
    i0000_nnnn_1100_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ocbi @<REG_N>")
    i0000_nnnn_1001_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ocbp @<REG_N>")
    i0000_nnnn_1010_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ocbwb @<REG_N>")
    i0000_nnnn_1011_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "pref @<REG_N>")
    i0000_nnnn_1000_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "clrmac")
    i0000_0000_0010_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "clrs")
    i0000_0000_0100_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "clrt")
    i0000_0000_0000_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldtlb")
    i0000_0000_0011_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sets")
    i0000_0000_0101_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sett")
    i0000_0000_0001_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "div0u")
    i0000_0000_0001_1001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "movt <REG_N>")
    i0000_nnnn_0010_1001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts FPSCR,<REG_N>")
    i0000_nnnn_0110_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts DBR,<REG_N>")
    i0000_nnnn_1111_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts MACH,<REG_N>")
    i0000_nnnn_0000_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rte")
    i0000_0000_0010_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rts")
    i0000_0000_0000_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sleep")
    i0000_0000_0001_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mac.l @<REG_M>+,@<REG_N>+")
    i0000_nnnn_mmmm_1111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l <REG_M>,@(<disp4dw>,<REG_N>)")
    i0001_nnnn_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "div0s <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "tst <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "or <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/str <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "xtrct <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mulu.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "muls.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/eq <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/hs <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/ge <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "div1 <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "dmulu.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/hi <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/gt <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "subc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "subv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "dmuls.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "addc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "addv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts.l FPUL,@-<REG_N>")
    i0100_nnnn_0101_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts.l FPSCR,@-<REG_N>")
    i0100_nnnn_0110_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts.l MACH,@-<REG_N>")
    i0100_nnnn_0000_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts.l MACL,@-<REG_N>")
    i0100_nnnn_0001_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "sts.l PR,@-<REG_N>")
    i0100_nnnn_0010_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l DBR,@-<REG_N>")
    i0100_nnnn_1111_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l SR,@-<REG_N>")
    i0100_nnnn_0000_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l GBR,@-<REG_N>")
    i0100_nnnn_0001_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l VBR,@-<REG_N>")
    i0100_nnnn_0010_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l SSR,@-<REG_N>")
    i0100_nnnn_0011_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l SPC,@-<REG_N>")
    i0100_nnnn_0100_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc <RM_BANK>,@-<REG_N>")
    i0100_nnnn_1mmm_0011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds.l @<REG_N>+,MACH")
    i0100_nnnn_0000_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds.l @<REG_N>+,MACL")
    i0100_nnnn_0001_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds.l @<REG_N>+,PR")
    i0100_nnnn_0010_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,SGR")
    i0100_nnnn_0011_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds.l @<REG_N>+,FPUL")
    i0100_nnnn_0101_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds.l @<REG_N>+,FPSCR")
    i0100_nnnn_0110_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,DBR")
    i0100_nnnn_1111_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,SR")
    i0100_nnnn_0000_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,GBR")
    i0100_nnnn_0001_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,VBR")
    i0100_nnnn_0010_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,SSR")
    i0100_nnnn_0011_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,SPC")
    i0100_nnnn_0100_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc.l @<REG_N>+,RM_BANK")
    i0100_nnnn_1mmm_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds <REG_N>,MACH")
    i0100_nnnn_0000_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds <REG_N>,MACL")
    i0100_nnnn_0001_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds <REG_N>,PR")
    i0100_nnnn_0010_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "lds <REG_N>,FPSCR")
    i0100_nnnn_0110_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,DBR")
    i0100_nnnn_1111_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,SR")
    i0100_nnnn_0000_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,GBR")
    i0100_nnnn_0001_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,VBR")
    i0100_nnnn_0010_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,SSR")
    i0100_nnnn_0011_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,SPC")
    i0100_nnnn_0100_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ldc <REG_N>,<RM_BANK>")
    i0100_nnnn_1mmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shll <REG_N>")
    i0100_nnnn_0000_0000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shal <REG_N>")
    i0100_nnnn_0010_0000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/pz <REG_N>")
    i0100_nnnn_0001_0001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shar <REG_N>")
    i0100_nnnn_0010_0001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rotcl <REG_N>")
    i0100_nnnn_0010_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rotl <REG_N>")
    i0100_nnnn_0000_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "cmp/pl <REG_N>")
    i0100_nnnn_0001_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rotcr <REG_N>")
    i0100_nnnn_0010_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "rotr <REG_N>")
    i0100_nnnn_0000_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shll2 <REG_N>")
    i0100_nnnn_0000_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shll16 <REG_N>")
    i0100_nnnn_0010_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shlr8 <REG_N>")
    i0100_nnnn_0001_1001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "jmp @<REG_N>")
    i0100_nnnn_0010_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "jsr @<REG_N>")
    i0100_nnnn_0000_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "tas.b @<REG_N>")
    i0100_nnnn_0001_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shad <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "shld <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mac.w @<REG_M>+,@<REG_N>+")
    i0100_nnnn_mmmm_1111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    // 5xxx
    (disas = "mov.l @(<disp4dw>,<REG_M>),<REG_N>")
    i0101_nnnn_mmmm_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    // 6xxx
    (disas = "mov.w @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "not <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "swap.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1000(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "swap.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "negc <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "extu.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "exts.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "exts.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    //8xxx

    (disas = "mov.w @(<PCdisp8w>),<REG_N>")
    i1001_nnnn_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }


    (disas = "bt <bdisp8>")
    i1000_1001_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "bt/s <bdisp8>")
    i1000_1101_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    
    //bxxx
    (disas = "bsr <bdisp12>")
    i1011_iiii_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }


    //Cxxx
    (disas = "mov.b R0,@(<disp8b>,GBR)")
    i1100_0000_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w R0,@(<disp8w>,GBR)")
    i1100_0001_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l R0,@(<disp8dw>,GBR)")
    i1100_0010_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "trapa #<imm8>")
    i1100_0011_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.b @(<GBRdisp8b>),R0")
    i1100_0100_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.w @(<GBRdisp8w>),R0")
    i1100_0101_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "mov.l @(<GBRdisp8dw>),R0")
    i1100_0110_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "tst #<imm8>,R0")
    i1100_1000_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "and #<imm8>,R0")
    i1100_1001_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "xor #<imm8>,R0")
    i1100_1010_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "or #<imm8>,R0")
    i1100_1011_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "tst.b #<imm8>,@(R0,GBR)")
    i1100_1100_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "and.b #<imm8>,@(R0,GBR)")
    i1100_1101_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "xor.b #<imm8>,@(R0,GBR)")
    i1100_1110_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "or.b #<imm8>,@(R0,GBR)")
    i1100_1111_iiii_iiii(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    //Fxxx
    (disas = "flds <FREG_N>,FPUL")
    i1111_nnnn_0001_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fneg <FREG_N_SD_F>")
    i1111_nnnn_0100_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fabs <FREG_N_SD_F>")
    i1111_nnnn_0101_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fsqrt <FREG_N>")
    i1111_nnnn_0110_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fldi0 <FREG_N>")
    i1111_nnnn_1000_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fldi1 <FREG_N>")
    i1111_nnnn_1001_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "ftrv xmtrx,<FV_N>")
    i1111_nn01_1111_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fcmp/eq <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0100(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmov.s @(R0,<REG_M>),<FREG_N_SD_A>")
    i1111_nnnn_mmmm_0110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)")
    i1111_nnnn_mmmm_0111(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmov.s @<REG_M>+,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1001(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@<REG_N>")
    i1111_nnnn_mmmm_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmov.s <FREG_M_SD_A>,@-<REG_N>")
    i1111_nnnn_mmmm_1011(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }


    (disas = "fcnvds <DR_N>,FPUL")
    i1111_nnnn_1011_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fcnvsd FPUL,<DR_N>")
    i1111_nnnn_1010_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fipr <FV_M>,<FV_N>")
    i1111_nnmm_1110_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "frchg")
    i1111_1011_1111_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fschg")
    i1111_0011_1111_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fsts FPUL,<FREG_N>")
    i1111_nnnn_0000_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fsrra <FREG_N>")
    i1111_nnnn_0111_1101(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "fmac <FREG_0>,<FREG_M>,<FREG_N>")
    i1111_nnnn_mmmm_1110(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }


    (disas = "sts PR,<REG_N>")
    i0000_nnnn_0010_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }
    (disas = "ldc <REG_N>,SGR")
    i0100_nnnn_0011_1010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }

    (disas = "stc.l SGR,@-<REG_N>")
    i0100_nnnn_0011_0010(dc, state, instr) {
        i_not_implemented(dc, state, instr);
    }
}

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

pub const fn build_opcode_tables(
    opcodes: &[sh4_opcodelistentry]
) -> ([fn(*mut Dreamcast, u16); 0x10000],
      [sh4_opcodelistentry; 0x10000])
{
    // The sentinel is always the last element of OPCODES
    let sentinel = opcodes[opcodes.len() - 1];

    let mut ptrs: [fn(*mut Dreamcast, u16); 0x10000] = [sentinel.oph; 0x10000];
    let mut descs: [sh4_opcodelistentry; 0x10000] = [sentinel; 0x10000];

    let mut i = 0;
    while i < opcodes.len() {
        let op = opcodes[i];
        if op.key == 0 {
            break; // stop at sentinel
        }

        let (count, shft) = match op.mask {
            0xFFFF => (1, 0),
            0xF0FF => (16, 8),
            0xF00F => (256, 4),
            0xF000 => (256*16, 0),
            0xFF00 => (256, 0),
            0xF08F => (256, 4),
            0xF1FF => (8, 9),
            0xF3FF => (4, 10),
            _ => (0, 0),
        };

        assert!(count != 0);

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

// Make the final tables visible to the crate.
const SH4_OP_TABLES: (
    [fn(*mut Dreamcast, u16); 0x10000],
    [sh4_opcodelistentry; 0x10000]
) = build_opcode_tables(OPCODES);

pub(crate) const SH4_OP_PTR:  [fn(*mut Dreamcast, u16); 0x10000] = SH4_OP_TABLES.0;
pub(crate) const SH4_OP_DESC: [sh4_opcodelistentry; 0x10000]     = SH4_OP_TABLES.1;

// // Re-export for parent (and callers) to `use sh4dec::{...}` or via the re-export in dreamcast_sh4.rs
// pub use SH4_OP_PTR as _;
// pub use SH4_OP_DESC as _;

pub fn format_disas(state:SH4DecoderState, instr: u16) -> String {
    let mut out = unsafe { SH4_OP_DESC.get_unchecked(instr as usize).diss }.to_string();

    // ---------------- General-purpose registers ----------------
    if out.contains("<REG_N>") {
        let n = (instr >> 8) & 0xF;
        out = out.replace("<REG_N>", &format!("r{}", n));
    }
    if out.contains("<REG_M>") {
        let m = (instr >> 4) & 0xF;
        out = out.replace("<REG_M>", &format!("r{}", m));
    }

    // ---------------- Immediates ----------------
    if out.contains("<IMM4>") {
        let imm = instr & 0xF;
        out = out.replace("<IMM4>", &format!("#{}", imm));
    }
    if out.contains("<IMM8>") || out.contains("<imm8>") {
        let imm = (instr & 0xFF) as i8;
        out = out.replace("<IMM8>", &format!("#{}", imm));
        out = out.replace("<imm8>", &format!("#{}", imm));
    }
    if out.contains("<simm8>") {
        let imm = (instr & 0xFF) as i8;
        out = out.replace("<simm8>", &format!("{}", imm));
    }
    if out.contains("<simm8hex>") {
        let imm = (instr & 0xFF) as i8;
        out = out.replace("<simm8hex>", &format!("{:#x}", imm));
    }

    // ---------------- Displacements ----------------
    if out.contains("<bdisp8>") {
        let disp = ((instr & 0xFF) as i8 as i32) << 1;
        out = out.replace("<bdisp8>", &format!("{:#x}", disp));
    }
    if out.contains("<bdisp12>") {
        let disp = ((instr & 0x0FFF) as i16 as i32) << 1;
        out = out.replace("<bdisp12>", &format!("{:#x}", disp));
    }

    // 4-bit disps
    if out.contains("<disp4b>") {
        let d = instr & 0xF;
        out = out.replace("<disp4b>", &format!("{:#x}", d));
    }
    if out.contains("<disp4w>") {
        let d = (instr & 0xF) << 1;
        out = out.replace("<disp4w>", &format!("{:#x}", d));
    }
    if out.contains("<disp4dw>") {
        let d = (instr & 0xF) << 2;
        out = out.replace("<disp4dw>", &format!("{:#x}", d));
    }

    // 8-bit disps
    if out.contains("<disp8b>") {
        let d = instr & 0xFF;
        out = out.replace("<disp8b>", &format!("{:#x}", d));
    }
    if out.contains("<disp8w>") {
        let d = (instr & 0xFF) << 1;
        out = out.replace("<disp8w>", &format!("{:#x}", d));
    }
    if out.contains("<disp8dw>") {
        let d = (instr & 0xFF) << 2;
        out = out.replace("<disp8dw>", &format!("{:#x}", d));
    }

    // PC relative
    if out.contains("<PCdisp8d>") {
        let d = (instr & 0xFF) << 2;
        out = out.replace("<PCdisp8d>", &format!("{:#x}", d));
    }
    if out.contains("<PCdisp8w>") {
        let d = (instr & 0xFF) << 1;
        out = out.replace("<PCdisp8w>", &format!("{:#x}", d));
    }

    // GBR disps
    if out.contains("<GBRdisp8b>") {
        let d = instr & 0xFF;
        out = out.replace("<GBRdisp8b>", &format!("{:#x}", d));
    }
    if out.contains("<GBRdisp8w>") {
        let d = (instr & 0xFF) << 1;
        out = out.replace("<GBRdisp8w>", &format!("{:#x}", d));
    }
    if out.contains("<GBRdisp8dw>") {
        let d = (instr & 0xFF) << 2;
        out = out.replace("<GBRdisp8dw>", &format!("{:#x}", d));
    }

    // ---------------- Floating-point regs ----------------
    if out.contains("<FREG_N>") {
        let n = (instr >> 8) & 0xF;
        out = out.replace("<FREG_N>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M>") {
        let m = (instr >> 4) & 0xF;
        out = out.replace("<FREG_M>", &format!("fr{}", m));
    }
    if out.contains("<FREG_N_SD_F>") {
        let n = (instr >> 8) & 0xF;
        out = out.replace("<FREG_N_SD_F>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M_SD_F>") {
        let m = (instr >> 4) & 0xF;
        out = out.replace("<FREG_M_SD_F>", &format!("fr{}", m));
    }
    if out.contains("<FREG_N_SD_A>") {
        let n = (instr >> 8) & 0xF;
        out = out.replace("<FREG_N_SD_A>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M_SD_A>") {
        let m = (instr >> 4) & 0xF;
        out = out.replace("<FREG_M_SD_A>", &format!("fr{}", m));
    }

    if out.contains("<DR_N>") {
        let n = ((instr >> 8) & 0xE) >> 1; // even reg pair
        out = out.replace("<DR_N>", &format!("dr{}", n));
    }
    if out.contains("<FV_N>") {
        let n = ((instr >> 8) & 0xC) >> 2; // vector index
        out = out.replace("<FV_N>", &format!("fv{}", n));
    }
    if out.contains("<FV_M>") {
        let m = ((instr >> 4) & 0xC) >> 2;
        out = out.replace("<FV_M>", &format!("fv{}", m));
    }

    out
}
