//! backend_fns.rs — “dec” backend that records ops as [handler_ptr][args...]

use crate::dreamcast::Dreamcast;
// Adjust this path if needed:
use crate::dreamcast::sh4::backend_ipr;

use std::{cell::RefCell, mem, ptr::NonNull};

use paste::paste;

/// Every record starts with this function pointer, followed by that handler’s packed args.
/// The handler receives `data_ptr` (right after the function pointer) and must return
/// the pointer immediately after its own arguments (i.e., the next record’s handler).
type Handler = unsafe extern "C" fn(data_ptr: *mut u8) -> *mut u8;

thread_local! {
    /// A grow-only byte arena so all pointers remain valid until `clear`.
    static ARENA: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1 << 20)); // 1 MiB to start

    /// Starts (addresses) of records (each is the address where the Handler was written).
    static PTRS:  RefCell<Vec<NonNull<u8>>> = RefCell::new(Vec::with_capacity(1 << 16));
}

/* ------------------------- Byte-pack utilities (aligned) ------------------------- */

#[inline(always)]
fn align_up(off: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (off + (align - 1)) & !(align - 1)
}

#[inline(always)]
unsafe fn arena_push_aligned<T: Copy>(buf: &mut Vec<u8>, v: T) -> *mut u8 {
    let align = mem::align_of::<T>();
    let size  = mem::size_of::<T>();
    let base  = align_up(buf.len(), align);

    if buf.len() < base { buf.resize(base, 0); }
    let start = base;
    let end   = start + size;

    buf.reserve(size);
    // Extend with uninit then write
    let old_len = buf.len();
    buf.set_len(end);

    let dst = buf.as_mut_ptr().add(start) as *mut T;
    dst.write_unaligned(v);
    buf.as_mut_ptr().add(start)
}

/// Read aligned value from a moving pointer, advancing it. Must match `arena_push_aligned`.
#[inline(always)]
unsafe fn read_aligned<T: Copy>(p: &mut *mut u8) -> T {
    let align = mem::align_of::<T>();
    let addr  = *p as usize;
    let aligned = align_up(addr, align);
    *p = aligned as *mut u8;

    let val = (aligned as *const T).read_unaligned();
    *p = (*p).add(mem::size_of::<T>());
    val
}

/* ----------------------------- Recording primitive ------------------------------ */

#[inline(always)]
unsafe fn start_record(buf: &mut Vec<u8>, h: Handler) -> *mut u8 {
    // Write the handler pointer first; return the address where it was written
    arena_push_aligned::<Handler>(buf, h)
}

#[inline(always)]
fn push_ptr_start(start_ptr: *mut u8) {
    // Save record start (handler position)
    PTRS.with(|p| unsafe {
        p.borrow_mut()
            .push(NonNull::new_unchecked(start_ptr));
    });
}

/* ------------------------------- Public utilities ------------------------------- */

#[inline]
pub fn ptrs_snapshot() -> Vec<NonNull<u8>> {
    PTRS.with(|p| p.borrow().iter().copied().collect())
}

#[inline]
pub fn clear() {
    ARENA.with(|a| a.borrow_mut().clear());
    PTRS.with(|p| p.borrow_mut().clear());
}

/// Execute the single record starting at `record_ptr` (address of handler).
/// Returns the pointer to the *next* record’s handler (right after this record’s data).
#[inline]
pub unsafe fn step_once(mut record_ptr: *mut u8) -> *mut u8 {
    // Load handler
    let h = {
        // Align as when we wrote it
        let aligned = align_up(record_ptr as usize, mem::align_of::<Handler>()) as *mut u8;
        record_ptr = aligned;
        (aligned as *const Handler).read_unaligned()
    };
    // Data starts immediately after the stored function pointer:
    let mut data_ptr = record_ptr.add(mem::size_of::<Handler>());
    // Let the handler consume its args and return the next record pointer
    h(data_ptr)
}

/// Convenience: walk and execute a chain starting at `start` for `n` records.
#[inline]
pub unsafe fn run_all_from(mut start: *mut u8, n: usize) -> *mut u8 {
    let mut p = start;
    for _ in 0..n {
        p = step_once(p);
    }
    p
}

