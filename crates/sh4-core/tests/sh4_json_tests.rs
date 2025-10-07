// SingleStepTests integration test
use sh4_core::{Sh4Ctx, sh4_ipr_dispatcher, MemHandlers};
use sh4_core::sh4dec::{format_disas, SH4DecoderState};
use sh4_core::backend_ipr::sh4_store_fpscr;

mod test_reader;
use test_reader::{load_test_file, Sh4State};

// Memory operation expectation
#[derive(Debug, Clone)]
enum MemOp {
    Read { size: u8, addr: u32, value: u64 },
    Write { size: u8, addr: u32, value: u64 },
}

// Memory context for test execution
struct TestMemory {
    expectations: Vec<MemOp>,
    expectation_index: std::cell::Cell<usize>,
    default_opcode: u16,
    error: std::cell::RefCell<Option<String>>,
}

impl TestMemory {
    fn new(cycles: &[test_reader::Cycle], default_opcode: u16) -> Self {
        let mut expectations = Vec::new();

        for cycle in cycles {
            // Add fetch as a 16-bit read
            if let (Some(fetch_addr), Some(fetch_val)) = (cycle.fetch_addr, cycle.fetch_val) {
                expectations.push(MemOp::Read {
                    size: 16,
                    addr: fetch_addr,
                    value: fetch_val as u64,
                });
            }

            // Add data read
            if let (Some(read_addr), Some(read_val)) = (cycle.read_addr, cycle.read_val) {
                expectations.push(MemOp::Read {
                    size: 64, // Store full value, we'll mask based on actual read size
                    addr: read_addr,
                    value: read_val,
                });
            }

            // Add write
            if let (Some(write_addr), Some(write_val)) = (cycle.write_addr, cycle.write_val) {
                expectations.push(MemOp::Write {
                    size: 64, // Store full value, we'll mask based on actual write size
                    addr: write_addr,
                    value: write_val,
                });
            }
        }

        Self {
            expectations,
            expectation_index: std::cell::Cell::new(0),
            default_opcode,
            error: std::cell::RefCell::new(None),
        }
    }

    fn set_error(&self, msg: String) {
        *self.error.borrow_mut() = Some(msg);
    }

    fn handle_read(&self, size: u8, addr: u32) -> u64 {
        if self.error.borrow().is_some() {
            return 0; // Already have an error, just return default
        }
        let idx = self.expectation_index.get();

        // Special case for 16-bit reads: check if this matches an expectation
        if size == 16 && idx < self.expectations.len() {
            if let MemOp::Read { size: exp_size, addr: exp_addr, value } = self.expectations[idx] {
                // Match if it's an instruction fetch (size 16) or data read (size 64) at correct address
                if exp_addr == addr && (exp_size == 16 || exp_size == 64) {
                    self.expectation_index.set(idx + 1);
                    return value;
                }
            }
            // If address doesn't match and it's a 16-bit read, return default opcode (instruction fetch)
            return self.default_opcode as u64;
        }

        // For non-16-bit reads, must match expectations
        if idx >= self.expectations.len() {
            self.set_error(format!("Unexpected read{} at 0x{:08X}: no more expectations", size, addr));
            return 0;
        }

        match self.expectations[idx] {
            MemOp::Read { size: _exp_size, addr: exp_addr, value } => {
                if exp_addr != addr {
                    self.set_error(format!("Read{} address mismatch: expected 0x{:08X}, got 0x{:08X}",
                        size, exp_addr, addr));
                    return 0;
                }
                self.expectation_index.set(idx + 1);
                value
            }
            MemOp::Write { addr: exp_addr, .. } => {
                self.set_error(format!("Expected write at 0x{:08X}, but got read{} at 0x{:08X}",
                    exp_addr, size, addr));
                0
            }
        }
    }

