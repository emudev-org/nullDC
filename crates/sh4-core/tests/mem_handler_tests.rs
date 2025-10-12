use sh4_core::{MemHandlers, Sh4Ctx, sh4mem};
use std::ptr;

fn test_read8(_ctx: *mut u8, offset: u32) -> u8 {
    (offset & 0xFF) as u8
}

fn test_read16(_ctx: *mut u8, offset: u32) -> u16 {
    (offset & 0xFFFF) as u16
}

fn test_read32(_ctx: *mut u8, offset: u32) -> u32 {
    offset
}

fn test_read64(_ctx: *mut u8, offset: u32) -> u64 {
    offset as u64 | ((offset as u64) << 32)
}

fn test_write8(_ctx: *mut u8, _addr: u32, _value: u8) {}
fn test_write16(_ctx: *mut u8, _addr: u32, _value: u16) {}
fn test_write32(_ctx: *mut u8, _addr: u32, _value: u32) {}
fn test_write64(_ctx: *mut u8, _addr: u32, _value: u64) {}

#[test]
fn test_read_mem_sizes() {
    let mut ctx = Sh4Ctx::default();

    // Register test memory handlers for all regions
    let test_handler = MemHandlers {
        read8: test_read8,
        read16: test_read16,
        read32: test_read32,
        read64: test_read64,
        write8: test_write8,
        write16: test_write16,
        write32: test_write32,
        write64: test_write64,
    };

    for i in 0..256 {
        ctx.memhandlers[i] = test_handler;
        ctx.memcontexts[i] = ptr::null_mut();
        ctx.memmask[i] = 0xFFFFFF;
        ctx.memmap[i] = 0 as *mut u8; // Handler index 0
    }

    let ctx_ptr = &mut ctx as *mut Sh4Ctx;

    // Test read8
    let mut val8: u8 = 0;
    sh4mem::read_mem(ctx_ptr, 0x00123456, &mut val8);
    assert_eq!(val8, 0x56);

    // Test read16
    let mut val16: u16 = 0;
    sh4mem::read_mem(ctx_ptr, 0x00ABCDEF, &mut val16);

    assert_eq!(val16, 0xCDEF);

    // Test read32
    let mut val32: u32 = 0;
    sh4mem::read_mem(ctx_ptr, 0xDEADBEEF, &mut val32);

    assert_eq!(val32, 0xADBEEF); // Masked with 0xFFFFFF

    // Test read64
    let mut val64: u64 = 0;
    sh4mem::read_mem(ctx_ptr, 0x12345678, &mut val64);

    assert_eq!(val64, 0x0034567800345678); // Masked with 0xFFFFFF
}
