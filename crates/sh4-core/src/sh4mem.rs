use super::Sh4Ctx;

pub fn read_mem<T: Copy>(ctx: *mut Sh4Ctx, addr: u32, out: &mut T) -> bool {
    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;


        let base = (*ctx).memmap[region];
        if base.is_null() {
            return false;
        }
        // pointer to T
        let ptr = base.add(offset) as *const T;
        *out = *ptr;

        true
    }
}

pub fn write_mem<T: Copy>(ctx: *mut Sh4Ctx, addr: u32, data: T) -> bool {
    unsafe {
        let region = (addr >> 24) as usize;
        let offset = (addr & (*ctx).memmask[region]) as usize;

        let base = (*ctx).memmap[region];
        if base.is_null() {
            return false;
        }
        let ptr = base.add(offset) as *mut T;
        *ptr = data;

        true
    }
}


