// backend_fns.rs — single global buffer, 32-byte aligned, sequential packing, realloc growth

use super::Sh4Ctx;
use super::backend_ipr;

use core::{mem, ptr};
use std::alloc::{alloc, dealloc, realloc, handle_alloc_error, Layout};
use paste::paste;
use seq_macro::seq;
use std::ptr::{ addr_of, addr_of_mut };
/* ------------------------------ Types ------------------------------ */

/// Handler receives pointer just after the stored handler ptr, returns pointer
/// just after its consumed arguments (i.e., the next record’s handler ptr).
pub type Handler = unsafe extern "C" fn(data_ptr: *const u8) -> *const u8;

/* --------------------- Global, NOT thread-safe --------------------- */

static mut START: *mut u8 = ptr::null_mut();
static mut CUR:   *mut u8 = ptr::null_mut();
static mut END:   *mut u8 = ptr::null_mut();
static mut CAP:   usize = 0;
static mut STEPS: usize = 0;

/// Required buffer alignment for the whole allocation.
const BUF_ALIGN: usize = 32;

/* ------------------------------ Utils ------------------------------ */

#[inline(always)]
unsafe fn layout_for(size: usize) -> Layout {
    unsafe {
        // Keep the same alignment on alloc/realloc/dealloc to satisfy the contract.
        Layout::from_size_align_unchecked(size.max(1), BUF_ALIGN)
    }
}

#[inline(always)]
unsafe fn write_seq<T: Copy>(val: T) -> *mut u8 {
    unsafe {
        let sz = mem::size_of::<T>();
        assert!(CUR.add(sz) <= END, "Out of space in recorder buffer");
        let at = CUR;
        (at as *mut T).write_unaligned(val);
        CUR = CUR.add(sz);
        at
    }
}

#[inline(always)]
unsafe fn read_seq<T: Copy>(p: &mut *const u8) -> T {
    unsafe {
        let at = *p as *const T;
        let v = at.read_unaligned();
        *p = (*p).add(mem::size_of::<T>());
        v
    }
}

/* ---------------------------- Lifecycle API ---------------------------- */

/// Allocate/initialize the recorder with given initial capacity (bytes).
pub unsafe fn dec_start(initial_capacity_bytes: usize) {
    unsafe {
        let cap = initial_capacity_bytes.max(64);
        let layout = layout_for(cap);
        let p = alloc(layout);
        if p.is_null() { handle_alloc_error(layout) }
        STEPS = 0;
        START = p;
        CUR   = p;
        END   = p.add(cap);
        CAP   = cap;
    }
}

#[inline(never)]
unsafe extern "C" fn unreachable_stub(_: *const u8) -> *const u8 {
    unsafe {
        core::hint::unreachable_unchecked();
    }
}

pub fn dec_reserve_dispatcher() {
    unsafe {
        write_seq::<Handler>(unreachable_stub);
    }
}

/// Optionally shrink to the exact used size and return (base, used_bytes).
/// Ownership remains here; call `dec_free()` when done.
pub unsafe fn dec_finalize_shrink() -> (*const u8, usize) {
    unsafe {
        write_seq::<u32>(STEPS as u32);

        let used = (CUR as usize).saturating_sub(START as usize);
        let old_layout = layout_for(CAP);
        let new_layout = layout_for(used);
        let new_ptr = if used == CAP {
            START
        } else {
            let p = realloc(START, old_layout, used);
            if p.is_null() { handle_alloc_error(new_layout) }
            p
        };
        START = ptr::null_mut();
        CUR   = ptr::null_mut();
        END   = ptr::null_mut();
        CAP   = 0;
        (new_ptr, used)
    }
}

/// Free the recorder buffer.
pub unsafe fn dec_free(ptr: *const u8) {
    unsafe {
        if !ptr.is_null() {
            dealloc(ptr as *mut u8, layout_for(CAP));
        }
    }
}

/* ---------------------------- Recording API ---------------------------- */

/// Begin a record: write the handler pointer; return pointer to first arg slot.
#[inline(always)]
unsafe fn emit_record(h: Handler) -> *mut u8 {
    unsafe {
        // Packed sequentially: no per-field alignment between items.
        let _where = write_seq::<Handler>(h);
        // Data ptr starts right after the handler
        _where.add(mem::size_of::<Handler>())
    }
}

