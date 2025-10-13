use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::asic;

const SPG_TRIGGER_POS_ADDR: u32 = 0x005F_80C4;
const SPG_HBLANK_INT_ADDR: u32 = 0x005F_80C8;
const SPG_VBLANK_INT_ADDR: u32 = 0x005F_80CC;
const SPG_CONTROL_ADDR: u32 = 0x005F_80D0;
const SPG_HBLANK_ADDR: u32 = 0x005F_80D4;
const SPG_LOAD_ADDR: u32 = 0x005F_80D8;
const SPG_VBLANK_ADDR: u32 = 0x005F_80DC;
const SPG_WIDTH_ADDR: u32 = 0x005F_80E0;
const SPG_STATUS_ADDR: u32 = 0x005F_810C;

const SH4_CLOCK: u64 = 200_000_000;
const PIXEL_CLOCK: u64 = 27_000_000;

#[derive(Default)]
struct Registers {
    trigger_pos: u32,
    hblank_int: u32,
    vblank_int: u32,
    control: u32,
    hblank: u32,
    load: u32,
    vblank: u32,
    width: u32,
}

impl Registers {
    fn reset(&mut self) {
        self.trigger_pos = 0;
        self.hblank_int = 0x031D_0000;
        self.vblank_int = 0x0150_0104;
        self.control = 0;
        self.hblank = 0x007E_0345;
        self.load = 0x0106_0359;
        self.vblank = 0x0150_0104;
        self.width = 0x07F1_933F;
    }
}

struct SpgState {
    regs: Registers,
    line_cycles: u32,
    total_lines: u32,
    cycle_acc: u64,
    scanline: u32,
    field: u8,
    in_vblank: bool,
}

impl Default for SpgState {
    fn default() -> Self {
        let mut state = SpgState {
            regs: Registers::default(),
            line_cycles: 1,
            total_lines: 1,
            cycle_acc: 0,
            scanline: 0,
            field: 0,
            in_vblank: false,
        };
        state.regs.reset();
        state.recompute_timing();
        state
    }
}

impl SpgState {
    fn reset(&mut self) {
        self.regs.reset();
        self.line_cycles = 1;
        self.total_lines = 1;
        self.cycle_acc = 0;
        self.scanline = 0;
        self.field = 0;
        self.in_vblank = false;
        self.recompute_timing();
    }

    fn handles(&self, addr: u32) -> bool {
        matches!(
            addr,
            SPG_TRIGGER_POS_ADDR
                | SPG_HBLANK_INT_ADDR
                | SPG_VBLANK_INT_ADDR
                | SPG_CONTROL_ADDR
                | SPG_HBLANK_ADDR
                | SPG_LOAD_ADDR
                | SPG_VBLANK_ADDR
                | SPG_WIDTH_ADDR
                | SPG_STATUS_ADDR
        )
    }

