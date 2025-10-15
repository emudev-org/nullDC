use once_cell::sync::Lazy;
use std::sync::Mutex;

use arm7di_core::{Arm7Context, Arm7Di};
use crate::asic;

const REG_SPACE_SIZE: usize = 0x8000;
const REG_MASK: u32 = 0x7FFF;

const SCIEB_ADDR: usize = 0x289C;
const SCIPD_ADDR: usize = 0x289C + 4;
const SCIRE_ADDR: usize = 0x289C + 8;
const SCILV0_ADDR: usize = 0x2800 + 0xA8;
const SCILV1_ADDR: usize = 0x2800 + 0xAC;
const SCILV2_ADDR: usize = 0x2800 + 0xB0;

const MCIEB_ADDR: usize = 0x28B4;
const MCIPD_ADDR: usize = 0x28B4 + 4;
const MCIRE_ADDR: usize = 0x28B4 + 8;

const SCIEB_ADDR_HIGH: usize = SCIEB_ADDR + 2;
const MCIEB_ADDR_HIGH: usize = MCIEB_ADDR + 2;

const REG_L_ADDR: usize = 0x2D00;
const REG_M_ADDR: usize = 0x2D04;

const SPU_IRQ_EXT_BIT: u8 = 1;

struct AicaState {
    regs: [u8; REG_SPACE_SIZE],
    scieb: u32,
    scipd: u32,
    mcieb: u32,
    mcipd: u32,
    vreg: u8,
    arm_reset: u8,
}

impl Default for AicaState {
    fn default() -> Self {
        Self {
            regs: [0; REG_SPACE_SIZE],
            scieb: 0,
            scipd: 0,
            mcieb: 0,
            mcipd: 0,
            vreg: 0,
            arm_reset: 0,
        }
    }
}

impl AicaState {
    fn reset(&mut self) {
        self.regs.fill(0);
        self.scieb = 0;
        self.scipd = 0;
        self.mcieb = 0;
        self.mcipd = 0;
        self.vreg = 0;
        self.arm_reset = 0;
    }

    fn read_u8(&self, offset: usize) -> u8 {
        self.regs.get(offset).copied().unwrap_or(0)
    }

    fn read_u16(&self, offset: usize) -> u16 {
        if offset + 2 > REG_SPACE_SIZE {
            return 0;
        }
        let bytes = [self.regs[offset], self.regs[offset + 1]];
        u16::from_le_bytes(bytes)
    }

    fn read_u32(&self, offset: usize) -> u32 {
        if offset + 4 > REG_SPACE_SIZE {
            return 0;
        }
        let bytes = [
            self.regs[offset],
            self.regs[offset + 1],
            self.regs[offset + 2],
            self.regs[offset + 3],
        ];
        u32::from_le_bytes(bytes)
    }

    fn write_u8(&mut self, offset: usize, value: u8) {
        if offset < REG_SPACE_SIZE {
            self.regs[offset] = value;
        }
    }

    fn write_u16(&mut self, offset: usize, value: u16) {
        if offset + 2 > REG_SPACE_SIZE {
            return;
        }
        let bytes = value.to_le_bytes();
        self.regs[offset] = bytes[0];
        self.regs[offset + 1] = bytes[1];
    }

    fn write_u32(&mut self, offset: usize, value: u32) {
        if offset + 4 > REG_SPACE_SIZE {
            return;
        }
        let bytes = value.to_le_bytes();
        self.regs[offset] = bytes[0];
        self.regs[offset + 1] = bytes[1];
        self.regs[offset + 2] = bytes[2];
        self.regs[offset + 3] = bytes[3];
    }

    fn sync_scipd(&mut self) {
        let value = self.scipd;
        self.write_u32(SCIPD_ADDR, value);
    }

    fn sync_mcipd(&mut self) {
        let value = self.mcipd;
        self.write_u32(MCIPD_ADDR, value);
    }

