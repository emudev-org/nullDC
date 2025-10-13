//! Minimal ARM7 (ARMv4) disassembler used by the debugger UI.
//! The goal is to produce readable mnemonics that closely match
//! the interpreter's behaviour rather than cycle-perfect decode.

#![allow(clippy::many_single_char_names)]

#[derive(Clone, Copy)]
pub struct Arm7DecoderState {
    /// Current instruction address as seen by the CPU fetch unit.
    /// Visible PC values in ARM state are this address + 8.
    pub pc: u32,
}

pub fn format_arm_instruction(state: Arm7DecoderState, opcode: u32) -> String {
    if opcode == 0 {
        return "nop".to_string();
    }

    let cond = condition_suffix(opcode);

    if let Some(text) = decode_branch_exchange(cond, opcode) {
        return text;
    }

    if let Some(text) = decode_psr_transfer(cond, opcode) {
        return text;
    }

    if let Some(text) = decode_swap(cond, opcode) {
        return text;
    }

    if let Some(text) = decode_multiply(cond, opcode) {
        return text;
    }

    if let Some(text) = decode_halfword_transfer(cond, opcode) {
        return text;
    }

    match (opcode >> 25) & 0x7 {
        0b000 | 0b001 => decode_data_processing(cond, opcode),
        0b010 | 0b011 => decode_single_data_transfer(cond, opcode),
        0b100 => decode_block_transfer(cond, opcode),
        0b101 => decode_branch(cond, state, opcode),
        0b110 => format!("cdp{cond} <coproc instruction>"),
        0b111 => decode_software_interrupt(cond, opcode),
        _ => format!(".word 0x{opcode:08X}"),
    }
}

fn condition_suffix(opcode: u32) -> &'static str {
    match (opcode >> 28) & 0xF {
        0x0 => "eq",
        0x1 => "ne",
        0x2 => "cs",
        0x3 => "cc",
        0x4 => "mi",
        0x5 => "pl",
        0x6 => "vs",
        0x7 => "vc",
        0x8 => "hi",
        0x9 => "ls",
        0xA => "ge",
        0xB => "lt",
        0xC => "gt",
        0xD => "le",
        0xE => "",
        0xF => "nv",
        _ => "",
    }
}

fn reg_name(index: u32) -> &'static str {
    match index {
        13 => "sp",
        14 => "lr",
        15 => "pc",
        _ => REG_NAMES[index as usize],
    }
}

const REG_NAMES: [&str; 16] = [
    "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "r13", "r14",
    "r15",
];

fn decode_branch_exchange(cond: &str, opcode: u32) -> Option<String> {
    if opcode & 0x0FFF_FFF0 == 0x012F_FFF0 {
        let rm = opcode & 0xF;
        return Some(format!("bx{cond} {}", reg_name(rm)));
    }
    None
}

fn decode_psr_transfer(cond: &str, opcode: u32) -> Option<String> {
    if opcode & 0x0FBF_0FFF == 0x010F_0000 {
        let rd = (opcode >> 12) & 0xF;
        return Some(format!("mrs{cond} {}, cpsr", reg_name(rd)));
    }
    if opcode & 0x0FBF_0FFF == 0x014F_0000 {
        let rd = (opcode >> 12) & 0xF;
        return Some(format!("mrs{cond} {}, spsr", reg_name(rd)));
    }
    if opcode & 0x0DBF_F000 == 0x0120_F000 {
        let mask = format_psr_mask(opcode);
        let operand = format_operand2(opcode);
        return Some(format!("msr{cond} cpsr_{mask}, {operand}"));
    }
    if opcode & 0x0DBF_F000 == 0x0160_F000 {
        let mask = format_psr_mask(opcode);
        let operand = format_operand2(opcode);
        return Some(format!("msr{cond} spsr_{mask}, {operand}"));
    }
    None
}