/* ----------------------------- Macro: define ops --------------------------------
   For each `sh4_*` we generate:
     - a recorder function with the exact signature
     - a handler that unpacks the same arguments and calls backend_ipr::sh4_*
----------------------------------------------------------------------------------*/

macro_rules! define_op {
    ($name:ident ( $($arg:ident : $ty:ty),* $(,)? )) => {
        paste! {
            #[inline(always)]
            pub fn $name( $($arg : $ty),* ) {
                ARENA.with(|arena| {
                    let mut buf = arena.borrow_mut();
                    // Write handler pointer
                    let start = unsafe { start_record(&mut *buf, [<handler_ $name>] as Handler) };
                    // Pack arguments
                    $( unsafe { arena_push_aligned::<$ty>(&mut *buf, $arg); } )*
                    // Track record start
                    push_ptr_start(start);
                });
            }

            #[inline(always)]
            unsafe extern "C" fn [<handler_ $name>](mut p: *mut u8) -> *mut u8 {
                $( let $arg : $ty = read_aligned::<$ty>(&mut p); )*
                // Call through to the immediate backend
                backend_ipr::$name( $($arg),* );
                // p now points right after our args => next record’s handler
                p
            }
        }
    };
}


/* ------------------------------- Op definitions ---------------------------------
   Below are all the ops you listed, with signatures mirroring backend_ipr.rs.
   Add/remove lines as needed; the macro will generate both the recorder & handler.
----------------------------------------------------------------------------------*/

// Integer ops
define_op!(sh4_muls32 (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_store32 (dst: *mut u32, src: *const u32));
define_op!(sh4_store32i(dst: *mut u32, imm: u32));
define_op!(sh4_and    (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_xor    (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_sub    (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_add    (dst: *mut u32, src_n: *const u32, src_m: *const u32));
define_op!(sh4_addi   (dst: *mut u32, src_n: *const u32, imm: u32));
define_op!(sh4_neg    (dst: *mut u32, src_n: *const u32));
define_op!(sh4_extub  (dst: *mut u32, src: *const u32));
define_op!(sh4_dt     (sr_T: *mut u32, dst: *mut u32));
define_op!(sh4_shlr   (sr_T: *mut u32, dst: *mut u32, src_n: *const u32));
define_op!(sh4_shllf  (dst: *mut u32, src_n: *const u32, amt: u32));
define_op!(sh4_shlrf  (dst: *mut u32, src_n: *const u32, amt: u32));

// Memory ops (8/16/32/64 bit)
define_op!(sh4_write_mem8 (dc: *mut Dreamcast, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem16(dc: *mut Dreamcast, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem32(dc: *mut Dreamcast, addr: *const u32, data: *const u32));
define_op!(sh4_write_mem64(dc: *mut Dreamcast, addr: *const u32, data: *const u64));

define_op!(sh4_read_mems8 (dc: *mut Dreamcast, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mems16(dc: *mut Dreamcast, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mem32 (dc: *mut Dreamcast, addr: *const u32, data: *mut u32));
define_op!(sh4_read_mem64 (dc: *mut Dreamcast, addr: *const u32, data: *mut u64));
define_op!(sh4_read_mem32i(dc: *mut Dreamcast, addr: u32,          data: *mut u32));

// FP ops
define_op!(sh4_fadd (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fmul (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fdiv (dst: *mut f32, src_n: *const f32, src_m: *const f32));
define_op!(sh4_fsca (dst: *mut f32, index: *const u32));
define_op!(sh4_float(dst: *mut f32, src: *const u32));
define_op!(sh4_ftrc (dst: *mut u32, src: *const f32));

// Branching
// define_op!(sh4_branch_cond       (dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32));
// define_op!(sh4_branch_cond_delay (dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32));
// define_op!(sh4_branch_delay      (dc: *mut Dreamcast, target: u32));

#[inline(always)]
pub fn sh4_branch_cond(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    panic!("sh4_branch_cond is not implemented in backend_fns");
}

#[inline(always)]
pub fn sh4_branch_cond_delay(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    panic!("sh4_branch_cond_delay is not implemented in backend_fns");
}

#[inline(always)]
pub fn sh4_branch_delay(dc: *mut Dreamcast, target: u32) {
    panic!("sh4_branch_delay is not implemented in backend_fns");
}