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
pub fn GetDoubleReg(ptr: *const u32) -> f64 {
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
pub fn SetDoubleReg(ptr: *mut u32, val: f64) {
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
pub fn sh4_store_sr(dst: *mut u32, src: *const u32, r: *mut u32, r_bank: *mut u32) {
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

    #[cfg(not(target_arch = "x86_64"))]
    {
        // On non-x86_64 architectures, we can't set DAZ
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
pub fn sh4_dt(sr_T: *mut u32, dst: *mut u32) {
    unsafe {
        *dst = (*dst).wrapping_sub(1);
        *sr_T = if *dst == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_shlr(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n & 1; 
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
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        SetDoubleReg(dst, n + m);
    }
}

#[inline(always)]
pub fn sh4_fsub_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        SetDoubleReg(dst, n - m);
    }
}

#[inline(always)]
pub fn sh4_fmul_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        SetDoubleReg(dst, n * m);
    }
}

#[inline(always)]
pub fn sh4_fdiv_d(dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        SetDoubleReg(dst, n / m);
    }
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
        let clamped = (*src).min(0x7FFFFFBF as f32);
        let mut as_i = clamped as i32 as u32;
        if as_i == 0x80000000 {
            if (*src) > 0.0 {
                as_i = as_i.wrapping_sub(1);
            }
        }
        *dst = as_i;
    }
}