#[inline(always)]
unsafe fn emit_arg<T: Copy>(v: T) { unsafe { let _ = write_seq::<T>(v); } }

/* ----------------------------- Execution API ----------------------------- */

/// Execute one record located at `record_ptr` (address where the handler pointer is stored).
/// Returns pointer to the next record’s handler.
#[inline(always)]
unsafe fn step_once(record_ptr: *const u8) -> *const u8 {
    unsafe {
        // Read handler (unaligned)
        let h = (record_ptr as *const Handler).read_unaligned();
        // Data immediately follows the handler
        let data_ptr = record_ptr.add(mem::size_of::<Handler>());
        h(data_ptr)
    }
}

#[inline(always)]
pub unsafe fn dec_run_block(record_ptr: *const u8) -> u32 {
    unsafe {
        let data = step_once(record_ptr);

        *(data as *const u32)
    }
}

// Generate executor_1 .. executor_24
seq!(N in 1..=24 {
    paste! {
        unsafe extern "C" fn [<executor_ N>](mut data: *const u8) -> *const u8 {
            // Unroll N times
            seq!(i in 0..N { unsafe { data = step_once(data); } });
            data
        }
    }
});

// Build the dispatch table: index 0 => 1 step, …, index 23 => 24 steps
pub static EXECUTORS: [Handler; 24] = seq!(N in 1..=24 {
    [
        #(
            paste! { [<executor_ N>] },
        )*
    ]
});


pub fn dec_patch_dispatcher() {
    unsafe {
        assert!(STEPS > 0, "Too few steps");
        assert!(STEPS <= 24, "Too many steps");

        let old_cur = CUR;
        CUR = START;
        write_seq::<Handler>(EXECUTORS[STEPS-1]);
        CUR = old_cur;
    }
}

/* ----------------------- Macro to define ops & handlers ----------------------- */

macro_rules! define_op {
    ($name:ident ( $($arg:ident : $ty:ty),* $(,)? )) => {
        paste! {
            #[inline(always)]
            pub fn $name( $($arg : $ty),* ) {
                unsafe {
                    STEPS += 1;
                    let mut _data = emit_record([<handler_ $name>] as Handler);
                    // pack each argument sequentially (unaligned)
                    $( { let _ = _data; emit_arg::<$ty>($arg); } )*
                }
            }

            #[inline(always)]
            unsafe extern "C" fn [<handler_ $name>](mut p: *const u8) -> *const u8 {
                unsafe {
                    // read args in the same order (unaligned)
                    $( let $arg : $ty = read_seq::<$ty>(&mut p); )*
                    backend_ipr::$name( $($arg),* );
                    // p now points just after our last arg => next record’s handler
                    p
                }
            }
        }
    };
}

/* ------------------------------ Declarations ------------------------------ */
/* Mirror backend_ipr.rs signatures */

