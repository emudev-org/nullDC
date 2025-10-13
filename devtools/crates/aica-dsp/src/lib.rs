use wasm_bindgen::prelude::*;

mod dsp;

// Static memory buffers
static mut AICA_REG: [u8; 0x8000] = [0; 0x8000];
static mut AICA_RAM: [u8; 2 * 1024 * 1024] = [0; 2 * 1024 * 1024];
const ARAM_MASK: u32 = (2 * 1024 * 1024 - 1) as u32;

// CommonData_struct at offset 0x2800
#[repr(C, packed)]
pub struct CommonData {
    // +0
    pub mvol_ver_dac18b_mem8mb_mono: u32,
    // +4
    pub rbp_rbl_testb0: u32,
    // +8
    pub mibuf_flags: u32,
    // +C
    pub mobuf_mslc_afset: u32,
    // +10
    pub eg_sgc_lp: u32,
    // +14
    pub ca: u32,
    // Padding to 0x80
    pub pad_med_0: [u8; 0x6C - 8],
    // +80 onwards...
    pub mrwinh_etc: [u32; 0x500 / 4 - 0x80 / 4],
}

impl CommonData {
    pub fn rbp(&self) -> u32 {
        (self.rbp_rbl_testb0 & 0xFFF) * 2048
    }

    pub fn rbl(&self) -> u32 {
        let rbl_field = (self.rbp_rbl_testb0 >> 13) & 0x3;
        match rbl_field {
            0 => 8 * 1024,
            1 => 16 * 1024,
            2 => 32 * 1024,
            3 => 64 * 1024,
            _ => 0,
        }
    }
}

// DSPData_struct at offset 0x3000
#[repr(C)]
pub struct DSPData {
    // +0x000
    pub coef: [u32; 128],
    // +0x200
    pub madrs: [u32; 64],
    // +0x300
    pub _pad0: [u8; 0x100],
    // +0x400
    pub mpro: [u32; 128 * 4],
    // +0xC00
    pub _pad1: [u8; 0x400],
    // +0x1000
    pub temp: [DualReg; 128],
    // +0x1400
    pub mems: [DualReg; 32],
    // +0x1500
    pub mixs: [DualReg; 16],
    // +0x1580
    pub efreg: [u32; 16],
    // +0x15C0
    pub exts: [u32; 2],
}

#[repr(C)]
pub struct DualReg {
    pub l: u32,
    pub h: u32,
}

// Instruction structure
pub struct Inst {
    pub tra: u32,
    pub twt: u32,
    pub twa: u32,
    pub xsel: u32,
    pub ysel: u32,
    pub ira: u32,
    pub iwt: u32,
    pub iwa: u32,
    pub ewt: u32,
    pub ewa: u32,
    pub adrl: u32,
    pub frcl: u32,
    pub shift: u32,
    pub yrl: u32,
    pub negb: u32,
    pub zero: u32,
    pub bsel: u32,
    pub nofl: u32,
    pub table: u32,
    pub mwt: u32,
    pub mrd: u32,
    pub masa: u32,
    pub adreb: u32,
    pub nxadr: u32,
}

// Helper functions to access DSPData
fn get_dsp_data() -> &'static mut DSPData {
    unsafe {
        let ptr = (&raw mut AICA_REG).cast::<u8>().add(0x3000).cast::<DSPData>();
        &mut *ptr
    }
}

fn get_common_data() -> &'static CommonData {
    unsafe {
        let ptr = (&raw const AICA_REG).cast::<u8>().add(0x2800).cast::<CommonData>();
        &*ptr
    }
}

pub fn get_mems(idx: usize) -> i32 {
    let dsp = get_dsp_data();
    (dsp.mems[idx].l | (dsp.mems[idx].h << 8)) as i32
}

pub fn set_mems(idx: usize, val: i32) {
    let dsp = get_dsp_data();
    dsp.mems[idx].l = (val & 0xFF) as u32;
    dsp.mems[idx].h = ((val >> 8) & 0xFFFF) as u32;
}

pub fn get_mixs(idx: usize) -> i32 {
    let dsp = get_dsp_data();
    (dsp.mixs[idx].l | (dsp.mixs[idx].h << 4)) as i32
}

pub fn get_temp(idx: usize) -> i32 {
    let dsp = get_dsp_data();
    (dsp.temp[idx].l | (dsp.temp[idx].h << 8)) as i32
}

pub fn set_temp(idx: usize, val: i32) {
    let dsp = get_dsp_data();
    dsp.temp[idx].l = (val & 0xFF) as u32;
    dsp.temp[idx].h = ((val >> 8) & 0xFFFF) as u32;
}

// WASM exports
#[wasm_bindgen]
pub fn read_reg(addr: u32) -> u32 {
    const AICA_REG_SIZE: usize = 0x8000;
    unsafe {
        let offset = (addr as usize) & 0x7FFF;
        let ptr = (&raw const AICA_REG).cast::<u8>();
        if offset + 4 <= AICA_REG_SIZE {
            u32::from_le_bytes([
                *ptr.add(offset),
                *ptr.add(offset + 1),
                *ptr.add(offset + 2),
                *ptr.add(offset + 3),
            ])
        } else {
            0
        }
    }
}

#[wasm_bindgen]
pub fn write_reg(addr: u32, data: u32) {
    const AICA_REG_SIZE: usize = 0x8000;
    unsafe {
        let offset = (addr as usize) & 0x7FFF;
        let ptr = (&raw mut AICA_REG).cast::<u8>();
        if offset + 4 <= AICA_REG_SIZE {
            let bytes = data.to_le_bytes();
            *ptr.add(offset) = bytes[0];
            *ptr.add(offset + 1) = bytes[1];
            *ptr.add(offset + 2) = bytes[2];
            *ptr.add(offset + 3) = bytes[3];
        }
    }
}

#[wasm_bindgen]
pub fn step(step: i32) {
    dsp::step(step);
}

#[wasm_bindgen]
pub fn step128_start() {
    dsp::step_128_start();
}

#[wasm_bindgen]
pub fn step128_end() {
    dsp::step_128_end();
}

#[wasm_bindgen]
pub fn get_dsp_registers() -> Vec<i32> {
    dsp::get_dsp_registers()
}


// Export memory access for debugging
pub fn get_aica_ram() -> &'static mut [u8] {
    unsafe {
        let ptr = (&raw mut AICA_RAM).cast::<u8>();
        std::slice::from_raw_parts_mut(ptr, 2 * 1024 * 1024)
    }
}

pub fn get_aram_mask() -> u32 {
    ARAM_MASK
}
