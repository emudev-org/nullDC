use super::Sh4Ctx;
use super::sh4mem::{read_mem, write_mem};

// Helper functions for double precision register access
// SH4 stores double precision values in a mixed-endian format:
// A double at DR[n] is stored as: fr[(n<<1) + 0] (high 32 bits) and fr[(n<<1) + 1] (low 32 bits)
// But the actual double value needs the low part first, then high part
#[repr(C)]
union DoubleReg {
    dbl: f64,
    sgl: [f32; 2],
}

#[inline(always)]
pub fn get_double_reg(ptr: *const u32) -> f64 {
    unsafe {
        let mut t = DoubleReg { dbl: 0.0 };
        // ptr points to fr[(n<<1)], which is the high 32 bits
        // We need to swap the order: sgl[1] = high, sgl[0] = low
        let fr_ptr = ptr as *const f32;
        t.sgl[1] = *fr_ptr.add(0);  // high part (fr[n<<1 + 0])
        t.sgl[0] = *fr_ptr.add(1);  // low part (fr[n<<1 + 1])
        t.dbl
    }
}

#[inline(always)]
pub fn set_double_reg(ptr: *mut u32, val: f64) {
    unsafe {
        let t = DoubleReg { dbl: val };
        // ptr points to fr[(n<<1)], which should get the high 32 bits
        let fr_ptr = ptr as *mut f32;
        *fr_ptr.add(0) = t.sgl[1];  // high part to fr[n<<1 + 0]
        *fr_ptr.add(1) = t.sgl[0];  // low part to fr[n<<1 + 1]
    }
}

#[inline(always)]
pub fn sh4_muls32(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let a = (*src_n) as i32;
        let b = (*src_m) as i32;
        *dst = a.wrapping_mul(b) as u32;
    }
}

#[inline(always)]
pub fn sh4_store32(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src;
    }
}

#[inline(always)]
pub fn sh4_store64(dst: *mut u64, src: *const u64) {
    unsafe {
        *dst = *src;
    }
}

#[inline(always)]
pub fn sh4_store_sr_rest(dst: *mut u32, src: *const u32, r: *mut u32, r_bank: *mut u32) {
    unsafe {
        // Bit layout: MD(30), RB(29), BL(28), FD(15), IMASK(7-4), M(9), Q(8), S(1), T(0)
        const SR_MASK: u32 = 0x700083F2;

        let mut new_val = *src & SR_MASK;
        let old_val = *dst;

        let old_rb = (old_val >> 29) & 1;
        let new_rb = (new_val >> 29) & 1;
        let new_md = (new_val >> 30) & 1;

        if new_md != 0 {
            if old_rb != new_rb {
                sh4_rbank_switch(r, r_bank);
            }
        } else {
            if new_rb != 0 {
                new_val &= !(1 << 29);
            }
            if old_rb != 0 {
                sh4_rbank_switch(r, r_bank);
            }
        }

        *dst = new_val;
    }
}

#[inline(always)]
pub fn sh4_store_fpscr(dst: *mut u32, src: *const u32, fr: *mut u32, xf: *mut u32) {
    unsafe {
        let new_val = *src;
        let old_val = *dst;
        let changed = old_val ^ new_val;

        // Check if RM (rounding mode) bits changed (bits 0-1)
        if changed & 0x3 != 0 {
            let rm = new_val & 0x3;
            set_host_rounding_mode(rm);
        }

        // Check if DN bit changed (bit 18)
        if changed & (1 << 18) != 0 {
            // DN bit changed, sync host FPU DAZ flag
            let dn = (new_val >> 18) & 1;
            set_host_daz(dn != 0);
        }

        if changed & (1 << 21) != 0 {
            sh4_frchg(fr, xf);
        }

        *dst = new_val;
    }
}