fn format_psr_mask(opcode: u32) -> String {
    let mut mask = String::new();
    if opcode & (1 << 19) != 0 {
        mask.push('f');
    }
    if opcode & (1 << 18) != 0 {
        mask.push('s');
    }
    if opcode & (1 << 17) != 0 {
        mask.push('x');
    }
    if opcode & (1 << 16) != 0 {
        mask.push('c');
    }
    if mask.is_empty() {
        mask.push('c');
        mask.push('x');
        mask.push('s');
        mask.push('f');
    }
    mask
}

fn decode_swap(cond: &str, opcode: u32) -> Option<String> {
    if ((opcode >> 23) & 0x1F == 0b00010)
        && ((opcode >> 21) & 1 == 0)
        && ((opcode >> 8) & 0xF == 0)
        && ((opcode >> 4) & 0xF == 0x9)
    {
        let rn = (opcode >> 16) & 0xF;
        let rd = (opcode >> 12) & 0xF;
        let rm = opcode & 0xF;
        let b = if opcode & (1 << 22) != 0 { "b" } else { "" };
        return Some(format!(
            "swp{b}{cond} {}, {}, [{}]",
            reg_name(rd),
            reg_name(rm),
            reg_name(rn)
        ));
    }
    None
}

fn decode_multiply(cond: &str, opcode: u32) -> Option<String> {
    if opcode & 0x0FC0_FFF0 == 0x0000_0090 {
        let accumulate = (opcode >> 21) & 1 != 0;
        let set_flags = (opcode >> 20) & 1 != 0;
        let rd = (opcode >> 16) & 0xF;
        let rn = (opcode >> 12) & 0xF;
        let rs = (opcode >> 8) & 0xF;
        let rm = opcode & 0xF;
        let s = if set_flags { "s" } else { "" };
        if accumulate {
            return Some(format!(
                "mla{s}{cond} {}, {}, {}, {}",
                reg_name(rd),
                reg_name(rm),
                reg_name(rs),
                reg_name(rn)
            ));
        } else {
            return Some(format!(
                "mul{s}{cond} {}, {}, {}",
                reg_name(rd),
                reg_name(rm),
                reg_name(rs)
            ));
        }
    }
    if opcode & 0x0F80_0FF0 == 0x0080_0090 {
        let u = (opcode >> 23) & 1 != 0;
        let a = (opcode >> 21) & 1 != 0;
        let s = if (opcode >> 20) & 1 != 0 { "s" } else { "" };
        let rd_hi = (opcode >> 16) & 0xF;
        let rd_lo = (opcode >> 12) & 0xF;
        let rs = (opcode >> 8) & 0xF;
        let rm = opcode & 0xF;
        let mnemonic = match (u, a) {
            (false, false) => "umull",
            (false, true) => "umlal",
            (true, false) => "smull",
            (true, true) => "smlal",
        };
        return Some(format!(
            "{mnemonic}{s}{cond} {}, {}, {}, {}",
            reg_name(rd_lo),
            reg_name(rd_hi),
            reg_name(rm),
            reg_name(rs)
        ));
    }
    None
}

fn decode_halfword_transfer(cond: &str, opcode: u32) -> Option<String> {
    if opcode & 0x0E40_0F0 != 0x0000_090 {
        return None;
    }
    let p = opcode & (1 << 24) != 0;
    let u = opcode & (1 << 23) != 0;
    let i = opcode & (1 << 22) != 0;
    let w = opcode & (1 << 21) != 0;
    let l = opcode & (1 << 20) != 0;
    let rn = (opcode >> 16) & 0xF;
    let rd = (opcode >> 12) & 0xF;
    let s = opcode & (1 << 6) != 0;
    let h = opcode & (1 << 5) != 0;

    let mnemonic = match (l, s, h) {
        (true, true, false) => "ldrsh",
        (true, true, true) => "ldrsb",
        (true, false, true) => "ldrh",
        (false, false, true) => "strh",
        _ => return None,
    };

    let offset = if i {
        format_immediate_offset(u, ((opcode >> 8) & 0xF) << 4 | (opcode & 0xF))
    } else {
        format_register_offset(u, opcode, false)
    };

    let address = build_address(reg_name(rn), offset, p, w);
    Some(format!("{mnemonic}{cond} {}, {}", reg_name(rd), address))
}

