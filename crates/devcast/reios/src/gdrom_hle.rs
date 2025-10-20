/*
    Basic GD-ROM syscall emulation
    Adapted from some (very) old pre-nulldc HLE code
    Ported from reference/devcast/libswirl/reios/gdrom_hle.cpp
*/

use crate::traits::{ReiosSh4Memory, ReiosSh4Context, ReiosDisc};

// GD-ROM syscall constants
pub const SYSCALL_GDROM: u32 = 0x00;

pub const GDROM_SEND_COMMAND: u32 = 0x00;
pub const GDROM_CHECK_COMMAND: u32 = 0x01;
pub const GDROM_MAIN: u32 = 0x02;
pub const GDROM_INIT: u32 = 0x03;
pub const GDROM_CHECK_DRIVE: u32 = 0x04;
pub const GDROM_ABORT_COMMAND: u32 = 0x08;
pub const GDROM_RESET: u32 = 0x09;
pub const GDROM_SECTOR_MODE: u32 = 0x0A;

// GD-ROM command codes
pub const GDCC_PIOREAD: u32 = 16;
pub const GDCC_DMAREAD: u32 = 17;
pub const GDCC_GETTOC: u32 = 18;
pub const GDCC_GETTOC2: u32 = 19;
pub const GDCC_PLAY: u32 = 20;
pub const GDCC_PLAY_SECTOR: u32 = 21;
pub const GDCC_PAUSE: u32 = 22;
pub const GDCC_RELEASE: u32 = 23;
pub const GDCC_INIT: u32 = 24;
pub const GDCC_SEEK: u32 = 27;
pub const GDCC_READ: u32 = 28;
pub const GDCC_STOP: u32 = 33;
pub const GDCC_GETSCD: u32 = 34;
pub const GDCC_GETSES: u32 = 35;

// TOC macros
#[inline]
pub const fn ctoc_lba(n: u32) -> u32 {
    n
}

#[inline]
pub const fn ctoc_adr(n: u32) -> u32 {
    n << 24
}

#[inline]
pub const fn ctoc_ctrl(n: u32) -> u32 {
    n << 28
}

#[inline]
pub const fn ctoc_track(n: u32) -> u32 {
    n << 16
}

/// Swap bytes for big-endian to little-endian conversion
#[inline]
fn swap32(a: u32) -> u32 {
    ((a & 0xff) << 24) | ((a & 0xff00) << 8) | ((a >> 8) & 0xff00) | ((a >> 24) & 0xff)
}

/// GD-ROM HLE state
pub struct GdromHleState {
    /// Sector mode (4 u32 values)
    sec_mode: [u32; 4],
    /// Last command ID for command checking
    last_cmd: u32,
    /// Request ID counter
    dw_req_id: u32,
}

impl GdromHleState {
    pub fn new() -> Self {
        Self {
            sec_mode: [0; 4],
            last_cmd: 0xFFFFFFFF,
            dw_req_id: 0xF0FFFFFF,
        }
    }

    /// Read session information from disc
    fn gdrom_hle_read_ses(&self, mem: &dyn ReiosSh4Memory, addr: u32) {
        let s = mem.read_mem32(addr + 0);
        let b = mem.read_mem32(addr + 4);
        let ba = mem.read_mem32(addr + 8);
        let bb = mem.read_mem32(addr + 12);

        println!("GDROM_HLE_ReadSES: doing nothing w/ {}, {}, {}, {}", s, b, ba, bb);
    }

    /// Read TOC (Table of Contents) from disc
    fn gdrom_hle_read_toc(&self, mem: &mut dyn ReiosSh4Memory, disc: &dyn ReiosDisc, addr: u32) {
        let s = mem.read_mem32(addr + 0);
        let b = mem.read_mem32(addr + 4);

        println!("GDROM READ TOC : {:X} {:X}", s, b);

        // Get TOC buffer (102 u32 values)
        let mut toc_buffer = vec![0u32; 102];
        disc.get_toc(&mut toc_buffer, s);

        // The syscall swaps to LE it seems
        for i in 0..102 {
            toc_buffer[i] = swap32(toc_buffer[i]);
        }

        // Write to memory
        for (i, &val) in toc_buffer.iter().enumerate() {
            mem.write_mem32(b + (i as u32 * 4), val);
        }
    }

