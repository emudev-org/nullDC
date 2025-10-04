#![allow(clippy::missing_safety_doc)]

use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::hint::black_box;
use std::ptr::{self, null_mut};

#[repr(C)]
pub struct Ctx {
    pub a: u64,
    pub b: u64,
    pub c: u64,
}

// --- TLS raw pointer (no Cell/UnsafeCell references involved) ---

#[thread_local]
pub static mut CTX_TLS: *mut Ctx = null_mut();

#[inline(always)]
pub unsafe fn ctx_set(p: *mut Ctx) { CTX_TLS = p; }

#[inline(always)]
pub unsafe fn ctx_get() -> *mut Ctx { CTX_TLS }

#[inline(always)]
pub unsafe fn ctx_clear() { CTX_TLS = null_mut(); }

// --- Helpers to allocate/free Ctx with std::alloc ---

pub unsafe fn ctx_alloc_bind(init: Ctx) -> *mut Ctx {
    let layout = Layout::new::<Ctx>();
    let p = alloc(layout) as *mut Ctx;
    if p.is_null() { handle_alloc_error(layout); }
    ptr::write(p, init);      // initialize in place (no &/&mut)
    ctx_set(p);
    p
}

pub unsafe fn ctx_unbind_free() {
    let p = ctx_get();
    if !p.is_null() {
        ctx_clear();
        ptr::drop_in_place(p);
        dealloc(p as *mut u8, Layout::new::<Ctx>());
    }
}

// --- Workloads ---

/// Baseline: pass ctx as a parameter; no TLS load inside the loop.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn param_sum(ctx: *mut Ctx, n: u64) -> u64 {
    let mut acc = 0u64;
    let mut i = 0u64;
    while i < n {
        // Simulate touching multiple fields
        (*ctx).a = (*ctx).a.wrapping_add(1);
        (*ctx).b ^= (*ctx).a;
        (*ctx).c = (*ctx).c.wrapping_add((*ctx).b);
        acc = acc.wrapping_add((*ctx).a ^ (*ctx).b ^ (*ctx).c).wrapping_add(i);
        i += 1;
    }
    black_box(acc)
}

/// TLS read hoisted once: amortized TLS cost.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn tls_sum_hoisted(n: u64) -> u64 {
    let ctx = ctx_get();                 // one TLS read
    if ctx.is_null() { return 0; }
    let mut acc = 0u64;
    let mut i = 0u64;
    while i < n {
        (*ctx).a = (*ctx).a.wrapping_add(1);
        (*ctx).b ^= (*ctx).a;
        (*ctx).c = (*ctx).c.wrapping_add((*ctx).b);
        acc = acc.wrapping_add((*ctx).a ^ (*ctx).b ^ (*ctx).c).wrapping_add(i);
        i += 1;
    }
    black_box(acc)
}

/// TLS read per iteration: worst case; forces a TLS access in the loop.
#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn tls_sum_per_iter(n: u64) -> u64 {
    let mut acc = 0u64;
    let mut i = 0u64;
    while i < n {
        let ctx = ctx_get();             // TLS each iteration
        if ctx.is_null() { return 0; }
        (*ctx).a = (*ctx).a.wrapping_add(1);
        (*ctx).b ^= (*ctx).a;
        (*ctx).c = (*ctx).c.wrapping_add((*ctx).b);
        acc = acc.wrapping_add((*ctx).a ^ (*ctx).b ^ (*ctx).c).wrapping_add(i);
        i += 1;
    }
    black_box(acc)
}
