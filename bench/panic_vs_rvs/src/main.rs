use std::panic;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;

/// Simulate an instruction with Result error handling
#[inline(never)]
fn instr_result(x: u64) -> Result<u64, ()> {
    if x % 1000 == 0 { // ~0.1% error rate
        Err(())
    } else {
        Ok(x.wrapping_mul(3).wrapping_add(1))
    }
}

/// Simulate an instruction that panics on error
#[inline(never)]
fn instr_panic(x: u64) -> u64 {
    if x % 1000 == 0 {
        panic!("illegal instruction");
    } else {
        x.wrapping_mul(3).wrapping_add(1)
    }
}

/// Run N iterations with Result-based error codes (resuming after error)
fn loop_with_result(n: usize) -> u64 {
    let mut acc = 1u64;
    let mut i = 0;

    while i < n {
        match instr_result(acc ^ (i as u64)) {
            Ok(v) => acc = v,
            Err(_) => {
                // skip this instruction, acc unchanged
            }
        }
        i += 1;
    }

    acc
}

/// Run N iterations with panic/unwind (resuming after error)
fn loop_with_panic(n: usize) -> u64 {
    let mut acc = 1u64;
    let mut i = 0;

    while i < n {
        let result = catch_unwind(AssertUnwindSafe(|| {
            while i < n {
                // advance i first, so the faulting instruction is "consumed"
                let j = i;
                i += 1;
                acc = instr_panic(acc ^ (j as u64));
            }
        }));

        if result.is_err() {
            // panic already consumed instruction j, so just resume
            continue;
        }
    }

    acc
}




fn main() {
    panic::set_hook(Box::new(|_| {}));

    let n = 10_000_000; // simulate 10 million instructions

    // Warmup
    loop_with_result(n);
    loop_with_panic(n);

    // Benchmark Result
    let start = Instant::now();
    let res1 = loop_with_result(n);
    let dur_result = start.elapsed();

    // Benchmark Panic
    let start = Instant::now();
    let res2 = loop_with_panic(n);
    let dur_panic = start.elapsed();

    println!("Result-based error codes: {:?} {}", dur_result, res1);
    println!("Panic + catch_unwind:     {:?} {}", dur_panic, res2);
}
