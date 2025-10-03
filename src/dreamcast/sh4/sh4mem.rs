use crate::dreamcast::Dreamcast;

pub fn read_mem<T: Copy>(dc: &mut Dreamcast, addr: u32, out: &mut T) -> bool {
    let region = (addr >> 24) as usize;
    let offset = (addr & dc.memmask[region]) as usize;

    unsafe {
        let base = dc.memmap[region];
        if base.is_null() {
            return false;
        }
        // pointer to T
        let ptr = base.add(offset) as *const T;
        *out = *ptr;
    }

    true
}

pub fn write_mem<T: Copy>(dc: &mut Dreamcast, addr: u32, data: T) -> bool {
    let region = (addr >> 24) as usize;
    let offset = (addr & dc.memmask[region]) as usize;

    unsafe {
        let base = dc.memmap[region];
        if base.is_null() {
            return false;
        }
        let ptr = base.add(offset) as *mut T;
        *ptr = data;
    }

    true
}