#[inline(always)]
pub fn sh4_fcmp_eq(sr_T: *mut u32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *sr_T = if *src_m == *src_n { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_gt(sr_T: *mut u32, src_n: *const f32, src_m: *const f32) {
    unsafe {
        *sr_T = if *src_n > *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_eq_d(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        *sr_T = if m == n { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcmp_gt_d(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let n = GetDoubleReg(src_n);
        let m = GetDoubleReg(src_m);
        *sr_T = if n > m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_fcnvds(dst: *mut u32, src: *const u32) {
    unsafe {
        let d = GetDoubleReg(src);
        let f = d as f32;
        *dst = f.to_bits();
    }
}

#[inline(always)]
pub fn sh4_fcnvsd(dst: *mut u32, src: *const u32) {
    unsafe {
        let f = f32::from_bits(*src);
        SetDoubleReg(dst, f as f64);
    }
}

#[inline(always)]
pub fn sh4_branch_cond(ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *T == condition {
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
pub fn sh4_branch_cond_delay(ctx: *mut Sh4Ctx, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *T == condition {
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
    unsafe {
        let val = GetDoubleReg(src);
        SetDoubleReg(dst, val.sqrt());
    }
}

#[inline(always)]
pub fn sh4_float_d(dst: *mut u32, src: *const u32) {
    unsafe {
        let val = *src as i32 as f64;
        SetDoubleReg(dst, val);
    }
}

#[inline(always)]
pub fn sh4_ftrc_d(dst: *mut u32, src: *const u32) {
    unsafe {
        let val = GetDoubleReg(src);
        let clamped = val.min(0x7FFFFFBF as f64);
        let mut as_i = clamped as i32 as u32;
        if as_i == 0x80000000 {
            if val > 0.0 {
                as_i = as_i.wrapping_sub(1);
            }
        }
        *dst = as_i;
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
pub fn sh4_rte(ctx: *mut Sh4Ctx, spc: *const u32, ssr: *const u32) {
    unsafe {
        let newpc = *spc;
        (*ctx).sr.0 = *ssr;
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
pub fn sh4_tas(sr_T: *mut u32, ctx: *mut Sh4Ctx, addr: *const u32) {
    unsafe {
        let mut val: u8 = 0;
        let _ = read_mem::<u8>(ctx, *addr, &mut val);
        *sr_T = if val == 0 { 1 } else { 0 };
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
pub fn sh4_cmp_eq(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if *src_n == *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_hs(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if *src_n >= *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_ge(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if (*src_n as i32) >= (*src_m as i32) { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_hi(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if *src_n > *src_m { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_gt(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if (*src_n as i32) > (*src_m as i32) { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_pz(sr_T: *mut u32, src: *const u32) {
    unsafe {
        *sr_T = if (*src as i32) >= 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_pl(sr_T: *mut u32, src: *const u32) {
    unsafe {
        *sr_T = if (*src as i32) > 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_tst(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        *sr_T = if (*src_n & *src_m) == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_shll(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n >> 31;
        *dst = *src_n << 1;
    }
}

#[inline(always)]
pub fn sh4_shal(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n >> 31;
        *dst = ((*src_n as i32) << 1) as u32;
    }
}

#[inline(always)]
pub fn sh4_shar(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n & 1;
        *dst = ((*src_n as i32) >> 1) as u32;
    }
}

#[inline(always)]
pub fn sh4_cmp_eq_imm(sr_T: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *sr_T = if *src == imm { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_tst_imm(sr_T: *mut u32, src: *const u32, imm: u32) {
    unsafe {
        *sr_T = if (*src & imm) == 0 { 1 } else { 0 };
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
pub fn sh4_rotcl(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        let t = *sr_T;
        *sr_T = *src_n >> 31;
        *dst = (*src_n << 1) | t;
    }
}

#[inline(always)]
pub fn sh4_rotl(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n >> 31;
        *dst = (*src_n << 1) | *sr_T;
    }
}

#[inline(always)]
pub fn sh4_rotcr(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        let t = *src_n & 1;
        *dst = (*src_n >> 1) | ((*sr_T) << 31);
        *sr_T = t;
    }
}

#[inline(always)]
pub fn sh4_rotr(sr_T: *mut u32, dst: *mut u32, src_n: *const u32) {
    unsafe {
        *sr_T = *src_n & 1;
        *dst = (*src_n >> 1) | ((*sr_T) << 31);
    }
}

#[inline(always)]
pub fn sh4_movt(dst: *mut u32, sr_T: *const u32) {
    unsafe {
        *dst = *sr_T;
    }
}

#[inline(always)]
pub fn sh4_clrt(sr_T: *mut u32) {
    unsafe {
        *sr_T = 0;
    }
}

#[inline(always)]
pub fn sh4_sett(sr_T: *mut u32) {
    unsafe {
        *sr_T = 1;
    }
}

#[inline(always)]
pub fn sh4_negc(sr_T: *mut u32, dst: *mut u32, src: *const u32) {
    unsafe {
        let tmp = 0u32.wrapping_sub(*src);
        *dst = tmp.wrapping_sub(*sr_T);
        *sr_T = if tmp > 0 { 1 } else { 0 };
        if tmp < *dst {
            *sr_T = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_addc(sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let tmp1 = (*src_n).wrapping_add(*src_m);
        let tmp0 = *src_n;
        *dst = tmp1.wrapping_add(*sr_T);
        *sr_T = if tmp0 > tmp1 { 1 } else { 0 };
        if tmp1 > *dst {
            *sr_T = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_addv(sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let br = (*src_n as i32 as i64).wrapping_add(*src_m as i32 as i64);
        *sr_T = if br >= 0x80000000 || br < -0x80000000i64 { 1 } else { 0 };
        *dst = (*src_n).wrapping_add(*src_m);
    }
}

#[inline(always)]
pub fn sh4_subc(sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let tmp1 = (*src_n).wrapping_sub(*src_m);
        let tmp0 = *src_n;
        *dst = tmp1.wrapping_sub(*sr_T);
        *sr_T = if tmp0 < tmp1 { 1 } else { 0 };
        if tmp1 < *dst {
            *sr_T = 1;
        }
    }
}

#[inline(always)]
pub fn sh4_subv(sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let br = (*src_n as i32 as i64).wrapping_sub(*src_m as i32 as i64);
        *sr_T = if br >= 0x80000000 || br < -0x80000000i64 { 1 } else { 0 };
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
pub fn sh4_div0u(sr: *mut super::SrStatus, sr_T: *mut u32) {
    unsafe {
        (*sr).set_M(false);
        (*sr).set_Q(false);
        *sr_T = 0;
    }
}

#[inline(always)]
pub fn sh4_div0s(sr: *mut super::SrStatus, sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let q = (*src_n >> 31) != 0;
        let m = (*src_m >> 31) != 0;
        (*sr).set_Q(q);
        (*sr).set_M(m);
        *sr_T = if m ^ q { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_cmp_str(sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let temp = *src_n ^ *src_m;
        let hh = (temp & 0xFF000000) >> 24;
        let hl = (temp & 0x00FF0000) >> 16;
        let lh = (temp & 0x0000FF00) >> 8;
        let ll = temp & 0x000000FF;
        let result = hh & hl & lh & ll;
        *sr_T = if result == 0 { 1 } else { 0 };
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
pub fn sh4_div1(sr: *mut super::SrStatus, sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
    unsafe {
        let old_q = (*sr).Q() as u32;
        let new_q = if (*src_n & 0x80000000) != 0 { 1 } else { 0 };
        (*sr).set_Q(new_q != 0);

        let mut rn = (*src_n << 1) | *sr_T;
        let tmp0 = rn;
        let tmp2 = *src_m;
        let sr_m = (*sr).M() as u32;

        let tmp1: u32;
        if old_q == 0 {
            if sr_m == 0 {
                rn = rn.wrapping_sub(tmp2);
                tmp1 = if rn > tmp0 { 1 } else { 0 };
                let updated_q = if new_q == 0 { tmp1 } else { if tmp1 == 0 { 1 } else { 0 } };
                (*sr).set_Q(updated_q != 0);
            } else {
                rn = rn.wrapping_add(tmp2);
                tmp1 = if rn < tmp0 { 1 } else { 0 };
                let updated_q = if new_q == 0 { if tmp1 == 0 { 1 } else { 0 } } else { tmp1 };
                (*sr).set_Q(updated_q != 0);
            }
        } else {
            if sr_m == 0 {
                rn = rn.wrapping_add(tmp2);
                tmp1 = if rn < tmp0 { 1 } else { 0 };
                let updated_q = if new_q == 0 { tmp1 } else { if tmp1 == 0 { 1 } else { 0 } };
                (*sr).set_Q(updated_q != 0);
            } else {
                rn = rn.wrapping_sub(tmp2);
                tmp1 = if rn > tmp0 { 1 } else { 0 };
                let updated_q = if new_q == 0 { if tmp1 == 0 { 1 } else { 0 } } else { tmp1 };
                (*sr).set_Q(updated_q != 0);
            }
        }

        *dst = rn;
        let final_q = (*sr).Q() as u32;
        *sr_T = if final_q == sr_m { 1 } else { 0 };
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
pub fn sh4_tst_mem(sr_T: *mut u32, ctx: *mut Sh4Ctx, base: *const u32, index: *const u32, imm: u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(ctx, addr, &mut temp);
        *sr_T = if (temp as u32 & imm) == 0 { 1 } else { 0 };
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


