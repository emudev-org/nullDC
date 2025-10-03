use crate::dreamcast::Dreamcast;

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
pub fn sh4_write_mem8(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    panic!("sh4_write_mem8 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_write_mem16(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    panic!("sh4_write_mem16 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_write_mem32(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    panic!("sh4_write_mem32 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_write_mem64(dc: *mut Dreamcast, addr: *const u32, data: *const u64) {
    panic!("sh4_write_mem64 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_read_mems8(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    panic!("sh4_read_mems8 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_read_mems16(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    panic!("sh4_read_mems16 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_read_mem32(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    panic!("sh4_read_mems32 is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_read_mem64(dc: *mut Dreamcast, addr: *const u32, data: *mut u64) {
    panic!("sh4_read_mems64 is not implemented in backend_dec");
}

pub fn sh4_read_mem32i(dc: *mut Dreamcast, addr: u32, data: *mut u32) {
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
pub fn sh4_branch_cond(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    panic!("sh4_branch_cond is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_branch_cond_delay(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    panic!("sh4_branch_cond_delay is not implemented in backend_dec");
}

#[inline(always)]
pub fn sh4_branch_delay(dc: *mut Dreamcast, target: u32) {
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
