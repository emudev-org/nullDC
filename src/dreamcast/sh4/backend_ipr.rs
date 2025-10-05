use crate::dreamcast::Dreamcast;
use crate::dreamcast::sh4::sh4mem::{read_mem, write_mem};

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