    fn handle_write(&self, size: u8, addr: u32, value: u64) {
        if self.error.borrow().is_some() {
            return; // Already have an error
        }

        let idx = self.expectation_index.get();

        if idx >= self.expectations.len() {
            self.set_error(format!("Unexpected write{} at 0x{:08X} = 0x{:X}: no more expectations",
                size, addr, value));
            return;
        }

        match self.expectations[idx] {
            MemOp::Write { size: _exp_size, addr: exp_addr, value: exp_value } => {
                if exp_addr != addr {
                    self.set_error(format!("Write{} address mismatch: expected 0x{:08X}, got 0x{:08X}",
                        size, exp_addr, addr));
                    return;
                }

                // Mask both values to the actual write size
                let mask = match size {
                    8 => 0xFF,
                    16 => 0xFFFF,
                    32 => 0xFFFFFFFF,
                    64 => 0xFFFFFFFFFFFFFFFF,
                    _ => {
                        self.set_error(format!("Invalid write size: {}", size));
                        return;
                    }
                };
                let expected = exp_value & mask;
                let actual = value & mask;

                if actual != expected {
                    self.set_error(format!("Write{} value mismatch at 0x{:08X}: expected 0x{:X}, got 0x{:X}",
                        size, addr, expected, actual));
                    return;
                }

                self.expectation_index.set(idx + 1);
            }
            MemOp::Read { addr: exp_addr, .. } => {
                self.set_error(format!("Expected read at 0x{:08X}, but got write{} at 0x{:08X}",
                    exp_addr, size, addr));
            }
        }
    }
}

extern "C" fn test_mem_read8(ctx: *mut u8, addr: u32) -> u8 {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        (mem.handle_read(8, addr) & 0xFF) as u8
    }
}

extern "C" fn test_mem_read16(ctx: *mut u8, addr: u32) -> u16 {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        (mem.handle_read(16, addr) & 0xFFFF) as u16
    }
}

extern "C" fn test_mem_read32(ctx: *mut u8, addr: u32) -> u32 {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        (mem.handle_read(32, addr) & 0xFFFFFFFF) as u32
    }
}

extern "C" fn test_mem_read64(ctx: *mut u8, addr: u32) -> u64 {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        mem.handle_read(64, addr)
    }
}

extern "C" fn test_mem_write8(ctx: *mut u8, addr: u32, value: u8) {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        mem.handle_write(8, addr, value as u64);
    }
}

extern "C" fn test_mem_write16(ctx: *mut u8, addr: u32, value: u16) {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        mem.handle_write(16, addr, value as u64);
    }
}

extern "C" fn test_mem_write32(ctx: *mut u8, addr: u32, value: u32) {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        mem.handle_write(32, addr, value as u64);
    }
}

extern "C" fn test_mem_write64(ctx: *mut u8, addr: u32, value: u64) {
    unsafe {
        let mem = &*(ctx as *mut TestMemory);
        mem.handle_write(64, addr, value);
    }
}

// Macro to generate test functions for each test case
// Takes the test file name (without .json.bin extension) as a string literal
macro_rules! test_case {
    ($name:literal) => {
        paste::paste! {
            #[test]
            fn [<test_ $name>]() {
                let test_path = concat!("../../vendor/sh4-tests/", $name, ".json.bin");
                run_test_file(test_path);
            }
        }
    };
}

// Macro for expected failures (known bugs in emulator)
macro_rules! test_case_expected_fail {
    ($name:literal, $reason:literal) => {
        paste::paste! {
            #[test]
            #[should_panic(expected = $reason)]
            fn [<test_ $name>]() {
                let test_path = concat!("../../vendor/sh4-tests/", $name, ".json.bin");
                run_test_file(test_path);
            }
        }
    };
}