    /// Read sectors from disc to memory
    fn read_sectors_to(
        &self,
        mem: &mut dyn ReiosSh4Memory,
        disc: &dyn ReiosDisc,
        addr: u32,
        sector: u32,
        count: u32,
    ) {
        // Try to get direct memory pointer
        let size = count * 2048;
        if let Some(ptr) = mem.get_mem_ptr(addr, size) {
            // Direct read to memory
            unsafe {
                let slice = std::slice::from_raw_parts_mut(ptr, size as usize);
                disc.read_sector(slice, sector, count, 2048);
            }
        } else {
            // Read sector by sector and write via memory interface
            let mut temp = vec![0u8; 2048];
            for i in 0..count {
                disc.read_sector(&mut temp, sector + i, 1, 2048);
                mem.write_mem_block(addr + (i * 2048), &temp);
            }
        }
    }

    /// DMA read from GD-ROM
    fn gdrom_hle_read_dma(
        &self,
        mem: &mut dyn ReiosSh4Memory,
        disc: &dyn ReiosDisc,
        addr: u32,
    ) {
        let s = mem.read_mem32(addr + 0x00); // Sector
        let n = mem.read_mem32(addr + 0x04); // Number of sectors
        let b = mem.read_mem32(addr + 0x08); // Buffer address
        let u = mem.read_mem32(addr + 0x0C); // Unknown parameter

        println!("GDROM:\tDMA READ Sector={}, Num={}, Buffer=0x{:08X}, Unk01=0x{:08X}", s, n, b, u);
        self.read_sectors_to(mem, disc, b, s, n);
    }

    /// PIO read from GD-ROM
    fn gdrom_hle_read_pio(
        &self,
        mem: &mut dyn ReiosSh4Memory,
        disc: &dyn ReiosDisc,
        addr: u32,
    ) {
        let s = mem.read_mem32(addr + 0x00);
        let n = mem.read_mem32(addr + 0x04);
        let b = mem.read_mem32(addr + 0x08);
        let u = mem.read_mem32(addr + 0x0C);

        println!("GDROM:\tPIO READ Sector={}, Num={}, Buffer=0x{:08X}, Unk01=0x{:08X}", s, n, b, u);
        self.read_sectors_to(mem, disc, b, s, n);
    }

    /// Get subcode data
    fn gdcc_hle_getscd(&self, mem: &dyn ReiosSh4Memory, addr: u32) {
        let s = mem.read_mem32(addr + 0x00);
        let n = mem.read_mem32(addr + 0x04);
        let b = mem.read_mem32(addr + 0x08);
        let u = mem.read_mem32(addr + 0x0C);

        println!("GDROM: Doing nothing for GETSCD [0]={}, [1]={}, [2]=0x{:08X}, [3]=0x{:08X}", s, n, b, u);
    }

