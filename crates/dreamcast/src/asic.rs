use once_cell::sync::Lazy;
use std::sync::Mutex;

use sh4_core::{intc_clear_interrupt, intc_raise_interrupt, InterruptSourceId};

const SB_ISTNRM: u32 = 0x005F_6900;
const SB_ISTEXT: u32 = 0x005F_6904;
const SB_ISTERR: u32 = 0x005F_6908;
const SB_IML2NRM: u32 = 0x005F_6910;
const SB_IML2EXT: u32 = 0x005F_6914;
const SB_IML2ERR: u32 = 0x005F_6918;
const SB_IML4NRM: u32 = 0x005F_6920;
const SB_IML4EXT: u32 = 0x005F_6924;
const SB_IML4ERR: u32 = 0x005F_6928;
const SB_IML6NRM: u32 = 0x005F_6930;
const SB_IML6EXT: u32 = 0x005F_6934;
const SB_IML6ERR: u32 = 0x005F_6938;

#[derive(Default)]
struct AsicState {
    ist_nrm: u32,
    ist_ext: u32,
    ist_err: u32,
    iml2_nrm: u32,
    iml2_ext: u32,
    iml2_err: u32,
    iml4_nrm: u32,
    iml4_ext: u32,
    iml4_err: u32,
    iml6_nrm: u32,
    iml6_ext: u32,
    iml6_err: u32,
}

impl AsicState {
    fn reset(&mut self) {
        *self = Self::default();
        self.recompute_pending();
    }

    fn handles(&self, addr: u32) -> bool {
        matches!(
            addr,
            SB_ISTNRM
                | SB_ISTEXT
                | SB_ISTERR
                | SB_IML2NRM
                | SB_IML2EXT
                | SB_IML2ERR
                | SB_IML4NRM
                | SB_IML4EXT
                | SB_IML4ERR
                | SB_IML6NRM
                | SB_IML6EXT
                | SB_IML6ERR
        )
    }

    fn read(&self, addr: u32) -> u32 {
        match addr {
            SB_ISTNRM => {
                let mut value = self.ist_nrm & 0x3FFF_FFFF;
                if self.ist_ext != 0 {
                    value |= 0x4000_0000;
                }
                if self.ist_err != 0 {
                    value |= 0x8000_0000;
                }
                value
            }
            SB_ISTEXT => self.ist_ext,
            SB_ISTERR => self.ist_err,
            SB_IML2NRM => self.iml2_nrm,
            SB_IML2EXT => self.iml2_ext,
            SB_IML2ERR => self.iml2_err,
            SB_IML4NRM => self.iml4_nrm,
            SB_IML4EXT => self.iml4_ext,
            SB_IML4ERR => self.iml4_err,
            SB_IML6NRM => self.iml6_nrm,
            SB_IML6EXT => self.iml6_ext,
            SB_IML6ERR => self.iml6_err,
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, value: u32) {
        match addr {
            SB_ISTNRM => {
                self.ist_nrm &= !value;
                self.recompute_pending();
            }
            SB_ISTEXT => {
                // writes ignored; use cancel for clearing
            }
            SB_ISTERR => {
                self.ist_err &= !value;
                self.recompute_pending();
            }
            SB_IML2NRM => {
                self.iml2_nrm = value;
                self.recompute_pending();
            }
            SB_IML2EXT => {
                self.iml2_ext = value;
                self.recompute_pending();
            }
            SB_IML2ERR => {
                self.iml2_err = value;
                self.recompute_pending();
            }
            SB_IML4NRM => {
                self.iml4_nrm = value;
                self.recompute_pending();
            }
            SB_IML4EXT => {
                self.iml4_ext = value;
                self.recompute_pending();
            }
            SB_IML4ERR => {
                self.iml4_err = value;
                self.recompute_pending();
            }
            SB_IML6NRM => {
                self.iml6_nrm = value;
                self.recompute_pending();
            }
            SB_IML6EXT => {
                self.iml6_ext = value;
                self.recompute_pending();
            }
            SB_IML6ERR => {
                self.iml6_err = value;
                self.recompute_pending();
            }
            _ => {}
        }
    }

    fn raise_normal(&mut self, bit: u8) {
        self.ist_nrm |= 1u32 << bit;
        self.recompute_pending();
    }

    fn raise_external(&mut self, bit: u8) {
        self.ist_ext |= 1u32 << bit;
        self.recompute_pending();
    }

    fn raise_error(&mut self, bit: u8) {
        self.ist_err |= 1u32 << bit;
        self.recompute_pending();
    }

    fn cancel_external(&mut self, bit: u8) {
        self.ist_ext &= !(1u32 << bit);
        self.recompute_pending();
    }

    fn recompute_pending(&self) {
        let level6 = (self.ist_nrm & self.iml6_nrm) != 0
            || (self.ist_ext & self.iml6_ext) != 0
            || (self.ist_err & self.iml6_err) != 0;
        let level4 = (self.ist_nrm & self.iml4_nrm) != 0
            || (self.ist_ext & self.iml4_ext) != 0
            || (self.ist_err & self.iml4_err) != 0;
        let level2 = (self.ist_nrm & self.iml2_nrm) != 0
            || (self.ist_ext & self.iml2_ext) != 0
            || (self.ist_err & self.iml2_err) != 0;

        update_irq(InterruptSourceId::Irl9, level6);
        update_irq(InterruptSourceId::Irl11, level4);
        update_irq(InterruptSourceId::Irl13, level2);
    }
}

fn update_irq(line: InterruptSourceId, active: bool) {
    if active {
        intc_raise_interrupt(line);
    } else {
        intc_clear_interrupt(line);
    }
}

static ASIC: Lazy<Mutex<AsicState>> = Lazy::new(|| Mutex::new(AsicState::default()));

pub fn reset() {
    if let Ok(mut state) = ASIC.lock() {
        state.reset();
    }
}

pub fn handles_address(addr: u32) -> bool {
    if let Ok(state) = ASIC.lock() {
        state.handles(addr)
    } else {
        false
    }
}

pub fn read(addr: u32, size: usize) -> u32 {
    if let Ok(state) = ASIC.lock() {
        let value = state.read(addr);
        mask_value_for_size(value, size)
    } else {
        0
    }
}

pub fn write(addr: u32, size: usize, value: u32) {
    let narrowed = match size {
        1 => value as u8 as u32,
        2 => value as u16 as u32,
        _ => value,
    };

    if let Ok(mut state) = ASIC.lock() {
        state.write(addr, narrowed);
    }
}

pub fn raise_normal(bit: u8) {
    if let Ok(mut state) = ASIC.lock() {
        state.raise_normal(bit);
    }
}

pub fn raise_external(bit: u8) {
    if let Ok(mut state) = ASIC.lock() {
        state.raise_external(bit);
    }
}

pub fn raise_error(bit: u8) {
    if let Ok(mut state) = ASIC.lock() {
        state.raise_error(bit);
    }
}

pub fn cancel_external(bit: u8) {
    if let Ok(mut state) = ASIC.lock() {
        state.cancel_external(bit);
    }
}

fn mask_value_for_size(value: u32, size: usize) -> u32 {
    match size {
        1 => value & 0xFF,
        2 => value & 0xFFFF,
        _ => value,
    }
}
