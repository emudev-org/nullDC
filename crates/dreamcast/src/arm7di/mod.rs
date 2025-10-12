//! Simplified ARM7 (ARMv4) direct interpreter.
//!
//! This is a clean-room Rust translation of the interpreter used by
//! libswirl's AICA ARM7 core. The goal of this port is functional parity
//! rather than raw speed; the implementation favours clarity and correctness
//! over micro-optimisations.
//!
//! The interpreter only executes ARM state (no Thumb) because the Dreamcast's
//! audio ARM never enables the T bit. Coprocessor instructions are treated as
//! no-ops, which matches the behaviour relied upon by the original project.

#![allow(clippy::too_many_arguments)]
#![allow(dead_code)]

use core::ptr::NonNull;

const FLAG_N: u32 = 1 << 31;
const FLAG_Z: u32 = 1 << 30;
const FLAG_C: u32 = 1 << 29;
const FLAG_V: u32 = 1 << 28;
const FLAG_I: u32 = 1 << 7;
const FLAG_F: u32 = 1 << 6;
const MODE_MASK: u32 = 0x1F;

const MODE_USR: u32 = 0x10;
const MODE_FIQ: u32 = 0x11;
const MODE_IRQ: u32 = 0x12;
const MODE_SVC: u32 = 0x13;
const MODE_ABT: u32 = 0x17;
const MODE_UND: u32 = 0x1B;
const MODE_SYS: u32 = 0x1F;

/// Floating register indices (match libswirl layout for simple interop).
pub const RN_CPSR: usize = 16;
pub const RN_SPSR: usize = 17;
pub const R13_IRQ: usize = 18;
pub const R14_IRQ: usize = 19;
pub const SPSR_IRQ: usize = 20;
pub const R13_USR: usize = 26;
pub const R14_USR: usize = 27;
pub const R13_SVC: usize = 28;
pub const R14_SVC: usize = 29;
pub const SPSR_SVC: usize = 30;
pub const R13_ABT: usize = 31;
pub const R14_ABT: usize = 32;
pub const SPSR_ABT: usize = 33;
pub const R13_UND: usize = 34;
pub const R14_UND: usize = 35;
pub const SPSR_UND: usize = 36;
pub const R8_FIQ: usize = 37;
pub const R9_FIQ: usize = 38;
pub const R10_FIQ: usize = 39;
pub const R11_FIQ: usize = 40;
pub const R12_FIQ: usize = 41;
pub const R13_FIQ: usize = 42;
pub const R14_FIQ: usize = 43;
pub const SPSR_FIQ: usize = 44;
pub const RN_PSR_FLAGS: usize = 45;
pub const R15_ARM_NEXT: usize = 46;
pub const INTR_PEND: usize = 47;
pub const CYCL_CNT: usize = 48;
pub const RN_ARM_REG_COUNT: usize = 49;

/// Callback types for memory/device access.
pub type Read8Fn = fn(addr: u32, ctx: &mut Arm7Context) -> u8;
pub type Read32Fn = fn(addr: u32, ctx: &mut Arm7Context) -> u32;
pub type Write8Fn = fn(addr: u32, value: u8, ctx: &mut Arm7Context);
pub type Write32Fn = fn(addr: u32, value: u32, ctx: &mut Arm7Context);

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct ArmPsr {
    raw: u32,
}

impl ArmPsr {
    #[inline]
    pub fn new(raw: u32) -> Self {
        Self { raw }
    }

    #[inline]
    pub fn raw(self) -> u32 {
        self.raw
    }

    #[inline]
    pub fn set_raw(&mut self, raw: u32) {
        self.raw = raw;
    }

    #[inline]
    pub fn n(self) -> bool {
        self.raw & FLAG_N != 0
    }

    #[inline]
    pub fn z(self) -> bool {
        self.raw & FLAG_Z != 0
    }

    #[inline]
    pub fn c(self) -> bool {
        self.raw & FLAG_C != 0
    }

    #[inline]
    pub fn v(self) -> bool {
        self.raw & FLAG_V != 0
    }

