use std::ptr;

use sh4_core::MemHandlers;
use sh4_core::sh4mem;

use crate::{Dreamcast, aica, asic, gdrom, spg, system_bus};

const AREA0_MASK: u32 = 0x01FF_FFFF;
const BIOS_START: u32 = 0x0000_0000;
const BIOS_END: u32 = 0x001F_FFFF;
const FLASH_START: u32 = 0x0020_0000;
const FLASH_END: u32 = 0x0021_FFFF;

fn area_0_read<T: sh4mem::MemoryData>(ctx: *mut u8, addr: u32) -> T {
    let dc = unsafe { &*(ctx as *const Dreamcast) };
    let size = std::mem::size_of::<T>();
    let masked_addr = addr & AREA0_MASK;
    let base = masked_addr >> 16;

    if addr < 0x0000_1000 {
        println!("area0 read low address (possible null pointer): addr=0x{masked_addr:08X}");
    }

    match base {
        0x0000..=0x001F => {
            return read_from_slice(&dc.bios_rom[..], masked_addr as usize, size).unwrap_or_else(
                || {
                    log_unaligned("BIOS", "read", masked_addr, size);
                    T::default()
                },
            );
        }
        0x0020..=0x0021 => {
            let relative = (masked_addr - FLASH_START) as usize;
            return read_from_slice(&dc.bios_flash[..], relative, size).unwrap_or_else(|| {
                log_unaligned("FLASH", "read", masked_addr, size);
                T::default()
            });
        }
        0x005F => {
            if gdrom::handles_address(masked_addr) {
                let value = gdrom::read(masked_addr, size);
                return sh4mem::MemoryData::from_u32(value);
            }
            if asic::handles_address(masked_addr) {
                let value = asic::read(masked_addr, size);
                return sh4mem::MemoryData::from_u32(value);
            }
            if spg::handles_address(masked_addr) {
                let value = spg::read(masked_addr, size);
                return sh4mem::MemoryData::from_u32(value);
            }
            return handle_system_bus_read(dc, masked_addr, size);
        }
        0x0060 => {
            warn_unimplemented("MODEM", "read", masked_addr, size);
        }
        0x0061..=0x006F => {
            warn_unimplemented("G2 reserved", "read", masked_addr, size);
        }
        0x0070 => {
            if aica::handles_address(masked_addr) {
                let value = aica::read_from_sh4(&dc.arm_ctx, masked_addr, size);
                return sh4mem::MemoryData::from_u32(value);
            } else {
                warn_unimplemented("AICA control", "read", masked_addr, size);
            }
        }
        0x0071 => {
            warn_unimplemented("AICA RTC", "read", masked_addr, size);
        }
        0x0080..=0x00FF => {
            let offset = (masked_addr & crate::AUDIORAM_MASK) as usize;
            return read_from_slice(&dc.audio_ram[..], offset, size).unwrap_or_else(|| {
                log_unaligned("AICA", "read", masked_addr, size);
                T::default()
            });
        }
        0x0100..=0x01FF => {
            warn_unimplemented("External device", "read", masked_addr, size);
        }
        _ => { /* fallthrough */ }
    }

    log_unhandled("read", masked_addr, size);
    T::default()
}

static mut SYSTEM_BUS: system_bus::SystemBus = system_bus::SystemBus::default();