define_op!(sh4_muls32 (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_store32 (dst: *mut u32, src: *const u32));
define_op!(sh4_store32i (dst: *mut u32, imm: u32));
define_op!(sh4_and (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_xor (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_sub (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_add (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_addi (dst: *mut u32, src_n: *const u32, imm: u32));
define_op!(sh4_andi (dst: *mut u32, src: *const u32, imm: u32));
define_op!(sh4_neg (dst: *mut u32, src_n: *const u32));
define_op!(sh4_extub (dst: *mut u32, src: *const u32));
define_op!(sh4_dt (sr_T: *mut u32, dst: *mut u32));
define_op!(sh4_shlr (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_shllf (dst: *mut u32, src_n: *const u32, amt: u32));
define_op!(sh4_shlrf (dst: *mut u32, src_n: *const u32, amt: u32));

define_op!(sh4_write_mem8 (ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem16 (ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem32 (ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem64 (ctx: *mut Sh4Ctx, addr: *const u32, data: *const u64));

define_op!(sh4_read_mems8 (ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mems16 (ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mem32 (ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mem64 (ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u64));
define_op!(sh4_read_mem32i (ctx: *mut Sh4Ctx, addr: u32,          data: *mut u32));

define_op!(sh4_fadd (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fmul (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fdiv (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fsca (dst: *mut f32, index: *const u32));
define_op!(sh4_float (dst: *mut f32, src: *const u32));
define_op!(sh4_ftrc (dst: *mut u32, src: *const f32));

// Double precision versions
define_op!(sh4_fadd_d (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fsub_d (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fmul_d (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fdiv_d (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fsqrt_d (dst: *mut u32, src: *const u32));
define_op!(sh4_float_d (dst: *mut u32, src: *const u32));
define_op!(sh4_ftrc_d (dst: *mut u32, src: *const u32));

// define_op!(sh4_branch_cond       (ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32));
// define_op!(sh4_branch_cond_delay (ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32));
// define_op!(sh4_branch_delay      (ctx: *mut Sh4Ctx, target: u32));

define_op!(sh4_dec_branch_cond (dst: *mut u32, jdyn: *const u32, condition: u32, next: u32, target: u32));
define_op!(sh4_dec_call_decode (ctx: *mut Sh4Ctx));

#[inline(always)]
pub fn sh4_branch_cond(ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        (*ctx).dec_branch = 1;
        (*ctx).dec_branch_cond = condition;
        (*ctx).dec_branch_next = next;
        (*ctx).dec_branch_target = target;
    }
}

#[inline(always)]
pub fn sh4_branch_cond_delay(ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        sh4_store32(addr_of_mut!((*ctx).virt_jdyn), addr_of!((*ctx).sr_T));

        (*ctx).dec_branch = 1;
        (*ctx).dec_branch_cond = condition;
        (*ctx).dec_branch_next = next;
        (*ctx).dec_branch_target = target;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_branch_delay(ctx: *mut Sh4Ctx, target: u32) {
    unsafe {
        (*ctx).dec_branch = 2;
        (*ctx).dec_branch_target = target;
        (*ctx).dec_branch_dslot = 1;
    }
}

define_op!(sh4_read_mems16_i (ctx: *mut Sh4Ctx, addr: u32, data: *mut u32));

define_op!(sh4_fcmp_eq (sr_T: *mut u32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fcmp_gt (sr_T: *mut u32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fcmp_eq_d (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fcmp_gt_d (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fcnvds (dst: *mut u32, src: *const u32));
define_op!(sh4_fcnvsd (dst: *mut u32, src: *const u32));

define_op!(sh4_or (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_fsub (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fneg (dst: *mut u32, src: *const u32));
define_op!(sh4_fabs (dst: *mut u32, src: *const u32));
define_op!(sh4_fsqrt (dst: *mut f32, src: *const f32));
define_op!(sh4_fstsi (dst: *mut f32, imm: f32));

// frchg/fschg handled specially - panic for recompiler (must invalidate code)
#[inline(always)]
pub fn sh4_frchg() {
    panic!("frchg not supported in recompiler - requires code invalidation");
}

#[inline(always)]
pub fn sh4_fschg() {
    panic!("fschg not supported in recompiler - requires code invalidation");
}

// Branches handled specially - not using define_op! because they set decoder state
// Updated to use pointer parameters matching backend_ipr.rs signatures

#[inline(always)]
pub fn sh4_jmp(ctx: *mut Sh4Ctx, src: *const u32) {
    unsafe {
        (*ctx).dec_branch = 3;
        (*ctx).dec_branch_target_dynamic = src;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_jsr(ctx: *mut Sh4Ctx, src: *const u32, next_pc: u32) {
    unsafe {
        sh4_store32i(addr_of_mut!((*ctx).pr), next_pc);
        (*ctx).dec_branch = 3;
        (*ctx).dec_branch_target_dynamic = src;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_braf(ctx: *mut Sh4Ctx, src: *const u32, pc: u32) {
    unsafe {
        let target = (*src).wrapping_add(pc);
        (*ctx).dec_branch = 2;
        (*ctx).dec_branch_target = target;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_bsrf(ctx: *mut Sh4Ctx, src: *const u32, pc: u32) {
    unsafe {
        sh4_store32i(addr_of_mut!((*ctx).pr), pc.wrapping_add(4));
        let target = (*src).wrapping_add(pc);
        (*ctx).dec_branch = 2;
        (*ctx).dec_branch_target = target;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_rts(ctx: *mut Sh4Ctx, pr: *const u32) {
    unsafe {
        (*ctx).dec_branch = 3;
        (*ctx).dec_branch_target_dynamic = pr;
        (*ctx).dec_branch_dslot = 1;
    }
}

#[inline(always)]
pub fn sh4_rte(ctx: *mut Sh4Ctx, spc: *const u32, ssr: *const u32) {
    unsafe {
        // SR is restored from SSR AFTER delay slot execution
        (*ctx).dec_branch = 4;
        (*ctx).dec_branch_target_dynamic = spc;
        (*ctx).dec_branch_ssr = ssr;
        (*ctx).dec_branch_dslot = 1;
    }
}

define_op!(sh4_shad (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_shld (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_tas (sr_T: *mut u32, ctx: *mut Sh4Ctx, addr: *const u32));
define_op!(sh4_not (dst: *mut u32, src: *const u32));
define_op!(sh4_extuw (dst: *mut u32, src: *const u32));
define_op!(sh4_extsb (dst: *mut u32, src: *const u32));
define_op!(sh4_extsw (dst: *mut u32, src: *const u32));
define_op!(sh4_swapb (dst: *mut u32, src: *const u32));
define_op!(sh4_swapw (dst: *mut u32, src: *const u32));
define_op!(sh4_xtrct (dst: *mut u32, src_n: *const u32, src_m: *const u32));

define_op!(sh4_cmp_eq (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_hs (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_ge (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_hi (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_gt (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_pz (sr_T: *mut u32, src: *const u32));
define_op!(sh4_cmp_pl (sr_T: *mut u32, src: *const u32));
define_op!(sh4_tst (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));

define_op!(sh4_shll (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_shal (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_shar (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));

define_op!(sh4_cmp_eq_imm (sr_T: *mut u32, src: *const u32, imm: u32));
define_op!(sh4_tst_imm (sr_T: *mut u32, src: *const u32, imm: u32));
define_op!(sh4_and_imm (dst: *mut u32, src: *const u32, imm: u32));
define_op!(sh4_xor_imm (dst: *mut u32, src: *const u32, imm: u32));
define_op!(sh4_or_imm (dst: *mut u32, src: *const u32, imm: u32));

define_op!(sh4_rotcl (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_rotl (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_rotcr (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_rotr (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));

define_op!(sh4_movt (dst: *mut u32, sr_T: *const u32));
define_op!(sh4_clrt (sr_T: *mut u32));
define_op!(sh4_sett (sr_T: *mut u32));

define_op!(sh4_negc (sr_T: *mut u32, dst: *mut u32, src: *const u32));
define_op!(sh4_addc (sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_addv (sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_subc (sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_subv (sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32));

define_op!(sh4_muluw (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_mulsw (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_div0u (sr: *mut crate::SrStatus, sr_T: *mut u32));
define_op!(sh4_div0s (sr: *mut crate::SrStatus, sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_cmp_str (sr_T: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_dmulu (dst: *mut u64, src_n: *const u32, src_m: *const u32));
define_op!(sh4_dmuls (dst: *mut u64, src_n: *const u32, src_m: *const u32));
define_op!(sh4_div1 (sr: *mut crate::SrStatus, sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32));

define_op!(sh4_write_mem8_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32));
define_op!(sh4_write_mem16_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32));
define_op!(sh4_write_mem32_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32));
define_op!(sh4_read_mems8_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32));
define_op!(sh4_read_mems16_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32));
define_op!(sh4_read_mem32_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32));
define_op!(sh4_read_mem64_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u64));
define_op!(sh4_write_mem64_indexed (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u64));

define_op!(sh4_tst_mem (sr_T: *mut u32, ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u32));
define_op!(sh4_and_mem (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8));
define_op!(sh4_xor_mem (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8));
define_op!(sh4_or_mem (ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8));

define_op!(sh4_write_mem8_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32));
define_op!(sh4_write_mem16_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32));
define_op!(sh4_write_mem32_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32));
define_op!(sh4_read_mems8_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32));
define_op!(sh4_read_mems16_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32));
define_op!(sh4_read_mem32_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32));
define_op!(sh4_write_mem64_disp (ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u64));

define_op!(sh4_fsrra (dst: *mut f32, src: *const f32));
define_op!(sh4_fipr (fr: *mut f32, n: usize, m: usize));
define_op!(sh4_fmac (dst: *mut f32, fr0: *const f32, src_m: *const f32));
define_op!(sh4_ftrv (fr: *mut f32, xf: *const f32, n: usize));

define_op!(sh4_mac_w_mul (mac_full: *mut u64, temp0: *const u32, temp1: *const u32));
define_op!(sh4_mac_l_mul (mac_full: *mut u64, temp0: *const u32, temp1: *const u32));

