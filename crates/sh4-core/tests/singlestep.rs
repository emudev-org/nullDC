// SingleStepTests integration test
use sh4_core::{Sh4Ctx, sh4_ipr_dispatcher};

mod test_reader;
use test_reader::{load_test_file, Sh4State};

fn load_state_into_ctx(ctx: &mut Sh4Ctx, state: &Sh4State) {
    unsafe {
        // Load general registers
        ctx.r.copy_from_slice(&state.r);
        ctx.r_bank.copy_from_slice(&state.r_bank);

        // Load FP registers - FR bank based on FPSCR.FR bit
        // For now, load FP0 into FR
        for i in 0..16 {
            ctx.fr.u32s[i] = state.fp0[i];
            ctx.xf.u32s[i] = state.fp1[i];
        }

        // Load control registers
        ctx.pc0 = state.pc;
        ctx.pc1 = state.pc.wrapping_add(2);
        ctx.pc2 = state.pc.wrapping_add(4);
        ctx.gbr = state.gbr;
        ctx.sr.0 = state.sr & !1; // SR without T bit
        ctx.sr_T = state.sr & 1; // Extract T bit
        ctx.ssr = state.ssr;
        ctx.spc = state.spc;
        ctx.vbr = state.vbr;
        ctx.sgr = state.sgr;
        ctx.dbr = state.dbr;
        ctx.mac.parts.l = state.macl;
        ctx.mac.parts.h = state.mach;
        ctx.pr = state.pr;
        ctx.fpscr.0 = state.fpscr;
        ctx.fpul = state.fpul;
    }
}

fn compare_floats(mine: f32, theirs: f32) -> bool {
    // Regular float equality
    if mine == theirs {
        return true;
    }

    // Get u32 versions
    let mydata = mine.to_bits();
    let theirdata = theirs.to_bits();

    // Integer (exact bit) equality
    if mydata == theirdata {
        return true;
    }

    // Check for both different NaN but still NaN
    if mine.is_nan() && theirs.is_nan() {
        return true;
    }

    // Check for rounding-level errors
    let diff = mydata.wrapping_sub(theirdata);
    if diff < 5 || diff > 0xFFFFFFFD {
        return true;
    }

    // More rounding
    if (theirs - mine).abs() < 0.0000001 {
        return true;
    }

    // Special cases from the README
    if (mydata == 0x7F800000 && theirdata == 0xFF7FFFFF) ||
       (mydata == 0x36865c49 && theirdata == 0xb1e2c629) ||
       (mydata == 0x7ff84903 && theirdata == 0x7fc00000) ||
       (mydata == 0xff800000 && theirdata == 0x7F7FFFFF) {
        return true;
    }

    false
}

