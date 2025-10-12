// Test reader for SingleStepTests binary format
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Sh4State {
    pub r: [u32; 16],     // R0-R15
    pub r_bank: [u32; 8], // R_0-R_7
    pub fp0: [u32; 16],   // FP bank 0
    pub fp1: [u32; 16],   // FP bank 1
    pub pc: u32,
    pub gbr: u32,
    pub sr: u32,
    pub ssr: u32,
    pub spc: u32,
    pub vbr: u32,
    pub sgr: u32,
    pub dbr: u32,
    pub macl: u32,
    pub mach: u32,
    pub pr: u32,
    pub fpscr: u32,
    pub fpul: u32,
}

#[derive(Debug, Clone)]
pub struct Cycle {
    pub fetch_addr: Option<u32>,
    pub fetch_val: Option<u32>,
    pub write_addr: Option<u32>,
    pub write_val: Option<u64>,
    pub read_addr: Option<u32>,
    pub read_val: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct Test {
    pub initial: Sh4State,
    pub final_state: Sh4State,
    pub cycles: Vec<Cycle>,
    pub opcodes: Vec<u32>,
}

fn read_u32_le(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

fn read_i32_le(buf: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ])
}

fn read_u64_le(buf: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ])
}

fn load_state(buf: &[u8], ptr: usize) -> (usize, Sh4State) {
    let full_sz = read_i32_le(buf, ptr) as usize;
    let mut offset = ptr + 8;

    let mut state = Sh4State {
        r: [0; 16],
        r_bank: [0; 8],
        fp0: [0; 16],
        fp1: [0; 16],
        pc: 0,
        gbr: 0,
        sr: 0,
        ssr: 0,
        spc: 0,
        vbr: 0,
        sgr: 0,
        dbr: 0,
        macl: 0,
        mach: 0,
        pr: 0,
        fpscr: 0,
        fpul: 0,
    };

    // Read R0-R15
    for i in 0..16 {
        state.r[i] = read_u32_le(buf, offset);
        offset += 4;
    }

    // Read R_0-R_7
    for i in 0..8 {
        state.r_bank[i] = read_u32_le(buf, offset);
        offset += 4;
    }

    // Read FP bank 0
    for i in 0..16 {
        state.fp0[i] = read_u32_le(buf, offset);
        offset += 4;
    }

    // Read FP bank 1
    for i in 0..16 {
        state.fp1[i] = read_u32_le(buf, offset);
        offset += 4;
    }

    // Read remaining registers
    state.pc = read_u32_le(buf, offset);
    offset += 4;
    state.gbr = read_u32_le(buf, offset);
    offset += 4;
    state.sr = read_u32_le(buf, offset);
    offset += 4;
    state.ssr = read_u32_le(buf, offset);
    offset += 4;
    state.spc = read_u32_le(buf, offset);
    offset += 4;
    state.vbr = read_u32_le(buf, offset);
    offset += 4;
    state.sgr = read_u32_le(buf, offset);
    offset += 4;
    state.dbr = read_u32_le(buf, offset);
    offset += 4;
    state.macl = read_u32_le(buf, offset);
    offset += 4;
    state.mach = read_u32_le(buf, offset);
    offset += 4;
    state.pr = read_u32_le(buf, offset);
    offset += 4;
    state.fpscr = read_u32_le(buf, offset);
    offset += 4;
    state.fpul = read_u32_le(buf, offset);

    (full_sz, state)
}

fn load_cycles(buf: &[u8], ptr: usize) -> (usize, Vec<Cycle>) {
    let full_sz = read_i32_le(buf, ptr) as usize;
    let mut offset = ptr + 12;
    let mut cycles = Vec::new();

    for _ in 0..4 {
        let actions = read_u32_le(buf, offset);
        offset += 4;
        let fetch_addr = read_u32_le(buf, offset);
        offset += 4;
        let fetch_val = read_u32_le(buf, offset);
        offset += 4;
        let write_addr = read_u32_le(buf, offset);
        offset += 4;
        let write_val = read_u64_le(buf, offset);
        offset += 8;
        let read_addr = read_u32_le(buf, offset);
        offset += 4; // Changed from u64 to u32
        let read_val = read_u64_le(buf, offset);
        offset += 8;

        let cycle = Cycle {
            fetch_addr: if actions & 4 != 0 {
                Some(fetch_addr)
            } else {
                None
            },
            fetch_val: if actions & 4 != 0 {
                Some(fetch_val)
            } else {
                None
            },
            write_addr: if actions & 2 != 0 {
                Some(write_addr)
            } else {
                None
            },
            write_val: if actions & 2 != 0 {
                Some(write_val)
            } else {
                None
            },
            read_addr: if actions & 1 != 0 {
                Some(read_addr)
            } else {
                None
            },
            read_val: if actions & 1 != 0 {
                Some(read_val)
            } else {
                None
            },
        };

        cycles.push(cycle);
    }

    (full_sz, cycles)
}

fn load_opcodes(buf: &[u8], ptr: usize) -> (usize, Vec<u32>) {
    let full_sz = read_i32_le(buf, ptr) as usize;
    let mut offset = ptr + 8;
    let mut opcodes = Vec::new();

    for _ in 0..5 {
        opcodes.push(read_u32_le(buf, offset));
        offset += 4;
    }

    (full_sz, opcodes)
}

fn decode_test(buf: &[u8], ptr: usize) -> (usize, Test) {
    let full_sz = read_i32_le(buf, ptr) as usize;
    let mut offset = ptr + 4;

    let (sz, initial) = load_state(buf, offset);
    offset += sz;

    let (sz, final_state) = load_state(buf, offset);
    offset += sz;

    let (sz, cycles) = load_cycles(buf, offset);
    offset += sz;

    let (_sz, opcodes) = load_opcodes(buf, offset);

    let test = Test {
        initial,
        final_state,
        cycles,
        opcodes,
    };

    (full_sz, test)
}

pub fn load_test_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<Test>> {
    let mut file = File::open(path)?;
    let mut content = Vec::new();
    file.read_to_end(&mut content)?;

    let mut tests = Vec::new();
    let mut ptr = 0;

    // Each file contains 500 tests
    for _ in 0..500 {
        let (sz, test) = decode_test(&content, ptr);
        ptr += sz;
        tests.push(test);
    }

    Ok(tests)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_file() {
        // Try to load a test file if it exists
        let test_path = "../../vendor/sh4-tests/0000000000001000_sz0_pr0.json.bin";
        if std::path::Path::new(test_path).exists() {
            let tests = load_test_file(test_path).expect("Failed to load test file");
            assert_eq!(tests.len(), 500);
            println!("Loaded {} tests", tests.len());
            println!("First test initial PC: 0x{:08X}", tests[0].initial.pc);
            println!("First test opcodes: {:?}", tests[0].opcodes);
        } else {
            println!("Test file not found, skipping");
        }
    }
}
