use super::Sh4Ctx;
use std::ptr;

pub fn read_mem<T: Copy>(ctx: *mut Sh4Ctx, addr: u32, out: &mut T) -> bool {
    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;

        let base = (*ctx).memmap[region];
        if (base as usize) < 256 {
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
        if (base as usize) < 256 {
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


