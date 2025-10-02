//! dreamcast_sh4.rs — 1:1 Rust translation of the provided C++/C code snippet.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::f32::consts::PI;
use std::ptr;

// -----------------------------------------------------------------------------
// types.h
// -----------------------------------------------------------------------------

type u8_ = u8;
type u16_ = u16;
type u32_ = u32;
type u64_ = u64;

type s8_ = i8;
type s16_ = i16;
type s32_ = i32;
type s64_ = i64;

type f32_ = f32;
type f64_ = f64;

// -----------------------------------------------------------------------------
// dreamcast.h
// -----------------------------------------------------------------------------

const SYSRAM_SIZE: u32 = 16 * 1024 * 1024;
const VIDEORAM_SIZE: u32 = 8 * 1024 * 1024;

const SYSRAM_MASK: u32 = SYSRAM_SIZE - 1;
const VIDEORAM_MASK: u32 = VIDEORAM_SIZE - 1;

// Keep the same field order and names for familiarity.
#[repr(C)]
pub struct sh4_opcodelistentry {
    pub oph: fn(&mut Dreamcast, u16),
    pub handler_name: &'static str,
    pub mask: u16,
    pub key: u16,
    pub diss: &'static str,
    pub is_branch: u64,
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
    pub pc: u32,

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
            pc: 0,

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

    pub OpPtr: Box<[fn(&mut Dreamcast, u16); 0x10000]>,
    pub OpDesc: Box<[*const sh4_opcodelistentry; 0x10000]>,
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
        let op_ptr = {
            let v = vec![i_not_implemented as fn(&mut Dreamcast, u16); 0x10000];
            v.into_boxed_slice().try_into().expect("len matches")
        };
        let op_desc = {
            let v = vec![ptr::null::<sh4_opcodelistentry>(); 0x10000];
            v.into_boxed_slice().try_into().expect("len matches")
        };

         Self {
            ctx: Sh4Ctx::default(),
            memmap: [ptr::null_mut(); 256],
            memmask: [0; 256],
            sys_ram,
            video_ram,
            OpPtr: op_ptr,
            OpDesc: op_desc,
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
fn GetN(str_: u16) -> u32 { ((str_ >> 8) & 0xF) as u32 }
#[inline(always)]
fn GetM(str_: u16) -> u32 { ((str_ >> 4) & 0xF) as u32 }
#[inline(always)]
fn GetImm4(str_: u16) -> u32 { (str_ & 0xF) as u32 }
#[inline(always)]
fn GetImm8(str_: u16) -> u32 { (str_ & 0xFF) as u32 }
#[inline(always)]
fn GetSImm8(str_: u16) -> i8 { (str_ & 0xFF) as i8 }
#[inline(always)]
fn GetImm12(str_: u16) -> u32 { (str_ & 0xFFF) as u32 }
#[inline(always)]
fn GetSImm12(str_: u16) -> i16 { (((GetImm12(str_) as u16) << 4) as i16) >> 4 }

// -----------------------------------------------------------------------------
// sh4impl / sh4op
// -----------------------------------------------------------------------------

fn i_not_implemented(dc: &mut Dreamcast, instr: u16) {
    let pc = dc.ctx.pc;
    let desc_ptr = dc.OpDesc[instr as usize];
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

// Helper macro to declare SH-4 opcode handlers with the correct signature.
macro_rules! sh4op {
    {
        $(
            $name:ident ($dc:ident, $instr:ident) { $($body:tt)* }
        )*
    } => {
        $(
            #[allow(non_snake_case)]
            fn $name($dc: &mut Dreamcast, $instr: u16) {
                $($body)*
            }
        )*
    };
}
// -----------------------------------------------------------------------------
// Implemented handlers (as per your snippet). Unimplemented ones are stubbed.
// -----------------------------------------------------------------------------


// Branch helpers and delay slot execution
fn branch_target_s8(op: u16, pc: u32) -> u32 {
    (GetSImm8(op) as i32 as i64 * 2 + 2 + pc as i64) as u32
}
fn branch_target_s12(op: u16, pc: u32) -> u32 {
    (GetSImm12(op) as i32 as i64 * 2 + 2 + pc as i64) as u32
}

fn ExecuteDelayslot(dc: &mut Dreamcast) {
    let addr = dc.ctx.pc;
    dc.ctx.pc = dc.ctx.pc.wrapping_add(2);

    let mut instr: u16 = 0;
    let _ = read_mem::<u16>(dc, addr, &mut instr);
    if instr != 0 {
        let f = dc.OpPtr[instr as usize];
        f(dc, instr);
    }
}

sh4op! {
    // mul.l <REG_M>,<REG_N>
    i0000_nnnn_mmmm_0111(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.macl = ((dc.ctx.r[n] as i32 as i64) * (dc.ctx.r[m] as i32 as i64)) as u32;
    }

    // nop
    i0000_0000_0000_1001(dc, instr) {
        // no-op
    }

    // sts FPUL,<REG_N>
    i0000_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] = dc.ctx.fpul;
    }

    // sts MACL,<REG_N>
    i0000_nnnn_0001_1010(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] = dc.ctx.macl;
    }

    // mov.b <REG_M>,@<REG_N>
    i0010_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        let _ = write_mem::<u8>(dc, dc.ctx.r[n], dc.ctx.r[m] as u8);
    }

    // mov.w <REG_M>,@<REG_N>
    i0010_nnnn_mmmm_0001(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        let _ = write_mem::<u16>(dc, dc.ctx.r[n], dc.ctx.r[m] as u16);
    }

    // mov.l <REG_M>,@<REG_N>
    i0010_nnnn_mmmm_0010(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        let _ = write_mem::<u32>(dc, dc.ctx.r[n], dc.ctx.r[m]);
    }

    // and <REG_M>,<REG_N>
    i0010_nnnn_mmmm_1001(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] &= dc.ctx.r[m];
    }

    // xor <REG_M>,<REG_N>
    i0010_nnnn_mmmm_1010(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] ^= dc.ctx.r[m];
    }

    // sub <REG_M>,<REG_N>
    i0011_nnnn_mmmm_1000(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] = dc.ctx.r[n].wrapping_sub(dc.ctx.r[m]);
    }

    // add <REG_M>,<REG_N>
    i0011_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] = dc.ctx.r[n].wrapping_add(dc.ctx.r[m]);
    }

    // dt <REG_N>
    i0100_nnnn_0001_0000(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] = dc.ctx.r[n].wrapping_sub(1);
        dc.ctx.sr_T = if dc.ctx.r[n] == 0 { 1 } else { 0 };
    }

    // shlr <REG_N>
    i0100_nnnn_0000_0001(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.sr_T = dc.ctx.r[n] & 1;
        dc.ctx.r[n] >>= 1;
    }

    // shll8 <REG_N>
    i0100_nnnn_0001_1000(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] <<= 8;
    }

    // shlr2 <REG_N>
    i0100_nnnn_0000_1001(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] >>= 2;
    }

    // shlr16 <REG_N>
    i0100_nnnn_0010_1001(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] >>= 16;
    }

    // mov.b @<REG_M>,<REG_N>
    i0110_nnnn_mmmm_0000(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;

        let mut data: i8 = 0;
        let _ = read_mem::<i8>(dc, dc.ctx.r[m], &mut data);
        dc.ctx.r[n] = data as i32 as u32;
    }

    // mov <REG_M>,<REG_N>
    i0110_nnnn_mmmm_0011(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] = dc.ctx.r[m];
    }

    // neg <REG_M>,<REG_N>
    i0110_nnnn_mmmm_1011(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] = (0u32).wrapping_sub(dc.ctx.r[m]);
    }

    // extu.b <REG_M>,<REG_N>
    i0110_nnnn_mmmm_1100(dc, instr) {
        let n = GetN(instr) as usize;
        let m = GetM(instr) as usize;
        dc.ctx.r[n] = (dc.ctx.r[m] as u8) as u32;
    }

    // add #<imm>,<REG_N>
    i0111_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr) as usize;
        let stmp1 = GetSImm8(instr) as i32;
        dc.ctx.r[n] = dc.ctx.r[n].wrapping_add(stmp1 as u32);
    }

    // bf <bdisp8>
    i1000_1011_iiii_iiii(dc, instr) {
        if dc.ctx.sr_T == 0 {
            dc.ctx.pc = branch_target_s8(instr, dc.ctx.pc);
        }
    }

    // bf.s <bdisp8>
    i1000_1111_iiii_iiii(dc, instr) {
        if dc.ctx.sr_T == 0 {
            let newpc = branch_target_s8(instr, dc.ctx.pc);
            ExecuteDelayslot(dc);
            dc.ctx.pc = newpc;
        }
    }

    // bra <bdisp12>
    i1010_iiii_iiii_iiii(dc, instr) {
        let newpc = branch_target_s12(instr, dc.ctx.pc);
        ExecuteDelayslot(dc);
        dc.ctx.pc = newpc;
    }

    // mova @(<disp>,PC),R0
    i1100_0111_iiii_iiii(dc, instr) {
        // ((pc+2) & ~3) + (imm8 << 2)
        let base = (dc.ctx.pc.wrapping_add(2)) & 0xFFFFFFFC;
        dc.ctx.r[0] = base.wrapping_add((GetImm8(instr) << 2) as u32);
    }

    // mov.l @(<disp>,PC),<REG_N>
    i1101_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr) as usize;
        let disp = GetImm8(instr);
        let addr = ((dc.ctx.pc.wrapping_add(2)) & 0xFFFFFFFC).wrapping_add((disp << 2) as u32);
        let mut tmp: u32 = 0;
        let _ = read_mem::<u32>(dc, addr, &mut tmp);
        dc.ctx.r[n] = tmp;
    }

    // mov #<imm>,<REG_N>
    i1110_nnnn_iiii_iiii(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.r[n] = (GetSImm8(instr) as i8) as i32 as u32;
    }

    // fadd <FREG_M>,<FREG_N> (single precision only)
    i1111_nnnn_mmmm_0000(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr) as usize;
            let m = GetM(instr) as usize;
            unsafe { dc.ctx.fr.f32s[n] = dc.ctx.fr.f32s[n] + dc.ctx.fr.f32s[m] };
        } else {
            debug_assert!(false);
        }
    }

    // fmul <FREG_M>,<FREG_N>
    i1111_nnnn_mmmm_0010(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr) as usize;
            let m = GetM(instr) as usize;
            unsafe { dc.ctx.fr.f32s[n] = dc.ctx.fr.f32s[n] * dc.ctx.fr.f32s[m]; }
        } else {
            debug_assert!(false);
        }
    }

    // fdiv <FREG_M>,<FREG_N>
    i1111_nnnn_mmmm_0011(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr) as usize;
            let m = GetM(instr) as usize;
            unsafe { dc.ctx.fr.f32s[n] = dc.ctx.fr.f32s[n] / dc.ctx.fr.f32s[m]; }
        } else {
            debug_assert!(false);
        }
    }

    // fmov.s @<REG_M>,<FREG_N>
    i1111_nnnn_mmmm_1000(dc, instr) {
        if dc.ctx.fpscr_SZ == 0 {
            let n = GetN(instr) as usize;
            let m = GetM(instr) as usize;
            let mut tmp: u32 = 0;
            let _ = read_mem::<u32>(dc, dc.ctx.r[m], &mut tmp);
            unsafe { dc.ctx.fr.u32s[n] = tmp; }
        } else {
            debug_assert!(false);
        }
    }

    // fmov <FREG_M>,<FREG_N>
    i1111_nnnn_mmmm_1100(dc, instr) {
        if dc.ctx.fpscr_SZ == 0 {
            let n = GetN(instr) as usize;
            let m = GetM(instr) as usize;
            unsafe { dc.ctx.fr.f32s[n] = dc.ctx.fr.f32s[m] };
        } else {
            debug_assert!(false);
        }
    }

    // FSCA FPUL, DRn (1111_nnn0_1111_1101)
    i1111_nnn0_1111_1101(dc, instr) {
        let n = (GetN(instr) & 0xE) as usize;
        if dc.ctx.fpscr_PR == 0 {
            let pi_index = dc.ctx.fpul & 0xFFFF;
            // rads = (index / (65536/2)) * pi
            let rads = (pi_index as f32) / (65536.0f32 / 2.0) * PI;
            unsafe {
                dc.ctx.fr.f32s[n + 0] = rads.sin();
                dc.ctx.fr.f32s[n + 1] = rads.cos();
            }
            
        } else {
            debug_assert!(false);
        }
    }

    // float FPUL,<FREG_N>
    i1111_nnnn_0010_1101(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr) as usize;
            unsafe { dc.ctx.fr.f32s[n] = (dc.ctx.fpul as i32) as f32; }
        } else {
            debug_assert!(false);
        }
    }

    // ftrc <FREG_N>, FPUL
    i1111_nnnn_0011_1101(dc, instr) {
        if dc.ctx.fpscr_PR == 0 {
            let n = GetN(instr) as usize;
            // saturate to 0x7FFFFFBF as in original snippet
            let v = unsafe { dc.ctx.fr.f32s[n] };
            let clamped = v.min(0x7FFFFFBF as f32);
            let mut as_i = clamped as i32 as u32;
            if as_i == 0x80000000 {
                if v > 0.0 {
                    as_i = as_i.wrapping_sub(1);
                }
            }
            dc.ctx.fpul = as_i;
        } else {
            debug_assert!(false);
        }
    }

    // lds <REG_N>,FPUL
    i0100_nnnn_0101_1010(dc, instr) {
        let n = GetN(instr) as usize;
        dc.ctx.fpul = dc.ctx.r[n];
    }

}
// -----------------------------------------------------------------------------
// Declare all unimplemented handlers as stubs (1:1 names) ---------------------
// -----------------------------------------------------------------------------

