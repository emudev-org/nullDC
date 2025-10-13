use super::Sh4Ctx;
use std::ptr;

mod sealed {
    pub trait IntType {}
    impl IntType for u8 {}
    impl IntType for u16 {}
    impl IntType for u32 {}
    impl IntType for u64 {}
}

pub trait MemoryData: sealed::IntType + Copy + Default + std::fmt::LowerHex {
    fn from_u32(v: u32) -> Self;
    fn to_u32(self) -> u32;
}

impl MemoryData for u8 {
    fn from_u32(v: u32) -> Self {
        v as u8
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}
impl MemoryData for u16 {
    fn from_u32(v: u32) -> Self {
        v as u16
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}
impl MemoryData for u32 {
    fn from_u32(v: u32) -> Self {
        v
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}
impl MemoryData for u64 {
    fn from_u32(v: u32) -> Self {
        v as u64
    }
    fn to_u32(self) -> u32 {
        self as u32
    }
}

pub const MAX_MEMHANDLERS: usize = 256;

pub fn read_mem<T: Copy>(ctx: *mut Sh4Ctx, addr: u32, out: &mut T) -> bool {
    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;

        let base = (*ctx).memmap[region];
        if (base as usize) < MAX_MEMHANDLERS {
            let handler = (*ctx).memhandlers.get_unchecked(base as usize);
            let context = *(*ctx).memcontexts.get_unchecked(base as usize);

            match std::mem::size_of::<T>() {
                1 => {
                    let value = (handler.read8)(context, offset as u32);
                    *out = std::mem::transmute_copy(&value);
                }
                2 => {
                    let value = (handler.read16)(context, offset as u32);
                    *out = std::mem::transmute_copy(&value);
                }
                4 => {
                    let value = (handler.read32)(context, offset as u32);
                    *out = std::mem::transmute_copy(&value);
                }
                8 => {
                    let value = (handler.read64)(context, offset as u32);
                    *out = std::mem::transmute_copy(&value);
                }
                _ => panic!("Unsupported read size: {}", std::mem::size_of::<T>()),
            }
        } else {
            // pointer to T
            let ptr = base.add(offset) as *const T;
            *out = ptr::read_unaligned(ptr);
        }

        true
    }
}

pub fn write_mem<T: Copy>(ctx: *mut Sh4Ctx, addr: u32, data: T) -> bool {
    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;

        let base = (*ctx).memmap[region];
        if (base as usize) < MAX_MEMHANDLERS {
            let handler = (*ctx).memhandlers.get_unchecked(base as usize);
            let context = *(*ctx).memcontexts.get_unchecked(base as usize);

            match std::mem::size_of::<T>() {
                1 => {
                    let value: u8 = std::mem::transmute_copy(&data);
                    (handler.write8)(context, offset as u32, value);
                }
                2 => {
                    let value: u16 = std::mem::transmute_copy(&data);
                    (handler.write16)(context, offset as u32, value);
                }
                4 => {
                    let value: u32 = std::mem::transmute_copy(&data);
                    (handler.write32)(context, offset as u32, value);
                }
                8 => {
                    let value: u64 = std::mem::transmute_copy(&data);
                    (handler.write64)(context, offset as u32, value);
                }
                _ => panic!("Unsupported write size: {}", std::mem::size_of::<T>()),
            }
        } else {
            let ptr = base.add(offset) as *mut T;
            ptr::write_unaligned(ptr, data);
        }

        true
    }
}

pub fn write_mem_sq(ctx: *mut Sh4Ctx, addr: u32, data: *const u32) {

    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;

        let base = (*ctx).memmap[region];
        if (base as usize) < MAX_MEMHANDLERS {
            let handler = (*ctx).memhandlers.get_unchecked(base as usize);
            let context = *(*ctx).memcontexts.get_unchecked(base as usize);

            (handler.write256)(context, offset as u32, data);
        } else {
            let ptr = base.add(offset) as *mut u32;
            ptr::copy_nonoverlapping(data, ptr, 32/4);
        }
    }
}