    #[inline]
    pub fn set_n(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_N;
        } else {
            self.raw &= !FLAG_N;
        }
    }

    #[inline]
    pub fn set_z(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_Z;
        } else {
            self.raw &= !FLAG_Z;
        }
    }

    #[inline]
    pub fn set_c(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_C;
        } else {
            self.raw &= !FLAG_C;
        }
    }

    #[inline]
    pub fn set_v(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_V;
        } else {
            self.raw &= !FLAG_V;
        }
    }

    #[inline]
    pub fn irq_masked(self) -> bool {
        self.raw & FLAG_I != 0
    }

    #[inline]
    pub fn fiq_masked(self) -> bool {
        self.raw & FLAG_F != 0
    }

    #[inline]
    pub fn set_irq_mask(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_I;
        } else {
            self.raw &= !FLAG_I;
        }
    }

    #[inline]
    pub fn set_fiq_mask(&mut self, value: bool) {
        if value {
            self.raw |= FLAG_F;
        } else {
            self.raw &= !FLAG_F;
        }
    }

    #[inline]
    pub fn mode(self) -> u32 {
        self.raw & MODE_MASK
    }

    #[inline]
    pub fn set_mode(&mut self, mode: u32) {
        self.raw = (self.raw & !MODE_MASK) | (mode & MODE_MASK);
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub union ArmRegUnion {
    pub u32_: u32,
    pub i32_: i32,
    pub psr: ArmPsr,
}

impl Default for ArmRegUnion {
    fn default() -> Self {
        Self { u32_: 0 }
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
pub struct ArmReg {
    data: ArmRegUnion,
}

impl ArmReg {
    #[inline]
    pub fn get(&self) -> u32 {
        unsafe { self.data.u32_ }
    }

    #[inline]
    pub fn set(&mut self, value: u32) {
        self.data = ArmRegUnion { u32_: value };
    }

    #[inline]
    pub fn get_psr(&self) -> ArmPsr {
        unsafe { self.data.psr }
    }

    #[inline]
    pub fn set_psr(&mut self, value: ArmPsr) {
        self.data = ArmRegUnion { psr: value };
    }
}

#[derive(Clone)]
pub struct Arm7Context {
    pub regs: [ArmReg; RN_ARM_REG_COUNT],
    pub aica_ram: Option<NonNull<u8>>,
    pub aram_mask: u32,
    pub arm_irq_enable: bool,
    pub arm_fiq_enable: bool,
    pub arm_mode: u32,
    pub aica_interr: bool,
    pub aica_reg_l: u32,
    pub e68k_out: bool,
    pub e68k_reg_l: u32,
    pub e68k_reg_m: u32,
    pub enabled: bool,
    pub read8: Option<Read8Fn>,
    pub read32: Option<Read32Fn>,
    pub write8: Option<Write8Fn>,
    pub write32: Option<Write32Fn>,
}

impl Arm7Context {
    pub fn new() -> Self {
        Self {
            regs: [ArmReg::default(); RN_ARM_REG_COUNT],
            aica_ram: None,
            aram_mask: 0,
            arm_irq_enable: false,
            arm_fiq_enable: false,
            arm_mode: MODE_SYS,
            aica_interr: false,
            aica_reg_l: 0,
            e68k_out: false,
            e68k_reg_l: 0,
            e68k_reg_m: 0,
            enabled: false,
            read8: None,
            read32: None,
            write8: None,
            write32: None,
        }
    }

    #[inline]
    fn fetch32(&mut self, addr: u32) -> u32 {
        if let Some(callback) = self.read32 {
            callback(addr, self)
        } else if let Some(ptr) = self.aica_ram {
            let mask = self.aram_mask;
            unsafe {
                let base = ptr.as_ptr();
                let ptr = base.add((addr & mask) as usize);
                u32::from_le_bytes([
                    ptr.read(),
                    ptr.add(1).read(),
                    ptr.add(2).read(),
                    ptr.add(3).read(),
                ])
            }
        } else {
            0
        }
    }

    #[inline]
    fn read8(&mut self, addr: u32) -> u8 {
        if let Some(callback) = self.read8 {
            callback(addr, self)
        } else if let Some(ptr) = self.aica_ram {
            let mask = self.aram_mask;
            unsafe { ptr.as_ptr().add((addr & mask) as usize).read() }
        } else {
            0
        }
    }

    #[inline]
    fn read32(&mut self, addr: u32) -> u32 {
        if let Some(callback) = self.read32 {
            callback(addr, self)
        } else if let Some(ptr) = self.aica_ram {
            let mask = self.aram_mask;
            unsafe {
                let base = ptr.as_ptr().add((addr & mask) as usize);
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

    #[inline]
    fn write8(&mut self, addr: u32, value: u8) {
        if let Some(callback) = self.write8 {
            callback(addr, value, self);
        } else if let Some(ptr) = self.aica_ram {
            let mask = self.aram_mask;
            unsafe {
                ptr.as_ptr().add((addr & mask) as usize).write(value);
            }
        }
    }

    #[inline]
    fn write32(&mut self, addr: u32, value: u32) {
        if let Some(callback) = self.write32 {
            callback(addr, value, self);
        } else if let Some(ptr) = self.aica_ram {
            let mask = self.aram_mask;
            unsafe {
                let base = ptr.as_ptr().add((addr & mask) as usize);
                let bytes = value.to_le_bytes();
                base.write(bytes[0]);
                base.add(1).write(bytes[1]);
                base.add(2).write(bytes[2]);
                base.add(3).write(bytes[3]);
            }
        }
    }
}

pub struct Arm7Di<'a> {
    ctx: &'a mut Arm7Context,
}

impl<'a> Arm7Di<'a> {
    pub fn new(ctx: &'a mut Arm7Context) -> Self {
        Self { ctx }
    }

    #[inline]
    fn flags(&self) -> ArmPsr {
        self.ctx.regs[RN_PSR_FLAGS].get_psr()
    }

    #[inline]
    fn set_flags(&mut self, psr: ArmPsr) {
        self.ctx.regs[RN_PSR_FLAGS].set_psr(psr);
    }

    #[inline]
    fn update_flags_from_result(
        &mut self,
        result: u32,
        carry: Option<bool>,
        overflow: Option<bool>,
    ) {
        let mut psr = self.flags();
        psr.set_n(result >= 0x8000_0000);
        psr.set_z(result == 0);
        if let Some(cf) = carry {
            psr.set_c(cf);
        }
        if let Some(vf) = overflow {
            psr.set_v(vf);
        }
        self.set_flags(psr);
    }

    #[inline]
    fn set_logic_flags(&mut self, result: u32, carry: bool) {
        self.update_flags_from_result(result, Some(carry), None);
    }

    #[inline]
    fn set_compare_flags(&mut self, result: u32, carry: bool, overflow: bool) {
        self.update_flags_from_result(result, Some(carry), Some(overflow));
    }

    #[inline]
    fn condition_passed(&self, opcode: u32) -> bool {
        let cond = opcode >> 28;
        let psr = self.flags();
        match cond {
            0x0 => psr.z(),
            0x1 => !psr.z(),
            0x2 => psr.c(),
            0x3 => !psr.c(),
            0x4 => psr.n(),
            0x5 => !psr.n(),
            0x6 => psr.v(),
            0x7 => !psr.v(),
            0x8 => psr.c() && !psr.z(),
            0x9 => !psr.c() || psr.z(),
            0xA => psr.n() == psr.v(),
            0xB => psr.n() != psr.v(),
            0xC => !psr.z() && (psr.n() == psr.v()),
            0xD => psr.z() || (psr.n() != psr.v()),
            0xE => true,
            0xF => true, // NV is treated as unconditional
            _ => true,
        }
    }

    #[inline]
    fn add_with_carry(a: u32, b: u32, carry_in: bool) -> (u32, bool, bool) {
        let carry = if carry_in { 1 } else { 0 };
        let sum = (a as u64) + (b as u64) + (carry as u64);
        let result = sum as u32;
        let carry_out = (sum >> 32) != 0;
        let overflow = (((a ^ result) & (b ^ result)) & 0x8000_0000) != 0;
        (result, carry_out, overflow)
    }

    #[inline]
    fn sub_with_carry(a: u32, b: u32, carry_in: bool) -> (u32, bool, bool) {
        let (result, carry, overflow) = Self::add_with_carry(a, !b, carry_in);
        (result, carry, overflow)
    }

    fn barrel_shift(
        &self,
        value: u32,
        shift_type: u32,
        amount: u32,
        carry_in: bool,
    ) -> (u32, bool) {
        match shift_type {
            0 => {
                // LSL
                if amount == 0 {
                    (value, carry_in)
                } else if amount < 32 {
                    (value << amount, (value >> (32 - amount)) & 1 != 0)
                } else if amount == 32 {
                    (0, (value & 1) != 0)
                } else {
                    (0, false)
                }
            }
            1 => {
                // LSR
                if amount == 0 || amount == 32 {
                    (0, (value >> 31) != 0)
                } else if amount < 32 {
                    (value >> amount, (value >> (amount - 1)) & 1 != 0)
                } else {
                    (0, false)
                }
            }
            2 => {
                // ASR
                if amount == 0 || amount >= 32 {
                    let bit = (value >> 31) != 0;
                    (if bit { 0xFFFF_FFFF } else { 0 }, bit)
                } else {
                    let result = ((value as i32) >> amount) as u32;
                    (result, (value >> (amount - 1)) & 1 != 0)
                }
            }
            3 => {
                // ROR / RRX
                let rot = amount % 32;
                if amount == 0 {
                    let carry_out = (value & 1) != 0;
                    ((carry_in as u32) << 31 | (value >> 1), carry_out)
                } else if rot == 0 {
                    (value, (value >> 31) != 0)
                } else {
                    let result = value.rotate_right(rot);
                    (result, (result >> 31) != 0)
                }
            }
            _ => (value, carry_in),
        }
    }

    fn decode_operand2(&mut self, opcode: u32) -> (u32, bool) {
        let psr = self.flags();
        if opcode & (1 << 25) != 0 {
            let imm = opcode & 0xFF;
            let rot = ((opcode >> 8) & 0xF) * 2;
            let result = imm.rotate_right(rot);
            let carry = if rot == 0 {
                psr.c()
            } else {
                (result >> 31) != 0
            };
            (result, carry)
        } else {
            let rm = (opcode & 0xF) as usize;
            let value = self.ctx.regs[rm].get();
            if opcode & (1 << 4) == 0 {
                let shift_type = (opcode >> 5) & 0x3;
                let amount = (opcode >> 7) & 0x1F;
                self.barrel_shift(value, shift_type, amount, psr.c())
            } else {
                let shift_type = (opcode >> 5) & 0x3;
                let rs = ((opcode >> 8) & 0xF) as usize;
                let amount = self.ctx.regs[rs].get() & 0xFF;
                if amount == 0 {
                    (value, psr.c())
                } else if amount >= 32 {
                    match shift_type {
                        0 => {
                            let carry = if amount == 32 {
                                (value & 1) != 0
                            } else {
                                false
                            };
                            (0, carry)
                        }
                        1 => {
                            if amount == 32 {
                                (0, (value >> 31) != 0)
                            } else {
                                (0, false)
                            }
                        }
                        2 => {
                            let bit = (value >> 31) != 0;
                            (if bit { 0xFFFF_FFFF } else { 0 }, bit)
                        }
                        3 => {
                            let rot = amount % 32;
                            let result = if rot == 0 {
                                value
                            } else {
                                value.rotate_right(rot)
                            };
                            (result, (result >> 31) != 0)
                        }
                        _ => (value, psr.c()),
                    }
                } else {
                    self.barrel_shift(value, shift_type, amount, psr.c())
                }
            }
        }
    }

    fn write_pc(&mut self, value: u32) {
        let aligned = value & !3;
        self.ctx.regs[R15_ARM_NEXT].set(aligned);
        self.ctx.regs[15].set(aligned.wrapping_add(8));
    }

    fn restore_cpsr_from_spsr(&mut self) {
        let mode = self.ctx.arm_mode;
        if mode != MODE_USR && mode != MODE_SYS {
            let spsr = self.ctx.regs[RN_SPSR].get_psr();
            self.ctx.regs[RN_CPSR].set_psr(spsr);
            self.set_flags(spsr);
            self.ctx.arm_irq_enable = !spsr.irq_masked();
            self.ctx.arm_fiq_enable = !spsr.fiq_masked();
            self.ctx.arm_mode = spsr.mode();
        }
    }

    fn exec_data_processing(&mut self, opcode: u32) -> u32 {
        let op = (opcode >> 21) & 0xF;
        let set_flags = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as usize;
        let rd = ((opcode >> 12) & 0xF) as usize;

        let (operand2, shifter_carry) = self.decode_operand2(opcode);
        let rn_val = if op == 0xD || op == 0xF {
            // MOV/MVN ignore rn
            0
        } else {
            self.ctx.regs[rn].get()
        };
        let mut cycles = 1u32;

        match op {
            0x0 => {
                // AND
                let result = rn_val & operand2;
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_logic_flags(result, shifter_carry);
                    }
                }
            }
            0x1 => {
                // EOR
                let result = rn_val ^ operand2;
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_logic_flags(result, shifter_carry);
                    }
                }
            }
            0x2 => {
                // SUB
                let (result, carry, overflow) = Self::sub_with_carry(rn_val, operand2, true);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x3 => {
                // RSB
                let (result, carry, overflow) = Self::sub_with_carry(operand2, rn_val, true);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x4 => {
                // ADD
                let (result, carry, overflow) = Self::add_with_carry(rn_val, operand2, false);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x5 => {
                // ADC
                let carry_in = self.flags().c();
                let (result, carry, overflow) = Self::add_with_carry(rn_val, operand2, carry_in);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x6 => {
                // SBC
                let carry_in = self.flags().c();
                let (result, carry, overflow) = Self::sub_with_carry(rn_val, operand2, carry_in);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x7 => {
                // RSC
                let carry_in = self.flags().c();
                let (result, carry, overflow) = Self::sub_with_carry(operand2, rn_val, carry_in);
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_compare_flags(result, carry, overflow);
                    }
                }
            }
            0x8 => {
                // TST
                if set_flags {
                    let result = rn_val & operand2;
                    self.set_logic_flags(result, shifter_carry);
                }
            }
            0x9 => {
                // TEQ
                if set_flags {
                    let result = rn_val ^ operand2;
                    self.set_logic_flags(result, shifter_carry);
                }
            }
            0xA => {
                // CMP
                if set_flags {
                    let (result, carry, overflow) = Self::sub_with_carry(rn_val, operand2, true);
                    self.set_compare_flags(result, carry, overflow);
                }
            }
            0xB => {
                // CMN
                if set_flags {
                    let (result, carry, overflow) = Self::add_with_carry(rn_val, operand2, false);
                    self.set_compare_flags(result, carry, overflow);
                }
            }
            0xC => {
                // ORR
                let result = rn_val | operand2;
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_logic_flags(result, shifter_carry);
                    }
                }
            }
            0xD => {
                // MOV
                if rd == 15 {
                    self.write_pc(operand2);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(operand2);
                    if set_flags {
                        self.set_logic_flags(operand2, shifter_carry);
                    }
                }
            }
            0xE => {
                // BIC
                let result = rn_val & !operand2;
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_logic_flags(result, shifter_carry);
                    }
                }
            }
            0xF => {
                // MVN
                let result = !operand2;
                if rd == 15 {
                    self.write_pc(result);
                    if set_flags {
                        self.restore_cpsr_from_spsr();
                    }
                } else {
                    self.ctx.regs[rd].set(result);
                    if set_flags {
                        self.set_logic_flags(result, shifter_carry);
                    }
                }
            }
            _ => {}
        }

        cycles
    }

    fn exec_mrs_msr(&mut self, opcode: u32) -> bool {
        // MRS/MSR instructions share the data-processing group but have unique opcode forms.
        // Detect and handle them before the generic logic runs.
        if opcode & 0x0FBF_0FFF == 0x010F_0000 {
            // MRS Rd, CPSR
            let rd = ((opcode >> 12) & 0xF) as usize;
            let cpsr = self.ctx.regs[RN_CPSR].get();
            self.ctx.regs[rd].set(cpsr);
            return true;
        }
        if opcode & 0x0FBF_0FFF == 0x014F_0000 {
            // MRS Rd, SPSR
            let rd = ((opcode >> 12) & 0xF) as usize;
            let spsr = self.ctx.regs[RN_SPSR].get();
            self.ctx.regs[rd].set(spsr);
            return true;
        }

        let is_immediate = opcode & (1 << 25) != 0;
        if opcode & 0x0DBF_F000 == 0x0120_F000 {
            // MSR (register / immediate) to CPSR
            let field_mask = opcode >> 16 & 0xF;
            let value = if is_immediate {
                let rotate = ((opcode >> 8) & 0xF) * 2;
                (opcode & 0xFF).rotate_right(rotate)
            } else {
                let rm = (opcode & 0xF) as usize;
                self.ctx.regs[rm].get()
            };
            self.write_psr(false, field_mask as u8, value);
            return true;
        }
        if opcode & 0x0DBF_F000 == 0x0160_F000 {
            // MSR ... SPSR
            let field_mask = opcode >> 16 & 0xF;
            let value = if is_immediate {
                let rotate = ((opcode >> 8) & 0xF) * 2;
                (opcode & 0xFF).rotate_right(rotate)
            } else {
                let rm = (opcode & 0xF) as usize;
                self.ctx.regs[rm].get()
            };
            self.write_psr(true, field_mask as u8, value);
            return true;
        }

        false
    }

    fn is_swap(opcode: u32) -> bool {
        ((opcode >> 23) & 0x1F) == 0b00010
            && ((opcode >> 21) & 1) == 0
            && ((opcode >> 8) & 0xF) == 0
            && ((opcode >> 4) & 0xF) == 0x9
    }

    fn exec_swap(&mut self, opcode: u32) -> u32 {
        let rn = ((opcode >> 16) & 0xF) as usize;
        let rd = ((opcode >> 12) & 0xF) as usize;
        let rm = (opcode & 0xF) as usize;
        let address = self.ctx.regs[rn].get();
        let is_byte = (opcode >> 22) & 1 != 0;

        if is_byte {
            let temp = self.ctx.read8(address) as u32;
            self.ctx.write8(address, self.ctx.regs[rm].get() as u8);
            self.ctx.regs[rd].set(temp);
        } else {
            let temp = self.ctx.read32(address & !3);
            self.ctx.write32(address & !3, self.ctx.regs[rm].get());
            self.ctx.regs[rd].set(temp);
        }

        1
    }

    fn write_psr(&mut self, spsr: bool, mask: u8, value: u32) {
        if spsr {
            if self.ctx.arm_mode == MODE_USR || self.ctx.arm_mode == MODE_SYS {
                // In user/system mode SPSR is not accessible.
                return;
            }
            let mut reg = self.ctx.regs[RN_SPSR].get_psr();
            if mask & 0x8 != 0 {
                reg.set_n((value & FLAG_N) != 0);
                reg.set_z((value & FLAG_Z) != 0);
                reg.set_c((value & FLAG_C) != 0);
                reg.set_v((value & FLAG_V) != 0);
            }
            if mask & 0x4 != 0 {
                reg.set_fiq_mask(value & FLAG_F != 0);
                reg.set_irq_mask(value & FLAG_I != 0);
            }
            if mask & 0x1 != 0 {
                reg.set_mode(value & MODE_MASK);
            }
            self.ctx.regs[RN_SPSR].set_psr(reg);
        } else {
            let mut cpsr = self.ctx.regs[RN_CPSR].get_psr();
            if mask & 0x8 != 0 {
                cpsr.set_n((value & FLAG_N) != 0);
                cpsr.set_z((value & FLAG_Z) != 0);
                cpsr.set_c((value & FLAG_C) != 0);
                cpsr.set_v((value & FLAG_V) != 0);
            }
            if mask & 0x4 != 0 {
                cpsr.set_fiq_mask(value & FLAG_F != 0);
                cpsr.set_irq_mask(value & FLAG_I != 0);
            }
            if mask & 0x1 != 0 && (self.ctx.arm_mode != MODE_USR && self.ctx.arm_mode != MODE_SYS) {
                cpsr.set_mode(value & MODE_MASK);
                self.ctx.arm_mode = cpsr.mode();
            }
            self.ctx.arm_irq_enable = !cpsr.irq_masked();
            self.ctx.arm_fiq_enable = !cpsr.fiq_masked();
            self.ctx.regs[RN_CPSR].set_psr(cpsr);
            self.set_flags(cpsr);
        }
    }

    fn exec_multiply(&mut self, opcode: u32) -> u32 {
        let a_bit = (opcode >> 21) & 1 != 0;
        let s_bit = (opcode >> 20) & 1 != 0;
        let rd = ((opcode >> 16) & 0xF) as usize;
        let rn = ((opcode >> 12) & 0xF) as usize;
        let rs = ((opcode >> 8) & 0xF) as usize;
        let rm = (opcode & 0xF) as usize;

        let mul = self.ctx.regs[rm].get();
        let by = self.ctx.regs[rs].get();
        let mut result = mul.wrapping_mul(by);
        if a_bit {
            result = result.wrapping_add(self.ctx.regs[rn].get());
        }
        self.ctx.regs[rd].set(result);
        if s_bit {
            self.update_flags_from_result(result, None, None);
        }
        1
    }

    fn exec_single_data_transfer(&mut self, opcode: u32) -> u32 {
        let i = opcode & (1 << 25) != 0;
        let p = opcode & (1 << 24) != 0;
        let u = opcode & (1 << 23) != 0;
        let b = opcode & (1 << 22) != 0;
        let w = opcode & (1 << 21) != 0;
        let l = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as usize;
        let rd = ((opcode >> 12) & 0xF) as usize;

        let base = self.ctx.regs[rn].get();
        let offset = if i {
            // Register offset with optional shift.
            let (value, _) = self.decode_operand2(opcode & !0x0200_0000);
            value
        } else {
            opcode & 0xFFF
        };

        let offset = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };

        let address = if p { offset } else { base };

        let mut data = 0u32;
        if l {
            if b {
                data = self.ctx.read8(address) as u32;
            } else {
                data = self.ctx.read32(address & !3);
            }
            if rd == 15 {
                self.write_pc(data);
            } else {
                self.ctx.regs[rd].set(data);
            }
        } else {
            let value = if rd == 15 {
                self.ctx.regs[15].get().wrapping_add(4)
            } else {
                self.ctx.regs[rd].get()
            };
            if b {
                self.ctx.write8(address, value as u8);
            } else {
                self.ctx.write32(address & !3, value);
            }
        }

        if w || !p {
            let new_base = if p { offset } else { offset };
            self.ctx.regs[rn].set(new_base);
        }

        1
    }

    fn exec_halfword_transfer(&mut self, opcode: u32) -> u32 {
        let p = opcode & (1 << 24) != 0;
        let u = opcode & (1 << 23) != 0;
        let i = opcode & (1 << 22) != 0;
        let w = opcode & (1 << 21) != 0;
        let l = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as usize;
        let rd = ((opcode >> 12) & 0xF) as usize;
        let s = opcode & (1 << 6) != 0;
        let h = opcode & (1 << 5) != 0;

        let base = self.ctx.regs[rn].get();
        let offset = if i {
            ((opcode >> 8) & 0xF) << 4 | (opcode & 0xF)
        } else {
            self.ctx.regs[(opcode & 0xF) as usize].get()
        };
        let offset = if u {
            base.wrapping_add(offset)
        } else {
            base.wrapping_sub(offset)
        };
        let address = if p { offset } else { base };

        if l {
            let value = if h {
                // halfword load
                let data = self.ctx.read32(address & !1);
                if address & 1 != 0 {
                    ((data >> 8) & 0xFFFF) as u32
                } else {
                    (data & 0xFFFF) as u32
                }
            } else if s {
                // signed byte
                self.ctx.read8(address) as i8 as i32 as u32
            } else {
                // signed halfword
                let data = self.ctx.read32(address & !1);
                let half = if address & 1 != 0 {
                    ((data >> 8) & 0xFFFF) as u32
                } else {
                    (data & 0xFFFF) as u32
                };
                (half as i16) as i32 as u32
            };
            if rd == 15 {
                self.write_pc(value);
            } else {
                self.ctx.regs[rd].set(value);
            }
        } else {
            let value = self.ctx.regs[rd].get();
            if h {
                // store halfword
                self.ctx
                    .write32(address & !1, (value & 0xFFFF) | ((value & 0xFFFF) << 16));
            } else {
                // store double? treat as halfword
                self.ctx.write8(address, value as u8);
            }
        }

        if w || !p {
            self.ctx.regs[rn].set(offset);
        }

        1
    }

    fn exec_block_transfer(&mut self, opcode: u32) -> u32 {
        let p = opcode & (1 << 24) != 0;
        let u = opcode & (1 << 23) != 0;
        let s = opcode & (1 << 22) != 0;
        let w = opcode & (1 << 21) != 0;
        let l = opcode & (1 << 20) != 0;
        let rn = ((opcode >> 16) & 0xF) as usize;
        let reg_list = opcode & 0xFFFF;
        let base = self.ctx.regs[rn].get();
        let reg_count = reg_list.count_ones();
        if reg_count == 0 {
            return 1;
        }

        let step: i32 = if u { 4 } else { -4 };
        let mut address = if p {
            base.wrapping_add_signed(step)
        } else {
            base
        };

        for reg in 0..16 {
            if reg_list & (1 << reg) == 0 {
                continue;
            }

            let effective = address;

            if l {
                let value = if ((opcode >> 21) & 1) != 0 && ((opcode >> 5) & 1) != 0 {
                    // Rare signed variants: best-effort using existing helpers
                    self.ctx.read32(effective & !3)
                } else {
                    self.ctx.read32(effective & !3)
                };

                if reg == 15 {
                    self.write_pc(value);
                } else {
                    self.ctx.regs[reg as usize].set(value);
                }
            } else {
                let value = if reg == 15 {
                    self.ctx.regs[15].get().wrapping_add(4)
                } else {
                    self.ctx.regs[reg as usize].get()
                };
                self.ctx.write32(effective & !3, value);
            }

            address = address.wrapping_add_signed(step);
        }

        if s {
            if l && reg_list & (1 << 15) != 0 {
                self.restore_cpsr_from_spsr();
            }
        }

        if w {
            let offset = (reg_count as i32) * step;
            let final_base = base.wrapping_add_signed(offset);
            self.ctx.regs[rn].set(final_base);
        }

        1
    }

    fn exec_branch(&mut self, opcode: u32) -> u32 {
        let link = opcode & (1 << 24) != 0;
        let offset = opcode & 0x00FF_FFFF;
        let offset = if offset & 0x0080_0000 != 0 {
            offset | 0xFF00_0000
        } else {
            offset
        };
        let offset = ((offset as i32) << 2) as u32;
        let next = self.ctx.regs[R15_ARM_NEXT].get();
        if link {
            self.ctx.regs[14].set(self.ctx.regs[15].get().wrapping_sub(4));
        }
        self.write_pc(next.wrapping_add(offset));
        1
    }

    fn exec_software_interrupt(&mut self, _opcode: u32) -> u32 {
        // Switch to Supervisor mode and branch to 0x08.
        let mut cpsr = self.ctx.regs[RN_CPSR].get_psr();
        let return_address = self.ctx.regs[15].get().wrapping_sub(4);
        cpsr.set_mode(MODE_SVC);
        cpsr.set_irq_mask(true);
        self.ctx.arm_mode = MODE_SVC;
        self.ctx.regs[RN_SPSR].set_psr(self.ctx.regs[RN_CPSR].get_psr());
        self.ctx.regs[RN_CPSR].set_psr(cpsr);
        self.set_flags(cpsr);
        self.ctx.arm_irq_enable = false;
        self.ctx.arm_fiq_enable = !cpsr.fiq_masked();
        self.ctx.regs[14].set(return_address);
        self.write_pc(0x08);
        4
    }

    fn exec_single_opcode(&mut self, opcode: u32) -> u32 {
        if !self.condition_passed(opcode) {
            return 1;
        }

        if self.exec_mrs_msr(opcode) {
            return 1;
        }

        if Self::is_swap(opcode) {
            return self.exec_swap(opcode);
        }

        // Multiply / multiply-accumulate detection
        if opcode & 0x0FC0_FFF0 == 0x0000_0090 {
            return self.exec_multiply(opcode);
        }

        // Halfword/signed transfer detection
        if opcode & 0x0E40_0F0 == 0x0000_090 {
            return self.exec_halfword_transfer(opcode);
        }

        let op_class = (opcode >> 25) & 0x7;
        match op_class {
            0b000 | 0b001 => self.exec_data_processing(opcode),
            0b010 | 0b011 => self.exec_single_data_transfer(opcode),
            0b100 => self.exec_block_transfer(opcode),
            0b101 => self.exec_branch(opcode),
            0b110 => 1, // Coprocessor (ignored)
            0b111 => self.exec_software_interrupt(opcode),
            _ => 1,
        }
    }

    pub fn update_interrupts(&mut self) {
        let pending = self.ctx.e68k_out && self.ctx.arm_fiq_enable;
        self.ctx.regs[INTR_PEND].set(pending as u32);
    }

    pub fn cpu_update_flags(&mut self) {
        let cpsr = self.ctx.regs[RN_CPSR].get_psr();
        self.ctx.regs[RN_PSR_FLAGS].set_psr(cpsr);
        self.ctx.arm_irq_enable = !cpsr.irq_masked();
        self.ctx.arm_fiq_enable = !cpsr.fiq_masked();
        self.ctx.arm_mode = cpsr.mode();
        self.update_interrupts();
    }

    pub fn cpu_update_cpsr(&mut self) {
        let psr = self.ctx.regs[RN_PSR_FLAGS].get_psr();
        self.ctx.regs[RN_CPSR].set_psr(psr);
        self.ctx.arm_irq_enable = !psr.irq_masked();
        self.ctx.arm_fiq_enable = !psr.fiq_masked();
        self.ctx.arm_mode = psr.mode();
    }

    pub fn cpu_switch_mode(&mut self, mode: u32, save_state: bool) {
        self.cpu_update_cpsr();

        let old_mode = self.ctx.arm_mode;
        match old_mode {
            MODE_USR | MODE_SYS => {
                self.ctx.regs[R13_USR].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_USR].set(self.ctx.regs[14].get());
                self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
            }
            MODE_FIQ => {
                self.ctx.regs[R8_FIQ].set(self.ctx.regs[8].get());
                self.ctx.regs[R9_FIQ].set(self.ctx.regs[9].get());
                self.ctx.regs[R10_FIQ].set(self.ctx.regs[10].get());
                self.ctx.regs[R11_FIQ].set(self.ctx.regs[11].get());
                self.ctx.regs[R12_FIQ].set(self.ctx.regs[12].get());
                self.ctx.regs[R13_FIQ].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_FIQ].set(self.ctx.regs[14].get());
                self.ctx.regs[SPSR_FIQ].set(self.ctx.regs[RN_SPSR].get());
            }
            MODE_IRQ => {
                self.ctx.regs[R13_IRQ].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_IRQ].set(self.ctx.regs[14].get());
                self.ctx.regs[SPSR_IRQ].set(self.ctx.regs[RN_SPSR].get());
            }
            MODE_SVC => {
                self.ctx.regs[R13_SVC].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_SVC].set(self.ctx.regs[14].get());
                self.ctx.regs[SPSR_SVC].set(self.ctx.regs[RN_SPSR].get());
            }
            MODE_ABT => {
                self.ctx.regs[R13_ABT].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_ABT].set(self.ctx.regs[14].get());
                self.ctx.regs[SPSR_ABT].set(self.ctx.regs[RN_SPSR].get());
            }
            MODE_UND => {
                self.ctx.regs[R13_UND].set(self.ctx.regs[13].get());
                self.ctx.regs[R14_UND].set(self.ctx.regs[14].get());
                self.ctx.regs[SPSR_UND].set(self.ctx.regs[RN_SPSR].get());
            }
            _ => {}
        }

        let mut cpsr = self.ctx.regs[RN_CPSR].get_psr();
        cpsr.set_mode(mode);
        self.ctx.arm_mode = mode;
        self.ctx.regs[RN_CPSR].set_psr(cpsr);

        match mode {
            MODE_USR | MODE_SYS => {
                self.ctx.regs[13].set(self.ctx.regs[R13_USR].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_USR].get());
                self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
            }
            MODE_FIQ => {
                self.ctx.regs[8].set(self.ctx.regs[R8_FIQ].get());
                self.ctx.regs[9].set(self.ctx.regs[R9_FIQ].get());
                self.ctx.regs[10].set(self.ctx.regs[R10_FIQ].get());
                self.ctx.regs[11].set(self.ctx.regs[R11_FIQ].get());
                self.ctx.regs[12].set(self.ctx.regs[R12_FIQ].get());
                self.ctx.regs[13].set(self.ctx.regs[R13_FIQ].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_FIQ].get());
                if save_state {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
                } else {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[SPSR_FIQ].get());
                }
            }
            MODE_IRQ => {
                self.ctx.regs[13].set(self.ctx.regs[R13_IRQ].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_IRQ].get());
                if save_state {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
                } else {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[SPSR_IRQ].get());
                }
            }
            MODE_SVC => {
                self.ctx.regs[13].set(self.ctx.regs[R13_SVC].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_SVC].get());
                if save_state {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
                } else {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[SPSR_SVC].get());
                }
            }
            MODE_ABT => {
                self.ctx.regs[13].set(self.ctx.regs[R13_ABT].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_ABT].get());
                if save_state {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
                } else {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[SPSR_ABT].get());
                }
            }
            MODE_UND => {
                self.ctx.regs[13].set(self.ctx.regs[R13_UND].get());
                self.ctx.regs[14].set(self.ctx.regs[R14_UND].get());
                if save_state {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[RN_CPSR].get());
                } else {
                    self.ctx.regs[RN_SPSR].set(self.ctx.regs[SPSR_UND].get());
                }
            }
            _ => {}
        }

        self.cpu_update_flags();
    }

    pub fn cpu_fiq(&mut self) {
        let cpsr = self.ctx.regs[RN_CPSR].get_psr();
        if cpsr.fiq_masked() {
            return;
        }
        self.ctx.regs[RN_SPSR].set_psr(cpsr);
        let mut new_cpsr = cpsr;
        new_cpsr.set_mode(MODE_FIQ);
        new_cpsr.set_irq_mask(true);
        new_cpsr.set_fiq_mask(true);
        self.ctx.regs[RN_CPSR].set_psr(new_cpsr);
        self.set_flags(new_cpsr);
        self.ctx.arm_mode = MODE_FIQ;
        self.ctx.arm_irq_enable = false;
        self.ctx.arm_fiq_enable = false;
        self.ctx.regs[14].set(self.ctx.regs[15].get().wrapping_sub(4));
        self.write_pc(0x1C);
    }

    pub fn single_op(&mut self, opcode: u32) -> u32 {
        self.exec_single_opcode(opcode)
    }

    pub fn step(&mut self) -> u32 {
        if self.ctx.regs[INTR_PEND].get() != 0 {
            self.cpu_fiq();
        }
        let pc = self.ctx.regs[R15_ARM_NEXT].get();
        let opcode = self.ctx.fetch32(pc);
        self.ctx.regs[R15_ARM_NEXT].set(pc.wrapping_add(4));
        self.ctx.regs[15].set(self.ctx.regs[R15_ARM_NEXT].get().wrapping_add(4));
        self.exec_single_opcode(opcode)
    }

    pub fn step_many(&mut self, min_cycles: u32) -> u32 {
        let mut cycles = 0;
        while cycles < min_cycles {
            cycles += self.step();
        }
        cycles
    }
}