fn load_state_into_ctx(ctx: &mut Sh4Ctx, state: &Sh4State) {
    unsafe {
        // Set SR and FPSCR FIRST to establish which register banks are active
        // This ensures subsequent register loads go into the correct banks

        // Initialize SR with inverted RB bit to force bank switch detection
        ctx.sr.0 = (state.sr & !1) ^ (1 << 29); // SR without T bit, RB inverted
        ctx.sr_T = 0;

        // Set fpscr with DN bit inverted to force DAZ sync
        // Only invert bit 18 (DN) to avoid triggering bank switches (FR bit)
        ctx.fpscr.0 = state.fpscr ^ (1 << 18);

        // Now use the special store functions to set the correct values
        // This will trigger bank switches if needed
        use sh4_core::backend_ipr::{sh4_store32, sh4_store_sr_rest, sh4_store_fpscr};
        ctx.sr_T = state.sr & 1;
        sh4_store_sr_rest(&mut ctx.sr.0, &state.sr, &mut ctx.r[0], &mut ctx.r_bank[0]);
        sh4_store_fpscr(&mut ctx.fpscr.0, &state.fpscr, &mut ctx.fr.u32s[0], &mut ctx.xf.u32s[0]);

        // NOW load registers - they will go into whichever banks are currently active
        ctx.r.copy_from_slice(&state.r);
        ctx.r_bank.copy_from_slice(&state.r_bank);

        // Load FP registers - they go into current FR/XF based on FPSCR.FR
        for i in 0..16 {
            ctx.fr.u32s[i] = state.fp0[i];
            ctx.xf.u32s[i] = state.fp1[i];
        }

        // Load other control registers
        ctx.pc0 = state.pc;
        ctx.pc1 = state.pc.wrapping_add(2);
        ctx.pc2 = state.pc.wrapping_add(4);
        ctx.gbr = state.gbr;
        ctx.ssr = state.ssr;
        ctx.spc = state.spc;
        ctx.vbr = state.vbr;
        ctx.sgr = state.sgr;
        ctx.dbr = state.dbr;
        ctx.mac.parts.l = state.macl;
        ctx.mac.parts.h = state.mach;
        ctx.pr = state.pr;
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

fn compare_states(ctx: &Sh4Ctx, expected: &Sh4State, initial: &Sh4State) -> Result<(), String> {
    let mut errors = Vec::new();

    // Compare general registers R0-R15
    for i in 0..16 {
        if ctx.r[i] != expected.r[i] {
            errors.push(format!("R{} mismatch: got 0x{:08X}, expected 0x{:08X} (initial: 0x{:08X})",
                i, ctx.r[i], expected.r[i], initial.r[i]));
        }
    }

    // Compare banked registers R_0-R_7
    for i in 0..8 {
        if ctx.r_bank[i] != expected.r_bank[i] {
            errors.push(format!("R_{}bank mismatch: got 0x{:08X}, expected 0x{:08X} (initial: 0x{:08X})",
                i, ctx.r_bank[i], expected.r_bank[i], initial.r_bank[i]));
        }
    }

    // Compare FP registers - FP0 (main bank)
    unsafe {
        for i in 0..16 {
            let mine = f32::from_bits(ctx.fr.u32s[i]);
            let theirs = f32::from_bits(expected.fp0[i]);
            if !compare_floats(mine, theirs) {
                errors.push(format!("FP0[{}] mismatch: got 0x{:08X} ({:?}), expected 0x{:08X} ({:?})",
                    i, ctx.fr.u32s[i], mine, expected.fp0[i], theirs));
            }
        }

        // Compare FP registers - FP1 (alternate bank)
        for i in 0..16 {
            let mine = f32::from_bits(ctx.xf.u32s[i]);
            let theirs = f32::from_bits(expected.fp1[i]);
            if !compare_floats(mine, theirs) {
                errors.push(format!("FP1[{}] mismatch: got 0x{:08X} ({:?}), expected 0x{:08X} ({:?})",
                    i, ctx.xf.u32s[i], mine, expected.fp1[i], theirs));
            }
        }
    }

    // Compare PC (use pc0 which should have the final PC after execution)
    if ctx.pc0 != expected.pc {
        errors.push(format!("PC mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.pc0, expected.pc));
    }

    // Compare control registers
    if ctx.gbr != expected.gbr {
        errors.push(format!("GBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.gbr, expected.gbr));
    }

    // Compare SR - reconstruct from sr.0 and sr_T
    let ctx_sr = ctx.sr.0 | ctx.sr_T;
    if ctx_sr != expected.sr {
        errors.push(format!("SR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx_sr, expected.sr));
    }

    if ctx.ssr != expected.ssr {
        errors.push(format!("SSR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.ssr, expected.ssr));
    }

    if ctx.spc != expected.spc {
        errors.push(format!("SPC mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.spc, expected.spc));
    }

    if ctx.vbr != expected.vbr {
        errors.push(format!("VBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.vbr, expected.vbr));
    }

    if ctx.sgr != expected.sgr {
        errors.push(format!("SGR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.sgr, expected.sgr));
    }

    if ctx.dbr != expected.dbr {
        errors.push(format!("DBR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.dbr, expected.dbr));
    }

    // Compare MAC registers
    unsafe {
        if ctx.mac.parts.l != expected.macl {
            errors.push(format!("MACL mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.mac.parts.l, expected.macl));
        }
        if ctx.mac.parts.h != expected.mach {
            errors.push(format!("MACH mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.mac.parts.h, expected.mach));
        }
    }

    if ctx.pr != expected.pr {
        errors.push(format!("PR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.pr, expected.pr));
    }

    // Compare FPSCR
    if ctx.fpscr.0 != expected.fpscr {
        errors.push(format!("FPSCR mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.fpscr.0, expected.fpscr));
    }

    // Compare FPUL
    if ctx.fpul != expected.fpul {
        errors.push(format!("FPUL mismatch: got 0x{:08X}, expected 0x{:08X}", ctx.fpul, expected.fpul));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
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

// Helper to extract opcode pattern from test file path and convert to u16
// E.g., "0000mmmm00000011_sz0_pr0.json.bin" -> binary pattern with wildcards as 0
fn parse_opcode_pattern_from_path(test_path: &str) -> Option<(u16, String)> {
    let file_name = std::path::Path::new(test_path)
        .file_name()?
        .to_str()?;

    // Extract the pattern part before the first underscore or .json
    let pattern = file_name.split('_').next()?;

    if pattern.len() != 16 {
        return None;
    }

    let mut opcode: u16 = 0;
    for c in pattern.chars() {
        opcode <<= 1;
        if c == '1' {
            opcode |= 1;
        }
        // '0', 'n', 'm', 'd', 'i' all become 0
    }

    Some((opcode, pattern.to_string()))
}

// Generalized test runner that works for any test file
fn run_test_file(test_path: &str) {
    if !std::path::Path::new(test_path).exists() {
        panic!("Test file not found: {}", test_path);
    }

    let tests = load_test_file(test_path)
        .unwrap_or_else(|e| panic!("Failed to load test file {}: {}", test_path, e));

    // Parse opcode pattern from filename for disassembly
    let opcode_info = parse_opcode_pattern_from_path(test_path);

    let mut passed = 0;
    let mut failed = 0;

    for (test_idx, test) in tests.iter().enumerate() {
        // Create CPU context and memory for each test
        let mut ctx = Sh4Ctx::default();

        // Use opcodes[4] as the default/fallback opcode
        let default_opcode = test.opcodes[4] as u16;
        let mut test_memory = TestMemory::new(&test.cycles, default_opcode);

        // Setup memory handlers
        let handlers = MemHandlers {
            read8: test_mem_read8,
            read16: test_mem_read16,
            read32: test_mem_read32,
            read64: test_mem_read64,
            write8: test_mem_write8,
            write16: test_mem_write16,
            write32: test_mem_write32,
            write64: test_mem_write64,
        };

        for i in 0..256 {
            ctx.memhandlers[i] = handlers;
            ctx.memcontexts[i] = &mut test_memory as *mut TestMemory as *mut u8;
            ctx.memmask[i] = 0xFFFFFFFF;
            ctx.memmap[i] = 0 as *mut u8; // Handler index 0
        }

        // Load initial state
        load_state_into_ctx(&mut ctx, &test.initial);

        // Execute for the number of cycles in the test
        ctx.remaining_cycles = test.cycles.len() as i32;

        // Catch panics from memory validation and execution
        let exec_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unsafe {
                sh4_ipr_dispatcher(&mut ctx);
            }
        }));

        let result = match exec_result {
            Ok(_) => {
                // Check if there was a memory error
                if let Some(err) = test_memory.error.borrow().clone() {
                    Err(err)
                } else {
                    // Execution succeeded, now compare states
                    match compare_states(&ctx, &test.final_state, &test.initial) {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    }
                }
            }
            Err(e) => {
                // Extract panic message
                if let Some(s) = e.downcast_ref::<&str>() {
                    Err(s.to_string())
                } else if let Some(s) = e.downcast_ref::<String>() {
                    Err(s.clone())
                } else {
                    Err("Unknown panic".to_string())
                }
            }
        };

        match result {
            Ok(_) => {
                passed += 1;
            }
            Err(e) => {
                println!("Test {} failed: {}", test_idx, e);
                println!("  Opcodes: {:04X?}", test.opcodes);

                // Print disassembly if we have opcode info
                if let Some((opcode, ref pattern)) = opcode_info {
                    let decoder_state = SH4DecoderState {
                        pc: test.initial.pc,
                        fpscr_PR: (test.initial.fpscr & (1 << 19)) != 0,
                        fpscr_SZ: (test.initial.fpscr & (1 << 20)) != 0,
                    };
                    let disasm = format_disas(decoder_state, opcode);
                    println!("  Pattern: {} -> {}", pattern, disasm);
                }

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

    if failed > 0 {
        // Generate a helpful error message with disassembly
        let error_msg = if let Some((opcode, ref pattern)) = opcode_info {
            let decoder_state = SH4DecoderState {
                pc: 0,
                fpscr_PR: test_path.contains("_pr1"),
                fpscr_SZ: test_path.contains("_sz1"),
            };
            let disasm = format_disas(decoder_state, opcode);
            format!("{} tests failed - Pattern: {} -> {}", failed, pattern, disasm)
        } else {
            format!("{} tests failed", failed)
        };
        panic!("{}", error_msg);
    }
}

// Generate test cases using the macro
test_case!("0000000000001000_sz0_pr0");
test_case!("0000000000001001_sz0_pr0");
test_case!("0000000000001011_sz0_pr0");
test_case!("0000000000011000_sz0_pr0");
test_case!("0000000000011001_sz0_pr0");
// SLEEP instruction not implemented - should consume all remaining cycles
test_case_expected_fail!("0000000000011011_sz0_pr0", "tests failed");
test_case!("0000000000101000_sz0_pr0");
// RTE test data seems incorrect
// Per SH4 manual: "The other bits—S, T, M, Q, FD, BL, and RB—after modification are used for delay slot instruction execution"
// This means SSR.RB should be used during delay slot, not current SR.RB
test_case_expected_fail!("0000000000101011_sz0_pr0", "tests failed");
test_case!("0000000000111000_sz0_pr0");
test_case!("0000000001001000_sz0_pr0");
test_case!("0000000001011000_sz0_pr0");
test_case!("0000mmmm00000011_sz0_pr0");
test_case!("0000mmmm00100011_sz0_pr0");
test_case!("0000nnnn00000010_sz0_pr0");
test_case!("0000nnnn00001010_sz0_pr0");
test_case!("0000nnnn00010010_sz0_pr0");
test_case!("0000nnnn00011010_sz0_pr0");
test_case!("0000nnnn00100010_sz0_pr0");
test_case!("0000nnnn00101001_sz0_pr0");
test_case!("0000nnnn00101010_sz0_pr0");
test_case!("0000nnnn00110010_sz0_pr0");
test_case!("0000nnnn00111010_sz0_pr0");
test_case!("0000nnnn01000010_sz0_pr0");
test_case!("0000nnnn01011010_sz0_pr0");
test_case!("0000nnnn01101010_sz0_pr0");
test_case!("0000nnnn10010011_sz0_pr0");
test_case!("0000nnnn10100011_sz0_pr0");
test_case!("0000nnnn10110011_sz0_pr0");
test_case!("0000nnnn11000011_sz0_pr0");
test_case!("0000nnnn11111010_sz0_pr0");
test_case!("0000nnnn1mmm0010_sz0_pr0");
test_case!("0000nnnnmmmm0100_sz0_pr0");
test_case!("0000nnnnmmmm0101_sz0_pr0");
test_case!("0000nnnnmmmm0110_sz0_pr0");
test_case!("0000nnnnmmmm0111_sz0_pr0");
test_case!("0000nnnnmmmm1100_sz0_pr0");
test_case!("0000nnnnmmmm1101_sz0_pr0");
test_case!("0000nnnnmmmm1110_sz0_pr0");
test_case!("0001nnnnmmmmdddd_sz0_pr0");
test_case!("0010nnnnmmmm0000_sz0_pr0");
test_case!("0010nnnnmmmm0001_sz0_pr0");
test_case!("0010nnnnmmmm0010_sz0_pr0");
test_case!("0010nnnnmmmm0100_sz0_pr0");
test_case!("0010nnnnmmmm0101_sz0_pr0");
test_case!("0010nnnnmmmm0110_sz0_pr0");
test_case!("0010nnnnmmmm0111_sz0_pr0");
test_case!("0010nnnnmmmm1000_sz0_pr0");
test_case!("0010nnnnmmmm1001_sz0_pr0");
test_case!("0010nnnnmmmm1010_sz0_pr0");
test_case!("0010nnnnmmmm1011_sz0_pr0");
test_case!("0010nnnnmmmm1100_sz0_pr0");
test_case!("0010nnnnmmmm1101_sz0_pr0");
test_case!("0010nnnnmmmm1110_sz0_pr0");
test_case!("0010nnnnmmmm1111_sz0_pr0");
test_case!("0011nnnnmmmm0000_sz0_pr0");
test_case!("0011nnnnmmmm0010_sz0_pr0");
test_case!("0011nnnnmmmm0011_sz0_pr0");
test_case!("0011nnnnmmmm0100_sz0_pr0");
test_case!("0011nnnnmmmm0101_sz0_pr0");
test_case!("0011nnnnmmmm0110_sz0_pr0");
test_case!("0011nnnnmmmm0111_sz0_pr0");
test_case!("0011nnnnmmmm1000_sz0_pr0");
test_case!("0011nnnnmmmm1010_sz0_pr0");
test_case!("0011nnnnmmmm1011_sz0_pr0");
test_case!("0011nnnnmmmm1100_sz0_pr0");
test_case!("0011nnnnmmmm1101_sz0_pr0");
test_case!("0011nnnnmmmm1110_sz0_pr0");
test_case!("0011nnnnmmmm1111_sz0_pr0");
test_case!("0100mmmm00000110_sz0_pr0");
test_case!("0100mmmm00000111_sz0_pr0");
test_case!("0100mmmm00001010_sz0_pr0");
test_case!("0100mmmm00001011_sz0_pr0");
test_case!("0100mmmm00001110_sz0_pr0");
test_case!("0100mmmm00010110_sz0_pr0");
test_case!("0100mmmm00010111_sz0_pr0");
test_case!("0100mmmm00011010_sz0_pr0");
test_case!("0100mmmm00011110_sz0_pr0");
test_case!("0100mmmm00100110_sz0_pr0");
test_case!("0100mmmm00100111_sz0_pr0");
test_case!("0100mmmm00101010_sz0_pr0");
test_case!("0100mmmm00101011_sz0_pr0");
test_case!("0100mmmm00101110_sz0_pr0");
test_case!("0100mmmm00110111_sz0_pr0");
test_case!("0100mmmm00111110_sz0_pr0");
test_case!("0100mmmm01000111_sz0_pr0");
test_case!("0100mmmm01001110_sz0_pr0");
test_case!("0100mmmm01010110_sz0_pr0");
test_case!("0100mmmm01011010_sz0_pr0");
test_case!("0100mmmm01100110_sz0_pr0");
test_case!("0100mmmm01101010_sz0_pr0");
test_case!("0100mmmm11110110_sz0_pr0");
test_case!("0100mmmm11111010_sz0_pr0");
test_case!("0100mmmm1nnn0111_sz0_pr0");
test_case!("0100mmmm1nnn1110_sz0_pr0");
test_case!("0100nnnn00000000_sz0_pr0");
test_case!("0100nnnn00000001_sz0_pr0");
test_case!("0100nnnn00000010_sz0_pr0");
test_case!("0100nnnn00000011_sz0_pr0");
test_case!("0100nnnn00000100_sz0_pr0");
test_case!("0100nnnn00000101_sz0_pr0");
test_case!("0100nnnn00001000_sz0_pr0");
test_case!("0100nnnn00001001_sz0_pr0");
test_case!("0100nnnn00010000_sz0_pr0");
test_case!("0100nnnn00010001_sz0_pr0");
test_case!("0100nnnn00010010_sz0_pr0");
test_case!("0100nnnn00010011_sz0_pr0");
test_case!("0100nnnn00010101_sz0_pr0");
test_case!("0100nnnn00011000_sz0_pr0");
test_case!("0100nnnn00011001_sz0_pr0");
test_case!("0100nnnn00011011_sz0_pr0");
test_case!("0100nnnn00100000_sz0_pr0");
test_case!("0100nnnn00100001_sz0_pr0");
test_case!("0100nnnn00100010_sz0_pr0");
test_case!("0100nnnn00100011_sz0_pr0");
test_case!("0100nnnn00100100_sz0_pr0");
test_case!("0100nnnn00100101_sz0_pr0");
test_case!("0100nnnn00101000_sz0_pr0");
test_case!("0100nnnn00101001_sz0_pr0");
test_case!("0100nnnn00110010_sz0_pr0");
test_case!("0100nnnn00110011_sz0_pr0");
test_case!("0100nnnn01000011_sz0_pr0");
test_case!("0100nnnn01010010_sz0_pr0");
test_case!("0100nnnn01100010_sz0_pr0");
test_case!("0100nnnn11110010_sz0_pr0");
test_case!("0100nnnn1mmm0011_sz0_pr0");
test_case!("0100nnnnmmmm1100_sz0_pr0");
test_case!("0100nnnnmmmm1101_sz0_pr0");
test_case!("0101nnnnmmmmdddd_sz0_pr0");
test_case!("0110nnnnmmmm0000_sz0_pr0");
test_case!("0110nnnnmmmm0001_sz0_pr0");
test_case!("0110nnnnmmmm0010_sz0_pr0");
test_case!("0110nnnnmmmm0011_sz0_pr0");
test_case!("0110nnnnmmmm0100_sz0_pr0");
test_case!("0110nnnnmmmm0101_sz0_pr0");
test_case!("0110nnnnmmmm0110_sz0_pr0");
test_case!("0110nnnnmmmm0111_sz0_pr0");
test_case!("0110nnnnmmmm1000_sz0_pr0");
test_case!("0110nnnnmmmm1001_sz0_pr0");
test_case!("0110nnnnmmmm1010_sz0_pr0");
test_case!("0110nnnnmmmm1011_sz0_pr0");
test_case!("0110nnnnmmmm1100_sz0_pr0");
test_case!("0110nnnnmmmm1101_sz0_pr0");
test_case!("0110nnnnmmmm1110_sz0_pr0");
test_case!("0110nnnnmmmm1111_sz0_pr0");
test_case!("0111nnnniiiiiiii_sz0_pr0");
test_case!("10000000nnnndddd_sz0_pr0");
test_case!("10000001nnnndddd_sz0_pr0");
test_case!("10000100mmmmdddd_sz0_pr0");
test_case!("10000101mmmmdddd_sz0_pr0");
test_case!("10001000iiiiiiii_sz0_pr0");
test_case!("10001001dddddddd_sz0_pr0");
test_case!("10001011dddddddd_sz0_pr0");
test_case!("10001101dddddddd_sz0_pr0");
test_case!("10001111dddddddd_sz0_pr0");
test_case!("1001nnnndddddddd_sz0_pr0");
test_case!("1010dddddddddddd_sz0_pr0");
test_case!("1011dddddddddddd_sz0_pr0");
test_case!("11000000dddddddd_sz0_pr0");
test_case!("11000001dddddddd_sz0_pr0");
test_case!("11000010dddddddd_sz0_pr0");
test_case_expected_fail!("11000011iiiiiiii_sz0_pr0", "tests failed");
test_case!("11000100dddddddd_sz0_pr0");
test_case!("11000101dddddddd_sz0_pr0");
test_case!("11000110dddddddd_sz0_pr0");
test_case!("11000111dddddddd_sz0_pr0");
test_case!("11001000iiiiiiii_sz0_pr0");
test_case!("11001001iiiiiiii_sz0_pr0");
test_case!("11001010iiiiiiii_sz0_pr0");
test_case!("11001011iiiiiiii_sz0_pr0");
test_case!("11001100iiiiiiii_sz0_pr0");
test_case!("11001101iiiiiiii_sz0_pr0");
test_case!("11001110iiiiiiii_sz0_pr0");
test_case!("11001111iiiiiiii_sz0_pr0");
test_case!("1101nnnndddddddd_sz0_pr0");
test_case!("1110nnnniiiiiiii_sz0_pr0");
test_case!("1111001111111101_sz0_pr0");
test_case!("1111101111111101_sz0_pr0");
test_case!("1111mmm000111101_sz0_pr1");
test_case_expected_fail!("1111mmm010111101_sz0_pr0", "tests failed"); // this is actually undefined behaviour in the doc
test_case!("1111mmmm00011101_sz0_pr0");
test_case!("1111mmmm00111101_sz0_pr0");
test_case_expected_fail!("1111nn0111111101_sz0_pr0", "tests failed"); // ftrv xtrmx, fvN: rounding / nan handling is different vs tests
test_case_expected_fail!("1111nnmm11101101_sz0_pr0", "tests failed"); // fipr fnN, fvM: rounding / nan handling is different vs tests
test_case!("1111nnn000101101_sz0_pr1");
test_case!("1111nnn001001101_sz0_pr1");
test_case!("1111nnn001011101_sz0_pr1");
test_case!("1111nnn001101101_sz0_pr1");
test_case_expected_fail!("1111nnn010101101_sz0_pr0", "tests failed"); // this is actually undefined behaviour in the doc
test_case!("1111nnn011111101_sz0_pr0");
test_case!("1111nnn0mmm00000_sz0_pr1");
test_case!("1111nnn0mmm00001_sz0_pr1");
test_case!("1111nnn0mmm00010_sz0_pr1");
test_case!("1111nnn0mmm00011_sz0_pr1");
test_case!("1111nnn0mmm00100_sz0_pr1");
test_case!("1111nnn0mmm00101_sz0_pr1");
test_case!("1111nnn0mmm01100_sz1_pr0");
test_case!("1111nnn0mmm11100_sz1_pr0");
test_case!("1111nnn0mmmm0110_sz1_pr0");
test_case!("1111nnn0mmmm1000_sz1_pr0");
test_case!("1111nnn0mmmm1001_sz1_pr0");
test_case!("1111nnn1mmm01100_sz1_pr0");
test_case!("1111nnn1mmm11100_sz1_pr0");
test_case!("1111nnn1mmmm0110_sz1_pr0");
test_case!("1111nnn1mmmm1000_sz1_pr0");
test_case!("1111nnn1mmmm1001_sz1_pr0");
test_case!("1111nnnn00001101_sz0_pr0");
test_case!("1111nnnn00101101_sz0_pr0");
test_case!("1111nnnn01001101_sz0_pr0");
test_case!("1111nnnn01011101_sz0_pr0");
test_case!("1111nnnn01101101_sz0_pr0");
test_case!("1111nnnn01111101_sz0_pr0");
test_case!("1111nnnn10001101_sz0_pr0");
test_case!("1111nnnn10011101_sz0_pr0");
test_case!("1111nnnnmmm00111_sz1_pr0");
test_case!("1111nnnnmmm01010_sz1_pr0");
test_case!("1111nnnnmmm01011_sz1_pr0");
test_case!("1111nnnnmmm10111_sz1_pr0");
test_case!("1111nnnnmmm11010_sz1_pr0");
test_case!("1111nnnnmmm11011_sz1_pr0");
test_case!("1111nnnnmmmm0000_sz0_pr0");
test_case!("1111nnnnmmmm0001_sz0_pr0");
test_case!("1111nnnnmmmm0010_sz0_pr0");
test_case!("1111nnnnmmmm0011_sz0_pr0");
test_case!("1111nnnnmmmm0100_sz0_pr0");
test_case!("1111nnnnmmmm0101_sz0_pr0");
test_case!("1111nnnnmmmm0110_sz0_pr0");
test_case!("1111nnnnmmmm0111_sz0_pr0");
test_case!("1111nnnnmmmm1000_sz0_pr0");
test_case!("1111nnnnmmmm1001_sz0_pr0");
test_case!("1111nnnnmmmm1010_sz0_pr0");
test_case!("1111nnnnmmmm1011_sz0_pr0");
test_case!("1111nnnnmmmm1100_sz0_pr0");
test_case!("1111nnnnmmmm1110_sz0_pr0");
