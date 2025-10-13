/*
    This file is part of libswirl
*/
// #include "license/bsd"

// PVR memory operations from pvr_mem.h

pub static mut EMU_VRAM: *mut u8 = std::ptr::null_mut();

pub const VRAM_SIZE: usize = 8 * 1024 * 1024;
pub const VRAM_MASK: u32 = (VRAM_SIZE - 1) as u32;
pub const VRAM_BANK_BIT: u32 = 0x400000;

#[inline(always)]
pub unsafe fn pvr_map32(offset32: u32) -> u32 {
    // 64b wide bus is achieved by interleaving the banks every 32 bits
    let static_bits = (VRAM_MASK - (VRAM_BANK_BIT * 2 - 1)) | 3;
    let offset_bits = (VRAM_BANK_BIT - 1) & !3;

    let bank = (offset32 & VRAM_BANK_BIT) / VRAM_BANK_BIT;

    let mut rv = offset32 & static_bits;

    rv |= (offset32 & offset_bits) * 2;

    rv |= bank * 4;

    rv
}

#[inline(always)]
pub fn vrf(vram: *const u8, addr: u32) -> f32 {
    unsafe { *(vram.add(pvr_map32(addr) as usize) as *const f32) }
}

#[inline(always)]
pub fn vri(vram: *const u8, addr: u32) -> u32 {
    unsafe { *(vram.add(pvr_map32(addr) as usize) as *const u32) }
}

// Write operations
#[inline(always)]
pub fn pvr_write_area1_16(_ctx: *mut u8, addr: u32, data: u16) {
    unsafe {
        let vram = EMU_VRAM;
        let _vaddr = addr & VRAM_MASK;
        *(vram.add(pvr_map32(addr) as usize) as *mut u16) = data;
    }
}

#[inline(always)]
pub fn pvr_write_area1_32(_ctx: *mut u8, addr: u32, data: u32) {
    unsafe {
        let vram = EMU_VRAM;
        let _vaddr = addr & VRAM_MASK;
        *(vram.add(pvr_map32(addr) as usize) as *mut u32) = data;
    }
}
