// src/sh4dec.rs

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::ptr::{addr_of, addr_of_mut};

// Pull in the machine types and backends from the parent module.
use super::Sh4Ctx;

#[derive(Copy, Clone)]
pub struct sh4_opcodelistentry {
    pub oph: fn(*mut Sh4Ctx, u16),
    pub dech: fn(*mut Sh4Ctx, u16),
    pub handler_name: &'static str,
    pub mask: u16,
    pub key: u16,
    pub diss: &'static str,
}

const fn parse_opcode(pattern: &str) -> (u16, u16) {
    let bytes = pattern.as_bytes();
    let mut i = 1; // skip leading 'i'
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
        } else if c != b'_' {
            // wildcard
            mask = mask << 1;
            key = key << 1;
        }
        i += 1;
    }
    (mask, key)
}

#[derive(Copy, Clone)]
pub struct SH4DecoderState {
    pub pc: u32,
    pub fpscr_PR: bool,
    pub fpscr_SZ: bool,
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
                       ; backend = crate::backend_ipr);
            )*
        }
        // Decoder expansion
        pub(crate) mod dec {
            use super::*;
            $(
                sh4op!(@emit $name ( $($params)* ) { $($body)* }
                       ; backend = crate::backend_fns);
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

    // --- (dc, opcode) ---------------------------------------------------------------------------
    (@emit
        $name:ident ( $dc:ident , $opcode:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: *mut Sh4Ctx, $opcode: u16) {
            #[allow(unused_unsafe)]
            unsafe {
                #[allow(unused_imports)]
                use $backend as backend;
                { $($body)* }
            }
        }
    };

    // --- (dc, state, opcode): inject a local state struct ---------------------------------------
    (@emit
        $name:ident ( $dc:ident , $state:ident , $opcode:ident )
        { $($body:tt)* }
        ; backend = $backend:path
    ) => {
        #[allow(non_snake_case)]
        pub(crate) fn $name($dc: *mut Sh4Ctx, $opcode: u16) {
            #[allow(unused_unsafe)]
            unsafe {
                #[allow(unused_imports)]
                use $backend as backend;

                let $state = SH4DecoderState {
                    pc: (*$dc).pc0,
                    fpscr_PR: (*$dc).fpscr.pr(),
                    fpscr_SZ: (*$dc).fpscr.sz(),
                };

                { $($body)* }
            }
        }
    };
}

#[inline(always)]
fn GetN(str_: u16) -> usize {
    ((str_ >> 8) & 0xF) as usize
}
#[inline(always)]
fn GetM(str_: u16) -> usize {
    ((str_ >> 4) & 0xF) as usize
}
#[inline(always)]
fn GetImm4(str_: u16) -> u32 {
    (str_ & 0xF) as u32
}
#[inline(always)]
fn GetImm8(str_: u16) -> u32 {
    (str_ & 0xFF) as u32
}
#[inline(always)]
fn GetSImm8(str_: u16) -> i32 {
    (str_ & 0xFF) as i8 as i32
}
#[inline(always)]
fn GetImm12(str_: u16) -> u32 {
    (str_ & 0xFFF) as u32
}
#[inline(always)]
fn GetSImm12(str_: u16) -> i32 {
    ((((GetImm12(str_) as u16) << 4) as i16) >> 4) as i32
}

#[inline(always)]
fn data_target_s8(pc: u32, disp8: i32) -> u32 {
    ((pc.wrapping_add(4)) & 0xFFFF_FFFC).wrapping_add((disp8 << 2) as u32)
}
#[inline(always)]
fn branch_target_s8(pc: u32, disp8: i32) -> u32 {
    (disp8 as i64 * 2 + 4 + pc as i64) as u32
}
#[inline(always)]
fn branch_target_s12(pc: u32, disp12: i32) -> u32 {
    (disp12 as i64 * 2 + 4 + pc as i64) as u32
}

fn i_not_implemented(dc: *mut Sh4Ctx, state: SH4DecoderState, opcode: u16) {
    let desc_ptr: *const sh4_opcodelistentry = &SH4_OP_DESC[opcode as usize];
    let diss = unsafe {
        if desc_ptr.is_null() {
            "missing"
        } else {
            let d = &*desc_ptr;
            if d.diss.is_empty() { "missing" } else { d.diss }
        }
    };
    panic!(
        "{:08X}: {:04X} {} [i_not_implemented]",
        state.pc, opcode, diss
    );
}

fn i_not_known(dc: *mut Sh4Ctx, opcode: u16) {
    unsafe {
        let pc = (*dc).pc0;
        let desc_ptr = &SH4_OP_DESC[opcode as usize];
        panic!("{:08X}: {:04X} {} [i_not_known]", pc, opcode, desc_ptr.diss);
    }
}

sh4op! {
    (disas = "mul.l <REG_M>,<REG_N>")
    i0000_nnnn_mmmm_0111(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_muls32(addr_of_mut!((*dc).mac.parts.l), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "nop")
    i0000_0000_0000_1001(dc, opcode) {
        // no-op
    }

    (disas = "sts FPUL,<REG_N>")
    i0000_nnnn_0101_1010(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).fpul));
    }

    (disas = "sts MACL,<REG_N>")
    i0000_nnnn_0001_1010(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).mac.parts.l));
    }

    (disas = "mov.b <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0000(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem8(dc, addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mov.w <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0001(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem16(dc, addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mov.l <REG_M>,@<REG_N>")
    i0010_nnnn_mmmm_0010(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem32(dc, addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "and <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1001(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_and(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "xor <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1010(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_xor(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "sub <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1000(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_sub(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "add <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1100(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_add(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "dt <REG_N>")
    i0100_nnnn_0001_0000(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_dt(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]));
    }

    (disas = "shlr <REG_N>")
    i0100_nnnn_0000_0001(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_shlr(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "shll8 <REG_N>")
    i0100_nnnn_0001_1000(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_shllf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 8);
    }

    (disas = "shlr2 <REG_N>")
    i0100_nnnn_0000_1001(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_shlrf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 2);
    }

    (disas = "shlr16 <REG_N>")
    i0100_nnnn_0010_1001(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_shlrf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 16);
    }

    (disas = "mov.b @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0000(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);

        backend::sh4_read_mems8(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0011(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "neg <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1011(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_neg(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "extu.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1100(dc, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_extub(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "add #<simm8>,<REG_N>")
    i0111_nnnn_iiii_iiii(dc, opcode) {
        let n = GetN(opcode);
        let stmp1 = GetSImm8(opcode);
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), stmp1 as u32);
    }

    (disas = "bf <bdisp8>")
    i1000_1011_iiii_iiii(dc, state, opcode) {
        let disp8 = GetSImm8(opcode);
        let next = state.pc.wrapping_add(2);
        let target = branch_target_s8(state.pc, disp8);
        backend::sh4_branch_cond(dc, addr_of!((*dc).sr_t), 0, next, target);
    }

    (disas = "bf/s <bdisp8>")
    i1000_1111_iiii_iiii(dc, state, opcode) {
        let disp8 = GetSImm8(opcode);
        let next = state.pc.wrapping_add(4);
        let target = branch_target_s8(state.pc, disp8);
        backend::sh4_branch_cond_delay(dc, addr_of!((*dc).sr_t), 0, next, target);
    }

    (disas = "bra <bdisp12>")
    i1010_iiii_iiii_iiii(dc, state, opcode) {
        let disp12 = GetSImm12(opcode);
        let target = branch_target_s12(state.pc, disp12);
        backend::sh4_branch_delay(dc, target);
    }

    (disas = "mova @(<PCdisp8d>),R0")
    i1100_0111_iiii_iiii(dc, state, opcode) {
        let disp8 = GetImm8(opcode) as i32;
        let addr = data_target_s8(state.pc, disp8);
        backend::sh4_store32i(addr_of_mut!((*dc).r[0]), addr);
    }
    (disas = "mov.b R0,@(<disp4b>,<REG_M>)")
    i1000_0000_mmmm_iiii(dc, state, opcode) {
        let m = GetM(opcode);
        let disp = GetImm4(opcode);
        backend::sh4_write_mem8_disp(dc, addr_of!((*dc).r[m]), disp, addr_of!((*dc).r[0]));
    }

    (disas = "mov.w R0,@(<disp4w>,<REG_M>)")
    i1000_0001_mmmm_iiii(dc, state, opcode) {
        let m = GetM(opcode);
        let disp = GetImm4(opcode) << 1;
        backend::sh4_write_mem16_disp(dc, addr_of!((*dc).r[m]), disp, addr_of!((*dc).r[0]));
    }

    (disas = "mov.b @(<disp4b>,<REG_M>),R0")
    i1000_0100_mmmm_iiii(dc, state, opcode) {
        let m = GetM(opcode);
        let disp = GetImm4(opcode);
        backend::sh4_read_mems8_disp(dc, addr_of!((*dc).r[m]), disp, addr_of_mut!((*dc).r[0]));
    }

    (disas = "mov.w @(<disp4w>,<REG_M>),R0")
    i1000_0101_mmmm_iiii(dc, state, opcode) {
        let m = GetM(opcode);
        let disp = GetImm4(opcode) << 1;
        backend::sh4_read_mems16_disp(dc, addr_of!((*dc).r[m]), disp, addr_of_mut!((*dc).r[0]));
    }

    (disas = "cmp/eq #<simm8hex>,R0")
    i1000_1000_iiii_iiii(dc, state, opcode) {
        let imm = GetSImm8(opcode) as u32;
        backend::sh4_cmp_eq_imm(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[0]), imm);
    }


    (disas = "mov.l @(<PCdisp8d>),<REG_N>")
    i1101_nnnn_iiii_iiii(dc, state, opcode) {
        let n = GetN(opcode);
        let disp8 = GetImm8(opcode) as i32;
        let addr = data_target_s8(state.pc, disp8);

        backend::sh4_read_mem32i(dc, addr, addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov #<simm8hex>,<REG_N>")
    i1110_nnnn_iiii_iiii(dc, opcode) {
        let n = GetN(opcode);
        let imm = GetSImm8(opcode);
        backend::sh4_store32i(addr_of_mut!((*dc).r[n]), imm as u32);
    }

    (disas = "fadd <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0000(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_fadd(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m])); };
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            let m = (GetM(opcode) >> 1) & 0x7;
            backend::sh4_fadd_d(addr_of_mut!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[m << 1]));
        }
    }

    (disas = "fsub <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0001(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_fsub(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m])); };
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            let m = (GetM(opcode) >> 1) & 0x7;
            backend::sh4_fsub_d(addr_of_mut!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[m << 1]));
        }
    }

    (disas = "fmul <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0010(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_fmul(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m])); };
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            let m = (GetM(opcode) >> 1) & 0x7;
            backend::sh4_fmul_d(addr_of_mut!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[m << 1]));
        }
    }

    (disas = "fdiv <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0011(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_fdiv(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m])); };
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            let m = (GetM(opcode) >> 1) & 0x7;
            backend::sh4_fdiv_d(addr_of_mut!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fr.u32s[m << 1]));
        }
    }

    (disas = "fmov.s @<REG_M>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1000(dc, state, opcode) {
        if !state.fpscr_SZ {
            // SZ=0: Transfer single 32-bit value
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_read_mem32(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).fr.u32s[n])); }
        } else {
            // SZ=1: Transfer 64-bit value (pair of registers)
            let n = GetN(opcode);
            let m = GetM(opcode);
            let n_d = n >> 1;
            if (n & 0x1) == 0 {
                backend::sh4_read_mem64(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).fr.u64s[n_d]));
            } else {
                backend::sh4_read_mem64(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).xf.u64s[n_d]));
            }
        }
    }

    (disas = "fmov <FREG_M_SD_A>,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1100(dc, state, opcode) {
        if !state.fpscr_SZ {
            // SZ=0: Transfer single 32-bit value
            let n = GetN(opcode);
            let m = GetM(opcode);
            unsafe { backend::sh4_store32(addr_of_mut!((*dc).fr.u32s[n]), addr_of!((*dc).fr.u32s[m])); }
        } else {
            // SZ=1: Transfer 64-bit value (pair of registers)
            let n = GetN(opcode);
            let m = GetM(opcode);
            let n_d = n >> 1;
            let m_d = m >> 1;
            if (n & 0x1) == 0 {
                if (m & 0x1) == 0 {
                    backend::sh4_store64(addr_of_mut!((*dc).fr.u64s[n_d]), addr_of!((*dc).fr.u64s[m_d]));
                } else {
                    backend::sh4_store64(addr_of_mut!((*dc).fr.u64s[n_d]), addr_of!((*dc).xf.u64s[m_d]));
                }
            } else {
                if (m & 0x1) == 0 {
                    backend::sh4_store64(addr_of_mut!((*dc).xf.u64s[n_d]), addr_of!((*dc).fr.u64s[m_d]));
                } else {
                    backend::sh4_store64(addr_of_mut!((*dc).xf.u64s[n_d]), addr_of!((*dc).xf.u64s[m_d]));
                }
            }
        }
    }

    (disas = "fsca FPUL,<DR_N>")
    i1111_nnn0_1111_1101(dc, state, opcode) {
        let n = (GetN(opcode) & 0xE) as usize;
        if !state.fpscr_PR {
            unsafe { backend::sh4_fsca(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fpul)); }

        } else {
            panic!("fsca: double precision mode not supported");
        }
    }

    (disas = "float FPUL,<FREG_N_SD_F>")
    i1111_nnnn_0010_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            unsafe { backend::sh4_float(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fpul)); }
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            backend::sh4_float_d(addr_of_mut!((*dc).fr.u32s[n << 1]), addr_of!((*dc).fpul));
        }
    }

    (disas = "ftrc <FREG_N>,FPUL")
    i1111_nnnn_0011_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            unsafe { backend::sh4_ftrc(addr_of_mut!((*dc).fpul), addr_of!((*dc).fr.f32s[n])); }
        } else {
            let n = (GetN(opcode) >> 1) & 0x7;
            backend::sh4_ftrc_d(addr_of_mut!((*dc).fpul), addr_of!((*dc).fr.u32s[n << 1]));
        }
    }

    (disas = "lds <REG_N>,FPUL")
    i0100_nnnn_0101_1010(dc, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).fpul), addr_of!((*dc).r[n]));
    }

    (disas = "stc SR,<REG_N>")
    i0000_nnnn_0000_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).sr.0));
        backend::sh4_or(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).sr_t));
    }

    (disas = "stc GBR,<REG_N>")
    i0000_nnnn_0001_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).gbr));
    }

    (disas = "stc VBR,<REG_N>")
    i0000_nnnn_0010_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).vbr));
    }

    (disas = "stc SSR,<REG_N>")
    i0000_nnnn_0011_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).ssr));
    }

    (disas = "sts SGR,<REG_N>")
    i0000_nnnn_0011_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).sgr));
    }

    (disas = "stc SPC,<REG_N>")
    i0000_nnnn_0100_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).spc));
    }

    (disas = "stc RM_BANK,<REG_N>")
    i0000_nnnn_1mmm_0010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode) & 0x7;
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r_bank[m]));
    }

    (disas = "braf <REG_N>")
    i0000_nnnn_0010_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_braf(dc, addr_of!((*dc).r[n]), state.pc);
    }

    (disas = "bsrf <REG_N>")
    i0000_nnnn_0000_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_bsrf(dc, addr_of!((*dc).r[n]), state.pc);
    }

    (disas = "movca.l R0,@<REG_N>")
    i0000_nnnn_1100_0011(dc, state, opcode) {
        // Simplified implementation: write R0 to @Rn
        // Full implementation would need OIX cache handling for addresses with bit 25 set
        let n = GetN(opcode);
        backend::sh4_write_mem32(dc, addr_of!((*dc).r[n]), addr_of!((*dc).r[0]));
    }

    (disas = "ocbi @<REG_N>")
    i0000_nnnn_1001_0011(dc, state, opcode) {
        // Operand cache block invalidate - no-op for interpreter
    }

    (disas = "ocbp @<REG_N>")
    i0000_nnnn_1010_0011(dc, state, opcode) {
        // Operand cache block purge - no-op for interpreter
    }

    (disas = "ocbwb @<REG_N>")
    i0000_nnnn_1011_0011(dc, state, opcode) {
        // Operand cache block write-back - no-op for interpreter (would need OIX handling)
    }

    (disas = "pref @<REG_N>")
    i0000_nnnn_1000_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_pref(dc, addr_of!((*dc).r[n]), addr_of!((*dc).sq_both[0]), addr_of!((*dc).qacr0_base), addr_of!((*dc).qacr1_base));
    }

    (disas = "mov.b <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem8_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mov.w <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem16_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mov.l <REG_M>,@(R0,<REG_N>)")
    i0000_nnnn_mmmm_0110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem32_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "clrmac")
    i0000_0000_0010_1000(dc, state, opcode) {
        backend::sh4_store32i(addr_of_mut!((*dc).mac.parts.h), 0);
        backend::sh4_store32i(addr_of_mut!((*dc).mac.parts.l), 0);
    }

    (disas = "clrs")
    i0000_0000_0100_1000(dc, state, opcode) {
        // Clear S bit (bit 1) in SR
        backend::sh4_andi(addr_of_mut!((*dc).sr.0), addr_of!((*dc).sr.0), 0xFFFFFFFD);
    }

    (disas = "clrt")
    i0000_0000_0000_1000(dc, state, opcode) {
        backend::sh4_clrt(addr_of_mut!((*dc).sr_t));
    }

    (disas = "ldtlb")
    i0000_0000_0011_1000(dc, state, opcode) {
        // LDTLB - Load TLB entry
        // This is a privileged MMU operation that loads UTLB from PTEH/PTEL/PTEA
        // For the interpreter, we can leave this as a no-op or minimal implementation
        // TODO: Implement proper MMU handling if needed
    }

    (disas = "sets")
    i0000_0000_0101_1000(dc, state, opcode) {
        // Set S bit (bit 1) in SR
        backend::sh4_or_imm(addr_of_mut!((*dc).sr.0), addr_of!((*dc).sr.0), 2);
    }

    (disas = "sett")
    i0000_0000_0001_1000(dc, state, opcode) {
        backend::sh4_sett(addr_of_mut!((*dc).sr_t));
    }

    (disas = "div0u")
    i0000_0000_0001_1001(dc, state, opcode) {
        backend::sh4_div0u(addr_of_mut!((*dc).sr), addr_of_mut!((*dc).sr_t));
    }

    (disas = "movt <REG_N>")
    i0000_nnnn_0010_1001(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_movt(addr_of_mut!((*dc).r[n]), addr_of!((*dc).sr_t));
    }

    (disas = "sts FPSCR,<REG_N>")
    i0000_nnnn_0110_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).fpscr.0));
    }

    (disas = "sts DBR,<REG_N>")
    i0000_nnnn_1111_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).dbr));
    }

    (disas = "sts MACH,<REG_N>")
    i0000_nnnn_0000_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).mac.parts.h));
    }

    (disas = "rte")
    i0000_0000_0010_1011(dc, state, opcode) {
        // FIXME: RTE dslot insn access uses MD before change
        backend::sh4_store32(addr_of_mut!((*dc).virt_jdyn), addr_of!((*dc).spc));
        backend::sh4_andi(addr_of_mut!((*dc).sr_t), addr_of!((*dc).ssr), 1);
        backend::sh4_store_sr_rest(addr_of_mut!((*dc).sr.0), addr_of!((*dc).ssr), addr_of_mut!((*dc).r[0]), addr_of_mut!((*dc).r_bank[0]));
        backend::sh4_rte(dc, addr_of!((*dc).virt_jdyn));
    }

    (disas = "rts")
    i0000_0000_0000_1011(dc, state, opcode) {
        backend::sh4_rts(dc, addr_of!((*dc).pr));
    }

    (disas = "sleep")
    i0000_0000_0001_1011(dc, state, opcode) {
        // Sleep - suspend execution until interrupt (no-op in interpreter)
        // In a real implementation, this would halt the CPU until an interrupt occurs
    }

    (disas = "mov.b @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mems8_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov.w @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mems16_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov.l @(R0,<REG_M>),<REG_N>")
    i0000_nnnn_mmmm_1110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mem32_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mac.l @<REG_M>+,@<REG_N>+")
    i0000_nnnn_mmmm_1111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if n == m {
            // Special case: read from r[n], then r[n]+4
            backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
            backend::sh4_read_mem32_disp(dc, addr_of!((*dc).r[n]), 4, addr_of_mut!((*dc).temp[1]));
            backend::sh4_mac_l_mul(addr_of_mut!((*dc).mac.full), addr_of!((*dc).temp[0]), addr_of!((*dc).temp[1]));
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
        } else {
            backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
            backend::sh4_read_mem32(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).temp[1]));
            backend::sh4_mac_l_mul(addr_of_mut!((*dc).mac.full), addr_of!((*dc).temp[0]), addr_of!((*dc).temp[1]));
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 4);
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
        }
    }

    (disas = "mov.l <REG_M>,@(<disp4dw>,<REG_N>)")
    i0001_nnnn_mmmm_iiii(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        let disp = GetImm4(opcode) << 2;
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), disp, addr_of!((*dc).r[m]));
    }

    (disas = "mov.b <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem8_disp(dc, addr_of!((*dc).r[n]), (-1i32) as u32, addr_of!((*dc).r[m]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-1i32) as u32);
    }

    (disas = "mov.w <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem16_disp(dc, addr_of!((*dc).r[n]), (-2i32) as u32, addr_of!((*dc).r[m]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-2i32) as u32);
    }

    (disas = "mov.l <REG_M>,@-<REG_N>")
    i0010_nnnn_mmmm_0110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).r[m]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "div0s <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_0111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_div0s(addr_of_mut!((*dc).sr), addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "tst <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1000(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_tst(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "or <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1011(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_or(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/str <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_str(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "xtrct <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_xtrct(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mulu.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_muluw(addr_of_mut!((*dc).mac.parts.l), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "muls.w <REG_M>,<REG_N>")
    i0010_nnnn_mmmm_1111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_mulsw(addr_of_mut!((*dc).mac.parts.l), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/eq <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0000(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_eq(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/hs <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_hs(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/ge <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0011(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_ge(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "div1 <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_div1(addr_of_mut!((*dc).sr), addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "dmulu.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_dmulu(addr_of_mut!((*dc).mac.full), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/hi <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_hi(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "cmp/gt <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_0111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_cmp_gt(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "subc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_subc(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "subv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1011(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_subv(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "dmuls.l <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_dmuls(addr_of_mut!((*dc).mac.full), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "addc <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_addc(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "addv <REG_M>,<REG_N>")
    i0011_nnnn_mmmm_1111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_addv(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "sts.l FPUL,@-<REG_N>")
    i0100_nnnn_0101_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).fpul));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "sts.l FPSCR,@-<REG_N>")
    i0100_nnnn_0110_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).fpscr.0));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "sts.l MACH,@-<REG_N>")
    i0100_nnnn_0000_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).mac.parts.h));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "sts.l MACL,@-<REG_N>")
    i0100_nnnn_0001_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).mac.parts.l));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "sts.l PR,@-<REG_N>")
    i0100_nnnn_0010_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).pr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l DBR,@-<REG_N>")
    i0100_nnnn_1111_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).dbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l SR,@-<REG_N>")
    i0100_nnnn_0000_0011(dc, state, opcode) {
        let n = GetN(opcode);
        // Combine sr.0 (without T bit) and sr_t (T bit) into full SR value
        backend::sh4_or(addr_of_mut!((*dc).temp[0]), addr_of!((*dc).sr.0), addr_of!((*dc).sr_t));
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).temp[0]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l GBR,@-<REG_N>")
    i0100_nnnn_0001_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).gbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l VBR,@-<REG_N>")
    i0100_nnnn_0010_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).vbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l SSR,@-<REG_N>")
    i0100_nnnn_0011_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).ssr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc.l SPC,@-<REG_N>")
    i0100_nnnn_0100_0011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).spc));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "stc <RM_BANK>,@-<REG_N>")
    i0100_nnnn_1mmm_0011(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode) & 0x7;
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).r_bank[m]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "lds.l @<REG_N>+,MACH")
    i0100_nnnn_0000_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).mac.parts.h));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "lds.l @<REG_N>+,MACL")
    i0100_nnnn_0001_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).mac.parts.l));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "lds.l @<REG_N>+,PR")
    i0100_nnnn_0010_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).pr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,SGR")
    i0100_nnnn_0011_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).sgr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "lds.l @<REG_N>+,FPUL")
    i0100_nnnn_0101_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).fpul));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "lds.l @<REG_N>+,FPSCR")
    i0100_nnnn_0110_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
        backend::sh4_store_fpscr(addr_of_mut!((*dc).fpscr.0), addr_of!((*dc).temp[0]),
                                  addr_of_mut!((*dc).fr.u32s[0]), addr_of_mut!((*dc).xf.u32s[0]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,DBR")
    i0100_nnnn_1111_0110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).dbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,SR")
    i0100_nnnn_0000_0111(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
        backend::sh4_andi(addr_of_mut!((*dc).sr_t), addr_of!((*dc).temp[0]), 1);
        backend::sh4_store_sr_rest(addr_of_mut!((*dc).sr.0), addr_of!((*dc).temp[0]), addr_of_mut!((*dc).r[0]), addr_of_mut!((*dc).r_bank[0]));
        // TODO: Recheck interrupts after SR change
    }

    (disas = "ldc.l @<REG_N>+,GBR")
    i0100_nnnn_0001_0111(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).gbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,VBR")
    i0100_nnnn_0010_0111(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).vbr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,SSR")
    i0100_nnnn_0011_0111(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).ssr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,SPC")
    i0100_nnnn_0100_0111(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).spc));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "ldc.l @<REG_N>+,RM_BANK")
    i0100_nnnn_1mmm_0111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode) & 0x7;
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).r_bank[m]));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 4);
    }

    (disas = "lds <REG_N>,MACH")
    i0100_nnnn_0000_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).mac.parts.h), addr_of!((*dc).r[n]));
    }

    (disas = "lds <REG_N>,MACL")
    i0100_nnnn_0001_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).mac.parts.l), addr_of!((*dc).r[n]));
    }

    (disas = "lds <REG_N>,PR")
    i0100_nnnn_0010_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).pr), addr_of!((*dc).r[n]));
    }

    (disas = "lds <REG_N>,FPSCR")
    i0100_nnnn_0110_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store_fpscr(addr_of_mut!((*dc).fpscr.0), addr_of!((*dc).r[n]),
                                  addr_of_mut!((*dc).fr.u32s[0]), addr_of_mut!((*dc).xf.u32s[0]));
    }

    (disas = "ldc <REG_N>,DBR")
    i0100_nnnn_1111_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).dbr), addr_of!((*dc).r[n]));
    }

    (disas = "ldc <REG_N>,SR")
    i0100_nnnn_0000_1110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_andi(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]), 1);
        backend::sh4_store_sr_rest(addr_of_mut!((*dc).sr.0), addr_of!((*dc).r[n]), addr_of_mut!((*dc).r[0]), addr_of_mut!((*dc).r_bank[0]));
        // TODO: Recheck interrupts after SR change
    }

    (disas = "ldc <REG_N>,GBR")
    i0100_nnnn_0001_1110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).gbr), addr_of!((*dc).r[n]));
    }

    (disas = "ldc <REG_N>,VBR")
    i0100_nnnn_0010_1110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).vbr), addr_of!((*dc).r[n]));
    }

    (disas = "ldc <REG_N>,SSR")
    i0100_nnnn_0011_1110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).ssr), addr_of!((*dc).r[n]));
    }

    (disas = "ldc <REG_N>,SPC")
    i0100_nnnn_0100_1110(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).spc), addr_of!((*dc).r[n]));
    }

    (disas = "ldc <REG_N>,<RM_BANK>")
    i0100_nnnn_1mmm_1110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode) & 0x7;
        backend::sh4_store32(addr_of_mut!((*dc).r_bank[m]), addr_of!((*dc).r[n]));
    }

    (disas = "shll <REG_N>")
    i0100_nnnn_0000_0000(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shll(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "shal <REG_N>")
    i0100_nnnn_0010_0000(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shal(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "cmp/pz <REG_N>")
    i0100_nnnn_0001_0001(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_cmp_pz(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]));
    }

    (disas = "shar <REG_N>")
    i0100_nnnn_0010_0001(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shar(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "rotcl <REG_N>")
    i0100_nnnn_0010_0100(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_rotcl(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "rotl <REG_N>")
    i0100_nnnn_0000_0100(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_rotl(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "cmp/pl <REG_N>")
    i0100_nnnn_0001_0101(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_cmp_pl(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[n]));
    }

    (disas = "rotcr <REG_N>")
    i0100_nnnn_0010_0101(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_rotcr(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "rotr <REG_N>")
    i0100_nnnn_0000_0101(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_rotr(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]));
    }

    (disas = "shll2 <REG_N>")
    i0100_nnnn_0000_1000(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shllf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 2);
    }

    (disas = "shll16 <REG_N>")
    i0100_nnnn_0010_1000(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shllf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 16);
    }

    (disas = "shlr8 <REG_N>")
    i0100_nnnn_0001_1001(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_shlrf(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 8);
    }

    (disas = "jmp @<REG_N>")
    i0100_nnnn_0010_1011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_jmp(dc, addr_of!((*dc).r[n]));
    }

    (disas = "jsr @<REG_N>")
    i0100_nnnn_0000_1011(dc, state, opcode) {
        let n = GetN(opcode);
        let next_pc = state.pc.wrapping_add(4);
        backend::sh4_jsr(dc, addr_of!((*dc).r[n]), next_pc);
    }

    (disas = "tas.b @<REG_N>")
    i0100_nnnn_0001_1011(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_tas(addr_of_mut!((*dc).sr_t), dc, addr_of!((*dc).r[n]));
    }

    (disas = "shad <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_shad(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "shld <REG_M>,<REG_N>")
    i0100_nnnn_mmmm_1101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_shld(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "mac.w @<REG_M>+,@<REG_N>+")
    i0100_nnnn_mmmm_1111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if n == m {
            // Special case: read from r[n], then r[n]+2
            backend::sh4_read_mems16(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
            backend::sh4_read_mems16_disp(dc, addr_of!((*dc).r[n]), 2, addr_of_mut!((*dc).temp[1]));
            backend::sh4_mac_w_mul(addr_of_mut!((*dc).mac.full), addr_of!((*dc).temp[0]), addr_of!((*dc).temp[1]));
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 2);
        } else {
            backend::sh4_read_mems16(dc, addr_of!((*dc).r[n]), addr_of_mut!((*dc).temp[0]));
            backend::sh4_read_mems16(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).temp[1]));
            backend::sh4_mac_w_mul(addr_of_mut!((*dc).mac.full), addr_of!((*dc).temp[0]), addr_of!((*dc).temp[1]));
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), 2);
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 2);
        }
    }

    // 5xxx
    (disas = "mov.l @(<disp4dw>,<REG_M>),<REG_N>")
    i0101_nnnn_mmmm_iiii(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        let disp = GetImm4(opcode) << 2;
        backend::sh4_read_mem32_disp(dc, addr_of!((*dc).r[m]), disp, addr_of_mut!((*dc).r[n]));
    }

    // 6xxx
    (disas = "mov.w @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0001(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mems16(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov.l @<REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
    }

    (disas = "mov.b @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mems8(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
        if n != m {
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 1);
        }
    }

    (disas = "mov.w @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mems16(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
        if n != m {
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 2);
        }
    }

    (disas = "mov.l @<REG_M>+,<REG_N>")
    i0110_nnnn_mmmm_0110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_read_mem32(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).r[n]));
        if n != m {
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 4);
        }
    }

    (disas = "not <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_0111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_not(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "swap.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1000(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_swapb(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "swap.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1001(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_swapw(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "negc <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_negc(addr_of_mut!((*dc).sr_t), addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "extu.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_extuw(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "exts.b <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_extsb(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    (disas = "exts.w <REG_M>,<REG_N>")
    i0110_nnnn_mmmm_1111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        backend::sh4_extsw(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[m]));
    }

    //8xxx

    (disas = "mov.w @(<PCdisp8w>),<REG_N>")
    i1001_nnnn_iiii_iiii(dc, state, opcode) {
        let n = GetN(opcode);
        let disp = GetImm8(opcode);
        let addr = (disp << 1).wrapping_add(state.pc).wrapping_add(4);
        backend::sh4_read_mems16_i(dc, addr, addr_of_mut!((*dc).r[n]));
    }


    (disas = "bt <bdisp8>")
    i1000_1001_iiii_iiii(dc, state, opcode) {
        let disp = GetSImm8(opcode);
        let target = branch_target_s8(state.pc, disp);
        let next = state.pc.wrapping_add(2);
        backend::sh4_branch_cond(dc, addr_of!((*dc).sr_t), 1, next, target);
    }

    (disas = "bt/s <bdisp8>")
    i1000_1101_iiii_iiii(dc, state, opcode) {
        let disp = GetSImm8(opcode);
        let target = branch_target_s8(state.pc, disp);
        let next = state.pc.wrapping_add(4);
        backend::sh4_branch_cond_delay(dc, addr_of!((*dc).sr_t), 1, next, target);
    }


    //bxxx
    (disas = "bsr <bdisp12>")
    i1011_iiii_iiii_iiii(dc, state, opcode) {
        let disp = GetSImm12(opcode);
        let target = branch_target_s12(state.pc, disp);
        let next_pc = state.pc.wrapping_add(4);
        backend::sh4_store32i(addr_of_mut!((*dc).pr), next_pc);
        backend::sh4_branch_delay(dc, target);
    }


    //Cxxx
    (disas = "mov.b R0,@(<disp8b>,GBR)")
    i1100_0000_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode);
        backend::sh4_write_mem8_disp(dc, addr_of!((*dc).gbr), disp, addr_of!((*dc).r[0]));
    }

    (disas = "mov.w R0,@(<disp8w>,GBR)")
    i1100_0001_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode) << 1;
        backend::sh4_write_mem16_disp(dc, addr_of!((*dc).gbr), disp, addr_of!((*dc).r[0]));
    }

    (disas = "mov.l R0,@(<disp8dw>,GBR)")
    i1100_0010_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode) << 2;
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).gbr), disp, addr_of!((*dc).r[0]));
    }

    (disas = "trapa #<imm8>")
    i1100_0011_iiii_iiii(dc, state, opcode) {
        // TRAPA - Trap instruction
        // This triggers a software exception (system call)
        // Sets TRA register to (imm << 2) and triggers exception 0x160
        // TODO: Implement full exception handling infrastructure
        let imm = GetImm8(opcode);
        panic!("TRAPA #{:#x} at PC {:#010x} - exception handling not implemented", imm, state.pc);
    }

    (disas = "mov.b @(<GBRdisp8b>),R0")
    i1100_0100_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode);
        backend::sh4_read_mems8_disp(dc, addr_of!((*dc).gbr), disp, addr_of_mut!((*dc).r[0]));
    }

    (disas = "mov.w @(<GBRdisp8w>),R0")
    i1100_0101_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode) << 1;
        backend::sh4_read_mems16_disp(dc, addr_of!((*dc).gbr), disp, addr_of_mut!((*dc).r[0]));
    }

    (disas = "mov.l @(<GBRdisp8dw>),R0")
    i1100_0110_iiii_iiii(dc, state, opcode) {
        let disp = GetImm8(opcode) << 2;
        backend::sh4_read_mem32_disp(dc, addr_of!((*dc).gbr), disp, addr_of_mut!((*dc).r[0]));
    }

    (disas = "tst #<imm8>,R0")
    i1100_1000_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_tst_imm(addr_of_mut!((*dc).sr_t), addr_of!((*dc).r[0]), imm);
    }

    (disas = "and #<imm8>,R0")
    i1100_1001_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_and_imm(addr_of_mut!((*dc).r[0]), addr_of!((*dc).r[0]), imm);
    }

    (disas = "xor #<imm8>,R0")
    i1100_1010_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_xor_imm(addr_of_mut!((*dc).r[0]), addr_of!((*dc).r[0]), imm);
    }

    (disas = "or #<imm8>,R0")
    i1100_1011_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_or_imm(addr_of_mut!((*dc).r[0]), addr_of!((*dc).r[0]), imm);
    }

    (disas = "tst.b #<imm8>,@(R0,GBR)")
    i1100_1100_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_tst_mem(addr_of_mut!((*dc).sr_t), dc, addr_of!((*dc).gbr), addr_of!((*dc).r[0]), imm as u32);
    }

    (disas = "and.b #<imm8>,@(R0,GBR)")
    i1100_1101_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_and_mem(dc, addr_of!((*dc).gbr), addr_of!((*dc).r[0]), imm as u8);
    }

    (disas = "xor.b #<imm8>,@(R0,GBR)")
    i1100_1110_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_xor_mem(dc, addr_of!((*dc).gbr), addr_of!((*dc).r[0]), imm as u8);
    }

    (disas = "or.b #<imm8>,@(R0,GBR)")
    i1100_1111_iiii_iiii(dc, state, opcode) {
        let imm = GetImm8(opcode);
        backend::sh4_or_mem(dc, addr_of!((*dc).gbr), addr_of!((*dc).r[0]), imm as u8);
    }

    //Fxxx
    (disas = "flds <FREG_N>,FPUL")
    i1111_nnnn_0001_1101(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).fpul), addr_of!((*dc).fr.u32s[n]));
    }

    (disas = "fneg <FREG_N_SD_F>")
    i1111_nnnn_0100_1101(dc, state, opcode) {
        let n = GetN(opcode);
        if !state.fpscr_PR {
            unsafe { backend::sh4_fneg(addr_of_mut!((*dc).fr.u32s[n]), addr_of!((*dc).fr.u32s[n])); };
        } else {
            let n_even = n & 0xE;
            unsafe { backend::sh4_fneg(addr_of_mut!((*dc).fr.u32s[n_even]), addr_of!((*dc).fr.u32s[n_even])); };
        }
    }

    (disas = "fabs <FREG_N_SD_F>")
    i1111_nnnn_0101_1101(dc, state, opcode) {
        let n = GetN(opcode);
        if !state.fpscr_PR {
            unsafe { backend::sh4_fabs(addr_of_mut!((*dc).fr.u32s[n]), addr_of!((*dc).fr.u32s[n])); };
        } else {
            let n_even = n & 0xE;
            unsafe { backend::sh4_fabs(addr_of_mut!((*dc).fr.u32s[n_even]), addr_of!((*dc).fr.u32s[n_even])); };
        }
    }

    (disas = "fsqrt <FREG_N>")
    i1111_nnnn_0110_1101(dc, state, opcode) {
        let n = GetN(opcode);
        if !state.fpscr_PR {
            unsafe { backend::sh4_fsqrt(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n])); };
        } else {
            let n_d = (n >> 1) & 0x7;
            backend::sh4_fsqrt_d(addr_of_mut!((*dc).fr.u32s[n_d << 1]), addr_of!((*dc).fr.u32s[n_d << 1]));
        }
    }

    (disas = "fldi0 <FREG_N>")
    i1111_nnnn_1000_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            unsafe { backend::sh4_fstsi(addr_of_mut!((*dc).fr.f32s[n]), 0.0f32); };
        }
        // else: no-op in double precision mode (C++ just returns)
    }

    (disas = "fldi1 <FREG_N>")
    i1111_nnnn_1001_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            unsafe { backend::sh4_fstsi(addr_of_mut!((*dc).fr.f32s[n]), 1.0f32); };
        }
        // else: no-op in double precision mode (C++ just returns)
    }

    (disas = "ftrv xmtrx,<FV_N>")
    i1111_nn01_1111_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = (GetN(opcode) & 0xC) as usize;
            backend::sh4_ftrv(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).xf.f32s[0]));
        } else {
            panic!("ftrv: double precision mode not supported");
        }
    }

    (disas = "fcmp/eq <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0100(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_PR {
            backend::sh4_fcmp_eq(addr_of_mut!((*dc).sr_t), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m]));
        } else {
            let n_d = (n >> 1) & 0x7;
            let m_d = (m >> 1) & 0x7;
            backend::sh4_fcmp_eq_d(addr_of_mut!((*dc).sr_t), addr_of!((*dc).fr.u32s[n_d << 1]), addr_of!((*dc).fr.u32s[m_d << 1]));
        }
    }

    (disas = "fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>")
    i1111_nnnn_mmmm_0101(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_PR {
            backend::sh4_fcmp_gt(addr_of_mut!((*dc).sr_t), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m]));
        } else {
            let n_d = (n >> 1) & 0x7;
            let m_d = (m >> 1) & 0x7;
            backend::sh4_fcmp_gt_d(addr_of_mut!((*dc).sr_t), addr_of!((*dc).fr.u32s[n_d << 1]), addr_of!((*dc).fr.u32s[m_d << 1]));
        }
    }

    (disas = "fmov.s @(R0,<REG_M>),<FREG_N_SD_A>")
    i1111_nnnn_mmmm_0110(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_SZ {
            backend::sh4_read_mem32_indexed(dc, addr_of!((*dc).r[m]), addr_of!((*dc).r[0]), addr_of_mut!((*dc).fr.u32s[n]));
        } else {
            let n_d = n >> 1;
            if (n & 0x1) == 0 {
                backend::sh4_read_mem64_indexed(dc, addr_of!((*dc).r[m]), addr_of!((*dc).r[0]), addr_of_mut!((*dc).fr.u64s[n_d]));
            } else {
                backend::sh4_read_mem64_indexed(dc, addr_of!((*dc).r[m]), addr_of!((*dc).r[0]), addr_of_mut!((*dc).xf.u64s[n_d]));
            }
        }
    }

    (disas = "fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)")
    i1111_nnnn_mmmm_0111(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_SZ {
            backend::sh4_write_mem32_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).fr.u32s[m]));
        } else {
            let m_d = m >> 1;
            if (m & 0x1) == 0 {
                backend::sh4_write_mem64_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).fr.u64s[m_d]));
            } else {
                backend::sh4_write_mem64_indexed(dc, addr_of!((*dc).r[0]), addr_of!((*dc).r[n]), addr_of!((*dc).xf.u64s[m_d]));
            }
        }
    }

    (disas = "fmov.s @<REG_M>+,<FREG_N_SD_A>")
    i1111_nnnn_mmmm_1001(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_SZ {
            backend::sh4_read_mem32(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).fr.u32s[n]));
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 4);
        } else {
            let n_d = n >> 1;
            if (n & 0x1) == 0 {
                backend::sh4_read_mem64(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).fr.u64s[n_d]));
            } else {
                backend::sh4_read_mem64(dc, addr_of!((*dc).r[m]), addr_of_mut!((*dc).xf.u64s[n_d]));
            }
            backend::sh4_addi(addr_of_mut!((*dc).r[m]), addr_of!((*dc).r[m]), 8);
        }
    }

    (disas = "fmov.s <FREG_M_SD_A>,@<REG_N>")
    i1111_nnnn_mmmm_1010(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_SZ {
            backend::sh4_write_mem32(dc, addr_of!((*dc).r[n]), addr_of!((*dc).fr.u32s[m]));
        } else {
            let m_d = m >> 1;
            if (m & 0x1) == 0 {
                backend::sh4_write_mem64(dc, addr_of!((*dc).r[n]), addr_of!((*dc).fr.u64s[m_d]));
            } else {
                backend::sh4_write_mem64(dc, addr_of!((*dc).r[n]), addr_of!((*dc).xf.u64s[m_d]));
            }
        }
    }

    (disas = "fmov.s <FREG_M_SD_A>,@-<REG_N>")
    i1111_nnnn_mmmm_1011(dc, state, opcode) {
        let n = GetN(opcode);
        let m = GetM(opcode);
        if !state.fpscr_SZ {
            backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).fr.u32s[m]));
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
        } else {
            let m_d = m >> 1;
            if (m & 0x1) == 0 {
                backend::sh4_write_mem64_disp(dc, addr_of!((*dc).r[n]), (-8i32) as u32, addr_of!((*dc).fr.u64s[m_d]));
            } else {
                backend::sh4_write_mem64_disp(dc, addr_of!((*dc).r[n]), (-8i32) as u32, addr_of!((*dc).xf.u64s[m_d]));
            }
            backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-8i32) as u32);
        }
    }


    (disas = "fcnvds <DR_N>,FPUL")
    i1111_nnnn_1011_1101(dc, state, opcode) {
        if state.fpscr_PR {
            let n_d = (GetN(opcode) >> 1) & 0x7;
            backend::sh4_fcnvds(addr_of_mut!((*dc).fpul), addr_of!((*dc).fr.u32s[n_d << 1]));
        } else {
            panic!("fcnvds: single precision is undefined behaviour");
        }
    }

    (disas = "fcnvsd FPUL,<DR_N>")
    i1111_nnnn_1010_1101(dc, state, opcode) {
        if state.fpscr_PR {
            let n_d = (GetN(opcode) >> 1) & 0x7;
            backend::sh4_fcnvsd(addr_of_mut!((*dc).fr.u32s[n_d << 1]), addr_of!((*dc).fpul));
        } else {
            panic!("fcnvsd: single precision is undefined behaviour");
        }
    }

    (disas = "fipr <FV_M>,<FV_N>")
    i1111_nnmm_1110_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = (GetN(opcode) & 0xC) as usize;
            let m = ((GetN(opcode) & 0x3) << 2) as usize;
            backend::sh4_fipr(addr_of_mut!((*dc).fr.f32s[n + 3]), addr_of!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[m]));
        } else {
            panic!("fipr: double precision mode not supported");
        }
    }

    (disas = "frchg")
    i1111_1011_1111_1101(dc, state, opcode) {
        // XOR FR bit (bit 21) in FPSCR
        backend::sh4_xor_imm(addr_of_mut!((*dc).fpscr.0), addr_of!((*dc).fpscr.0), 1 << 21);
        backend::sh4_frchg(addr_of_mut!((*dc).fr.u32s[0]), addr_of_mut!((*dc).xf.u32s[0]));
    }

    (disas = "fschg")
    i1111_0011_1111_1101(dc, state, opcode) {
        // XOR SZ bit (bit 20) in FPSCR and toggle fpscr_SZ cache
        backend::sh4_xor_imm(addr_of_mut!((*dc).fpscr.0), addr_of!((*dc).fpscr.0), 1 << 20);
        backend::sh4_fschg();
    }

    (disas = "fsts FPUL,<FREG_N>")
    i1111_nnnn_0000_1101(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).fr.u32s[n]), addr_of!((*dc).fpul));
    }

    (disas = "fsrra <FREG_N>")
    i1111_nnnn_0111_1101(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            backend::sh4_fsrra(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[n]));
        } else {
            panic!("fsrra: double precision mode not supported");
        }
    }

    (disas = "fmac <FREG_0>,<FREG_M>,<FREG_N>")
    i1111_nnnn_mmmm_1110(dc, state, opcode) {
        if !state.fpscr_PR {
            let n = GetN(opcode);
            let m = GetM(opcode);
            backend::sh4_fmac(addr_of_mut!((*dc).fr.f32s[n]), addr_of!((*dc).fr.f32s[0]), addr_of!((*dc).fr.f32s[m]));
        } else {
            panic!("fmac: double precision mode not supported");
        }
    }


    (disas = "sts PR,<REG_N>")
    i0000_nnnn_0010_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).r[n]), addr_of!((*dc).pr));
    }
    (disas = "ldc <REG_N>,SGR")
    i0100_nnnn_0011_1010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_store32(addr_of_mut!((*dc).sgr), addr_of!((*dc).r[n]));
    }

    (disas = "stc.l SGR,@-<REG_N>")
    i0100_nnnn_0011_0010(dc, state, opcode) {
        let n = GetN(opcode);
        backend::sh4_write_mem32_disp(dc, addr_of!((*dc).r[n]), (-4i32) as u32, addr_of!((*dc).sgr));
        backend::sh4_addi(addr_of_mut!((*dc).r[n]), addr_of!((*dc).r[n]), (-4i32) as u32);
    }

    (disas = "REIOS_OPCODE")
    // pub const REIOS_OPCODE: u16 = 0x085B;
    i0000_1000_0101_1011(dc, state, opcode) {
        // REIOS trap instruction - BIOS syscall emulation
        // Call trap_self if REIOS context is available
        if let Some(ref mut reios_ctx) = (*dc).reios_ctx {
            reios_ctx.trap_self(opcode, state.pc);
        }
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
    opcodes: &[sh4_opcodelistentry],
) -> (
    [fn(*mut Sh4Ctx, u16); 0x10000],
    [sh4_opcodelistentry; 0x10000],
) {
    // The sentinel is always the last element of OPCODES
    let sentinel = opcodes[opcodes.len() - 1];

    let mut ptrs: [fn(*mut Sh4Ctx, u16); 0x10000] = [sentinel.oph; 0x10000];
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
            0xF000 => (256 * 16, 0),
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
    [fn(*mut Sh4Ctx, u16); 0x10000],
    [sh4_opcodelistentry; 0x10000],
) = build_opcode_tables(OPCODES);

