use sh4_core::MemHandlers;
use sh4_core::sh4mem;

use crate::Dreamcast;

unsafe fn buffer_read<T: Copy>(base: *const u8, offset: u32) -> T {
    let src = base.add(offset as usize) as *const T;
    std::ptr::read_unaligned(src)
}

fn area_0_read<T: sh4mem::MemoryData>(ctx: *mut u8, offset: u32) -> T {
    let dc = unsafe { &*(ctx as *const Dreamcast) };

    if offset < 0x001F_FFFF {
        return unsafe { buffer_read::<T>(dc.bios_rom.as_ptr(), offset) };
    }

    println!("area_0_read::<u{}> {:x}", std::mem::size_of::<T>() * 8, offset);
    T::default()
}

fn area_0_write<T: sh4mem::MemoryData>(_ctx: *mut u8, addr: u32, value: T) {
    println!(
        "area_0_write::<u{}> {:x} data = {:x}",
        std::mem::size_of::<T>() * 8,
        addr,
        value
    );
}

pub const AREA0_HANDLERS: MemHandlers = MemHandlers {
    read8: area_0_read::<u8>,
    read16: area_0_read::<u16>,
    read32: area_0_read::<u32>,
    read64: area_0_read::<u64>,

    write8: area_0_write::<u8>,
    write16: area_0_write::<u16>,
    write32: area_0_write::<u32>,
    write64: area_0_write::<u64>,
};