    fn calc_level(&self, mut bit_index: u32) -> u32 {
        if bit_index > 7 {
            bit_index = 7;
        }
        let mask = 1u32 << bit_index;

        let scilv0 = self.read_u16(SCILV0_ADDR) as u32;
        let scilv1 = self.read_u16(SCILV1_ADDR) as u32;
        let scilv2 = self.read_u16(SCILV2_ADDR) as u32;

        let mut level = 0;
        if (scilv0 & mask) != 0 {
            level |= 1;
        }
        if (scilv1 & mask) != 0 {
            level |= 2;
        }
        if (scilv2 & mask) != 0 {
            level |= 4;
        }
        level
    }
}

static AICA: Lazy<Mutex<AicaState>> = Lazy::new(|| Mutex::new(AicaState::default()));

fn mask_value(value: u32, size: usize) -> u32 {
    match size {
        1 => value & 0xFF,
        2 => value & 0xFFFF,
        4 => value,
        _ => value,
    }
}

fn aram_mask(ctx: &Arm7Context, size: usize) -> Option<u32> {
    if ctx.aram_mask == 0 {
        return None;
    }
    let sub = (size as u32).saturating_sub(1);
    if ctx.aram_mask < sub {
        return None;
    }
    Some(ctx.aram_mask - sub)
}

fn update_e68k(ctx: &mut Arm7Context) {
    if !ctx.e68k_out && ctx.aica_interr {
        ctx.e68k_out = true;
        ctx.e68k_reg_l = ctx.aica_reg_l;
    } else if !ctx.aica_interr {
        ctx.e68k_out = false;
        ctx.e68k_reg_l = 0;
    }
}

fn set_arm_interrupt(ctx: &mut Arm7Context, pending_bits: u32, level: u32) {
    ctx.aica_interr = pending_bits != 0;
    ctx.aica_reg_l = level;
    update_e68k(ctx);

    let mut arm = Arm7Di::new(ctx);
    arm.update_interrupts();
}

fn accept_e68k(ctx: &mut Arm7Context) {
    ctx.e68k_out = false;
    update_e68k(ctx);

    let mut arm = Arm7Di::new(ctx);
    arm.update_interrupts();
}

fn update_arm_interrupts(ctx: &mut Arm7Context, state: &AicaState) {
    let pending = state.scieb & state.scipd;
    if pending != 0 {
        let bit_index = pending.trailing_zeros();
        let level = state.calc_level(bit_index);
        set_arm_interrupt(ctx, pending, level);
    } else {
        set_arm_interrupt(ctx, 0, 0);
    }
}

fn update_sh4_interrupts(state: &AicaState) {
    let pending = state.mcieb & state.mcipd;
    if pending != 0 {
        asic::raise_external(SPU_IRQ_EXT_BIT);
    } else {
        asic::cancel_external(SPU_IRQ_EXT_BIT);
    }
}

fn read_internal(ctx: &Arm7Context, offset: usize, size: usize, from_arm: bool) -> u32 {
    let state = AICA.lock().unwrap();

    let value = match size {
        1 => {
            if from_arm && offset == REG_L_ADDR {
                ctx.e68k_reg_l as u8 as u32
            } else if from_arm && offset == REG_M_ADDR {
                ctx.e68k_reg_m as u8 as u32
            } else if offset == REG_L_ADDR {
                state.read_u8(offset) as u32
            } else if offset == REG_M_ADDR {
                state.read_u8(offset) as u32
            } else if offset == 0x2C00 {
                state.arm_reset as u32
            } else if offset == 0x2C01 {
                state.vreg as u32
            } else {
                state.read_u8(offset) as u32
            }
        }
        2 => {
            if from_arm && offset == REG_L_ADDR {
                ctx.e68k_reg_l as u16 as u32
            } else if from_arm && offset == REG_M_ADDR {
                ctx.e68k_reg_m as u16 as u32
            } else if offset == 0x2C00 {
                u16::from_le_bytes([state.arm_reset, state.vreg]) as u32
            } else {
                state.read_u16(offset) as u32
            }
        }
        4 => state.read_u32(offset),
        _ => 0,
    };

    mask_value(value, size)
}