fn decode_data_processing(cond: &str, opcode: u32) -> String {
    let op = (opcode >> 21) & 0xF;
    let set_flags = (opcode >> 20) & 1 != 0;
    let rn = (opcode >> 16) & 0xF;
    let rd = (opcode >> 12) & 0xF;
    let operand2 = format_operand2(opcode);

    let (mnemonic, uses_rd, uses_rn, updates_flags) = match op {
        0x0 => ("and", true, true, true),
        0x1 => ("eor", true, true, true),
        0x2 => ("sub", true, true, true),
        0x3 => ("rsb", true, true, true),
        0x4 => ("add", true, true, true),
        0x5 => ("adc", true, true, true),
        0x6 => ("sbc", true, true, true),
        0x7 => ("rsc", true, true, true),
        0x8 => ("tst", false, true, false),
        0x9 => ("teq", false, true, false),
        0xA => ("cmp", false, true, false),
        0xB => ("cmn", false, true, false),
        0xC => ("orr", true, true, true),
        0xD => ("mov", true, false, true),
        0xE => ("bic", true, true, true),
        0xF => ("mvn", true, false, true),
        _ => return format!(".word 0x{opcode:08X}"),
    };

    let mut result = String::new();
    result.push_str(mnemonic);
    if updates_flags && set_flags {
        result.push('s');
    }
    result.push_str(cond);

    let mut operands = Vec::new();
    if uses_rd {
        operands.push(reg_name(rd).to_string());
    }
    if uses_rn {
        operands.push(reg_name(rn).to_string());
    }
    operands.push(operand2);

    result.push(' ');
    result.push_str(&operands.join(", "));
    result
}

fn decode_single_data_transfer(cond: &str, opcode: u32) -> String {
    let i = opcode & (1 << 25) != 0;
    let p = opcode & (1 << 24) != 0;
    let u = opcode & (1 << 23) != 0;
    let b = opcode & (1 << 22) != 0;
    let w = opcode & (1 << 21) != 0;
    let l = opcode & (1 << 20) != 0;
    let rn = (opcode >> 16) & 0xF;
    let rd = (opcode >> 12) & 0xF;

    let mnemonic = if l { "ldr" } else { "str" };
    let mnemonic = if b {
        format!("{mnemonic}b")
    } else {
        mnemonic.to_string()
    };

    let offset = if i {
        format_register_offset(u, opcode, true)
    } else {
        format_immediate_offset(u, opcode & 0xFFF)
    };

    let address = build_address(reg_name(rn), offset, p, w);
    format!("{mnemonic}{cond} {}, {}", reg_name(rd), address)
}

fn decode_block_transfer(cond: &str, opcode: u32) -> String {
    let p = opcode & (1 << 24) != 0;
    let u = opcode & (1 << 23) != 0;
    let s = opcode & (1 << 22) != 0;
    let w = opcode & (1 << 21) != 0;
    let l = opcode & (1 << 20) != 0;
    let rn = (opcode >> 16) & 0xF;
    let reg_list = opcode & 0xFFFF;

    let mode = match (p, u) {
        (false, false) => "da",
        (false, true) => "ia",
        (true, false) => "db",
        (true, true) => "ib",
    };

    let mut mnemonic = if l { "ldm" } else { "stm" }.to_string();
    mnemonic.push_str(mode);
    mnemonic.push_str(cond);
    if s {
        mnemonic.push('^');
    }

    let regs = format_register_list(reg_list);
    if w {
        format!("{mnemonic} {}, {}", format!("{}!", reg_name(rn)), regs)
    } else {
        format!("{mnemonic} {}, {}", reg_name(rn), regs)
    }
}

fn format_register_list(list: u32) -> String {
    if list == 0 {
        return "{}".to_string();
    }
    let mut parts = Vec::new();
    let mut start = None;
    let mut previous = 0;
    for reg in 0..16 {
        if list & (1 << reg) != 0 {
            if start.is_none() {
                start = Some(reg);
            }
            previous = reg;
        } else if let Some(s) = start.take() {
            append_reg_range(&mut parts, s, previous);
        }
    }
    if let Some(s) = start {
        append_reg_range(&mut parts, s, previous);
    }
    format!("{{{}}}", parts.join(", "))
}