// Set host FPU rounding mode
// SH4 FPSCR.RM (bits 0-1): 00 = round to nearest, 01 = round to zero, 10/11 = reserved
#[inline(always)]
fn set_host_rounding_mode(sh4_rm: u32) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // Get current MXCSR
        let mut mxcsr: u32 = 0;
        std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));

        // Clear rounding control bits (13-14)
        mxcsr &= !(0x3 << 13);

        // x86 MXCSR rounding modes (bits 13-14):
        // 00 = Round to nearest (even)
        // 01 = Round down (toward -∞)
        // 10 = Round up (toward +∞)
        // 11 = Round toward zero
        //
        // SH4 FPSCR.RM: 00 = nearest, 01 = zero
        let x86_rm = match sh4_rm {
            0 => 0,  // Round to nearest
            1 => 3,  // Round to zero
            _ => 0,  // Reserved modes default to round to nearest
        };
        mxcsr |= x86_rm << 13;

        // Set new MXCSR
        std::arch::asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // On ARM64, use FPCR (Floating-Point Control Register)
        let mut fpcr: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack));

        // Clear rounding mode bits (22-23)
        fpcr &= !(0x3 << 22);

        // ARM64 FPCR rounding modes (bits 22-23):
        // 00 = Round to nearest (ties to even)
        // 01 = Round toward +∞
        // 10 = Round toward -∞
        // 11 = Round toward zero
        //
        // SH4 FPSCR.RM: 00 = nearest, 01 = zero
        let arm_rm = match sh4_rm {
            0 => 0,  // Round to nearest
            1 => 3,  // Round to zero
            _ => 0,  // Reserved modes default to round to nearest
        };
        fpcr |= arm_rm << 22;

        std::arch::asm!("msr fpcr, {}", in(reg) fpcr, options(nomem, nostack));
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // On other architectures (including WASM), we can't set rounding mode
        // The emulator will use the default host rounding mode (round to nearest)
        let _ = sh4_rm;
    }
}

// Set host FPU Denormals-Are-Zero flag
#[inline(always)]
fn set_host_daz(enable: bool) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // Get current MXCSR
        let mut mxcsr: u32 = 0;
        std::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));

        // Bit 6 is DAZ (Denormals Are Zero)
        if enable {
            mxcsr |= 1 << 6;  // Set DAZ
        } else {
            mxcsr &= !(1 << 6);  // Clear DAZ
        }

        // Set new MXCSR
        std::arch::asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // On ARM64, use FPCR (Floating-Point Control Register)
        let mut fpcr: u64;
        std::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack));

        // Bit 24 is FZ (Flush-to-Zero), equivalent to DAZ on x86
        if enable {
            fpcr |= 1 << 24;  // Set FZ
        } else {
            fpcr &= !(1 << 24);  // Clear FZ
        }

        std::arch::asm!("msr fpcr, {}", in(reg) fpcr, options(nomem, nostack));
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        // On other architectures, we can't set DAZ/FZ
        // The emulator will need software emulation for denormals
        let _ = enable;
    }
}

// Swap R0-R7 with R0_BANK-R7_BANK (when RB bit changes)
#[inline(always)]
pub fn sh4_rbank_switch(r: *mut u32, r_bank: *mut u32) {
    unsafe {
        for i in 0..8 {
            let temp = *r.add(i);
            *r.add(i) = *r_bank.add(i);
            *r_bank.add(i) = temp;
        }
    }
}

#[inline(always)]
pub fn sh4_store32i(dst: *mut u32, imm: u32) {
    unsafe {
        *dst = imm;
    }
}

#[inline(always)]
pub fn sh4_and(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = *src_n & *src_m;
    }
}

#[inline(always)]
pub fn sh4_xor(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = *src_n ^ *src_m;
    }
}

#[inline(always)]
pub fn sh4_sub(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = (*src_n).wrapping_sub(*src_m);
    }
}

#[inline(always)]
pub fn sh4_add(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = (*src_n).wrapping_add(*src_m);
    }
}

#[inline(always)]
pub fn sh4_addi(dst: *mut u32, src_n: *const u32, imm: u32) {
    unsafe {
        *dst = (*src_n).wrapping_add(imm);
    }
}

#[inline(always)]
pub fn sh4_andi(dst: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *dst = *src & imm;
    }
}

