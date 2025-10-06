use crate::dreamcast::Dreamcast;
use crate::dreamcast::sh4::sh4mem::{read_mem, write_mem};

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
pub fn sh4_write_mem8(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u8>(dc, *addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u16>(dc, *addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32(dc: *mut Dreamcast, addr: *const u32, data: *const u32) {
    unsafe {
        let _ = write_mem::<u32>(dc, *addr, *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64(dc: *mut Dreamcast, addr: *const u32, data: *const u64) {
    unsafe {
        let _ = write_mem::<u64>(dc, *addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    unsafe {
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(dc, *addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    unsafe {
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(dc, *addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32(dc: *mut Dreamcast, addr: *const u32, data: *mut u32) {
    unsafe {
        let _ = read_mem::<u32>(dc, *addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mem64(dc: *mut Dreamcast, addr: *const u32, data: *mut u64) {
    unsafe {
        let _ = read_mem::<u64>(dc, *addr, &mut *data);
    }
}


#[inline(always)]
pub fn sh4_read_mem32i(dc: *mut Dreamcast, addr: u32, data: *mut u32) {
    unsafe {
        let _ = read_mem::<u32>(dc, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems16_i(dc: *mut Dreamcast, addr: u32, data: *mut u32) {
    unsafe {
        let mut temp: u16 = 0;
        let _ = read_mem::<u16>(dc, addr, &mut temp);
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
pub fn sh4_branch_cond(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *T == condition {
            (*dc).ctx.pc1 = target;
            (*dc).ctx.pc2 = target.wrapping_add(2);
        } else {
            // these are calcualted by the pipeline logic in the main loop, no need to do it here
            // but it is done anyway for validation purposes
            (*dc).ctx.pc1 = next;
            (*dc).ctx.pc2 = next.wrapping_add(2);
        }
    }
}

#[inline(always)]
pub fn sh4_branch_cond_delay(dc: *mut Dreamcast, T: *const u32, condition: u32, next: u32, target: u32) {
    unsafe {
        if *T == condition {
            (*dc).ctx.pc2 = target;
        } else {
            // this is calcualted by the pipeline logic in the main loop, no need to do it here
            // but it is done anyway for validation purposes
            (*dc).ctx.pc2 = next;
        }
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_branch_delay(dc: *mut Dreamcast, target: u32) {
    unsafe {
        (*dc).ctx.pc2 = target;
        (*dc).ctx.is_delayslot1 = 1;
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
pub fn sh4_dec_call_decode(dc: *mut Dreamcast) {
    use crate::dreamcast::sh4::sh4_fns_decode_on_demand;
    unsafe {
        sh4_fns_decode_on_demand(&mut *dc);
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
pub fn sh4_frchg() {
    // No-op for interpreter - bitfield operations done in frontend
}

#[inline(always)]
pub fn sh4_fschg() {
    // No-op for interpreter - bitfield operations done in frontend
}

#[inline(always)]
pub fn sh4_jmp(dc: *mut Dreamcast, src: *const u32) {
    unsafe {
        let newpc = *src;
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_jsr(dc: *mut Dreamcast, src: *const u32, next_pc: u32) {
    unsafe {
        let newpc = *src;
        (*dc).ctx.pr = next_pc;
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_braf(dc: *mut Dreamcast, src: *const u32, pc: u32) {
    unsafe {
        let newpc = (*src).wrapping_add(pc).wrapping_add(4);
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_bsrf(dc: *mut Dreamcast, src: *const u32, pc: u32) {
    unsafe {
        let newpc = (*src).wrapping_add(pc).wrapping_add(4);
        let newpr = pc.wrapping_add(4);
        (*dc).ctx.pr = newpr;
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_rts(dc: *mut Dreamcast, pr: *const u32) {
    unsafe {
        let newpc = *pr;
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
    }
}

#[inline(always)]
pub fn sh4_rte(dc: *mut Dreamcast, spc: *const u32, ssr: *const u32) {
    unsafe {
        let newpc = *spc;
        (*dc).ctx.sr.0 = *ssr;
        (*dc).ctx.pc2 = newpc;
        (*dc).ctx.is_delayslot1 = 1;
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
pub fn sh4_tas(sr_T: *mut u32, dc: *mut Dreamcast, addr: *const u32) {
    unsafe {
        let mut val: u8 = 0;
        let _ = read_mem::<u8>(dc, *addr, &mut val);
        *sr_T = if val == 0 { 1 } else { 0 };
        val |= 0x80;
        let _ = write_mem::<u8>(dc, *addr, val);
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
pub fn sh4_div0u(sr: *mut crate::dreamcast::sh4::SrStatus, sr_T: *mut u32) {
    unsafe {
        (*sr).set_M(false);
        (*sr).set_Q(false);
        *sr_T = 0;
    }
}

#[inline(always)]
pub fn sh4_div0s(sr: *mut crate::dreamcast::sh4::SrStatus, sr_T: *mut u32, src_n: *const u32, src_m: *const u32) {
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
pub fn sh4_div1(sr: *mut crate::dreamcast::sh4::SrStatus, sr_T: *mut u32, dst: *mut u32, src_n: *const u32, src_m: *const u32) {
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
pub fn sh4_write_mem8_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u8>(dc, addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u16>(dc, addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u32>(dc, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(dc, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(dc, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = read_mem::<u32>(dc, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_read_mem64_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *mut u64) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = read_mem::<u64>(dc, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64_indexed(dc: *mut Dreamcast, base: *const u32, index: *const u32, data: *const u64) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let _ = write_mem::<u64>(dc, addr, *data);
    }
}

// Read-modify-write operations for memory with GBR+R0 addressing
#[inline(always)]
pub fn sh4_tst_mem(sr_T: *mut u32, dc: *mut Dreamcast, base: *const u32, index: *const u32, imm: u32) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(dc, addr, &mut temp);
        *sr_T = if (temp as u32 & imm) == 0 { 1 } else { 0 };
    }
}

#[inline(always)]
pub fn sh4_and_mem(dc: *mut Dreamcast, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(dc, addr, &mut temp);
        temp &= imm;
        let _ = write_mem::<u8>(dc, addr, temp);
    }
}

#[inline(always)]
pub fn sh4_xor_mem(dc: *mut Dreamcast, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(dc, addr, &mut temp);
        temp ^= imm;
        let _ = write_mem::<u8>(dc, addr, temp);
    }
}

#[inline(always)]
pub fn sh4_or_mem(dc: *mut Dreamcast, base: *const u32, index: *const u32, imm: u8) {
    unsafe {
        let addr = (*base).wrapping_add(*index);
        let mut temp: u8 = 0;
        let _ = read_mem::<u8>(dc, addr, &mut temp);
        temp |= imm;
        let _ = write_mem::<u8>(dc, addr, temp);
    }
}

// Memory operations with displacement addressing (Rn + disp)
#[inline(always)]
pub fn sh4_write_mem8_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u8>(dc, addr, *data as u8);
    }
}

#[inline(always)]
pub fn sh4_write_mem16_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u16>(dc, addr, *data as u16);
    }
}

#[inline(always)]
pub fn sh4_write_mem32_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *const u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u32>(dc, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_read_mems8_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let mut read: i8 = 0;
        let _ = read_mem::<i8>(dc, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mems16_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let mut read: i16 = 0;
        let _ = read_mem::<i16>(dc, addr, &mut read);
        *data = read as i32 as u32;
    }
}

#[inline(always)]
pub fn sh4_read_mem32_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *mut u32) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = read_mem::<u32>(dc, addr, &mut *data);
    }
}

#[inline(always)]
pub fn sh4_write_mem64_disp(dc: *mut Dreamcast, base: *const u32, disp: u32, data: *const u64) {
    unsafe {
        let addr = (*base).wrapping_add(disp);
        let _ = write_mem::<u64>(dc, addr, *data);
    }
}

#[inline(always)]
pub fn sh4_fsrra(dst: *mut f32, src: *const f32) {
    unsafe {
        *dst = 1.0 / (*src).sqrt();
    }
}

#[inline(always)]
pub fn sh4_fipr(fr: *mut f32, n: usize, m: usize) {
    unsafe {
        let idp = *fr.add(n + 0) * *fr.add(m + 0)
                + *fr.add(n + 1) * *fr.add(m + 1)
                + *fr.add(n + 2) * *fr.add(m + 2)
                + *fr.add(n + 3) * *fr.add(m + 3);
        *fr.add(n + 3) = idp;
    }
}

#[inline(always)]
pub fn sh4_fmac(dst: *mut f32, fr0: *const f32, src_m: *const f32) {
    unsafe {
        *dst = (*dst as f64 + *fr0 as f64 * *src_m as f64) as f32;
    }
}

#[inline(always)]
pub fn sh4_ftrv(fr: *mut f32, xf: *const f32, n: usize) {
    unsafe {
        let v1 = *xf.add(0)  * *fr.add(n + 0) +
                 *xf.add(4)  * *fr.add(n + 1) +
                 *xf.add(8)  * *fr.add(n + 2) +
                 *xf.add(12) * *fr.add(n + 3);

        let v2 = *xf.add(1)  * *fr.add(n + 0) +
                 *xf.add(5)  * *fr.add(n + 1) +
                 *xf.add(9)  * *fr.add(n + 2) +
                 *xf.add(13) * *fr.add(n + 3);

        let v3 = *xf.add(2)  * *fr.add(n + 0) +
                 *xf.add(6)  * *fr.add(n + 1) +
                 *xf.add(10) * *fr.add(n + 2) +
                 *xf.add(14) * *fr.add(n + 3);

        let v4 = *xf.add(3)  * *fr.add(n + 0) +
                 *xf.add(7)  * *fr.add(n + 1) +
                 *xf.add(11) * *fr.add(n + 2) +
                 *xf.add(15) * *fr.add(n + 3);

        *fr.add(n + 0) = v1;
        *fr.add(n + 1) = v2;
        *fr.add(n + 2) = v3;
        *fr.add(n + 3) = v4;
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