fn append_reg_range(parts: &mut Vec<String>, start: u32, end: u32) {
    if start == end {
        parts.push(reg_name(start).to_string());
    } else {
        parts.push(format!("{}-{}", reg_name(start), reg_name(end)));
    }
}

fn decode_branch(cond: &str, state: Arm7DecoderState, opcode: u32) -> String {
    let link = opcode & (1 << 24) != 0;
    let mut offset = opcode & 0x00FF_FFFF;
    if offset & 0x0080_0000 != 0 {
        offset |= 0xFF00_0000;
    }
    let offset = ((offset as i32) << 2) as i32;
    let target = state.pc.wrapping_add(8).wrapping_add_signed(offset);
    let mnemonic = if link { "bl" } else { "b" };
    format!("{mnemonic}{cond} 0x{target:08X}")
}

fn decode_software_interrupt(cond: &str, opcode: u32) -> String {
    let imm = opcode & 0x00FF_FFFF;
    format!("swi{cond} #{imm:#x}")
}

fn format_operand2(opcode: u32) -> String {
    if opcode & (1 << 25) != 0 {
        let imm = opcode & 0xFF;
        let rot = ((opcode >> 8) & 0xF) * 2;
        let value = imm.rotate_right(rot);
        format!("#0x{value:X}")
    } else {
        let rm = opcode & 0xF;
        if opcode & (1 << 4) == 0 {
            let shift_type = (opcode >> 5) & 0x3;
            let amount = (opcode >> 7) & 0x1F;
            if amount == 0 {
                match shift_type {
                    0 => reg_name(rm).to_string(),
                    1 => format!("{}, lsr #32", reg_name(rm)),
                    2 => format!("{}, asr #32", reg_name(rm)),
                    3 => format!("{}, rrx", reg_name(rm)),
                    _ => reg_name(rm).to_string(),
                }
            } else {
                let shift = shift_name(shift_type);
                format!("{}, {} #{}", reg_name(rm), shift, amount)
            }
        } else {
            let shift_type = (opcode >> 5) & 0x3;
            let rs = (opcode >> 8) & 0xF;
            let shift = shift_name(shift_type);
            format!("{}, {} {}", reg_name(rm), shift, reg_name(rs))
        }
    }
}

fn shift_name(kind: u32) -> &'static str {
    match kind {
        0 => "lsl",
        1 => "lsr",
        2 => "asr",
        _ => "ror",
    }
}

fn format_immediate_offset(add: bool, offset: u32) -> Option<String> {
    if offset == 0 {
        None
    } else if add {
        Some(format!("#0x{offset:X}"))
    } else {
        Some(format!("#-0x{offset:X}"))
    }
}

fn format_register_offset(add: bool, opcode: u32, include_shift: bool) -> Option<String> {
    let rm = opcode & 0xF;
    let mut expr = reg_name(rm).to_string();
    if include_shift && opcode & (1 << 4) != 0 {
        let shift_type = (opcode >> 5) & 0x3;
        let rs = (opcode >> 8) & 0xF;
        expr = format!("{expr}, {} {}", shift_name(shift_type), reg_name(rs));
    } else if include_shift {
        let shift_type = (opcode >> 5) & 0x3;
        let amount = (opcode >> 7) & 0x1F;
        if amount != 0 {
            expr = format!("{expr}, {} #{}", shift_name(shift_type), amount);
        }
    }
    if add {
        Some(expr)
    } else {
        Some(format!("-{expr}"))
    }
}

fn build_address(
    base: &str,
    offset: Option<String>,
    pre_indexed: bool,
    write_back: bool,
) -> String {
    if pre_indexed {
        let mut text = match offset {
            Some(ref off) => format!("[{}, {}]", base, off),
            None => format!("[{}]", base),
        };
        if write_back {
            text.push('!');
        }
        text
    } else {
        let head = format!("[{}]", base);
        match offset {
            Some(off) => format!("{head}, {off}"),
            None => head,
        }
    }
}