fn compare_states(ctx: &Sh4Ctx, expected: &Sh4State) -> Result<(), String> {
    // Compare general registers R0-R15
    for i in 0..16 {
        if ctx.r[i] != expected.r[i] {
            return Err(format!("R{} mismatch: got 0x{:08X}, expected 0x{:08X}", i, ctx.r[i], expected.r[i]));
        }
    }

    // Compare banked registers R_0-R_7
    for i in 0..8 {
        if ctx.r_bank[i] != expected.r_bank[i] {
            return Err(format!("R_{}bank mismatch: got 0x{:08X}, expected 0x{:08X}", i, ctx.r_bank[i], expected.r_bank[i]));
        }
    }

    // Compare FP registers - FP0 (main bank)
    unsafe {
        for i in 0..16 {
            let mine = f32::from_bits(ctx.fr.u32s[i]);
            let theirs = f32::from_bits(expected.fp0[i]);
            if !compare_floats(mine, theirs) {
                return Err(format!("FP0[{}] mismatch: got 0x{:08X} ({:?}), expected 0x{:08X} ({:?})",
                    i, ctx.fr.u32s[i], mine, expected.fp0[i], theirs));
            }
        }

        // Compare FP registers - FP1 (alternate bank)
        for i in 0..16 {
            let mine = f32::from_bits(ctx.xf.u32s[i]);
            let theirs = f32::from_bits(expected.fp1[i]);
            if !compare_floats(mine, theirs) {
                return Err(format!("FP1[{}] mismatch: got 0x{:08X} ({:?}), expected 0x{:08X} ({:?})",
                    i, ctx.xf.u32s[i], mine, expected.fp1[i], theirs));
            }
        }
    }

    // Compare PC (use pc0 which should have the final PC after execution)
    if ctx.pc0 != expected.pc {
        return Err(format!("PC mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.pc0, expected.pc));
    }

    // Compare control registers
    if ctx.gbr != expected.gbr {
        return Err(format!("GBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.gbr, expected.gbr));
    }

    // Compare SR - reconstruct from sr.0 and sr_T
    let ctx_sr = ctx.sr.0 | ctx.sr_T;
    if ctx_sr != expected.sr {
        return Err(format!("SR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx_sr, expected.sr));
    }

    if ctx.ssr != expected.ssr {
        return Err(format!("SSR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.ssr, expected.ssr));
    }

    if ctx.spc != expected.spc {
        return Err(format!("SPC mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.spc, expected.spc));
    }

    if ctx.vbr != expected.vbr {
        return Err(format!("VBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.vbr, expected.vbr));
    }

    if ctx.sgr != expected.sgr {
        return Err(format!("SGR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.sgr, expected.sgr));
    }

    if ctx.dbr != expected.dbr {
        return Err(format!("DBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.dbr, expected.dbr));
    }

    // Compare MAC registers
    unsafe {
        if ctx.mac.parts.l != expected.macl {
            return Err(format!("MACL mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.mac.parts.l, expected.macl));
        }
        if ctx.mac.parts.h != expected.mach {
            return Err(format!("MACH mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.mac.parts.h, expected.mach));
        }
    }

    if ctx.pr != expected.pr {
        return Err(format!("PR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.pr, expected.pr));
    }

    // Compare FPSCR
    if ctx.fpscr.0 != expected.fpscr {
        return Err(format!("FPSCR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.fpscr.0, expected.fpscr));
    }

    // Compare FPUL
    if ctx.fpul != expected.fpul {
        return Err(format!("FPUL mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.fpul, expected.fpul));
    }

    Ok(())
}

#[test]
fn test_load_single_instruction() {
    // Test loading a single test file
    let test_path = "../../vendor/sh4-tests/0000000000001000_sz0_pr0.json.bin";
    if !std::path::Path::new(test_path).exists() {
        println!("Test file not found, skipping");
        return;
    }

    let tests = load_test_file(test_path).expect("Failed to load test file");
    println!("Loaded {} tests from file", tests.len());

    // Just verify we can load the file
    assert_eq!(tests.len(), 500);
}

#[test]
fn test_execute_all_instructions() {
    let test_path = "../../vendor/sh4-tests/0000000000001000_sz0_pr0.json.bin";
    if !std::path::Path::new(test_path).exists() {
        println!("Test file not found, skipping");
        return;
    }

    let tests = load_test_file(test_path).expect("Failed to load test file");

    let mut passed = 0;
    let mut failed = 0;

    for (test_idx, test) in tests.iter().enumerate() {
        // Create CPU context and memory for each test
        let mut ctx = Sh4Ctx::default();
        let mut memory = vec![0u8; 64 * 1024 * 1024];

        // Setup memory map
        for i in 0..256 {
            ctx.memmap[i] = memory.as_mut_ptr();
            ctx.memmask[i] = (memory.len() - 1) as u32;
        }

        // Load initial state
        load_state_into_ctx(&mut ctx, &test.initial);

        // Write instruction opcodes to memory
        let pc = test.initial.pc;
        for (i, &opcode) in test.opcodes.iter().enumerate() {
            let addr = pc.wrapping_add((i * 2) as u32);
            let offset = (addr as usize) & (memory.len() - 1);
            memory[offset] = (opcode & 0xFF) as u8;
            memory[offset + 1] = ((opcode >> 8) & 0xFF) as u8;
        }

        // Execute for the number of cycles in the test
        ctx.remaining_cycles = test.cycles.len() as i32;
        unsafe {
            sh4_ipr_dispatcher(&mut ctx);
        }

        // Compare final state
        match compare_states(&ctx, &test.final_state) {
            Ok(_) => passed += 1,
            Err(e) => {
                println!("Test {} failed: {}", test_idx, e);
                println!("  Opcodes: {:04X?}", test.opcodes);
                println!("  PC: 0x{:08X} -> 0x{:08X}", test.initial.pc, test.final_state.pc);
                failed += 1;
                if failed >= 10 {
                    println!("Stopping after 10 failures");
                    break;
                }
            }
        }
    }

    println!("\nTest results: {} passed, {} failed out of {} total",
             passed, failed, tests.len());

    assert_eq!(failed, 0, "{} tests failed", failed);
}

#[test]
fn test_execute_single_instruction() {
    let test_path = "../../vendor/sh4-tests/0000000000001000_sz0_pr0.json.bin";
    if !std::path::Path::new(test_path).exists() {
        println!("Test file not found, skipping");
        return;
    }

    let tests = load_test_file(test_path).expect("Failed to load test file");
    let test = &tests[0];

    // Create CPU context and memory
    let mut ctx = Sh4Ctx::default();
    let mut memory = vec![0u8; 64 * 1024 * 1024]; // 64MB for testing

    // Setup memory map - map all regions to our test memory
    for i in 0..256 {
        ctx.memmap[i] = memory.as_mut_ptr();
        ctx.memmask[i] = (memory.len() - 1) as u32;
    }

    // Load initial state
    load_state_into_ctx(&mut ctx, &test.initial);

    // Write instruction opcodes to memory at PC
    let pc = test.initial.pc;
    for (i, &opcode) in test.opcodes.iter().enumerate() {
        let addr = pc.wrapping_add((i * 2) as u32);
        let offset = (addr as usize) & (memory.len() - 1);
        memory[offset] = (opcode & 0xFF) as u8;
        memory[offset + 1] = ((opcode >> 8) & 0xFF) as u8;
    }

    // Execute for the number of cycles in the test
    ctx.remaining_cycles = test.cycles.len() as i32;
    unsafe {
        sh4_ipr_dispatcher(&mut ctx);
    }

    // Compare final state
    match compare_states(&ctx, &test.final_state) {
        Ok(_) => {},
        Err(e) => panic!("Test failed: {}", e),
    }
}