// Macro to mass-declare stubs based on the identifiers present in your code.
macro_rules! declare_stubs {
    ( $( $name:ident ),* $(,)? ) => {
        $(
            fn $name(dc: &mut Dreamcast, instr: u16) { i_not_implemented(dc, instr); }
        )*
    };
}

// From your snippet (all sh4op declarations + any handlers that appear in the table but not implemented above):
declare_stubs!(
    // 0xxx prefix (partial list per snippet)
    i0000_nnnn_0000_0010, i0000_nnnn_0001_0010, i0000_nnnn_0010_0010, i0000_nnnn_0011_0010,
    i0000_nnnn_0011_1010, i0000_nnnn_0100_0010, i0000_nnnn_1mmm_0010, i0000_nnnn_0010_0011,
    i0000_nnnn_0000_0013 /* bogus to avoid collision */,

    i0000_nnnn_0000_0011, i0000_nnnn_1100_0011, i0000_nnnn_1001_0011, i0000_nnnn_1010_0011,
    i0000_nnnn_1011_0011, i0000_nnnn_1000_0011, i0000_nnnn_mmmm_0100, i0000_nnnn_mmmm_0101,
    i0000_nnnn_mmmm_0110,
    i0000_0000_0010_1000, i0000_0000_0100_1000, i0000_0000_0000_1000, i0000_0000_0011_1000,
    i0000_0000_0101_1000, i0000_0000_0001_1000, i0000_0000_0001_1001, i0000_nnnn_0010_1001,
    // nop already implemented
    i0000_nnnn_0110_1010, i0000_nnnn_1111_1010, i0000_nnnn_0000_1010,
    // rtes/rts/sleep
    i0000_0000_0010_1011, i0000_0000_0000_1011, i0000_0000_0001_1011,
    i0000_nnnn_mmmm_1100, i0000_nnnn_mmmm_1101, i0000_nnnn_mmmm_1110, i0000_nnnn_mmmm_1111,
    i0001_nnnn_mmmm_iiii,

    // 2xxx additional
    i0010_nnnn_mmmm_0100, i0010_nnnn_mmmm_0101, i0010_nnnn_mmmm_0110, i0010_nnnn_mmmm_0111,
    i0010_nnnn_mmmm_1000, i0010_nnnn_mmmm_1011, i0010_nnnn_mmmm_1100, i0010_nnnn_mmmm_1101,
    i0010_nnnn_mmmm_1110, i0010_nnnn_mmmm_1111,

    // 3xxx others
    i0011_nnnn_mmmm_0000, i0011_nnnn_mmmm_0010, i0011_nnnn_mmmm_0011, i0011_nnnn_mmmm_0100,
    i0011_nnnn_mmmm_0101, i0011_nnnn_mmmm_0110, i0011_nnnn_mmmm_0111, i0011_nnnn_mmmm_1010,
    i0011_nnnn_mmmm_1011, i0011_nnnn_mmmm_1101, i0011_nnnn_mmmm_1110, i0011_nnnn_mmmm_1111,

    // 4xxx
    i0100_nnnn_0101_0010, i0100_nnnn_0110_0010, i0100_nnnn_0000_0010, i0100_nnnn_0001_0010,
    i0100_nnnn_0010_0010, i0100_nnnn_1111_0010, i0100_nnnn_0000_0011, i0100_nnnn_0001_0011,
    i0100_nnnn_0010_0011, i0100_nnnn_0011_0011, i0100_nnnn_0011_0010, i0100_nnnn_0100_0011,
    i0100_nnnn_1mmm_0011, i0100_nnnn_0000_0110, i0100_nnnn_0001_0110, i0100_nnnn_0010_0110,
    i0100_nnnn_0101_0110, i0100_nnnn_0110_0110, i0100_nnnn_1111_0110, i0100_nnnn_0000_0111,
    i0100_nnnn_0001_0111, i0100_nnnn_0010_0111, i0100_nnnn_0011_0111, i0100_nnnn_0011_0110,
    i0100_nnnn_0100_0111, i0100_nnnn_1mmm_0111, i0100_nnnn_0000_1010, i0100_nnnn_0001_1010,
    i0100_nnnn_0010_1010, /* i0100_nnnn_0101_1010 implemented above */ i0100_nnnn_0110_1010,
    i0100_nnnn_1111_1010, i0100_nnnn_0000_1110, i0100_nnnn_0001_1110, i0100_nnnn_0010_1110,
    i0100_nnnn_0011_1110, i0100_nnnn_0100_1110, i0100_nnnn_1mmm_1110, i0100_nnnn_0000_0000,
    i0100_nnnn_0010_0000, i0100_nnnn_0001_0001, i0100_nnnn_0010_0001, i0100_nnnn_0010_0100,
    i0100_nnnn_0000_0100, i0100_nnnn_0001_0101, i0100_nnnn_0010_0101, i0100_nnnn_0000_0101,
    i0100_nnnn_0000_1000, /* i0100_nnnn_0001_1000 impl */ i0100_nnnn_0010_1000,
    /* i0100_nnnn_0000_1001 impl */ i0100_nnnn_0001_1001, /* i0100_nnnn_0010_1001 impl */
    i0100_nnnn_0010_1011, i0100_nnnn_0000_1011, i0100_nnnn_0001_1011, i0100_nnnn_mmmm_1100,
    i0100_nnnn_mmmm_1101, i0100_nnnn_mmmm_1111,

    // 5xxx
    i0101_nnnn_mmmm_iiii,

    // 6xxx
    i0110_nnnn_mmmm_0001, i0110_nnnn_mmmm_0010, i0110_nnnn_mmmm_0100, i0110_nnnn_mmmm_0101,
    i0110_nnnn_mmmm_0110, i0110_nnnn_mmmm_0111, i0110_nnnn_mmmm_1000, i0110_nnnn_mmmm_1001,
    i0110_nnnn_mmmm_1010, /* i0110_nnnn_mmmm_1011 impl */ /* i0110_nnnn_mmmm_1100 impl */
    i0110_nnnn_mmmm_1101, i0110_nnnn_mmmm_1110, i0110_nnnn_mmmm_1111,

    // 8xxx
    i1000_1001_iiii_iiii, i1000_1101_iiii_iiii, i1000_1000_iiii_iiii,
    i1000_0000_mmmm_iiii, i1000_0001_mmmm_iiii, i1000_0100_mmmm_iiii, i1000_0101_mmmm_iiii,

    // 9xxx
    i1001_nnnn_iiii_iiii,

    // Bxxx
    i1011_iiii_iiii_iiii,

    // Cxxx
    i1100_0000_iiii_iiii, i1100_0001_iiii_iiii, i1100_0010_iiii_iiii, i1100_0011_iiii_iiii,
    i1100_0100_iiii_iiii, i1100_0101_iiii_iiii, i1100_0110_iiii_iiii,
    i1100_1000_iiii_iiii, i1100_1001_iiii_iiii, i1100_1010_iiii_iiii, i1100_1011_iiii_iiii,
    i1100_1100_iiii_iiii, i1100_1101_iiii_iiii, i1100_1110_iiii_iiii, i1100_1111_iiii_iiii,

    // Fxxx
    /* i1111_nnnn_mmmm_0000 impl */ i1111_nnnn_mmmm_0001,
    /* i1111_nnnn_mmmm_0010 impl */ /* i1111_nnnn_mmmm_0011 impl */
    i1111_nnnn_mmmm_0100, i1111_nnnn_mmmm_0101, i1111_nnnn_mmmm_0110, i1111_nnnn_mmmm_0111,
    /* i1111_nnnn_mmmm_1000 impl */ i1111_nnnn_mmmm_1001, i1111_nnnn_mmmm_1010,
    i1111_nnnn_mmmm_1011, /* i1111_nnnn_mmmm_1100 impl */ i1111_nnnn_0101_1101,
    /* i1111_nnn0_1111_1101 impl */ i1111_nnnn_1011_1101, i1111_nnnn_1010_1101,
    i1111_nnmm_1110_1101, i1111_nnnn_1000_1101, i1111_nnnn_1001_1101, i1111_nnnn_0001_1101,
    /* i1111_nnnn_0010_1101 impl */ i1111_nnnn_0100_1101, i1111_1011_1111_1101,
    i1111_0011_1111_1101, i1111_nnnn_0110_1101, /* i1111_nnnn_0011_1101 impl */
    i1111_nnnn_0000_1101, i1111_nn01_1111_1101, i1111_nnnn_1110_1110 /*typo guard*/,
    i1111_nnnn_0111_1101, i1111_nnnn_mmmm_1110,

    i0000_nnnn_0010_1010, i0100_nnnn_0011_1010,
);