pub(crate) const SH4_OP_PTR: [fn(*mut Sh4Ctx, u16); 0x10000] = SH4_OP_TABLES.0;
pub(crate) const SH4_OP_DESC: [sh4_opcodelistentry; 0x10000] = SH4_OP_TABLES.1;

// // Re-export for parent (and callers) to `use sh4dec::{...}` or via the re-export in dreamcast_sh4.rs
// pub use SH4_OP_PTR as _;
// pub use SH4_OP_DESC as _;

pub fn format_disas(state: SH4DecoderState, opcode: u16) -> String {
    let mut out = unsafe { SH4_OP_DESC.get_unchecked(opcode as usize).diss }.to_string();

    // ---------------- General-purpose registers ----------------
    if out.contains("<REG_N>") {
        let n = (opcode >> 8) & 0xF;
        out = out.replace("<REG_N>", &format!("r{}", n));
    }
    if out.contains("<REG_M>") {
        let m = (opcode >> 4) & 0xF;
        out = out.replace("<REG_M>", &format!("r{}", m));
    }

    // ---------------- Immediates ----------------
    if out.contains("<IMM4>") {
        let imm = opcode & 0xF;
        out = out.replace("<IMM4>", &format!("#{}", imm));
    }
    if out.contains("<IMM8>") || out.contains("<imm8>") {
        let imm = (opcode & 0xFF) as i8;
        out = out.replace("<IMM8>", &format!("#{}", imm));
        out = out.replace("<imm8>", &format!("#{}", imm));
    }
    if out.contains("<simm8>") {
        let imm = (opcode & 0xFF) as i8;
        out = out.replace("<simm8>", &format!("{}", imm));
    }
    if out.contains("<simm8hex>") {
        let imm = (opcode & 0xFF) as i8;
        out = out.replace("<simm8hex>", &format!("{:#x}", imm as i32));
    }

    // ---------------- Displacements ----------------
    if out.contains("<bdisp8>") {
        let disp = ((opcode & 0xFF) as i8 as i32) << 1;
        out = out.replace(
            "<bdisp8>",
            &format!("{:#x}", state.pc.wrapping_add(disp as u32)),
        );
    }
    if out.contains("<bdisp12>") {
        let disp = ((opcode & 0x0FFF) as i16 as i32) << 1;
        out = out.replace(
            "<bdisp12>",
            &format!("{:#x}", state.pc.wrapping_add(disp as u32)),
        );
    }

    // 4-bit disps
    if out.contains("<disp4b>") {
        let d = opcode & 0xF;
        out = out.replace("<disp4b>", &format!("{:#x}", d));
    }
    if out.contains("<disp4w>") {
        let d = (opcode & 0xF) << 1;
        out = out.replace("<disp4w>", &format!("{:#x}", d));
    }
    if out.contains("<disp4dw>") {
        let d = (opcode & 0xF) << 2;
        out = out.replace("<disp4dw>", &format!("{:#x}", d));
    }

    // 8-bit disps
    if out.contains("<disp8b>") {
        let d = opcode & 0xFF;
        out = out.replace("<disp8b>", &format!("{:#x}", d));
    }
    if out.contains("<disp8w>") {
        let d = (opcode & 0xFF) << 1;
        out = out.replace("<disp8w>", &format!("{:#x}", d));
    }
    if out.contains("<disp8dw>") {
        let d = (opcode & 0xFF) << 2;
        out = out.replace("<disp8dw>", &format!("{:#x}", d));
    }

    // PC relative
    if out.contains("<PCdisp8d>") {
        let d = (opcode & 0xFF) << 2;
        out = out.replace(
            "<PCdisp8d>",
            &format!("{:#x}", state.pc.wrapping_add(d as u32)),
        );
    }
    if out.contains("<PCdisp8w>") {
        let d = (opcode & 0xFF) << 1;
        out = out.replace(
            "<PCdisp8w>",
            &format!("{:#x}", state.pc.wrapping_add(d as u32)),
        );
    }

    // GBR disps
    if out.contains("<GBRdisp8b>") {
        let d = opcode & 0xFF;
        out = out.replace("<GBRdisp8b>", &format!("GBR + {:#x}", d));
    }
    if out.contains("<GBRdisp8w>") {
        let d = (opcode & 0xFF) << 1;
        out = out.replace("<GBRdisp8w>", &format!("GBR + {:#x}", d));
    }
    if out.contains("<GBRdisp8dw>") {
        let d = (opcode & 0xFF) << 2;
        out = out.replace("<GBRdisp8dw>", &format!("GBR + {:#x}", d));
    }

    // ---------------- Floating-point regs ----------------
    if out.contains("<FREG_N>") {
        let n = (opcode >> 8) & 0xF;
        out = out.replace("<FREG_N>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M>") {
        let m = (opcode >> 4) & 0xF;
        out = out.replace("<FREG_M>", &format!("fr{}", m));
    }
    if out.contains("<FREG_N_SD_F>") {
        let n = (opcode >> 8) & 0xF;
        out = out.replace("<FREG_N_SD_F>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M_SD_F>") {
        let m = (opcode >> 4) & 0xF;
        out = out.replace("<FREG_M_SD_F>", &format!("fr{}", m));
    }
    if out.contains("<FREG_N_SD_A>") {
        let n = (opcode >> 8) & 0xF;
        out = out.replace("<FREG_N_SD_A>", &format!("fr{}", n));
    }
    if out.contains("<FREG_M_SD_A>") {
        let m = (opcode >> 4) & 0xF;
        out = out.replace("<FREG_M_SD_A>", &format!("fr{}", m));
    }

    if out.contains("<DR_N>") {
        let n = ((opcode >> 8) & 0xE) >> 1; // even reg pair
        out = out.replace("<DR_N>", &format!("dr{}", n));
    }
    if out.contains("<FV_N>") {
        let n = ((opcode >> 8) & 0xC) >> 2; // vector index
        out = out.replace("<FV_N>", &format!("fv{}", n));
    }
    if out.contains("<FV_M>") {
        let m = ((opcode >> 4) & 0xC) >> 2;
        out = out.replace("<FV_M>", &format!("fv{}", m));
    }

    out
}
