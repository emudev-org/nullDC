// src/main.rs
#![feature(thread_local)]


#![allow(clippy::missing_safety_doc)]

mod core;

use std::time::Instant;

fn time(label: &str, f: impl FnOnce() -> u64) {
    let t0 = Instant::now();
    let r = f();
    let dt = t0.elapsed();
    println!("{label}: result={r} elapsed={:?}", dt);
}

fn main() {
    // Prepare a context and bind it (TLS path needs a bound context).
    unsafe {
        let _p = core::ctx_alloc_bind(core::Ctx { a: 1, b: 2, c: 3 });

        let n = 1_000_000_000;

        // Baseline: pass pointer as a parameter (no TLS inside the loop)
        time("param_sum(ctx, n)", || unsafe {
            core::param_sum(_p, n)
        });

        // TLS read hoisted once outside the loop
        time("tls_sum_hoisted(n)", || unsafe {
            core::tls_sum_hoisted(n)
        });

        // TLS read performed per iteration (worst case)
        time("tls_sum_per_iter(n)", || unsafe {
            core::tls_sum_per_iter(n)
        });

        core::ctx_unbind_free();
    }
}