#[inline(always)]
pub fn sh4_neg(dst: *mut u32, src_n: *const u32) {
    unsafe {
        *dst = (*src_n).wrapping_neg();
    }
}

#[inline(always)]
pub fn sh4_extub(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src as u8 as u32;
    }
}

#[inline(always)]
pub fn sh4_dt(sr_t: *mut u32, dst: *mut u32) {
    unsafe {
        *dst = (*dst).wrapping_sub(1);
        *sr_t = if *dst == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_shlr(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n & 1; 
        *dst = *src_n >> 1;
    }
}

#[inline(always)]
pub fn sh4_shllf(dst: *mut u32, src_n: *const u32, amt: u32) {
    unsafe {
        *dst = *src_n << amt;
    }
}

#[inline(always)]
pub fn sh4_shlrf(dst: *mut u32, src_n: *const u32, amt: u32) {
    unsafe {
        *dst = *src_n >> amt;
    }
}

#[inline(always)]
pub fn sh4_write_mem8(ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u8>(ctx, *addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16(ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u16>(ctx, *addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32(ctx: *mut Sh4Ctx, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u32>(ctx, *addr, *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64(ctx: *mut Sh4Ctx, addr: *const u32, data: *const u64) {
    unsafe {
        let _ = write_mem::<u64>(ctx, *addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8(ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32) {
    unsafe {
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(ctx, *addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16(ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32) {
    unsafe {
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(ctx, *addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32(ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u32) {
    unsafe {
        let _ = read_mem::<u32>(ctx, *addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mem64(ctx: *mut Sh4Ctx, addr: *const u32, data: *mut u64) {
    unsafe {
        let _ = read_mem::<u64>(ctx, *addr, &mut *data);
    }
}


#[inline(always)]
pub fn sh4_read_mem32i(ctx: *mut Sh4Ctx, addr: u32, data: *mut u32) {
    unsafe {
        let _ = read_mem::<u32>(ctx, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems16_i(ctx: *mut Sh4Ctx, addr: u32, data: *mut u32) {
    unsafe {
        let mut temp: u16 = 0;
        let _ = read_mem::<u16>(ctx, addr, &mut temp);
        *data = temp as i16 as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_fadd(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *dst = *src_n + *src_m;
    }
}

#[inline(always)]
pub fn sh4_fmul(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *dst = *src_n * *src_m;
    }
}

#[inline(always)]
pub fn sh4_fdiv(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *dst = *src_n / *src_m;
    }
}

// Double precision versions
#[inline(always)]
pub fn sh4_fadd_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    let n = get_double_reg(src_n);
    let m = get_double_reg(src_m);
    set_double_reg(dst, n + m);
}

#[inline(always)]
pub fn sh4_fsub_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    let n = get_double_reg(src_n);
    let m = get_double_reg(src_m);
    set_double_reg(dst, n - m);
}

#[inline(always)]
pub fn sh4_fmul_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    let n = get_double_reg(src_n);
    let m = get_double_reg(src_m);
    set_double_reg(dst, n * m);
}

#[inline(always)]
pub fn sh4_fdiv_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    let n = get_double_reg(src_n);
    let m = get_double_reg(src_m);
    set_double_reg(dst, n / m);
}

#[inline(always)]
pub fn sh4_fsca(dst: *mut f32, index: *const u32) {
    unsafe {
        let pi_index = *index & 0xFFFF;
        // rads = (index / (65536/2)) * pi
        let rads = (pi_index as f32) / (65536.0f32 / 2.0f32) * std::f32::consts::PI;

        *dst.add(0) = rads.sin();
        *dst.add(1) = rads.cos();
    }
}

#[inline(always)]
pub fn sh4_float(dst: *mut f32, src: *const u32) {
    unsafe {
        *dst = *src as i32 as f32;
    }
}

#[inline(always)]
pub fn sh4_ftrc(dst: *mut u32, src: *const f32) {
    unsafe {
        // SH4 FTRC: truncate float to int32 with saturation
        // For f32, due to precision limits, the max saturates to 0x7FFFFF80
        // (largest value less than INT_MAX that roundtrips exactly through f32)
        // - NaN converts to 0
        let val = *src;
        let result = if val.is_nan() {
            0
        } else if val >= 2147483520.0 {
            0x7FFFFF80  // Saturate to max f32-representable int
        } else if val < -2147483648.0 {
            0x80000000  // Saturate to INT_MIN
        } else {
            val as i32 as u32
        };
        *dst = result;
    }
}

#[inline(always)]
pub fn sh4_fcmp_eq(sr_t: *mut u32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *sr_t = if *src_m == *src_n { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_gt(sr_t: *mut u32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *sr_t = if *src_n > *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_eq_d(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = get_double_reg(src_n);
        let m = get_double_reg(src_m);
        *sr_t = if m == n { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_gt_d(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = get_double_reg(src_n);
        let m = get_double_reg(src_m);
        *sr_t = if n > m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcnvds(dst: *mut u32, src: *const u32) {
    unsafe {
        let d = get_double_reg(src);
        let f = d as f32;
        *dst = f.to_bits();
    }
}

#[inline(always)]
pub fn sh4_fcnvsd(dst: *mut u32, src: *const u32) {
    unsafe {
        let f = f32::from_bits(*src);
        set_double_reg(dst, f as f64);
    }
}

#[inline(always)]
pub fn sh4_branch_cond(ctx: *mut Sh4Ctx, t: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *t == condition {
            (*ctx).pc1 = target;
            (*ctx).pc2 = target.wrapping_add(2);
        } else {
            // these are calcualted by the pipeline logic in the main loop, no need to do it here
            // but it is done anyway for validation purposes
            (*ctx).pc1 = next;
            (*ctx).pc2 = next.wrapping_add(2);
        }
    }
}

#[inline(always)]
pub fn sh4_branch_cond_delay(ctx: *mut Sh4Ctx, t: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *t == condition {
            (*ctx).pc2 = target;
        } else {
            // this is calcualted by the pipeline logic in the main loop, no need to do it here
            // but it is done anyway for validation purposes
            (*ctx).pc2 = next;
        }
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_branch_delay(ctx: *mut Sh4Ctx, target: u32) {
    unsafe {
        (*ctx).pc2 = target;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_dec_branch_cond(dst: *mut u32, jdyn: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *jdyn == condition {
            *dst = target;
        } else {
            *dst = next;
        }
    }
}

#[inline(always)]
pub fn sh4_dec_call_decode(ctx: *mut Sh4Ctx) {
    use crate::sh4_fns_decode_on_demand;
    unsafe {
        sh4_fns_decode_on_demand(&mut *ctx);
    }
}

#[inline(always)]
pub fn sh4_or(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = *src_n | *src_m;
    }
}

#[inline(always)]
pub fn sh4_fsub(dst: *mut f32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *dst = *src_n - *src_m;
    }
}

#[inline(always)]
pub fn sh4_fneg(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src ^ 0x80000000;
    }
}

#[inline(always)]
pub fn sh4_fabs(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src & 0x7FFFFFFF;
    }
}

#[inline(always)]
pub fn sh4_fsqrt(dst: *mut f32, src: *const f32) {
    unsafe {
        *dst = (*src).sqrt();
    }
}

#[inline(always)]
pub fn sh4_fsqrt_d(dst: *mut u32, src: *const u32) {
    let val = get_double_reg(src);
    set_double_reg(dst, val.sqrt());
}

#[inline(always)]
pub fn sh4_float_d(dst: *mut u32, src: *const u32) {
    let val = unsafe { *src } as i32 as f64;
    set_double_reg(dst, val);
}

#[inline(always)]
pub fn sh4_ftrc_d(dst: *mut u32, src: *const u32) {
    unsafe {
        // SH4 FTRC: truncate double to int32 with saturation
        // - Values >= 2^31 saturate to 0x7FFFFFFF (INT_MAX)
        // - Values < -2^31 saturate to 0x80000000 (INT_MIN)
        // - NaN converts to 0
        let val = get_double_reg(src);
        let result = if val.is_nan() {
            0
        } else if val >= 2147483648.0 {
            0x7FFFFFFF  // Saturate to INT_MAX
        } else if val < -2147483648.0 {
            0x80000000  // Saturate to INT_MIN
        } else {
            val as i32 as u32
        };
        *dst = result;
    }
}

#[inline(always)]
pub fn sh4_fstsi(dst: *mut f32, imm: f32) {
    unsafe {
        *dst = imm;
    }
}

#[inline(always)]
pub fn sh4_frchg(fr: *mut u32, xf: *mut u32) {
    // Swap the first 16 registers between FR and XF banks
    unsafe {
        for i in 0..16 {
            let temp = *fr.add(i);
            *fr.add(i) = *xf.add(i);
            *xf.add(i) = temp;
        }
    }
}


#[inline(always)]
pub fn sh4_fschg() {
    // No-op for interpreter - bitfield operations done in frontend
}

#[inline(always)]
pub fn sh4_jmp(ctx: *mut Sh4Ctx, src: *const u32) {
    unsafe {
        let newpc = *src;
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_jsr(ctx: *mut Sh4Ctx, src: *const u32, next_pc: u32) {
    unsafe {
        let newpc = *src;
        (*ctx).pr = next_pc;
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_braf(ctx: *mut Sh4Ctx, src: *const u32, pc: u32) {
    unsafe {
        let newpc = (*src).wrapping_add(pc).wrapping_add(4);
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_bsrf(ctx: *mut Sh4Ctx, src: *const u32, pc: u32) {
    unsafe {
        let newpc = (*src).wrapping_add(pc).wrapping_add(4);
        let newpr = pc.wrapping_add(4);
        (*ctx).pr = newpr;
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_rts(ctx: *mut Sh4Ctx, pr: *const u32) {
    unsafe {
        let newpc = *pr;
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_rte(ctx: *mut Sh4Ctx, spc: *const u32) {
    unsafe {
        let newpc = *spc;
        (*ctx).pc2 = newpc;
        (*ctx).is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_shad(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let sgn = *src_m & 0x80000000;
        if sgn == 0 {
            *dst = *src_n << (*src_m & 0x1F);
        } else if (*src_m & 0x1F) == 0 {
            *dst = ((*src_n as i32) >> 31) as u32;
        } else {
            *dst = ((*src_n as i32) >> (((!*src_m) & 0x1F) + 1)) as u32;
        }
    }
}

#[inline(always)]
pub fn sh4_shld(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let sgn = *src_m & 0x80000000;
        if sgn == 0 {
            *dst = *src_n << (*src_m & 0x1F);
        } else if (*src_m & 0x1F) == 0 {
            *dst = 0;
        } else {
            *dst = *src_n >> (((!*src_m) & 0x1F) + 1);
        }
    }
}

#[inline(always)]
pub fn sh4_tas(sr_t: *mut u32, ctx: *mut Sh4Ctx, addr: *const u32) {
    unsafe {
        let mut val: u8 = 0;
        let _ = read_mem::<u8>(ctx, *addr, &mut val);
        *sr_t = if val == 0 { 1 } else { 0 };
        val |= 0x80;
        let _ = write_mem::<u8>(ctx, *addr, val);
    }
}

#[inline(always)]
pub fn sh4_not(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = !*src;
    }
}

#[inline(always)]
pub fn sh4_extuw(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src as u16 as u32;
    }
}

#[inline(always)]
pub fn sh4_extsb(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src as i8 as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_extsw(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = *src as i16 as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_swapb(dst: *mut u32, src: *const u32) {
    unsafe {
        let rg = *src;
        *dst = (rg & 0xFFFF0000) | ((rg & 0xFF) << 8) | ((rg >> 8) & 0xFF);
    }
}

#[inline(always)]
pub fn sh4_swapw(dst: *mut u32, src: *const u32) {
    unsafe {
        *dst = (*src << 16) | (*src >> 16);
    }
}

#[inline(always)]
pub fn sh4_xtrct(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = ((*src_n >> 16) & 0xFFFF) | ((*src_m << 16) & 0xFFFF0000);
    }
}


#[inline(always)]
pub fn sh4_cmp_eq(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if *src_n == *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_hs(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if *src_n >= *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_ge(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if (*src_n as i32) >= (*src_m as i32) { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_hi(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if *src_n > *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_gt(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if (*src_n as i32) > (*src_m as i32) { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_pz(sr_t: *mut u32, src: *const u32) {
    unsafe {
        *sr_t = if (*src as i32) >= 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_pl(sr_t: *mut u32, src: *const u32) {
    unsafe {
        *sr_t = if (*src as i32) > 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_tst(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_t = if (*src_n & *src_m) == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_shll(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n >> 31;
        *dst = *src_n << 1;
    }
}

#[inline(always)]
pub fn sh4_shal(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n >> 31;
        *dst = ((*src_n as i32) << 1) as u32;
    }
}

#[inline(always)]
pub fn sh4_shar(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n & 1;
        *dst = ((*src_n as i32) >> 1) as u32;
    }
}

#[inline(always)]
pub fn sh4_cmp_eq_imm(sr_t: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *sr_t = if *src == imm { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_tst_imm(sr_t: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *sr_t = if (*src & imm) == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_and_imm(dst: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *dst = *src & imm;
    }
}

#[inline(always)]
pub fn sh4_xor_imm(dst: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *dst = *src ^ imm;
    }
}

#[inline(always)]
pub fn sh4_or_imm(dst: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *dst = *src | imm;
    }
}

#[inline(always)]
pub fn sh4_rotcl(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        let t = *sr_t;
        *sr_t = *src_n >> 31;
        *dst = (*src_n << 1) | t;
    }
}

#[inline(always)]
pub fn sh4_rotl(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n >> 31;
        *dst = (*src_n << 1) | *sr_t;
    }
}

#[inline(always)]
pub fn sh4_rotcr(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        let t = *src_n & 1;
        *dst = (*src_n >> 1) | ((*sr_t) << 31);
        *sr_t = t;
    }
}

#[inline(always)]
pub fn sh4_rotr(sr_t: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_t = *src_n & 1;
        *dst = (*src_n >> 1) | ((*sr_t) << 31);
    }
}

#[inline(always)]
pub fn sh4_movt(dst: *mut u32, sr_t: *const u32) {
    unsafe {
        *dst = *sr_t;
    }
}

#[inline(always)]
pub fn sh4_clrt(sr_t: *mut u32) {
    unsafe {
        *sr_t = 0;
    }
}

#[inline(always)]
pub fn sh4_sett(sr_t: *mut u32) {
    unsafe {
        *sr_t = 1;
    }
}

#[inline(always)]
pub fn sh4_negc(sr_t: *mut u32, dst: *mut u32, src: *const u32) {
    unsafe {
        let tmp = 0u32.wrapping_sub(*src);
        *dst = tmp.wrapping_sub(*sr_t);
        *sr_t = if tmp > 0 { 1 } else { 0 };
        if tmp < *dst {
            *sr_t = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_addc(sr_t: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let tmp1 = (*src_n).wrapping_add(*src_m);
        let tmp0 = *src_n;
        *dst = tmp1.wrapping_add(*sr_t);
        *sr_t = if tmp0 > tmp1 { 1 } else { 0 };
        if tmp1 > *dst {
            *sr_t = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_addv(sr_t: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let br = (*src_n as i32 as i64).wrapping_add(*src_m as i32 as i64);
        *sr_t = if br >= 0x80000000 || br < -0x80000000i64 { 1 } else { 0 };
        *dst = (*src_n).wrapping_add(*src_m);
    }
}

#[inline(always)]
pub fn sh4_subc(sr_t: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let tmp1 = (*src_n).wrapping_sub(*src_m);
        let tmp0 = *src_n;
        *dst = tmp1.wrapping_sub(*sr_t);
        *sr_t = if tmp0 < tmp1 { 1 } else { 0 };
        if tmp1 < *dst {
            *sr_t = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_subv(sr_t: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let br = (*src_n as i32 as i64).wrapping_sub(*src_m as i32 as i64);
        *sr_t = if br >= 0x80000000 || br < -0x80000000i64 { 1 } else { 0 };
        *dst = (*src_n).wrapping_sub(*src_m);
    }
}

#[inline(always)]
pub fn sh4_muluw(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = ((*src_n as u16) as u32) * ((*src_m as u16) as u32);
    }
}

#[inline(always)]
pub fn sh4_mulsw(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = ((*src_n as i16) as i32 as u32).wrapping_mul((*src_m as i16) as i32 as u32);
    }
}

#[inline(always)]
pub fn sh4_div0u(sr: *mut super::SrStatus, sr_t: *mut u32) {
    unsafe {
        (*sr).set_m(false);
        (*sr).set_q(false);
        *sr_t = 0;
    }
}

#[inline(always)]
pub fn sh4_div0s(sr: *mut super::SrStatus, sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let q = (*src_n >> 31) != 0;
        let m = (*src_m >> 31) != 0;
        (*sr).set_q(q);
        (*sr).set_m(m);
        *sr_t = if m ^ q { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_str(sr_t: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        // CMP/STR checks if ANY byte pair is equal
        // XOR the values - equal bytes become 0
        let temp = *src_n ^ *src_m;
        let hh = (temp & 0xFF000000) == 0;
        let hl = (temp & 0x00FF0000) == 0;
        let lh = (temp & 0x0000FF00) == 0;
        let ll = (temp & 0x000000FF) == 0;
        // T=1 if any byte is equal (any bit is true)
        *sr_t = if hh || hl || lh || ll { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_dmulu(dst: *mut u64, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = (*src_n as u64) * (*src_m as u64);
    }
}

#[inline(always)]
pub fn sh4_dmuls(dst: *mut u64, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *dst = ((*src_n as i32) as i64 * (*src_m as i32) as i64) as u64;
    }
}

#[inline(always)]
pub fn sh4_div1(sr: *mut super::SrStatus, sr_t: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let old_q = (*sr).q() as u32;
        let sr_m = (*sr).m() as u32;
        let old_t = *sr_t;
        let rn_val = *src_n;

        let mut q = (rn_val >> 31) & 1;

        let mut rn = (rn_val << 1) | old_t;
        let tmp0 = rn;

        let tmp2 = if src_m == dst as *const u32 {
            rn
        } else {
            *src_m
        };

        let tmp1: u32;

        if old_q == 0 {
            if sr_m == 0 {
                rn = rn.wrapping_sub(tmp2);
                tmp1 = if rn > tmp0 { 1 } else { 0 };
                q = if q == 0 { tmp1 } else { if tmp1 == 0 { 1 } else { 0 } };
            } else {
                rn = rn.wrapping_add(tmp2);
                tmp1 = if rn < tmp0 { 1 } else { 0 };
                q = if q == 0 { if tmp1 == 0 { 1 } else { 0 } } else { tmp1 };
            }
        } else {
            if sr_m == 0 {
                rn = rn.wrapping_add(tmp2);
                tmp1 = if rn < tmp0 { 1 } else { 0 };
                q = if q == 0 { tmp1 } else { if tmp1 == 0 { 1 } else { 0 } };
            } else {
                rn = rn.wrapping_sub(tmp2);
                tmp1 = if rn > tmp0 { 1 } else { 0 };
                q = if q == 0 { if tmp1 == 0 { 1 } else { 0 } } else { tmp1 };
            }
        }

        let new_t = if q == sr_m { 1 } else { 0 };

        *dst = rn;
        (*sr).set_q(q != 0);
        *sr_t = new_t;
    }
}

// Memory operations with indexed addressing (R0 + Rn)
#[inline(always)]
pub fn sh4_write_mem8_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u8>(ctx, addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u16>(ctx, addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u32>(ctx, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(ctx, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(ctx, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = read_mem::<u32>(ctx, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mem64_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *mut u64) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = read_mem::<u64>(ctx, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64_indexed(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, data: *const u64) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u64>(ctx, addr, *data);
    }
}

// Read-modify-write operations for memory with GBR+R0 addressing
#[inline(always)]
pub fn sh4_tst_mem(sr_t: *mut u32, ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(ctx, addr, &mut temp);
        *sr_t = if (temp as u32 & imm) == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_and_mem(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(ctx, addr, &mut temp);
        temp &= imm;
        let _ = write_mem::<u8>(ctx, addr, temp);
    }
}

#[inline(always)]
pub fn sh4_xor_mem(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(ctx, addr, &mut temp);
        temp ^= imm;
        let _ = write_mem::<u8>(ctx, addr, temp);
    }
}

#[inline(always)]
pub fn sh4_or_mem(ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(ctx, addr, &mut temp);
        temp |= imm;
        let _ = write_mem::<u8>(ctx, addr, temp);
    }
}

// Memory operations with displacement addressing (Rn + disp)
#[inline(always)]
pub fn sh4_write_mem8_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u8>(ctx, addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u16>(ctx, addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u32>(ctx, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(ctx, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(ctx, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = read_mem::<u32>(ctx, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64_disp(ctx: *mut Sh4Ctx, base: *const u32, disp: u32, data: *const u64) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u64>(ctx, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_fsrra(dst: *mut f32, src: *const f32) {
    unsafe {
        *dst = 1.0 / (*src).sqrt();
    }
}

#[inline(always)]
pub fn sh4_fipr(dst: *mut f32, src1: *const f32, src2: *const f32) {
    unsafe {
        let idp = *src1.add(0) * *src2.add(0)
                + *src1.add(1) * *src2.add(1)
                + *src1.add(2) * *src2.add(2)
                + *src1.add(3) * *src2.add(3);
        *dst = idp;
    }
}

#[inline(always)]
pub fn sh4_fmac(dst: *mut f32, fr0: *const f32, src_m: *const f32) {
    unsafe {
        *dst = (*dst as f64 + *fr0 as f64 * *src_m as f64) as f32;
    }
}

#[inline(always)]
pub fn sh4_ftrv(fr: *mut f32, xf: *const f32) {
    unsafe {
        let v1 = *xf.add(0)  * *fr.add(0) +
                 *xf.add(4)  * *fr.add(1) +
                 *xf.add(8)  * *fr.add(2) +
                 *xf.add(12) * *fr.add(3);

        let v2 = *xf.add(1)  * *fr.add(0) +
                 *xf.add(5)  * *fr.add(1) +
                 *xf.add(9)  * *fr.add(2) +
                 *xf.add(13) * *fr.add(3);

        let v3 = *xf.add(2)  * *fr.add(0) +
                 *xf.add(6)  * *fr.add(1) +
                 *xf.add(10) * *fr.add(2) +
                 *xf.add(14) * *fr.add(3);

        let v4 = *xf.add(3)  * *fr.add(0) +
                 *xf.add(7)  * *fr.add(1) +
                 *xf.add(11) * *fr.add(2) +
                 *xf.add(15) * *fr.add(3);

        *fr.add(0) = v1;
        *fr.add(1) = v2;
        *fr.add(2) = v3;
        *fr.add(3) = v4;
    }
}

#[inline(always)]
pub fn sh4_mac_w_mul(mac_full: *mut u64, temp0: *const u32, temp1: *const u32) {
    unsafe {
        // temp0 and temp1 contain sign-extended 16-bit values as 32-bit
        let rn = *temp0 as i32;
        let rm = *temp1 as i32;
        let mul = rn * rm;
        *mac_full = (*mac_full as i64).wrapping_add(mul as i64) as u64;
    }
}

#[inline(always)]
pub fn sh4_mac_l_mul(mac_full: *mut u64, temp0: *const u32, temp1: *const u32) {
    unsafe {
        // temp0 and temp1 contain the 32-bit values
        let rn = *temp0 as i32 as i64;
        let rm = *temp1 as i32 as i64;
        let mul = rn * rm;
        *mac_full = (*mac_full as i64).wrapping_add(mul) as u64;
    }
}