// -----------------------------------------------------------------------------
// Opcode list (array) — translated 1:1 from your snippet
// -----------------------------------------------------------------------------

static MISSING_OPCODE: sh4_opcodelistentry = sh4_opcodelistentry {
    oph: i_not_implemented,
    handler_name: "i_not_implemented",
    mask: 0,
    key: 0,
    diss: "missing",
    is_branch: 0,
};

pub static OPCODES: &[sh4_opcodelistentry] = &[
    sh4_opcodelistentry { oph: i0000_nnnn_0010_0011, handler_name: "i0000_nnnn_0010_0011", mask: MASK_N, key: 0x0023, diss: "braf <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0000_0011, handler_name: "i0000_nnnn_0000_0011", mask: MASK_N, key: 0x0003, diss: "bsrf <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1100_0011, handler_name: "i0000_nnnn_1100_0011", mask: MASK_N, key: 0x00C3, diss: "movca.l R0, @<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1001_0011, handler_name: "i0000_nnnn_1001_0011", mask: MASK_N, key: 0x0093, diss: "ocbi @<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1010_0011, handler_name: "i0000_nnnn_1010_0011", mask: MASK_N, key: 0x00A3, diss: "ocbp @<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1011_0011, handler_name: "i0000_nnnn_1011_0011", mask: MASK_N, key: 0x00B3, diss: "ocbwb @<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1000_0011, handler_name: "i0000_nnnn_1000_0011", mask: MASK_N, key: 0x0083, diss: "pref @<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_0111, handler_name: "i0000_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x0007, diss: "mul.l <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0010_1000, handler_name: "i0000_0000_0010_1000", mask: MASK_NONE, key: 0x0028, diss: "clrmac", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0100_1000, handler_name: "i0000_0000_0100_1000", mask: MASK_NONE, key: 0x0048, diss: "clrs", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0000_1000, handler_name: "i0000_0000_0000_1000", mask: MASK_NONE, key: 0x0008, diss: "clrt", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0011_1000, handler_name: "i0000_0000_0011_1000", mask: MASK_NONE, key: 0x0038, diss: "ldtlb", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0101_1000, handler_name: "i0000_0000_0101_1000", mask: MASK_NONE, key: 0x0058, diss: "sets", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0001_1000, handler_name: "i0000_0000_0001_1000", mask: MASK_NONE, key: 0x0018, diss: "sett", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0001_1001, handler_name: "i0000_0000_0001_1001", mask: MASK_NONE, key: 0x0019, diss: "div0u", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0010_1001, handler_name: "i0000_nnnn_0010_1001", mask: MASK_N, key: 0x0029, diss: "movt <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0000_1001, handler_name: "i0000_0000_0000_1001", mask: MASK_NONE, key: 0x0009, diss: "nop", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0010_1011, handler_name: "i0000_0000_0010_1011", mask: MASK_NONE, key: 0x002B, diss: "rte", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0000_1011, handler_name: "i0000_0000_0000_1011", mask: MASK_NONE, key: 0x000B, diss: "rts", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_0000_0001_1011, handler_name: "i0000_0000_0001_1011", mask: MASK_NONE, key: 0x001B, diss: "sleep", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_1111, handler_name: "i0000_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x000F, diss: "mac.l @<REG_M>+,@<REG_N>+", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0111, handler_name: "i0010_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x2007, diss: "div0s <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1000, handler_name: "i0010_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x2008, diss: "tst <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1001, handler_name: "i0010_nnnn_mmmm_1001", mask: MASK_N_M, key: 0x2009, diss: "and <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1010, handler_name: "i0010_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x200A, diss: "xor <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1011, handler_name: "i0010_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x200B, diss: "or <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1100, handler_name: "i0010_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x200C, diss: "cmp/str <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1101, handler_name: "i0010_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x200D, diss: "xtrct <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1110, handler_name: "i0010_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x200E, diss: "mulu.w <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_1111, handler_name: "i0010_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x200F, diss: "muls.w <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0000, handler_name: "i0011_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x3000, diss: "cmp/eq <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0010, handler_name: "i0011_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x3002, diss: "cmp/hs <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0011, handler_name: "i0011_nnnn_mmmm_0011", mask: MASK_N_M, key: 0x3003, diss: "cmp/ge <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0100, handler_name: "i0011_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x3004, diss: "div1 <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0101, handler_name: "i0011_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x3005, diss: "dmulu.l <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0110, handler_name: "i0011_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x3006, diss: "cmp/hi <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_0111, handler_name: "i0011_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x3007, diss: "cmp/gt <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1000, handler_name: "i0011_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x3008, diss: "sub <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1010, handler_name: "i0011_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x300A, diss: "subc <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1011, handler_name: "i0011_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x300B, diss: "subv <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1100, handler_name: "i0011_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x300C, diss: "add <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1101, handler_name: "i0011_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x300D, diss: "dmuls.l <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1110, handler_name: "i0011_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x300E, diss: "addc <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0011_nnnn_mmmm_1111, handler_name: "i0011_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x300F, diss: "addv <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_0100, handler_name: "i0000_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x0004, diss: "mov.b <REG_M>,@(R0,<REG_N>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_0101, handler_name: "i0000_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x0005, diss: "mov.w <REG_M>,@(R0,<REG_N>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_0110, handler_name: "i0000_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x0006, diss: "mov.l <REG_M>,@(R0,<REG_N>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_1100, handler_name: "i0000_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x000C, diss: "mov.b @(R0,<REG_M>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_1101, handler_name: "i0000_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x000D, diss: "mov.w @(R0,<REG_M>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_mmmm_1110, handler_name: "i0000_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x000E, diss: "mov.l @(R0,<REG_M>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0001_nnnn_mmmm_iiii, handler_name: "i0001_nnnn_mmmm_iiii", mask: MASK_N_IMM8, key: 0x1000, diss: "mov.l <REG_M>,@(<disp4dw>,<REG_N>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i0101_nnnn_mmmm_iiii, handler_name: "i0101_nnnn_mmmm_iiii", mask: MASK_N_M_IMM4, key: 0x5000, diss: "mov.l @(<disp4dw>,<REG_M>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0000, handler_name: "i0010_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x2000, diss: "mov.b <REG_M>,@<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0001, handler_name: "i0010_nnnn_mmmm_0001", mask: MASK_N_M, key: 0x2001, diss: "mov.w <REG_M>,@<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0010, handler_name: "i0010_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x2002, diss: "mov.l <REG_M>,@<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0000, handler_name: "i0110_nnnn_mmmm_0000", mask: MASK_N_M, key: 0x6000, diss: "mov.b @<REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0001, handler_name: "i0110_nnnn_mmmm_0001", mask: MASK_N_M, key: 0x6001, diss: "mov.w @<REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0010, handler_name: "i0110_nnnn_mmmm_0010", mask: MASK_N_M, key: 0x6002, diss: "mov.l @<REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0100, handler_name: "i0010_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x2004, diss: "mov.b <REG_M>,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0101, handler_name: "i0010_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x2005, diss: "mov.w <REG_M>,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0010_nnnn_mmmm_0110, handler_name: "i0010_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x2006, diss: "mov.l <REG_M>,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0100, handler_name: "i0110_nnnn_mmmm_0100", mask: MASK_N_M, key: 0x6004, diss: "mov.b @<REG_M>+,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0101, handler_name: "i0110_nnnn_mmmm_0101", mask: MASK_N_M, key: 0x6005, diss: "mov.w @<REG_M>+,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0110, handler_name: "i0110_nnnn_mmmm_0110", mask: MASK_N_M, key: 0x6006, diss: "mov.l @<REG_M>+,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1000_0000_mmmm_iiii, handler_name: "i1000_0000_mmmm_iiii", mask: MASK_IMM8, key: 0x8000, diss: "mov.b R0,@(<disp4b>,<REG_M>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1000_0001_mmmm_iiii, handler_name: "i1000_0001_mmmm_iiii", mask: MASK_IMM8, key: 0x8100, diss: "mov.w R0,@(<disp4w>,<REG_M>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1000_0100_mmmm_iiii, handler_name: "i1000_0100_mmmm_iiii", mask: MASK_IMM8, key: 0x8400, diss: "mov.b @(<disp4b>,<REG_M>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1000_0101_mmmm_iiii, handler_name: "i1000_0101_mmmm_iiii", mask: MASK_IMM8, key: 0x8500, diss: "mov.w @(<disp4w>,<REG_M>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1001_nnnn_iiii_iiii, handler_name: "i1001_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0x9000, diss: "mov.w @(<PCdisp8w>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0000_iiii_iiii, handler_name: "i1100_0000_iiii_iiii", mask: MASK_IMM8, key: 0xC000, diss: "mov.b R0,@(<disp8b>,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0001_iiii_iiii, handler_name: "i1100_0001_iiii_iiii", mask: MASK_IMM8, key: 0xC100, diss: "mov.w R0,@(<disp8w>,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0010_iiii_iiii, handler_name: "i1100_0010_iiii_iiii", mask: MASK_IMM8, key: 0xC200, diss: "mov.l R0,@(<disp8dw>,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0100_iiii_iiii, handler_name: "i1100_0100_iiii_iiii", mask: MASK_IMM8, key: 0xC400, diss: "mov.b @(<GBRdisp8b>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0101_iiii_iiii, handler_name: "i1100_0101_iiii_iiii", mask: MASK_IMM8, key: 0xC500, diss: "mov.w @(<GBRdisp8w>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0110_iiii_iiii, handler_name: "i1100_0110_iiii_iiii", mask: MASK_IMM8, key: 0xC600, diss: "mov.l @(<GBRdisp8dw>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1101_nnnn_iiii_iiii, handler_name: "i1101_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0xD000, diss: "mov.l @(<PCdisp8d>),<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0011, handler_name: "i0110_nnnn_mmmm_0011", mask: MASK_N_M, key: 0x6003, diss: "mov <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_0111_iiii_iiii, handler_name: "i1100_0111_iiii_iiii", mask: MASK_IMM8, key: 0xC700, diss: "mova @(<PCdisp8d>),R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1110_nnnn_iiii_iiii, handler_name: "i1110_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0xE000, diss: "mov #<simm8hex>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0101_0010, handler_name: "i0100_nnnn_0101_0010", mask: MASK_N, key: 0x4052, diss: "sts.l FPUL,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0110_0010, handler_name: "i0100_nnnn_0110_0010", mask: MASK_N, key: 0x4062, diss: "sts.l FPSCR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0010, handler_name: "i0100_nnnn_0000_0010", mask: MASK_N, key: 0x4002, diss: "sts.l MACH,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0010, handler_name: "i0100_nnnn_0001_0010", mask: MASK_N, key: 0x4012, diss: "sts.l MACL,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0010, handler_name: "i0100_nnnn_0010_0010", mask: MASK_N, key: 0x4022, diss: "sts.l PR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1111_0010, handler_name: "i0100_nnnn_1111_0010", mask: MASK_N, key: 0x40F2, diss: "stc.l DBR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_0010, handler_name: "i0100_nnnn_0011_0010", mask: MASK_N, key: 0x4032, diss: "stc.l SGR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0011, handler_name: "i0100_nnnn_0000_0011", mask: MASK_N, key: 0x4003, diss: "stc.l SR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0011, handler_name: "i0100_nnnn_0001_0011", mask: MASK_N, key: 0x4013, diss: "stc.l GBR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0011, handler_name: "i0100_nnnn_0010_0011", mask: MASK_N, key: 0x4023, diss: "stc.l VBR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_0011, handler_name: "i0100_nnnn_0011_0011", mask: MASK_N, key: 0x4033, diss: "stc.l SSR,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0100_0011, handler_name: "i0100_nnnn_0100_0011", mask: MASK_N, key: 0x4043, diss: "stc.l SPC,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1mmm_0011, handler_name: "i0100_nnnn_1mmm_0011", mask: MASK_N_ML3BIT, key: 0x4083, diss: "stc <RM_BANK>,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0110, handler_name: "i0100_nnnn_0000_0110", mask: MASK_N, key: 0x4006, diss: "lds.l @<REG_N>+,MACH", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0110, handler_name: "i0100_nnnn_0001_0110", mask: MASK_N, key: 0x4016, diss: "lds.l @<REG_N>+,MACL", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0110, handler_name: "i0100_nnnn_0010_0110", mask: MASK_N, key: 0x4026, diss: "lds.l @<REG_N>+,PR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_0110, handler_name: "i0100_nnnn_0011_0110", mask: MASK_N, key: 0x4036, diss: "ldc.l @<REG_N>+,SGR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0101_0110, handler_name: "i0100_nnnn_0101_0110", mask: MASK_N, key: 0x4056, diss: "lds.l @<REG_N>+,FPUL", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0110_0110, handler_name: "i0100_nnnn_0110_0110", mask: MASK_N, key: 0x4066, diss: "lds.l @<REG_N>+,FPSCR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1111_0110, handler_name: "i0100_nnnn_1111_0110", mask: MASK_N, key: 0x40F6, diss: "ldc.l @<REG_N>+,DBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0111, handler_name: "i0100_nnnn_0000_0111", mask: MASK_N, key: 0x4007, diss: "ldc.l @<REG_N>+,SR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0111, handler_name: "i0100_nnnn_0001_0111", mask: MASK_N, key: 0x4017, diss: "ldc.l @<REG_N>+,GBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0111, handler_name: "i0100_nnnn_0010_0111", mask: MASK_N, key: 0x4027, diss: "ldc.l @<REG_N>+,VBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_0111, handler_name: "i0100_nnnn_0011_0111", mask: MASK_N, key: 0x4037, diss: "ldc.l @<REG_N>+,SSR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0100_0111, handler_name: "i0100_nnnn_0100_0111", mask: MASK_N, key: 0x4047, diss: "ldc.l @<REG_N>+,SPC", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1mmm_0111, handler_name: "i0100_nnnn_1mmm_0111", mask: MASK_N_ML3BIT, key: 0x4087, diss: "ldc.l @<REG_N>+,RM_BANK", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0000_0010, handler_name: "i0000_nnnn_0000_0010", mask: MASK_N, key: 0x0002, diss: "stc SR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0001_0010, handler_name: "i0000_nnnn_0001_0010", mask: MASK_N, key: 0x0012, diss: "stc GBR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0010_0010, handler_name: "i0000_nnnn_0010_0010", mask: MASK_N, key: 0x0022, diss: "stc VBR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0011_0010, handler_name: "i0000_nnnn_0011_0010", mask: MASK_N, key: 0x0032, diss: "stc SSR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0100_0010, handler_name: "i0000_nnnn_0100_0010", mask: MASK_N, key: 0x0042, diss: "stc SPC,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1mmm_0010, handler_name: "i0000_nnnn_1mmm_0010", mask: MASK_N_ML3BIT, key: 0x0082, diss: "stc RM_BANK,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0000_1010, handler_name: "i0000_nnnn_0000_1010", mask: MASK_N, key: 0x000A, diss: "sts MACH,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0001_1010, handler_name: "i0000_nnnn_0001_1010", mask: MASK_N, key: 0x001A, diss: "sts MACL,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0010_1010, handler_name: "i0000_nnnn_0010_1010", mask: MASK_N, key: 0x002A, diss: "sts PR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0011_1010, handler_name: "i0000_nnnn_0011_1010", mask: MASK_N, key: 0x003A, diss: "sts SGR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0101_1010, handler_name: "i0000_nnnn_0101_1010", mask: MASK_N, key: 0x005A, diss: "sts FPUL,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_0110_1010, handler_name: "i0000_nnnn_0110_1010", mask: MASK_N, key: 0x006A, diss: "sts FPSCR,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0000_nnnn_1111_1010, handler_name: "i0000_nnnn_1111_1010", mask: MASK_N, key: 0x00FA, diss: "sts DBR,<REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0100_nnnn_0000_1010, handler_name: "i0100_nnnn_0000_1010", mask: MASK_N, key: 0x400A, diss: "lds <REG_N>,MACH", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_1010, handler_name: "i0100_nnnn_0001_1010", mask: MASK_N, key: 0x401A, diss: "lds <REG_N>,MACL", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_1010, handler_name: "i0100_nnnn_0010_1010", mask: MASK_N, key: 0x402A, diss: "lds <REG_N>,PR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_1010, handler_name: "i0100_nnnn_0011_1010", mask: MASK_N, key: 0x403A, diss: "ldc <REG_N>,SGR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0101_1010, handler_name: "i0100_nnnn_0101_1010", mask: MASK_N, key: 0x405A, diss: "lds <REG_N>,FPUL", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0110_1010, handler_name: "i0100_nnnn_0110_1010", mask: MASK_N, key: 0x406A, diss: "lds <REG_N>,FPSCR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1111_1010, handler_name: "i0100_nnnn_1111_1010", mask: MASK_N, key: 0x40FA, diss: "ldc <REG_N>,DBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_1110, handler_name: "i0100_nnnn_0000_1110", mask: MASK_N, key: 0x400E, diss: "ldc <REG_N>,SR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_1110, handler_name: "i0100_nnnn_0001_1110", mask: MASK_N, key: 0x401E, diss: "ldc <REG_N>,GBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_1110, handler_name: "i0100_nnnn_0010_1110", mask: MASK_N, key: 0x402E, diss: "ldc <REG_N>,VBR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0011_1110, handler_name: "i0100_nnnn_0011_1110", mask: MASK_N, key: 0x403E, diss: "ldc <REG_N>,SSR", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0100_1110, handler_name: "i0100_nnnn_0100_1110", mask: MASK_N, key: 0x404E, diss: "ldc <REG_N>,SPC", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_1mmm_1110, handler_name: "i0100_nnnn_1mmm_1110", mask: MASK_N_ML3BIT, key: 0x408E, diss: "ldc <REG_N>,<RM_BANK>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0100_nnnn_0000_0000, handler_name: "i0100_nnnn_0000_0000", mask: MASK_N, key: 0x4000, diss: "shll <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0000, handler_name: "i0100_nnnn_0001_0000", mask: MASK_N, key: 0x4010, diss: "dt <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0000, handler_name: "i0100_nnnn_0010_0000", mask: MASK_N, key: 0x4020, diss: "shal <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0001, handler_name: "i0100_nnnn_0000_0001", mask: MASK_N, key: 0x4001, diss: "shlr <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0001, handler_name: "i0100_nnnn_0001_0001", mask: MASK_N, key: 0x4011, diss: "cmp/pz <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0001, handler_name: "i0100_nnnn_0010_0001", mask: MASK_N, key: 0x4021, diss: "shar <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0100, handler_name: "i0100_nnnn_0010_0100", mask: MASK_N, key: 0x4024, diss: "rotcl <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0100, handler_name: "i0100_nnnn_0000_0100", mask: MASK_N, key: 0x4004, diss: "rotl <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_0101, handler_name: "i0100_nnnn_0001_0101", mask: MASK_N, key: 0x4015, diss: "cmp/pl <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_0101, handler_name: "i0100_nnnn_0010_0101", mask: MASK_N, key: 0x4025, diss: "rotcr <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_0101, handler_name: "i0100_nnnn_0000_0101", mask: MASK_N, key: 0x4005, diss: "rotr <REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0100_nnnn_0000_1000, handler_name: "i0100_nnnn_0000_1000", mask: MASK_N, key: 0x4008, diss: "shll2 <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_1000, handler_name: "i0100_nnnn_0001_1000", mask: MASK_N, key: 0x4018, diss: "shll8 <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_1000, handler_name: "i0100_nnnn_0010_1000", mask: MASK_N, key: 0x4028, diss: "shll16 <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_1001, handler_name: "i0100_nnnn_0000_1001", mask: MASK_N, key: 0x4009, diss: "shlr2 <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_1001, handler_name: "i0100_nnnn_0001_1001", mask: MASK_N, key: 0x4019, diss: "shlr8 <REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_0010_1001, handler_name: "i0100_nnnn_0010_1001", mask: MASK_N, key: 0x4029, diss: "shlr16 <REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0100_nnnn_0010_1011, handler_name: "i0100_nnnn_0010_1011", mask: MASK_N, key: 0x402B, diss: "jmp @<REG_N>", is_branch: 1 },
    sh4_opcodelistentry { oph: i0100_nnnn_0000_1011, handler_name: "i0100_nnnn_0000_1011", mask: MASK_N, key: 0x400B, diss: "jsr @<REG_N>", is_branch: 1 },
    sh4_opcodelistentry { oph: i0100_nnnn_0001_1011, handler_name: "i0100_nnnn_0001_1011", mask: MASK_N, key: 0x401B, diss: "tas.b @<REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0100_nnnn_mmmm_1100, handler_name: "i0100_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x400C, diss: "shad <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_mmmm_1101, handler_name: "i0100_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x400D, diss: "shld <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0100_nnnn_mmmm_1111, handler_name: "i0100_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x400F, diss: "mac.w @<REG_M>+,@<REG_N>+", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_0111, handler_name: "i0110_nnnn_mmmm_0111", mask: MASK_N_M, key: 0x6007, diss: "not <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1000, handler_name: "i0110_nnnn_mmmm_1000", mask: MASK_N_M, key: 0x6008, diss: "swap.b <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1001, handler_name: "i0110_nnnn_mmmm_1001", mask: MASK_N_M, key: 0x6009, diss: "swap.w <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1010, handler_name: "i0110_nnnn_mmmm_1010", mask: MASK_N_M, key: 0x600A, diss: "negc <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1011, handler_name: "i0110_nnnn_mmmm_1011", mask: MASK_N_M, key: 0x600B, diss: "neg <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1100, handler_name: "i0110_nnnn_mmmm_1100", mask: MASK_N_M, key: 0x600C, diss: "extu.b <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1101, handler_name: "i0110_nnnn_mmmm_1101", mask: MASK_N_M, key: 0x600D, diss: "extu.w <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1110, handler_name: "i0110_nnnn_mmmm_1110", mask: MASK_N_M, key: 0x600E, diss: "exts.b <REG_M>,<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i0110_nnnn_mmmm_1111, handler_name: "i0110_nnnn_mmmm_1111", mask: MASK_N_M, key: 0x600F, diss: "exts.w <REG_M>,<REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i0111_nnnn_iiii_iiii, handler_name: "i0111_nnnn_iiii_iiii", mask: MASK_N_IMM8, key: 0x7000, diss: "add #<simm8>,<REG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1000_1011_iiii_iiii, handler_name: "i1000_1011_iiii_iiii", mask: MASK_IMM8, key: 0x8B00, diss: "bf <bdisp8>", is_branch: 1 },
    sh4_opcodelistentry { oph: i1000_1111_iiii_iiii, handler_name: "i1000_1111_iiii_iiii", mask: MASK_IMM8, key: 0x8F00, diss: "bf.s <bdisp8>", is_branch: 2 },
    sh4_opcodelistentry { oph: i1000_1001_iiii_iiii, handler_name: "i1000_1001_iiii_iiii", mask: MASK_IMM8, key: 0x8900, diss: "bt <bdisp8>", is_branch: 1 },
    sh4_opcodelistentry { oph: i1000_1101_iiii_iiii, handler_name: "i1000_1101_iiii_iiii", mask: MASK_IMM8, key: 0x8D00, diss: "bt.s <bdisp8>", is_branch: 2 },

    sh4_opcodelistentry { oph: i1000_1000_iiii_iiii, handler_name: "i1000_1000_iiii_iiii", mask: MASK_IMM8, key: 0x8800, diss: "cmp/eq #<simm8hex>,R0", is_branch: 0 },

    sh4_opcodelistentry { oph: i1010_iiii_iiii_iiii, handler_name: "i1010_iiii_iiii_iiii", mask: MASK_N_IMM8, key: 0xA000, diss: "bra <bdisp12>", is_branch: 2 },
    sh4_opcodelistentry { oph: i1011_iiii_iiii_iiii, handler_name: "i1011_iiii_iiii_iiii", mask: MASK_N_IMM8, key: 0xB000, diss: "bsr <bdisp12>", is_branch: 1 },

    sh4_opcodelistentry { oph: i1100_0011_iiii_iiii, handler_name: "i1100_0011_iiii_iiii", mask: MASK_IMM8, key: 0xC300, diss: "trapa #<imm8>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1100_1000_iiii_iiii, handler_name: "i1100_1000_iiii_iiii", mask: MASK_IMM8, key: 0xC800, diss: "tst #<imm8>,R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1001_iiii_iiii, handler_name: "i1100_1001_iiii_iiii", mask: MASK_IMM8, key: 0xC900, diss: "and #<imm8>,R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1010_iiii_iiii, handler_name: "i1100_1010_iiii_iiii", mask: MASK_IMM8, key: 0xCA00, diss: "xor #<imm8>,R0", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1011_iiii_iiii, handler_name: "i1100_1011_iiii_iiii", mask: MASK_IMM8, key: 0xCB00, diss: "or #<imm8>,R0", is_branch: 0 },

    sh4_opcodelistentry { oph: i1100_1100_iiii_iiii, handler_name: "i1100_1100_iiii_iiii", mask: MASK_IMM8, key: 0xCC00, diss: "tst.b #<imm8>,@(R0,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1101_iiii_iiii, handler_name: "i1100_1101_iiii_iiii", mask: MASK_IMM8, key: 0xCD00, diss: "and.b #<imm8>,@(R0,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1110_iiii_iiii, handler_name: "i1100_1110_iiii_iiii", mask: MASK_IMM8, key: 0xCE00, diss: "xor.b #<imm8>,@(R0,GBR)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1100_1111_iiii_iiii, handler_name: "i1100_1111_iiii_iiii", mask: MASK_IMM8, key: 0xCF00, diss: "or.b #<imm8>,@(R0,GBR)", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0000, handler_name: "i1111_nnnn_mmmm_0000", mask: MASK_N_M, key: 0xF000, diss: "fadd <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0001, handler_name: "i1111_nnnn_mmmm_0001", mask: MASK_N_M, key: 0xF001, diss: "fsub <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0010, handler_name: "i1111_nnnn_mmmm_0010", mask: MASK_N_M, key: 0xF002, diss: "fmul <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0011, handler_name: "i1111_nnnn_mmmm_0011", mask: MASK_N_M, key: 0xF003, diss: "fdiv <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0100, handler_name: "i1111_nnnn_mmmm_0100", mask: MASK_N_M, key: 0xF004, diss: "fcmp/eq <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0101, handler_name: "i1111_nnnn_mmmm_0101", mask: MASK_N_M, key: 0xF005, diss: "fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0110, handler_name: "i1111_nnnn_mmmm_0110", mask: MASK_N_M, key: 0xF006, diss: "fmov.s @(R0,<REG_M>),<FREG_N_SD_A>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_0111, handler_name: "i1111_nnnn_mmmm_0111", mask: MASK_N_M, key: 0xF007, diss: "fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1000, handler_name: "i1111_nnnn_mmmm_1000", mask: MASK_N_M, key: 0xF008, diss: "fmov.s @<REG_M>,<FREG_N_SD_A>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1001, handler_name: "i1111_nnnn_mmmm_1001", mask: MASK_N_M, key: 0xF009, diss: "fmov.s @<REG_M>+,<FREG_N_SD_A>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1010, handler_name: "i1111_nnnn_mmmm_1010", mask: MASK_N_M, key: 0xF00A, diss: "fmov.s <FREG_M_SD_A>,@<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1011, handler_name: "i1111_nnnn_mmmm_1011", mask: MASK_N_M, key: 0xF00B, diss: "fmov.s <FREG_M_SD_A>,@-<REG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1100, handler_name: "i1111_nnnn_mmmm_1100", mask: MASK_N_M, key: 0xF00C, diss: "fmov <FREG_M_SD_A>,<FREG_N_SD_A>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_nnnn_0101_1101, handler_name: "i1111_nnnn_0101_1101", mask: MASK_N, key: 0xF05D, diss: "fabs <FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnn0_1111_1101, handler_name: "i1111_nnn0_1111_1101", mask: MASK_NH3BIT, key: 0xF0FD, diss: "fsca FPUL,<DR_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_1011_1101, handler_name: "i1111_nnnn_1011_1101", mask: MASK_N, key: 0xF0BD, diss: "fcnvds <DR_N>,FPUL", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_1010_1101, handler_name: "i1111_nnnn_1010_1101", mask: MASK_N, key: 0xF0AD, diss: "fcnvsd FPUL,<DR_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnmm_1110_1101, handler_name: "i1111_nnmm_1110_1101", mask: MASK_N, key: 0xF0ED, diss: "fipr <FV_M>,<FV_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_nnnn_1000_1101, handler_name: "i1111_nnnn_1000_1101", mask: MASK_N, key: 0xF08D, diss: "fldi0 <FREG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_1001_1101, handler_name: "i1111_nnnn_1001_1101", mask: MASK_N, key: 0xF09D, diss: "fldi1 <FREG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0001_1101, handler_name: "i1111_nnnn_0001_1101", mask: MASK_N, key: 0xF01D, diss: "flds <FREG_N>,FPUL", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0010_1101, handler_name: "i1111_nnnn_0010_1101", mask: MASK_N, key: 0xF02D, diss: "float FPUL,<FREG_N_SD_F>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0100_1101, handler_name: "i1111_nnnn_0100_1101", mask: MASK_N, key: 0xF04D, diss: "fneg <FREG_N_SD_F>", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_1011_1111_1101, handler_name: "i1111_1011_1111_1101", mask: MASK_NONE, key: 0xFBFD, diss: "frchg", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_0011_1111_1101, handler_name: "i1111_0011_1111_1101", mask: MASK_NONE, key: 0xF3FD, diss: "fschg", is_branch: 0 },

    sh4_opcodelistentry { oph: i1111_nnnn_0110_1101, handler_name: "i1111_nnnn_0110_1101", mask: MASK_N, key: 0xF06D, diss: "fsqrt <FREG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0011_1101, handler_name: "i1111_nnnn_0011_1101", mask: MASK_N, key: 0xF03D, diss: "ftrc <FREG_N>,FPUL", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0000_1101, handler_name: "i1111_nnnn_0000_1101", mask: MASK_N, key: 0xF00D, diss: "fsts FPUL,<FREG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nn01_1111_1101, handler_name: "i1111_nn01_1111_1101", mask: MASK_NH2BIT, key: 0xF1FD, diss: "ftrv xmtrx,<FV_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_mmmm_1110, handler_name: "i1111_nnnn_mmmm_1110", mask: MASK_N_M, key: 0xF00E, diss: "fmac <FREG_0>,<FREG_M>,<FREG_N>", is_branch: 0 },
    sh4_opcodelistentry { oph: i1111_nnnn_0111_1101, handler_name: "i1111_nnnn_0111_1101", mask: MASK_N, key: 0xF07D, diss: "fsrra <FREG_N>", is_branch: 0 },

    sh4_opcodelistentry { oph: i_not_implemented, handler_name: "unknown_opcode", mask: MASK_NONE, key: 0, diss: "unknown_opcode", is_branch: 0 },
];



// static opcodes: &[sh4_opcodelistentry] = &[
//     // CPU
//     sh4_opcodelistentry{ oph:i0000_nnnn_0010_0011, handler_name:"i0000_nnnn_0010_0011", mask:Mask_n, key:0x0023, diss:"braf <REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_0000_0011, handler_name:"i0000_nnnn_0000_0011", mask:Mask_n, key:0x0003, diss:"bsrf <REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_1100_0011, handler_name:"i0000_nnnn_1100_0011", mask:Mask_n, key:0x00C3, diss:"movca.l R0, @<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_1001_0011, handler_name:"i0000_nnnn_1001_0011", mask:Mask_n, key:0x0093, diss:"ocbi @<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_1010_0011, handler_name:"i0000_nnnn_1010_0011", mask:Mask_n, key:0x00A3, diss:"ocbp @<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_1011_0011, handler_name:"i0000_nnnn_1011_0011", mask:Mask_n, key:0x00B3, diss:"ocbwb @<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_1000_0011, handler_name:"i0000_nnnn_1000_0011", mask:Mask_n, key:0x0083, diss:"pref @<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_0111, handler_name:"i0000_nnnn_mmmm_0111", mask:Mask_n_m, key:0x0007, diss:"mul.l <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0010_1000, handler_name:"i0000_0000_0010_1000", mask:Mask_none, key:0x0028, diss:"clrmac", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0100_1000, handler_name:"i0000_0000_0100_1000", mask:Mask_none, key:0x0048, diss:"clrs", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0000_1000, handler_name:"i0000_0000_0000_1000", mask:Mask_none, key:0x0008, diss:"clrt", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0011_1000, handler_name:"i0000_0000_0011_1000", mask:Mask_none, key:0x0038, diss:"ldtlb", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0101_1000, handler_name:"i0000_0000_0101_1000", mask:Mask_none, key:0x0058, diss:"sets", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0001_1000, handler_name:"i0000_0000_0001_1000", mask:Mask_none, key:0x0018, diss:"sett", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0001_1001, handler_name:"i0000_0000_0001_1001", mask:Mask_none, key:0x0019, diss:"div0u", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_0010_1001, handler_name:"i0000_nnnn_0010_1001", mask:Mask_n, key:0x0029, diss:"movt <REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0000_1001, handler_name:"i0000_0000_0000_1001", mask:Mask_none, key:0x0009, diss:"nop", is_branch:0},

//     sh4_opcodelistentry{ oph:i0000_0000_0010_1011, handler_name:"i0000_0000_0010_1011", mask:Mask_none, key:0x002B, diss:"rte", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0000_1011, handler_name:"i0000_0000_0000_1011", mask:Mask_none, key:0x000B, diss:"rts", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_0000_0001_1011, handler_name:"i0000_0000_0001_1011", mask:Mask_none, key:0x001B, diss:"sleep", is_branch:0},

//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_1111, handler_name:"i0000_nnnn_mmmm_1111", mask:Mask_n_m, key:0x000F, diss:"mac.l @<REG_M>+,@<REG_N>+", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0111, handler_name:"i0010_nnnn_mmmm_0117", mask:Mask_n_m, key:0x2007, diss:"div0s <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1000, handler_name:"i0010_nnnn_mmmm_1000", mask:Mask_n_m, key:0x2008, diss:"tst <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1001, handler_name:"i0010_nnnn_mmmm_1001", mask:Mask_n_m, key:0x2009, diss:"and <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1010, handler_name:"i0010_nnnn_mmmm_1010", mask:Mask_n_m, key:0x200A, diss:"xor <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1011, handler_name:"i0010_nnnn_mmmm_1011", mask:Mask_n_m, key:0x200B, diss:"or <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1100, handler_name:"i0010_nnnn_mmmm_1100", mask:Mask_n_m, key:0x200C, diss:"cmp/str <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1101, handler_name:"i0010_nnnn_mmmm_1101", mask:Mask_n_m, key:0x200D, diss:"xtrct <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1110, handler_name:"i0010_nnnn_mmmm_1110", mask:Mask_n_m, key:0x200E, diss:"mulu.w <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_1111, handler_name:"i0010_nnnn_mmmm_1111", mask:Mask_n_m, key:0x200F, diss:"muls.w <REG_M>,<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0000, handler_name:"i0011_nnnn_mmmm_0000", mask:Mask_n_m, key:0x3000, diss:"cmp/eq <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0010, handler_name:"i0011_nnnn_mmmm_0010", mask:Mask_n_m, key:0x3002, diss:"cmp/hs <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0011, handler_name:"i0011_nnnn_mmmm_0011", mask:Mask_n_m, key:0x3003, diss:"cmp/ge <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0100, handler_name:"i0011_nnnn_mmmm_0100", mask:Mask_n_m, key:0x3004, diss:"div1 <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0101, handler_name:"i0011_nnnn_mmmm_0101", mask:Mask_n_m, key:0x3005, diss:"dmulu.l <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0110, handler_name:"i0011_nnnn_mmmm_0110", mask:Mask_n_m, key:0x3006, diss:"cmp/hi <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_0111, handler_name:"i0011_nnnn_mmmm_0111", mask:Mask_n_m, key:0x3007, diss:"cmp/gt <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1000, handler_name:"i0011_nnnn_mmmm_1000", mask:Mask_n_m, key:0x3008, diss:"sub <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1010, handler_name:"i0011_nnnn_mmmm_1010", mask:Mask_n_m, key:0x300A, diss:"subc <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1011, handler_name:"i0011_nnnn_mmmm_1011", mask:Mask_n_m, key:0x300B, diss:"subv <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1100, handler_name:"i0011_nnnn_mmmm_1100", mask:Mask_n_m, key:0x300C, diss:"add <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1101, handler_name:"i0011_nnnn_mmmm_1101", mask:Mask_n_m, key:0x300D, diss:"dmuls.l <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1110, handler_name:"i0011_nnnn_mmmm_1110", mask:Mask_n_m, key:0x300E, diss:"addc <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0011_nnnn_mmmm_1111, handler_name:"i0011_nnnn_mmmm_1111", mask:Mask_n_m, key:0x300F, diss:"addv <REG_M>,<REG_N>", is_branch:0},

//     // Normal readm/writem
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_0100, handler_name:"i0000_nnnn_mmmm_0100", mask:Mask_n_m, key:0x0004, diss:"mov.b <REG_M>,@(R0,<REG_N>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_0101, handler_name:"i0000_nnnn_mmmm_0101", mask:Mask_n_m, key:0x0005, diss:"mov.w <REG_M>,@(R0,<REG_N>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_0110, handler_name:"i0000_nnnn_mmmm_0110", mask:Mask_n_m, key:0x0006, diss:"mov.l <REG_M>,@(R0,<REG_N>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_1100, handler_name:"i0000_nnnn_mmmm_1100", mask:Mask_n_m, key:0x000C, diss:"mov.b @(R0,<REG_M>),<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_1101, handler_name:"i0000_nnnn_mmmm_1101", mask:Mask_n_m, key:0x000D, diss:"mov.w @(R0,<REG_M>),<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0000_nnnn_mmmm_1110, handler_name:"i0000_nnnn_mmmm_1110", mask:Mask_n_m, key:0x000E, diss:"mov.l @(R0,<REG_M>),<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0001_nnnn_mmmm_iiii, handler_name:"i0001_nnnn_mmmm_iiii", mask:Mask_n_imm8, key:0x1000, diss:"mov.l <REG_M>,@(<disp4dw>,<REG_N>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i0101_nnnn_mmmm_iiii, handler_name:"i0101_nnnn_mmmm_iiii", mask:Mask_n_m_imm4, key:0x5000, diss:"mov.l @(<disp4dw>,<REG_M>),<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0000, handler_name:"i0010_nnnn_mmmm_0000", mask:Mask_n_m, key:0x2000, diss:"mov.b <REG_M>,@<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0001, handler_name:"i0010_nnnn_mmmm_0001", mask:Mask_n_m, key:0x2001, diss:"mov.w <REG_M>,@<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0010, handler_name:"i0010_nnnn_mmmm_0010", mask:Mask_n_m, key:0x2002, diss:"mov.l <REG_M>,@<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0000, handler_name:"i0110_nnnn_mmmm_0000", mask:Mask_n_m, key:0x6000, diss:"mov.b @<REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0001, handler_name:"i0110_nnnn_mmmm_0001", mask:Mask_n_m, key:0x6001, diss:"mov.w @<REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0010, handler_name:"i0110_nnnn_mmmm_0010", mask:Mask_n_m, key:0x6002, diss:"mov.l @<REG_M>,<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0100, handler_name:"i0010_nnnn_mmmm_0100", mask:Mask_n_m, key:0x2004, diss:"mov.b <REG_M>,@-<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0101, handler_name:"i0010_nnnn_mmmm_0101", mask:Mask_n_m, key:0x2005, diss:"mov.w <REG_M>,@-<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0010_nnnn_mmmm_0110, handler_name:"i0010_nnnn_mmmm_0110", mask:Mask_n_m, key:0x2006, diss:"mov.l <REG_M>,@-<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0100, handler_name:"i0110_nnnn_mmmm_0100", mask:Mask_n_m, key:0x6004, diss:"mov.b @<REG_M>+,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0101, handler_name:"i0110_nnnn_mmmm_0101", mask:Mask_n_m, key:0x6005, diss:"mov.w @<REG_M>+,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0110, handler_name:"i0110_nnnn_mmmm_0110", mask:Mask_n_m, key:0x6006, diss:"mov.l @<REG_M>+,<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i1000_0000_mmmm_iiii, handler_name:"i1000_0000_mmmm_iiii", mask:Mask_imm8, key:0x8000, diss:"mov.b R0,@(<disp4b>,<REG_M>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1000_0001_mmmm_iiii, handler_name:"i1000_0001_mmmm_iiii", mask:Mask_imm8, key:0x8100, diss:"mov.w R0,@(<disp4w>,<REG_M>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1000_0100_mmmm_iiii, handler_name:"i1000_0100_mmmm_iiii", mask:Mask_imm8, key:0x8400, diss:"mov.b @(<disp4b>,<REG_M>),R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1000_0101_mmmm_iiii, handler_name:"i1000_0101_mmmm_iiii", mask:Mask_imm8, key:0x8500, diss:"mov.w @(<disp4w>,<REG_M>),R0", is_branch:0},

//     sh4_opcodelistentry{ oph:i1001_nnnn_iiii_iiii, handler_name:"i1001_nnnn_iiii_iiii", mask:Mask_n_imm8, key:0x9000, diss:"mov.w @(<PCdisp8w>),<REG_N>", is_branch:0},

//     sh4_opcodelistentry{ oph:i1100_0000_iiii_iiii, handler_name:"i1100_0000_iiii_iiii", mask:Mask_imm8, key:0xC000, diss:"mov.b R0,@(<disp8b>,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0001_iiii_iiii, handler_name:"i1100_0001_iiii_iiii", mask:Mask_imm8, key:0xC100, diss:"mov.w R0,@(<disp8w>,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0010_iiii_iiii, handler_name:"i1100_0010_iiii_iiii", mask:Mask_imm8, key:0xC200, diss:"mov.l R0,@(<disp8dw>,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0100_iiii_iiii, handler_name:"i1100_0100_iiii_iiii", mask:Mask_imm8, key:0xC400, diss:"mov.b @(<GBRdisp8b>),R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0101_iiii_iiii, handler_name:"i1100_0101_iiii_iiii", mask:Mask_imm8, key:0xC500, diss:"mov.w @(<GBRdisp8w>),R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0110_iiii_iiii, handler_name:"i1100_0110_iiii_iiii", mask:Mask_imm8, key:0xC600, diss:"mov.l @(<GBRdisp8dw>),R0", is_branch:0},

//     // normal mov
//     sh4_opcodelistentry{ oph:i0110_nnnn_mmmm_0011, handler_name:"i0110_nnnn_mmmm_0011", mask:Mask_n_m, key:0x6003, diss:"mov <REG_M>,<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0111_iiii_iiii, handler_name:"i1100_0111_iiii_iiii", mask:Mask_imm8, key:0xC700, diss:"mova @(<PCdisp8d>),R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1110_nnnn_iiii_iiii, handler_name:"i1110_nnnn_iiii_iiii", mask:Mask_n_imm8, key:0xE000, diss:"mov #<simm8hex>,<REG_N>", is_branch:0},

//     // (… the rest of your table entries for 4xxx, 5xxx, 6xxx, 7xxx, 8xxx, 9xxx, Axxx, Bxxx, Cxxx, Fxxx as shown …)
//     // To keep this single message within limits, the remainder of the table follows the exact pattern above,
//     // directly mirroring each row in your snippet, mapping the same handler name to the same mask/key/diss/is_branch.

//     // Branches
//     sh4_opcodelistentry{ oph:i1000_1011_iiii_iiii, handler_name:"i1000_1011_iiii_iiii", mask:Mask_imm8, key:0x8B00, diss:"bf <bdisp8>", is_branch:1},
//     sh4_opcodelistentry{ oph:i1000_1111_iiii_iiii, handler_name:"i1000_1111_iiii_iiii", mask:Mask_imm8, key:0x8F00, diss:"bf.s <bdisp8>", is_branch:2},
//     sh4_opcodelistentry{ oph:i1000_1001_iiii_iiii, handler_name:"i1000_1001_iiii_iiii", mask:Mask_imm8, key:0x8900, diss:"bt <bdisp8>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1000_1101_iiii_iiii, handler_name:"i1000_1101_iiii_iiii", mask:Mask_imm8, key:0x8D00, diss:"bt.s <bdisp8>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1000_1000_iiii_iiii, handler_name:"i1000_1000_iiii_iiii", mask:Mask_imm8, key:0x8800, diss:"cmp/eq #<simm8hex>,R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1010_iiii_iiii_iiii, handler_name:"i1010_iiii_iiii_iiii", mask:Mask_n_imm8, key:0xA000, diss:"bra <bdisp12>", is_branch:2},
//     sh4_opcodelistentry{ oph:i1011_iiii_iiii_iiii, handler_name:"i1011_iiii_iiii_iiii", mask:Mask_n_imm8, key:0xB000, diss:"bsr <bdisp12>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_0011_iiii_iiii, handler_name:"i1100_0011_iiii_iiii", mask:Mask_imm8, key:0xC300, diss:"trapa #<imm8>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1000_iiii_iiii, handler_name:"i1100_1000_iiii_iiii", mask:Mask_imm8, key:0xC800, diss:"tst #<imm8>,R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1001_iiii_iiii, handler_name:"i1100_1001_iiii_iiii", mask:Mask_imm8, key:0xC900, diss:"and #<imm8>,R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1010_iiii_iiii, handler_name:"i1100_1010_iiii_iiii", mask:Mask_imm8, key:0xCA00, diss:"xor #<imm8>,R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1011_iiii_iiii, handler_name:"i1100_1011_iiii_iiii", mask:Mask_imm8, key:0xCB00, diss:"or #<imm8>,R0", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1100_iiii_iiii, handler_name:"i1100_1100_iiii_iiii", mask:Mask_imm8, key:0xCC00, diss:"tst.b #<imm8>,@(R0,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1101_iiii_iiii, handler_name:"i1100_1101_iiii_iiii", mask:Mask_imm8, key:0xCD00, diss:"and.b #<imm8>,@(R0,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1110_iiii_iiii, handler_name:"i1100_1110_iiii_iiii", mask:Mask_imm8, key:0xCE00, diss:"xor.b #<imm8>,@(R0,GBR)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1100_1111_iiii_iiii, handler_name:"i1100_1111_iiii_iiii", mask:Mask_imm8, key:0xCF00, diss:"or.b #<imm8>,@(R0,GBR)", is_branch:0},

//     // FPU ops (subset implemented above; entries mirror your snippet)
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0000, handler_name:"i1111_nnnn_mmmm_0000", mask:Mask_n_m, key:0xF000, diss:"fadd <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0001, handler_name:"i1111_nnnn_mmmm_0001", mask:Mask_n_m, key:0xF001, diss:"fsub <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0010, handler_name:"i1111_nnnn_mmmm_0010", mask:Mask_n_m, key:0xF002, diss:"fmul <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0011, handler_name:"i1111_nnnn_mmmm_0011", mask:Mask_n_m, key:0xF003, diss:"fdiv <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0100, handler_name:"i1111_nnnn_mmmm_0100", mask:Mask_n_m, key:0xF004, diss:"fcmp/eq <FREG_M_SD_F>,<FREG_N>_SD_F", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0101, handler_name:"i1111_nnnn_mmmm_0101", mask:Mask_n_m, key:0xF005, diss:"fcmp/gt <FREG_M_SD_F>,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0110, handler_name:"i1111_nnnn_mmmm_0110", mask:Mask_n_m, key:0xF006, diss:"fmov.s @(R0,<REG_M>),<FREG_N_SD_A>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_0111, handler_name:"i1111_nnnn_mmmm_0111", mask:Mask_n_m, key:0xF007, diss:"fmov.s <FREG_M_SD_A>,@(R0,<REG_N>)", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1000, handler_name:"i1111_nnnn_mmmm_1000", mask:Mask_n_m, key:0xF008, diss:"fmov.s @<REG_M>,<FREG_N_SD_A>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1001, handler_name:"i1111_nnnn_mmmm_1001", mask:Mask_n_m, key:0xF009, diss:"fmov.s @<REG_M>+,<FREG_N_SD_A>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1010, handler_name:"i1111_nnnn_mmmm_1010", mask:Mask_n_m, key:0xF00A, diss:"fmov.s <FREG_M_SD_A>,@<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1011, handler_name:"i1111_nnnn_mmmm_1011", mask:Mask_n_m, key:0xF00B, diss:"fmov.s <FREG_M_SD_A>,@-<REG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1100, handler_name:"i1111_nnnn_mmmm_1100", mask:Mask_n_m, key:0xF00C, diss:"fmov <FREG_M_SD_A>,<FREG_N_SD_A>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0101_1101, handler_name:"i1111_nnnn_0105_1101", mask:Mask_n, key:0xF05D, diss:"fabs <FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnn0_1111_1101, handler_name:"i1111_nnn0_1111_1101", mask:Mask_nh3bit, key:0xF0FD, diss:"fsca FPUL, <DR_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_1011_1101, handler_name:"i1111_nnnn_1011_1101", mask:Mask_n, key:0xF0BD, diss:"fcnvds <DR_N>,FPUL", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_1010_1101, handler_name:"i1111_nnnn_1010_1101", mask:Mask_n, key:0xF0AD, diss:"fcnvsd FPUL,<DR_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnmm_1110_1101, handler_name:"i1111_nnmm_1110_1101", mask:Mask_n, key:0xF0ED, diss:"fipr <FV_M>,<FV_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_1000_1101, handler_name:"i1111_nnnn_1000_1101", mask:Mask_n, key:0xF08D, diss:"fldi0 <FREG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_1001_1101, handler_name:"i1111_nnnn_1001_1101", mask:Mask_n, key:0xF09D, diss:"fldi1 <FREG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0001_1101, handler_name:"i1111_nnnn_0001_1101", mask:Mask_n, key:0xF01D, diss:"flds <FREG_N>,FPUL", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0010_1101, handler_name:"i1111_nnnn_0010_1101", mask:Mask_n, key:0xF02D, diss:"float FPUL,<FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0100_1101, handler_name:"i1111_nnnn_0100_1101", mask:Mask_n, key:0xF04D, diss:"fneg <FREG_N_SD_F>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_1011_1111_1101, handler_name:"i1111_1011_1111_1101", mask:Mask_none, key:0xFBFD, diss:"frchg", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_0011_1111_1101, handler_name:"i1111_0011_1111_1101", mask:Mask_none, key:0xF3FD, diss:"fschg", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0110_1101, handler_name:"i1111_nnnn_0110_1101", mask:Mask_n, key:0xF06D, diss:"fsqrt <FREG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0011_1101, handler_name:"i1111_nnnn_0011_1101", mask:Mask_n, key:0xF03D, diss:"ftrc <FREG_N>, FPUL", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0000_1101, handler_name:"i1111_nnnn_0000_1101", mask:Mask_n, key:0xF00D, diss:"fsts FPUL,<FREG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nn01_1111_1101, handler_name:"i1111_nn01_1111_1101", mask:Mask_nh2bit, key:0xF1FD, diss:"ftrv xmtrx,<FV_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_mmmm_1110, handler_name:"i1111_nnnn_mmmm_1110", mask:Mask_n_m, key:0xF00E, diss:"fmac <FREG_0>,<FREG_M>,<FREG_N>", is_branch:0},
//     sh4_opcodelistentry{ oph:i1111_nnnn_0111_1101, handler_name:"i1111_nnnn_0111_1101", mask:Mask_n, key:0xF07D, diss:"fsrra <FREG_N>", is_branch:0},

//     // End marker
//     sh4_opcodelistentry{ oph:i_not_implemented, handler_name:"unknown", mask:0, key:0, diss:"unknown_opcode", is_branch:0 },
// ];
pub fn build_opcode_tables(dc: &mut Dreamcast) {
    // Initialize defaults
    for i in 0..0x10000 {
        dc.OpPtr[i] = i_not_implemented;
        dc.OpDesc[i] = &MISSING_OPCODE;
    }

    let mut i2 = 0;
    
    loop {
        let oph = OPCODES[i2].oph;

        // Stop if we've reached the sentinel
        if oph as usize == i_not_implemented as usize {
            break;
        }

        let shft: u32;
        let count: u32;
        let mask = !(OPCODES[i2].mask as u32);
        let base = OPCODES[i2].key as u32;

        match OPCODES[i2].mask {
            MASK_NONE       => { count = 1; shft = 0; }
            MASK_N          => { count = 16; shft = 8; }
            MASK_N_M        => { count = 256; shft = 4; }
            MASK_N_M_IMM4   => { count = 256 * 16; shft = 0; }
            MASK_IMM8       => { count = 256; shft = 0; }
            MASK_N_ML3BIT   => { count = 256; shft = 4; }
            MASK_NH3BIT     => { count = 8; shft = 9; }
            MASK_NH2BIT     => { count = 4; shft = 10; }
            _               => panic!("Error: invalid mask"),
        }

        for i in 0..count {
            let idx = ((i << shft) & mask) + base;
            dc.OpPtr[idx as usize] = oph;
            dc.OpDesc[idx as usize] = &OPCODES[i2];
        }

        i2 += 1;
    }
}


pub static ROTO_BIN: &[u8] = include_bytes!("../roto.bin");


pub fn init_dreamcast(dc: &mut Dreamcast) {
    // Zero entire struct (like memset). In Rust, usually you'd implement Default.
    *dc = Dreamcast::default();

    // Build opcode tables
    build_opcode_tables(dc);

    // Setup memory map
    dc.memmap[0x0C] = dc.sys_ram.as_mut_ptr();
    dc.memmask[0x0C] = SYSRAM_MASK;
    dc.memmap[0x8C] = dc.sys_ram.as_mut_ptr();
    dc.memmask[0x8C] = SYSRAM_MASK;
    dc.memmap[0xA5] = dc.video_ram.as_mut_ptr();
    dc.memmask[0xA5] = VIDEORAM_MASK;

    // Set initial PC
    dc.ctx.pc = 0x8C01_0000;

    // Copy roto.bin from embedded ROTO_BIN
    let sysram_slice = &mut dc.sys_ram[0x10000..0x10000 + ROTO_BIN.len()];
    sysram_slice.copy_from_slice(ROTO_BIN);
}


pub fn run_dreamcast(dc: &mut Dreamcast) {
    loop {
        let mut instr: u16 = 0;

        // Equivalent of: read_mem(dc, dc->ctx.pc, instr);
        read_mem(dc, dc.ctx.pc, &mut instr);

        dc.ctx.pc = dc.ctx.pc.wrapping_add(2);

        // Call the opcode handler
        (dc.OpPtr[instr as usize])(dc, instr);

        // Break when remaining_cycles reaches 0
        dc.ctx.remaining_cycles -= 1;
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