fn write_internal(ctx: &mut Arm7Context, offset: usize, size: usize, value: u32, from_arm: bool) {
    let mut state = match AICA.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    match size {
        1 => match offset {
            0x2C00 => {
                state.arm_reset = value as u8;
            }
            0x2C01 => {
                state.vreg = value as u8;
            }
            SCIPD_ADDR => {
                if (value & (1 << 5)) != 0 {
                    state.scipd |= 1 << 5;
                    state.sync_scipd();
                    update_arm_interrupts(ctx, &state);
                }
                return;
            }
            SCIRE_ADDR => {
                let mask = mask_value(value, size);
                state.scipd &= !mask;
                state.sync_scipd();
                update_arm_interrupts(ctx, &state);
                return;
            }
            MCIPD_ADDR => {
                if (value & (1 << 5)) != 0 {
                    state.mcipd |= 1 << 5;
                    state.sync_mcipd();
                    update_sh4_interrupts(&state);
                }
                return;
            }
            MCIRE_ADDR => {
                let mask = mask_value(value, size);
                state.mcipd &= !mask;
                state.sync_mcipd();
                update_sh4_interrupts(&state);
                return;
            }
            REG_L_ADDR => {
                // read-only
            }
            REG_M_ADDR => {
                if from_arm && (value & 1) != 0 {
                    drop(state);
                    accept_e68k(ctx);
                    return;
                }
            }
            _ => state.write_u8(offset, value as u8),
        },
        2 => match offset {
            0x2C00 => {
                let bytes = (value as u16).to_le_bytes();
                state.arm_reset = bytes[0];
                state.vreg = bytes[1];
            }
            REG_L_ADDR => { /* read-only */ }
            REG_M_ADDR => {
                if from_arm && (value & 1) != 0 {
                    drop(state);
                    accept_e68k(ctx);
                    return;
                }
            }
            SCIPD_ADDR => {
                if (value & (1 << 5)) != 0 {
                    state.scipd |= 1 << 5;
                    state.sync_scipd();
                    update_arm_interrupts(ctx, &state);
                }
                return;
            }
            SCIRE_ADDR => {
                let mask = mask_value(value, size);
                state.scipd &= !mask;
                state.sync_scipd();
                update_arm_interrupts(ctx, &state);
                return;
            }
            MCIPD_ADDR => {
                if (value & (1 << 5)) != 0 {
                    state.mcipd |= 1 << 5;
                    state.sync_mcipd();
                    update_sh4_interrupts(&state);
                }
                return;
            }
            MCIRE_ADDR => {
                let mask = mask_value(value, size);
                state.mcipd &= !mask;
                state.sync_mcipd();
                update_sh4_interrupts(&state);
                return;
            }
            SCIEB_ADDR | SCIEB_ADDR_HIGH | MCIEB_ADDR | MCIEB_ADDR_HIGH => {
                let masked_value = mask_value(value, size) as u16;
                state.write_u16(offset, masked_value);
            }
            _ => state.write_u16(offset, mask_value(value, size) as u16),
        },
        4 => {
            match offset {
                SCIEB_ADDR => {
                    state.write_u32(offset, mask_value(value, size));
                    state.scieb = state.read_u32(SCIEB_ADDR);
                    update_arm_interrupts(ctx, &state);
                    return;
                }
                SCIPD_ADDR => {
                    if (value & (1 << 5)) != 0 {
                        state.scipd |= 1 << 5;
                        state.sync_scipd();
                        update_arm_interrupts(ctx, &state);
                    }
                    return;
                }
                SCIRE_ADDR => {
                    let mask = mask_value(value, size);
                    state.scipd &= !mask;
                    state.sync_scipd();
                    update_arm_interrupts(ctx, &state);
                    return;
                }
                MCIEB_ADDR => {
                    state.write_u32(offset, mask_value(value, size));
                    state.mcieb = state.read_u32(MCIEB_ADDR);
                    update_sh4_interrupts(&state);
                    return;
                }
                MCIPD_ADDR => {
                    if (value & (1 << 5)) != 0 {
                        state.mcipd |= 1 << 5;
                        state.sync_mcipd();
                        update_sh4_interrupts(&state);
                    }
                    return;
                }
                MCIRE_ADDR => {
                    let mask = mask_value(value, size);
                    state.mcipd &= !mask;
                    state.sync_mcipd();
                    update_sh4_interrupts(&state);
                    return;
                }
                REG_L_ADDR => { /* read-only */ }
                REG_M_ADDR => {
                    if from_arm && (value & 1) != 0 {
                        drop(state);
                        accept_e68k(ctx);
                        return;
                    }
                }
                _ => {
                    state.write_u32(offset, mask_value(value, size));
                }
            }
        }
        _ => {}
    }

    match offset {
        SCIEB_ADDR | SCIEB_ADDR_HIGH => {
            state.scieb = state.read_u32(SCIEB_ADDR);
            update_arm_interrupts(ctx, &state);
        }
        MCIEB_ADDR | MCIEB_ADDR_HIGH => {
            state.mcieb = state.read_u32(MCIEB_ADDR);
            update_sh4_interrupts(&state);
        }
        _ => {}
    }
}