fn area_0_write<T: sh4mem::MemoryData>(ctx: *mut u8, addr: u32, value: T) {
    let dc = unsafe { &mut *(ctx as *mut Dreamcast) };
    let size = std::mem::size_of::<T>();
    let masked_addr = addr & AREA0_MASK;
    let base = masked_addr >> 16;

    if addr < 0x0000_1000 {
        println!(
            "area0 write low address (possible null pointer): addr=0x{masked_addr:08X} value=0x{:x}",
            value
        );
    }

    match base {
        0x0000..=0x001F => {
            log_write_blocked("BIOS", masked_addr, size);
            return;
        }
        0x0020..=0x0021 => {
            let relative = (masked_addr - FLASH_START) as usize;
            if write_to_slice(&mut dc.bios_flash[..], relative, value, size).is_none() {
                log_unaligned("FLASH", "write", masked_addr, size);
            }
            return;
        }
        0x005F => {
            if gdrom::handles_address(masked_addr) {
                gdrom::write(masked_addr, size, value.to_u32());
                return;
            }
            if asic::handles_address(masked_addr) {
                asic::write(masked_addr, size, value.to_u32());
                return;
            }
            if spg::handles_address(masked_addr) {
                spg::write(masked_addr, size, value.to_u32());
                return;
            }
            handle_system_bus_write(dc, masked_addr, size, value);
            return;
        }
        0x0060 => {
            warn_unimplemented("MODEM", "write", masked_addr, size);
            return;
        }
        0x0061..=0x006F => {
            warn_unimplemented("G2 reserved", "write", masked_addr, size);
            return;
        }
        0x0070 => {
            if aica::handles_address(masked_addr) {
                aica::write_from_sh4(&mut dc.arm_ctx, masked_addr, size, value.to_u32());
            } else {
                warn_unimplemented("AICA control", "write", masked_addr, size);
            }
            return;
        }
        0x0071 => {
            warn_unimplemented("AICA RTC", "write", masked_addr, size);
            return;
        }
        0x0080..=0x00FF => {
            let offset = (masked_addr & crate::AUDIORAM_MASK) as usize;
            if write_to_slice(&mut dc.audio_ram[..], offset, value, size).is_none() {
                log_unaligned("AICA", "write", masked_addr, size);
            }
            return;
        }
        0x0100..=0x01FF => {
            warn_unimplemented("External device", "write", masked_addr, size);
            return;
        }
        _ => { /* fallthrough */ }
    }

    log_unhandled_write(masked_addr, size, value);
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

fn read_from_slice<T: Copy>(slice: &[u8], offset: usize, size: usize) -> Option<T> {
    if offset + size > slice.len() {
        return None;
    }
    unsafe { (slice.as_ptr().add(offset) as *const T).as_ref().copied() }
}

fn write_to_slice<T: Copy>(slice: &mut [u8], offset: usize, value: T, size: usize) -> Option<()> {
    if offset + size > slice.len() {
        return None;
    }
    unsafe {
        let dst = slice.as_mut_ptr().add(offset) as *mut T;
        ptr::write_unaligned(dst, value);
    }
    Some(())
}

fn log_unhandled(op: &str, addr: u32, size: usize) {
    println!("area0 {op} unhandled: addr=0x{addr:08X} size={size}");
}

fn log_unhandled_write<T: sh4mem::MemoryData>(addr: u32, size: usize, value: T) {
    println!(
        "area0 write unhandled: addr=0x{addr:08X} size={size} value=0x{:x}",
        value
    );
}

fn log_unaligned(region: &str, op: &str, addr: u32, size: usize) {
    println!("area0 {region} {op} out-of-range: addr=0x{addr:08X} size={size}");
}

fn log_write_blocked(region: &str, addr: u32, size: usize) {
    println!("area0 write blocked ({region}): addr=0x{addr:08X} size={size}");
}

fn warn_unimplemented(region: &str, op: &str, addr: u32, size: usize) {
    println!("area0 {region} {op} not implemented: addr=0x{addr:08X} size={size}");
}

fn handle_system_bus_read<T: sh4mem::MemoryData>(dc: &Dreamcast, addr: u32, size: usize) -> T {
    match addr {
        // System Bus
        0x005F_6800..=0x005F_7CFF => unsafe { T::from_u32(SYSTEM_BUS.read(addr, size as u32)) },
        // GD-ROM
        0x005F_7000..=0x005F_70FF => unsafe { T::from_u32(SYSTEM_BUS.read(addr, size as u32)) },
        // G1 i/f
        0x005F_7400..=0x005F_74FF => unsafe { T::from_u32(SYSTEM_BUS.read(addr, size as u32)) },
        // G2 i/f
        0x005F_7800..=0x005F_78FF => unsafe { T::from_u32(SYSTEM_BUS.read(addr, size as u32)) },
        // PVR i/f
        0x005F_7C00..=0x005F_7CFF => unsafe { T::from_u32(SYSTEM_BUS.read(addr, size as u32)) },
        0x005F_8000..=0x005F_9FFF => {
            if addr == 0x005F_8000 {
                return T::from_u32(0x17FD11DB);
            }
            if addr == 0x005F_8004 {
                return T::from_u32(0x00000011);
            }
            warn_unimplemented("TA/PVR core", "read", addr, size);
            T::default()
        }
        _ => {
            warn_unimplemented("Area 0 unassigned", "read", addr, size);
            T::default()
        }
    }
}

fn handle_system_bus_write<T: sh4mem::MemoryData>(
    dc: &mut Dreamcast,
    addr: u32,
    size: usize,
    value: T,
) {
    match addr {
        // System Bus
        0x005F_6800..=0x005F_7CFF => unsafe {
            SYSTEM_BUS.write(addr, value.to_u32(), size as u32);
        },
        // GD-ROM
        0x005F_7000..=0x005F_70FF => unsafe {
            SYSTEM_BUS.write(addr, value.to_u32(), size as u32);
        },
        // G1 i/f
        0x005F_7400..=0x005F_74FF => unsafe {
            SYSTEM_BUS.write(addr, value.to_u32(), size as u32);
        },
        // G2 i/f
        0x005F_7800..=0x005F_78FF => unsafe {
            SYSTEM_BUS.write(addr, value.to_u32(), size as u32);
        },
        // PVR i/f
        0x005F_7C00..=0x005F_7CFF => unsafe {
            SYSTEM_BUS.write(addr, value.to_u32(), size as u32);
        },
        0x005F_8000..=0x005F_9FFF => {
            warn_unimplemented("TA/PVR core", "write", addr, size);
        }
        _ => {
            warn_unimplemented("Area 0 unassigned", "write", addr, size);
        }
    }
}

pub fn reset() {
    unsafe {
        SYSTEM_BUS.setup();
    }
}