    /// Execute a GD-ROM command
    fn gd_hle_command(
        &self,
        mem: &mut dyn ReiosSh4Memory,
        ctx: &dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
        cc: u32,
        _prm: u32,
    ) {
        match cc {
            GDCC_GETTOC => {
                println!("GDROM:\t*FIXME* CMD GETTOC CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_GETTOC2 => {
                self.gdrom_hle_read_toc(mem, disc, ctx.get_r(5));
            }
            GDCC_GETSES => {
                println!("GDROM:\tGETSES CC:{:X} PRM:{:X}", cc, _prm);
                self.gdrom_hle_read_ses(mem, ctx.get_r(5));
            }
            GDCC_INIT => {
                println!("GDROM:\tCMD INIT CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_PIOREAD => {
                self.gdrom_hle_read_pio(mem, disc, ctx.get_r(5));
            }
            GDCC_DMAREAD => {
                println!("GDROM:\tCMD DMAREAD CC:{:X} PRM:{:X}", cc, _prm);
                self.gdrom_hle_read_dma(mem, disc, ctx.get_r(5));
            }
            GDCC_PLAY_SECTOR => {
                println!("GDROM:\tCMD PLAYSEC? CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_RELEASE => {
                println!("GDROM:\tCMD RELEASE? CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_STOP => {
                println!("GDROM:\tCMD STOP CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_SEEK => {
                println!("GDROM:\tCMD SEEK CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_PLAY => {
                println!("GDROM:\tCMD PLAY CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_PAUSE => {
                println!("GDROM:\tCMD PAUSE CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_READ => {
                println!("GDROM:\tCMD READ CC:{:X} PRM:{:X}", cc, _prm);
            }
            GDCC_GETSCD => {
                println!("GDROM:\tGETSCD CC:{:X} PRM:{:X}", cc, _prm);
                self.gdcc_hle_getscd(mem, ctx.get_r(5));
            }
            _ => {
                println!("GDROM:\tUnknown GDROM CC:{:X} PRM:{:X}", cc, _prm);
            }
        }
    }

    /// Main GD-ROM HLE operation handler
    pub fn gdrom_hle_op(
        &mut self,
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
    ) {
        let r6 = ctx.get_r(6);
        let r7 = ctx.get_r(7);

        if r6 == SYSCALL_GDROM {
            // GDROM SYSCALL
            match r7 {
                GDROM_SEND_COMMAND => {
                    // SEND GDROM COMMAND
                    let r4 = ctx.get_r(4);
                    let r5 = ctx.get_r(5);
                    println!("\nGDROM:\tHLE SEND COMMAND CC:{:X}  param ptr: {:X}", r4, r5);

                    self.gd_hle_command(mem, ctx, disc, r4, r5);
                    self.last_cmd = self.dw_req_id;
                    self.dw_req_id = self.dw_req_id.wrapping_sub(1);
                    ctx.set_r(0, self.last_cmd); // RET Request ID
                }
                GDROM_CHECK_COMMAND => {
                    // CHECK COMMAND
                    let r4 = ctx.get_r(4);
                    let r5 = ctx.get_r(5);
                    let result = if self.last_cmd == r4 { 2 } else { 0 }; // Finished : Invalid
                    println!("\nGDROM:\tHLE CHECK COMMAND REQID:{:X}  param ptr: {:X} -> {:X}", r4, r5, result);

                    ctx.set_r(0, result);
                    self.last_cmd = 0xFFFFFFFF; // INVALIDATE CHECK CMD
                }
                GDROM_MAIN => {
                    println!("\nGDROM:\tHLE GDROM_MAIN");
                    // No operation
                }
                GDROM_INIT => {
                    println!("\nGDROM:\tHLE GDROM_INIT");
                }
                GDROM_RESET => {
                    println!("\nGDROM:\tHLE GDROM_RESET");
                }
                GDROM_CHECK_DRIVE => {
                    let r4 = ctx.get_r(4);
                    println!("\nGDROM:\tHLE GDROM_CHECK_DRIVE r4:{:X}", r4);

                    mem.write_mem32(r4 + 0, 0x02); // STANDBY
                    mem.write_mem32(r4 + 4, disc.get_disc_type());
                    ctx.set_r(0, 0); // RET SUCCESS
                }
                GDROM_ABORT_COMMAND => {
                    let r4 = ctx.get_r(4);
                    println!("\nGDROM:\tHLE GDROM_ABORT_COMMAND r4:{:X}", r4);
                    ctx.set_r(0, 0xFFFFFFFF); // RET FAILURE (-1)
                }
                GDROM_SECTOR_MODE => {
                    let r4 = ctx.get_r(4);
                    println!("GDROM:\tHLE GDROM_SECTOR_MODE PTR_r4:{:X}", r4);

                    for i in 0..4 {
                        self.sec_mode[i] = mem.read_mem32(r4 + (i as u32 * 4));
                        print!("{:08X}", self.sec_mode[i]);
                        if i == 3 {
                            println!();
                        } else {
                            print!("\t");
                        }
                    }
                    ctx.set_r(0, 0); // RET SUCCESS
                }
                _ => {
                    println!("\nGDROM:\tUnknown SYSCALL: {:X}", r7);
                }
            }
        } else {
            // MISC
            println!("SYSCALL:\tSYSCALL: {:X}", r7);
        }
    }
}

impl Default for GdromHleState {
    fn default() -> Self {
        Self::new()
    }
}

// Global state for compatibility (can be removed when integrating into Dreamcast struct)
use std::sync::Mutex;
use once_cell::sync::Lazy;

static GDROM_HLE_STATE: Lazy<Mutex<GdromHleState>> = Lazy::new(|| Mutex::new(GdromHleState::new()));

/// Global function for GDROM HLE operation
pub fn gdrom_hle_op(
    mem: &mut dyn ReiosSh4Memory,
    ctx: &mut dyn ReiosSh4Context,
    disc: &dyn ReiosDisc,
) {
    let mut state = GDROM_HLE_STATE.lock().unwrap();
    state.gdrom_hle_op(mem, ctx, disc);
}