pub fn reset() {
    if let Ok(mut state) = AICA.lock() {
        state.reset();
    }
    asic::cancel_external(SPU_IRQ_EXT_BIT);
}

pub fn handles_address(addr: u32) -> bool {
    let base = addr & !REG_MASK;
    base == 0x0070_0000
}

pub fn read_from_sh4(ctx: &Arm7Context, addr: u32, size: usize) -> u32 {
    let offset = (addr & REG_MASK) as usize;
    read_internal(ctx, offset, size, false)
}

pub fn write_from_sh4(ctx: &mut Arm7Context, addr: u32, size: usize, value: u32) {
    let offset = (addr & REG_MASK) as usize;
    write_internal(ctx, offset, size, value, false);
}

pub fn read_from_arm(ctx: &mut Arm7Context, addr: u32, size: usize) -> u32 {
    let offset = (addr & REG_MASK) as usize;
    read_internal(ctx, offset, size, true)
}

pub fn write_from_arm(ctx: &mut Arm7Context, addr: u32, size: usize, value: u32) {
    let offset = (addr & REG_MASK) as usize;
    write_internal(ctx, offset, size, value, true);
}

pub fn arm_read8(addr: u32, ctx: &mut Arm7Context) -> u8 {
    let addr = addr & 0x00FF_FFFF;
    if addr < 0x0080_0000 {
        return if let (Some(ptr), Some(mask)) = (ctx.aica_ram, aram_mask(ctx, 1)) {
            let offset = (addr & mask) as usize;
            unsafe { ptr.as_ptr().add(offset).read() }
        } else {
            0
        };
    } else {
        read_from_arm(ctx, addr, 1) as u8
    }
}

pub fn arm_read32(addr: u32, ctx: &mut Arm7Context) -> u32 {
    let addr = addr & 0x00FF_FFFF;
    if addr < 0x0080_0000 {
        return if let (Some(ptr), Some(mask)) = (ctx.aica_ram, aram_mask(ctx, 4)) {
            let base = (addr & mask) as usize;
            unsafe {
                let data = ptr.as_ptr().add(base).cast::<u32>().read_unaligned();
                if addr & 3 != 0 {
                    let shift = (addr & 3) * 8;
                    (data >> shift) | (data << (32 - shift))
                } else {
                    data
                }
            }
        } else {
            0
        };
    } else {
        read_from_arm(ctx, addr, 4)
    }
}

pub fn arm_write8(addr: u32, value: u8, ctx: &mut Arm7Context) {
    let addr = addr & 0x00FF_FFFF;
    if addr < 0x0080_0000 {
        if let (Some(ptr), Some(mask)) = (ctx.aica_ram, aram_mask(ctx, 1)) {
            let offset = (addr & mask) as usize;
            unsafe { ptr.as_ptr().add(offset).write(value) };
        }
    } else {
        write_from_arm(ctx, addr, 1, value as u32);
    }
}

pub fn arm_write32(addr: u32, value: u32, ctx: &mut Arm7Context) {
    let addr = addr & 0x00FF_FFFF;
    if addr < 0x0080_0000 {
        if let (Some(ptr), Some(mask)) = (ctx.aica_ram, aram_mask(ctx, 4)) {
            let base = (addr & mask) as usize;
            unsafe {
                ptr.as_ptr().add(base).cast::<u32>().write_unaligned(value);
            }
        }
    } else {
        write_from_arm(ctx, addr, 4, value);
    }
}
