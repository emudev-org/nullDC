use crate::*;

// DSP state
static mut MDEC_CT: u32 = 1;
static mut ACC: i32 = 0;
static mut SHIFTED: i32 = 0;
static mut X: i32 = 0;
static mut Y: i32 = 0;
static mut B: i32 = 0;
static mut INPUTS: i32 = 0;
static mut MEMVAL: [i32; 4] = [0; 4];
static mut FRC_REG: i32 = 0;
static mut Y_REG: i32 = 0;
static mut ADRS_REG: u32 = 0;

// PACK: Convert 24-bit signed value to 16-bit floating point format
pub fn pack(val: i32) -> u16 {
    let sign = (val >> 23) & 0x1;
    let mut temp = (val ^ (val << 1)) & 0xFFFFFF;
    let mut exponent = 0;

    for _ in 0..12 {
        if temp & 0x800000 != 0 {
            break;
        }
        temp <<= 1;
        exponent += 1;
    }

    let mut result = if exponent < 12 {
        (val << exponent) & 0x3FFFFF
    } else {
        val << 11
    };

    result >>= 11;
    result |= sign << 15;
    result |= exponent << 11;

    result as u16
}

// UNPACK: Convert 16-bit floating point to 24-bit signed value
pub fn unpack(val: u16) -> i32 {
    let sign = ((val >> 15) & 0x1) as i32;
    let mut exponent = ((val >> 11) & 0xF) as i32;
    let mantissa = (val & 0x7FF) as i32;
    let mut uval = mantissa << 11;

    if exponent > 11 {
        exponent = 11;
    } else {
        uval |= (sign ^ 1) << 22;
    }

    uval |= sign << 23;
    uval = (uval << 8) >> 8; // Sign extend to 32 bits
    uval >>= exponent;

    uval
}

