use sh4_core::sh4mem::read_mem;
use crate::asic;
use crate::Dreamcast;
use crate::area0::SYSTEM_BUS;

// Maple pattern commands
const MP_START: u32 = 0;
const MP_SDCKB_OCCUPY: u32 = 2;
const MP_RESET: u32 = 3;
const MP_SDCKB_OCCUPY_CANCEL: u32 = 4;
const MP_NOP: u32 = 7;

// Helper to check if address is on SH4 RAM
fn is_on_sh4_ram(addr: u32) -> bool {
    let region = (addr >> 26) & 0x7;
    let area = (addr >> 29) & 0x7;
    region == 3 && area != 7
}

// Get maple port from recipient address
fn maple_get_port(reci: u32) -> u32 {
    match reci {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        _ => 5,
    }
}

// Get maple bus ID from recipient address
fn maple_get_bus_id(reci: u32) -> u32 {
    (reci >> 6) & 3
}

/// Process Maple DMA with no devices connected
/// Returns true if DMA completed successfully
pub fn maple_do_dma(dc: &mut Dreamcast) -> bool {
    // Get system bus registers
    let sb_mden = unsafe {
        SYSTEM_BUS.read(crate::system_bus::SB_MDEN_ADDR, 4)
    };

    // Check if DMA is enabled
    if (sb_mden & 1) == 0 {
        return false;
    }

    let sb_mdstar = unsafe {
        SYSTEM_BUS.read(crate::system_bus::SB_MDSTAR_ADDR, 4)
    };
    let mut addr = sb_mdstar;
    let mut last = false;

    while !last {
        // Read DMA header
        let mut header_1: u32 = 0;
        let mut header_2: u32 = 0;

        unsafe {
            read_mem(&mut dc.ctx, addr, &mut header_1);
            read_mem(&mut dc.ctx, addr + 4, &mut header_2);
        }

        header_2 &= 0x1FFF_FFE0;

        last = (header_1 >> 31) == 1; // Is last transfer?
        let plen = ((header_1 & 0xFF) + 1) as usize; // Transfer length (32-bit unit)
        let maple_op = (header_1 >> 8) & 7; // Pattern selection

        match maple_op {
            MP_START => {
                // Validate destination address is on SH4 RAM
                let mut dest_addr = header_2;
                if !is_on_sh4_ram(dest_addr) {
                    println!("MAPLE ERROR: DESTINATION NOT ON SH4 RAM 0x{:08X}", dest_addr);
                    dest_addr &= 0x00FF_FFFF;
                    dest_addr |= 0x0C00_0000; // Force to main RAM
                }

                // Read command data
                let mut command_word: u32 = 0;
                unsafe {
                    read_mem(&mut dc.ctx, addr + 8, &mut command_word);
                }

                let command = command_word & 0xFF;
                let reci = (command_word >> 8) & 0xFF;
                let port = maple_get_port(reci);
                let bus = maple_get_bus_id(reci);

                // Since no devices are connected, always return "no device" response
                // Write 0xFFFFFFFF to indicate no device
                if port != 5 && command != 1 {
                    println!("MAPLE: No device at bus {} port {} (cmd {})", bus, port, command);
                }

                // Write "no device" response (0xFFFFFFFF)
                let no_device_response: u32 = 0xFFFF_FFFF;
                unsafe {
                    let dest_ptr = (dest_addr & 0x0FFF_FFFF) as *mut u32;
                    if dest_addr >= 0x0C00_0000 && dest_addr < 0x0D00_0000 {
                        // Main RAM
                        let offset = (dest_addr & 0x00FF_FFFF) as usize;
                        if offset < dc.sys_ram.len() {
                            let ram_ptr = dc.sys_ram.as_mut_ptr().add(offset) as *mut u32;
                            *ram_ptr = no_device_response;
                        }
                    } else if dest_addr >= 0x8C00_0000 && dest_addr < 0x8D00_0000 {
                        // Main RAM (cached)
                        let offset = (dest_addr & 0x00FF_FFFF) as usize;
                        if offset < dc.sys_ram.len() {
                            let ram_ptr = dc.sys_ram.as_mut_ptr().add(offset) as *mut u32;
                            *ram_ptr = no_device_response;
                        }
                    } else if dest_addr >= 0xAC00_0000 && dest_addr < 0xAD00_0000 {
                        // Main RAM (uncached)
                        let offset = (dest_addr & 0x00FF_FFFF) as usize;
                        if offset < dc.sys_ram.len() {
                            let ram_ptr = dc.sys_ram.as_mut_ptr().add(offset) as *mut u32;
                            *ram_ptr = no_device_response;
                        }
                    }
                }

                // Move to next command
                addr = addr.wrapping_add(2 * 4 + (plen as u32) * 4);
            }

            MP_SDCKB_OCCUPY => {
                // SDCKB occupy - just skip
                addr = addr.wrapping_add(4);
            }

            MP_SDCKB_OCCUPY_CANCEL => {
                // SDCKB occupy cancel - just skip
                addr = addr.wrapping_add(4);
            }

            MP_RESET => {
                // Reset - just skip
                addr = addr.wrapping_add(4);
            }

            MP_NOP => {
                // NOP - just skip
                addr = addr.wrapping_add(4);
            }

            _ => {
                println!("MAPLE: Unknown maple_op == {} length {}", maple_op, plen * 4);
                addr = addr.wrapping_add(4);
            }
        }
    }

    // DMA complete - clear MDST and raise interrupt
    unsafe {
        SYSTEM_BUS.write(crate::system_bus::SB_MDST_ADDR, 0, 4);
    }
    asic::raise_normal(12); // MAPLE_DMA interrupt

    true
}