    fn read(&self, addr: u32) -> u32 {
        match addr {
            SPG_TRIGGER_POS_ADDR => self.regs.trigger_pos,
            SPG_HBLANK_INT_ADDR => self.regs.hblank_int,
            SPG_VBLANK_INT_ADDR => self.regs.vblank_int,
            SPG_CONTROL_ADDR => self.regs.control,
            SPG_HBLANK_ADDR => self.regs.hblank,
            SPG_LOAD_ADDR => self.regs.load,
            SPG_VBLANK_ADDR => self.regs.vblank,
            SPG_WIDTH_ADDR => self.regs.width,
            SPG_STATUS_ADDR => self.status_value(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: u32, value: u32) {
        match addr {
            SPG_TRIGGER_POS_ADDR => self.regs.trigger_pos = value,
            SPG_HBLANK_INT_ADDR => self.regs.hblank_int = value,
            SPG_VBLANK_INT_ADDR => self.regs.vblank_int = value,
            SPG_CONTROL_ADDR => {
                self.regs.control = value;
                self.recompute_timing();
            }
            SPG_HBLANK_ADDR => self.regs.hblank = value,
            SPG_LOAD_ADDR => {
                self.regs.load = value;
                self.recompute_timing();
            }
            SPG_VBLANK_ADDR => {
                self.regs.vblank = value;
                let total = self.total_lines.max(1);
                if self.scanline >= total {
                    self.scanline %= total;
                }
                self.update_in_vblank();
            }
            SPG_WIDTH_ADDR => self.regs.width = value,
            SPG_STATUS_ADDR => { /* read-only */ }
            _ => {}
        }
    }

    fn status_value(&self) -> u32 {
        let mut value = 0u32;
        value |= (self.scanline & 0x3FF) as u32;
        value |= ((self.field as u32) & 0x1) << 10;
        if self.in_vblank {
            value |= 1 << 11; // blank
            value |= 1 << 13; // vsync
        }
        value
    }

    fn recompute_timing(&mut self) {
        let hcount = (self.regs.load & 0x3FF) as u32;
        let vcount = ((self.regs.load >> 16) & 0x3FF) as u32;
        let h_total = (hcount + 1).max(1);
        let v_total = (vcount + 1).max(1);

        let mut line_cycles = ((SH4_CLOCK * h_total as u64) / (PIXEL_CLOCK.max(1))).max(1);
        if self.interlace_enabled() {
            line_cycles = line_cycles / 2;
            if line_cycles == 0 {
                line_cycles = 1;
            }
        }
        self.line_cycles = (line_cycles as u32).max(1);
        self.total_lines = v_total;
        if self.scanline >= self.total_lines {
            self.scanline %= self.total_lines;
        }
        self.update_in_vblank();
    }

    fn interlace_enabled(&self) -> bool {
        (self.regs.control & (1 << 4)) != 0
    }

    fn vblank_start(&self) -> u32 {
        (self.regs.vblank & 0x3FF) as u32
    }

    fn vblank_end(&self) -> u32 {
        ((self.regs.vblank >> 16) & 0x3FF) as u32
    }

    fn hblank_interrupt_line(&self) -> Option<u32> {
        let line = (self.regs.hblank_int >> 16) & 0x3FF;
        if line != 0 { Some(line) } else { None }
    }

    fn vblank_in_line(&self) -> Option<u32> {
        Some((self.regs.vblank_int & 0x3FF) as u32)
    }

    fn vblank_out_line(&self) -> Option<u32> {
        Some(((self.regs.vblank_int >> 16) & 0x3FF) as u32)
    }

    fn update_in_vblank(&mut self) {
        let start = self.vblank_start();
        let end = self.vblank_end();
        if start == end {
            self.in_vblank = false;
            return;
        }

        if start < end {
            self.in_vblank = self.scanline >= start && self.scanline < end;
        } else {
            self.in_vblank = self.scanline >= start || self.scanline < end;
        }
    }

    fn tick(&mut self, cycles: u32) {
        self.cycle_acc += cycles as u64;
        while self.cycle_acc >= self.line_cycles as u64 {
            self.cycle_acc -= self.line_cycles as u64;
            self.scanline = (self.scanline + 1) % self.total_lines.max(1);

            if let Some(line) = self.vblank_in_line() {
                if self.scanline == line {
                    asic::raise_normal(3);
                }
            }
            if let Some(line) = self.vblank_out_line() {
                if self.scanline == line {
                    asic::raise_normal(4);
                }
            }
            if let Some(line) = self.hblank_interrupt_line() {
                if self.scanline == line {
                    asic::raise_normal(5);
                }
            }

            if self.scanline == self.vblank_start() {
                self.in_vblank = true;
            }
            if self.scanline == self.vblank_end() {
                self.in_vblank = false;
            }

            if self.scanline == 0 {
                if self.interlace_enabled() {
                    self.field ^= 1;
                } else {
                    self.field = 0;
                }
                asic::raise_normal(5);
            }
        }
    }
}

static SPG: Lazy<Mutex<SpgState>> = Lazy::new(|| Mutex::new(SpgState::default()));

pub fn reset() {
    if let Ok(mut spg) = SPG.lock() {
        spg.reset();
    }
}

pub fn handles_address(addr: u32) -> bool {
    if let Ok(spg) = SPG.lock() {
        spg.handles(addr)
    } else {
        false
    }
}

pub fn read(addr: u32, size: usize) -> u32 {
    if let Ok(spg) = SPG.lock() {
        let value = spg.read(addr);
        mask_value(value, size)
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
    if let Ok(mut spg) = SPG.lock() {
        spg.write(addr, narrowed);
    }
}

pub fn tick(cycles: u32) {
    if let Ok(mut spg) = SPG.lock() {
        spg.tick(cycles);
    }
}

fn mask_value(value: u32, size: usize) -> u32 {
    match size {
        1 => value & 0xFF,
        2 => value & 0xFFFF,
        _ => value,
    }
}