// Main DSP step function
pub fn step(step_num: i32) {
    let step = step_num as usize;
    let dsp = get_dsp_data();
    let common = get_common_data();

    // Get instruction pointer
    let iptr = &dsp.mpro[step * 4..(step * 4 + 4)];

    // Decode instruction fields inline for performance
    let tra = (iptr[0] >> 9) & 0x7F;
    let twt = (iptr[0] >> 8) & 0x01;
    let twa = (iptr[0] >> 1) & 0x7F;

    let xsel = (iptr[1] >> 15) & 0x01;
    let ysel = (iptr[1] >> 13) & 0x03;
    let ira = (iptr[1] >> 7) & 0x3F;
    let iwt = (iptr[1] >> 6) & 0x01;
    let iwa = (iptr[1] >> 1) & 0x1F;

    let ewt = (iptr[2] >> 12) & 0x01;
    let adrl = (iptr[2] >> 7) & 0x01;
    let frcl = (iptr[2] >> 6) & 0x01;
    let shift = (iptr[2] >> 4) & 0x03;
    let yrl = (iptr[2] >> 3) & 0x01;
    let negb = (iptr[2] >> 2) & 0x01;
    let zero = (iptr[2] >> 1) & 0x01;
    let bsel = (iptr[2] >> 0) & 0x01;

    let coef = step;

    unsafe {
        // INPUTS RW
        INPUTS = if ira <= 0x1f {
            get_mems(ira as usize)
        } else if ira <= 0x2F {
            get_mixs((ira - 0x20) as usize) << 4
        } else if ira <= 0x31 {
            (dsp.exts[(ira - 0x30) as usize] as i32) << 8
        } else {
            0
        };

        // Sign extend to 24 bits
        INPUTS = (INPUTS << 8) >> 8;

        // Write to MEMS if needed
        if iwt != 0 {
            set_mems(iwa as usize, MEMVAL[step & 3]);
        }

        // Operand B
        B = if zero == 0 {
            let mut b = if bsel != 0 {
                ACC
            } else {
                let temp_val = get_temp(((tra + MDEC_CT) & 0x7F) as usize) << 2;
                (temp_val << 6) >> 6 // Sign extend to 26 bits
            };
            if negb != 0 {
                b = -b;
            }
            b
        } else {
            0
        };

        // Operand X
        X = if xsel != 0 {
            INPUTS
        } else {
            let temp_val = get_temp(((tra + MDEC_CT) & 0x7F) as usize);
            (temp_val << 8) >> 8 // Sign extend
        };

        // Operand Y
        Y = match ysel {
            0 => FRC_REG,
            1 => (dsp.coef[coef] >> 3) as i32,
            2 => (Y_REG >> 11) & 0x1FFF,
            3 => (Y_REG >> 4) & 0x0FFF,
            _ => 0,
        };

        if yrl != 0 {
            Y_REG = INPUTS;
        }

        // Shifter - uses ACC from previous step
        SHIFTED = match shift {
            0 => {
                let mut s = ACC >> 2;
                if s > 0x0007FFFF {
                    s = 0x0007FFFF;
                }
                if s < -0x00080000 {
                    s = -0x00080000;
                }
                s
            }
            1 => {
                let mut s = ACC >> 1;
                if s > 0x0007FFFF {
                    s = 0x0007FFFF;
                }
                if s < -0x00080000 {
                    s = -0x00080000;
                }
                s
            }
            2 => {
                let s = ACC >> 1;
                (s << 8) >> 8
            }
            3 => {
                let s = ACC >> 2;
                (s << 8) >> 8
            }
            _ => 0,
        };

        // ACCUM
        let y_signed = (Y << 19) >> 19; // Sign extend 13 bits
        let v = ((X as i64) * (y_signed as i64)) >> 10;
        let v_26bit = ((v << 6) >> 6) as i32; // Keep only 26 bits
        ACC = v_26bit + B;
        ACC = (ACC << 6) >> 6; // Keep only 26 bits

        // Write to TEMP
        if twt != 0 {
            set_temp(((twa + MDEC_CT) & 0x7F) as usize, SHIFTED);
        }

        // FRC_REG
        if frcl != 0 {
            FRC_REG = if shift == 3 {
                SHIFTED & 0x0FFF
            } else {
                (SHIFTED >> 11) & 0x1FFF
            };
        }

        // Memory operations (only on odd steps)
        if (step & 1) != 0 {
            let mwt = (iptr[2] >> 14) & 0x01;
            let mrd = (iptr[2] >> 13) & 0x01;

            if mrd != 0 || mwt != 0 {
                let table = (iptr[2] >> 15) & 0x01;
                let nofl = (iptr[3] >> 15) & 0x01;
                let masa = (iptr[3] >> 9) & 0x3F;
                let adreb = (iptr[3] >> 8) & 0x01;
                let nxadr = (iptr[3] >> 7) & 0x01;

                let mut addr = dsp.madrs[masa as usize];
                if adreb != 0 {
                    addr = addr.wrapping_add(ADRS_REG & 0x0FFF);
                }
                if nxadr != 0 {
                    addr = addr.wrapping_add(1);
                }
                if table == 0 {
                    addr = addr.wrapping_add(MDEC_CT);
                    addr &= common.rbl() - 1;
                } else {
                    addr &= 0xFFFF;
                }

                addr <<= 1; // Word to byte address
                addr = addr.wrapping_add(common.rbp());

                let ram = get_aica_ram();
                let addr_masked = (addr & get_aram_mask()) as usize;

                if mrd != 0 {
                    if nofl != 0 {
                        let val = i16::from_le_bytes([ram[addr_masked], ram[addr_masked + 1]]);
                        MEMVAL[(step + 2) & 3] = (val as i32) << 8;
                    } else {
                        let val = u16::from_le_bytes([ram[addr_masked], ram[addr_masked + 1]]);
                        MEMVAL[(step + 2) & 3] = unpack(val);
                    }
                }

                if mwt != 0 {
                    if nofl != 0 {
                        let val = (SHIFTED >> 8) as i16;
                        let bytes = val.to_le_bytes();
                        ram[addr_masked] = bytes[0];
                        ram[addr_masked + 1] = bytes[1];
                    } else {
                        let val = pack(SHIFTED);
                        let bytes = val.to_le_bytes();
                        ram[addr_masked] = bytes[0];
                        ram[addr_masked + 1] = bytes[1];
                    }
                }
            }
        }

        // ADRS_REG
        if adrl != 0 {
            ADRS_REG = if shift == 3 {
                ((SHIFTED >> 12) & 0xFFF) as u32
            } else {
                ((INPUTS >> 16) & 0xFFF) as u32
            };
        }

        // EFREG
        if ewt != 0 {
            let ewa_idx = ((iptr[2] >> 8) & 0x0F) as usize;
            dsp.efreg[ewa_idx] = (dsp.efreg[ewa_idx] as i32 + (SHIFTED >> 4)) as u32;
        }
    }
}

pub fn step_128_start() {
    let dsp = get_dsp_data();
    for i in 0..16 {
        dsp.efreg[i] = 0;
    }
}

pub fn step_128_end() {
    unsafe {
        MDEC_CT = MDEC_CT.wrapping_sub(1);
        if MDEC_CT == 0 {
            MDEC_CT = get_common_data().rbl();
        }
    }
}

pub fn get_dsp_registers() -> Vec<i32> {
    unsafe {
        vec![
            MDEC_CT as i32,
            ACC,
            SHIFTED,
            X,
            Y,
            B,
            INPUTS,
            MEMVAL[0],
            MEMVAL[1],
            MEMVAL[2],
            MEMVAL[3],
            FRC_REG,
            Y_REG,
            ADRS_REG as i32,
        ]
    }
}

