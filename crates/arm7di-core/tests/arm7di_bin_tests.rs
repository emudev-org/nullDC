// ARM7DI binary test harness
// Loads and executes test binaries from vendor/arm7di-tests-dreamcast/bins/

use arm7di_core::{Arm7Context, Arm7Di, R15_ARM_NEXT};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::ptr::NonNull;

const MEMORY_SIZE: usize = 128 * 1024; // 128KB test memory
const MAX_CYCLES: u32 = 100000; // Safety limit
const END_MARKER: u32 = 0xDEADBEEF; // Test completion marker

// Simple test memory - just a flat buffer
struct TestMemory {
    data: Vec<u8>,
}

impl TestMemory {
    fn new() -> Self {
        Self {
            data: vec![0; MEMORY_SIZE],
        }
    }

    fn load_binary<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let mut file = File::open(path)?;
        let bytes_read = file.read(&mut self.data)?;
        println!("Loaded {} bytes into test memory", bytes_read);
        Ok(())
    }
}

// Memory callbacks for ARM7DI
fn test_read8(addr: u32, ctx: &mut Arm7Context) -> u8 {
    if let Some(mem_ptr) = ctx.aica_ram {
        let mask = ctx.aram_mask;
        unsafe {
            mem_ptr.as_ptr().add((addr & mask) as usize).read()
        }
    } else {
        0
    }
}

fn test_read32(addr: u32, ctx: &mut Arm7Context) -> u32 {
    if let Some(mem_ptr) = ctx.aica_ram {
        let mask = ctx.aram_mask;
        unsafe {
            let base = mem_ptr.as_ptr().add((addr & mask) as usize);
            u32::from_le_bytes([
                base.read(),
                base.add(1).read(),
                base.add(2).read(),
                base.add(3).read(),
            ])
        }
    } else {
        0
    }
}

fn test_write8(addr: u32, value: u8, ctx: &mut Arm7Context) {
    if let Some(mem_ptr) = ctx.aica_ram {
        let mask = ctx.aram_mask;
        unsafe {
            mem_ptr.as_ptr().add((addr & mask) as usize).write(value);
        }
    }
}

fn test_write32(addr: u32, value: u32, ctx: &mut Arm7Context) {
    if let Some(mem_ptr) = ctx.aica_ram {
        let mask = ctx.aram_mask;
        unsafe {
            let base = mem_ptr.as_ptr().add((addr & mask) as usize);
            let bytes = value.to_le_bytes();
            base.write(bytes[0]);
            base.add(1).write(bytes[1]);
            base.add(2).write(bytes[2]);
            base.add(3).write(bytes[3]);
        }
    }
}

fn run_test_binary(test_path: &str) -> Result<(), String> {
    if !Path::new(test_path).exists() {
        return Err(format!("Test file not found: {}", test_path));
    }

    // Create test memory and load binary
    let mut memory = TestMemory::new();
    memory.load_binary(test_path)
        .map_err(|e| format!("Failed to load binary: {}", e))?;

    // Set up ARM7 context
    let mut ctx = Arm7Context::new();
    ctx.aica_ram = NonNull::new(memory.data.as_mut_ptr());
    ctx.aram_mask = (MEMORY_SIZE - 1) as u32;
    ctx.read8 = Some(test_read8);
    ctx.read32 = Some(test_read32);
    ctx.write8 = Some(test_write8);
    ctx.write32 = Some(test_write32);
    ctx.enabled = true;

    // Initialize CPU state - PC starts at 0x0
    ctx.regs[R15_ARM_NEXT].set(0);
    ctx.regs[15].set(8); // ARM mode: visible PC = actual PC + 8

    // Execute test
    let mut cycles = 0;

    loop {
        // Check for end marker at next instruction
        let next_pc = ctx.regs[R15_ARM_NEXT].get();

        let next_insn = ctx.read32(next_pc);

        if next_insn == END_MARKER {
            println!("Test completed after {} cycles", cycles);
            break;
        }

        // Check timeout
        if cycles >= MAX_CYCLES {
            return Err(format!("Test timeout after {} cycles (no END_MARKER found)", MAX_CYCLES));
        }

        // Execute one instruction
        let mut arm = Arm7Di::new(&mut ctx);
        arm.step();
        cycles += 1;
    }

    // Check result - r1 should be 0 for success
    let r1 = ctx.regs[1].get();
    if r1 == 0 {
        Ok(())
    } else {
        // Decode error flags
        let mut errors = Vec::new();
        if r1 & 0x10 != 0 {
            errors.push("BAD_Rd".to_string());
        }
        if r1 & 0x20 != 0 {
            errors.push("BAD_Rn".to_string());
        }
        if r1 & 0xFF00 != 0 {
            errors.push(format!("OTHER(0x{:x})", r1 & 0xFF00));
        }
        let remaining = r1 & !0x30 & 0xFF;
        if remaining != 0 {
            errors.push(format!("FLAGS(0x{:x})", remaining));
        }

        Err(format!("Test failed with r1 = 0x{:08X} ({})", r1, errors.join(", ")))
    }
}

// Macro to generate test functions
macro_rules! test_case {
    ($name:literal) => {
        paste::paste! {
            #[test]
            fn [<test_ $name:lower>]() {
                let test_path = concat!("../../vendor/arm7di-tests-dreamcast/bins/", $name, ".s.bin");
                match run_test_binary(test_path) {
                    Ok(_) => println!("✓ {} passed", $name),
                    Err(e) => panic!("{} failed: {}", $name, e),
                }
            }
        }
    };
}

// All 45 test cases (from vendor/arm7di-tests-dreamcast/bins/)
test_case!("ADC_1");
test_case!("ADD_1");
test_case!("AND_1");
test_case!("BIC_1");
test_case!("CMN_1");
test_case!("EOR_1");
test_case!("LDM_1");
test_case!("LDM_2");
test_case!("LDM_3");
test_case!("LDM_4");
test_case!("LDM_5");
test_case!("LDM_6");
test_case!("LDM_7");
test_case!("LDM_8");
test_case!("LDR_1");
test_case!("LDR_10");
test_case!("LDR_11");
test_case!("LDR_12");
test_case!("LDR_2");
test_case!("LDR_3");
test_case!("LDR_4");
test_case!("LDR_5");
test_case!("LDR_6");
test_case!("LDR_7");
test_case!("LDR_8");
test_case!("LDR_9");
test_case!("LDRB_1");
test_case!("LDRB_2");
test_case!("LDRB_3");
test_case!("LDRB_4");
test_case!("MLA_1");
test_case!("MOV_1");
test_case!("MRS_1");
test_case!("MSR_1");
test_case!("MUL_1");
test_case!("MVN_1");
test_case!("ORR_1");
test_case!("RSC_1");
test_case!("SBC_1");
test_case!("STM_1");
test_case!("STM_2");
test_case!("STM_3");
test_case!("STM_4");
test_case!("SWP_1");
test_case!("SWPB_1");
