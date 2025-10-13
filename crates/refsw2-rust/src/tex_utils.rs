/*
    This file is part of libswirl
*/
// #include "license/bsd"

// Texture utilities from TexUtils.h/cc

use std::f32::consts::PI;

pub static mut DETWIDDLE: [[[u32; 1024]; 11]; 2] = [[[0; 1024]; 11]; 2];
pub static mut BM_SIN90: [i8; 256] = [0; 256];
pub static mut BM_COS90: [i8; 256] = [0; 256];
pub static mut BM_COS360: [i8; 256] = [0; 256];

#[inline]
pub fn cclamp<T: Ord>(minv: T, maxv: T, x: T) -> T {
    x.clamp(minv, maxv)
}

// Unpack to 32-bit ARGB word
#[inline]
pub const fn argb1555_32(word: u16) -> u32 {
    let word = word as u32;
    let a = if (word & 0x8000) != 0 { 0xFF000000 } else { 0 };
    let r = ((word >> 0) & 0x1F) << 3;
    let g = ((word >> 5) & 0x1F) << 11;
    let b = ((word >> 10) & 0x1F) << 19;
    a | r | g | b
}

#[inline]
pub const fn argb565_32(word: u16) -> u32 {
    let word = word as u32;
    let r = ((word >> 0) & 0x1F) << 3;
    let g = ((word >> 5) & 0x3F) << 10;
    let b = ((word >> 11) & 0x1F) << 19;
    0xFF000000 | r | g | b
}

#[inline]
pub const fn argb4444_32(word: u16) -> u32 {
    let word = word as u32;
    let a = ((word >> 12) & 0xF) << 28;
    let r = ((word >> 0) & 0xF) << 4;
    let g = ((word >> 4) & 0xF) << 12;
    let b = ((word >> 8) & 0xF) << 20;
    a | r | g | b
}

#[inline]
pub const fn argb8888_32(word: u32) -> u32 {
    word // Just shuffles in original, kept as-is for now
}

#[inline]
fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | 0xFF000000
}

#[inline]
pub fn yuv422(y: i32, yu: i32, yv: i32) -> u32 {
    let yu = yu - 128;
    let yv = yv - 128;

    let r = y + yv * 11 / 8;
    let g = y - (yu * 11 + yv * 22) / 32;
    let b = y + yu * 110 / 64;

    pack_rgb(
        cclamp(0, 255, r) as u8,
        cclamp(0, 255, g) as u8,
        cclamp(0, 255, b) as u8,
    )
}

fn twiddle_slow(mut x: u32, mut y: u32, mut x_sz: u32, mut y_sz: u32) -> u32 {
    let mut rv = 0;
    let mut sh = 0;

    x_sz >>= 1;
    y_sz >>= 1;

    while x_sz != 0 || y_sz != 0 {
        if y_sz != 0 {
            let temp = y & 1;
            rv |= temp << sh;

            y_sz >>= 1;
            y >>= 1;
            sh += 1;
        }
        if x_sz != 0 {
            let temp = x & 1;
            rv |= temp << sh;

            x_sz >>= 1;
            x >>= 1;
            sh += 1;
        }
    }

    rv
}

#[inline]
pub fn twop(x: u32, y: u32, bcx: u32, bcy: u32) -> u32 {
    unsafe {
        DETWIDDLE[0][(bcy + 3) as usize][x as usize] +
        DETWIDDLE[1][(bcx + 3) as usize][y as usize]
    }
}

pub unsafe fn init_tex_utils() {
    // Initialize detwiddle tables
    unsafe {
        for s in 0..11 {
            let x_sz: u32 = 1024;
            let y_sz: u32 = 1 << s;
            for i in 0..x_sz as usize {
                DETWIDDLE[0][s][i] = twiddle_slow(i as u32, 0, x_sz, y_sz);
                DETWIDDLE[1][s][i] = twiddle_slow(0, i as u32, y_sz, x_sz);
            }
        }
    }

    // Initialize bump mapping tables
    unsafe {
        for i in 0..256 {
            let t = (i as f32 / 256.0) * (PI / 2.0);
            BM_SIN90[i] = (127.0 * t.sin()) as i8;
            BM_COS90[i] = (127.0 * t.cos()) as i8;

            let t2 = (i as f32 / 256.0) * (2.0 * PI);
            BM_COS360[i] = (127.0 * t2.cos()) as i8;
        }
    }
}
