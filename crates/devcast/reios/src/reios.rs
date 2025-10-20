/*
    REIOS - Extremely primitive BIOS replacement for Dreamcast
    Ported from reference/devcast/libswirl/reios/reios.cpp

    Many thanks to Lars Olsson (jlo@ludd.luth.se) for BIOS decompile work:
    - http://www.ludd.luth.se/~jlo/dc/bootROM.c
    - http://www.ludd.luth.se/~jlo/dc/bootROM.h
    - http://www.ludd.luth.se/~jlo/dc/security_stuff.c
*/

use std::collections::HashMap;
use crate::traits::{ReiosSh4Memory, ReiosSh4Context, ReiosDisc};

// REIOS trap opcode
pub const REIOS_OPCODE: u16 = 0x085B;

// BIOS syscall addresses
const DC_BIOS_SYSCALL_SYSTEM: u32 = 0x8C0000B0;
const DC_BIOS_SYSCALL_FONT: u32 = 0x8C0000B4;
const DC_BIOS_SYSCALL_FLASHROM: u32 = 0x8C0000B8;
const DC_BIOS_SYSCALL_GD: u32 = 0x8C0000BC;
const DC_BIOS_SYSCALL_MISC: u32 = 0x8C0000E0;

// GD-ROM bioscall entrypoint
const DC_BIOS_ENTRYPOINT_GD_DO_BIOSCALL: u32 = 0x8C0010F0;

// System info ID address
const SYSINFO_ID_ADDR: u32 = 0x8C001010;

// Flash ROM partition info (offset, size)
const FLASHROM_INFO: [(u32, u32); 5] = [
    (0 * 1024, 8 * 1024),
    (8 * 1024, 8 * 1024),
    (16 * 1024, 16 * 1024),
    (32 * 1024, 32 * 1024),
    (64 * 1024, 64 * 1024),
];

/// IP.BIN metadata extracted from disc
#[derive(Debug, Clone)]
pub struct IpBinMetadata {
    pub hardware_id: String,
    pub maker_id: String,
    pub device_info: String,
    pub area_symbols: String,
    pub peripherals: String,
    pub product_number: String,
    pub product_version: String,
    pub release_date: String,
    pub boot_filename: String,
    pub software_company: String,
    pub software_name: String,
    pub windows_ce: bool,
}

impl Default for IpBinMetadata {
    fn default() -> Self {
        Self {
            hardware_id: String::new(),
            maker_id: String::new(),
            device_info: String::new(),
            area_symbols: String::new(),
            peripherals: String::new(),
            product_number: String::new(),
            product_version: String::new(),
            release_date: String::new(),
            boot_filename: String::new(),
            software_company: String::new(),
            software_name: String::new(),
            windows_ce: false,
        }
    }
}

/// REIOS context - holds state for one REIOS instance
pub struct ReiosContext {
    /// Base FAD for disc operations
    base_fad: u32,
    /// Whether to descramble boot file
    descrambl: bool,
    /// Whether bootfile has been initialized
    bootfile_inited: bool,
    /// Pre-init completed
    pre_init: bool,
    /// IP.BIN metadata
    pub metadata: IpBinMetadata,
    /// Hook function registry (PC address -> hook ID)
    hooks: HashMap<u32, usize>,
    /// Reverse hook registry (hook ID -> PC address)
    hooks_rev: HashMap<usize, u32>,
    /// Stored memory context (implements ReiosSh4Memory)
    mem_ptr: Option<*mut dyn ReiosSh4Memory>,
    /// Stored SH4 context (implements ReiosSh4Context)
    ctx_ptr: Option<*mut dyn ReiosSh4Context>,
    /// Stored disc (implements ReiosDisc)
    disc_ptr: Option<*const dyn ReiosDisc>,
}

impl ReiosContext {
    /// Create a new REIOS context with runtime trait object pointers
    /// SAFETY: Caller must ensure pointers remain valid for the lifetime of ReiosContext
    pub unsafe fn new(
        mem: *mut dyn ReiosSh4Memory,
        ctx: *mut dyn ReiosSh4Context,
        disc: *const dyn ReiosDisc,
    ) -> Self {
        Self {
            base_fad: 45150,
            descrambl: false,
            bootfile_inited: false,
            pre_init: false,
            metadata: IpBinMetadata::default(),
            hooks: HashMap::new(),
            hooks_rev: HashMap::new(),
            mem_ptr: Some(mem),
            ctx_ptr: Some(ctx),
            disc_ptr: Some(disc),
        }
    }

    /// Initialize REIOS
    pub fn init(&mut self, mem: &mut dyn ReiosSh4Memory) -> bool {
        println!("reios: Init");

        // Clear BIOS RAM area
        let zeros = vec![0xFFu8; 64 * 1024];
        mem.write_mem_block(0x8C000000, &zeros);

        // Write REIOS_OPCODE at BIOS entry point
        mem.write_mem16(0xA0000000, REIOS_OPCODE);

        // Register hooks at specific addresses
        self.register_hook(0xA0000000, 0); // reios_boot
        self.register_hook(0x8C001000, 1); // reios_sys_system
        self.register_hook(0x8C001002, 2); // reios_sys_font
        self.register_hook(0x8C001004, 3); // reios_sys_flashrom
        self.register_hook(0x8C001006, 4); // reios_sys_gd
        self.register_hook(0x8C001008, 5); // reios_sys_misc
        self.register_hook(0x8C00043C, 6); // reios_exit
        self.register_hook(DC_BIOS_ENTRYPOINT_GD_DO_BIOSCALL, 7); // gd_do_bioscall

        true
    }

    /// Reset REIOS state
    pub fn reset(&mut self) {
        self.pre_init = false;
        self.bootfile_inited = false;
    }

    /// Register a hook at a specific PC address
    fn register_hook(&mut self, pc: u32, hook_id: usize) {
        let mapped = syscall_addr_map(pc);
        self.hooks.insert(mapped, hook_id);
        self.hooks_rev.insert(hook_id, pc);
    }

    /// Get hook address by ID
    fn hook_addr(&self, hook_id: usize) -> Option<u32> {
        self.hooks_rev.get(&hook_id).copied()
    }

    /// Pre-initialization - determine disc type and base FAD
    fn pre_init(&mut self, disc: &dyn ReiosDisc) {
        if self.pre_init {
            return;
        }

        // Check disc type
        let disc_type = disc.get_disc_type();
        if disc_type == 0x80 { // GDROM
            self.base_fad = 45150;
            self.descrambl = false;
        } else { // CDROM
            let mut ses = [0u8; 6];
            disc.get_session_info(&mut ses, 0);
            let session = ses[2];
            disc.get_session_info(&mut ses, session);
            self.base_fad = ((ses[3] as u32) << 16) | ((ses[4] as u32) << 8) | (ses[5] as u32);
            self.descrambl = true;
        }

        println!("reios: Pre-init - base_fad={}, descrambl={}", self.base_fad, self.descrambl);
        self.pre_init = true;
    }

    /// Read IP.BIN metadata from disc
    pub fn disk_id(&mut self, mem: &mut dyn ReiosSh4Memory, disc: &dyn ReiosDisc) -> &str {
        if !self.pre_init {
            self.pre_init(disc);
        }

        // Read IP.BIN sector to 0x8c008000
        let mut ip_bin = vec![0u8; 256];
        disc.read_sector(&mut ip_bin, self.base_fad, 1, 2048);

        // Also write to memory
        mem.write_mem_block(0x8c008000, &ip_bin);

        // Parse metadata
        self.metadata.hardware_id = String::from_utf8_lossy(&ip_bin[0..16]).trim_end().to_string();
        self.metadata.maker_id = String::from_utf8_lossy(&ip_bin[16..32]).trim_end().to_string();
        self.metadata.device_info = String::from_utf8_lossy(&ip_bin[32..48]).trim_end().to_string();
        self.metadata.area_symbols = String::from_utf8_lossy(&ip_bin[48..56]).trim_end().to_string();
        self.metadata.peripherals = String::from_utf8_lossy(&ip_bin[56..64]).trim_end().to_string();
        self.metadata.product_number = String::from_utf8_lossy(&ip_bin[64..74]).trim_end().to_string();
        self.metadata.product_version = String::from_utf8_lossy(&ip_bin[74..80]).trim_end().to_string();
        self.metadata.release_date = String::from_utf8_lossy(&ip_bin[80..96]).trim_end().to_string();
        self.metadata.boot_filename = String::from_utf8_lossy(&ip_bin[96..112]).trim_end().to_string();
        self.metadata.software_company = String::from_utf8_lossy(&ip_bin[112..128]).trim_end().to_string();
        self.metadata.software_name = String::from_utf8_lossy(&ip_bin[128..256]).trim_end().to_string();
        self.metadata.windows_ce = self.metadata.boot_filename.starts_with("0WINCEOS.BIN");

        &self.metadata.product_number
    }

    /// Locate bootfile on disc
    fn locate_bootfile(&mut self, mem: &mut dyn ReiosSh4Memory, disc: &dyn ReiosDisc, bootfile: &str) -> bool {
        let data_len = 2048 * 1024;
        let mut temp = vec![0u8; data_len];

        // Read first sector (PVD check)
        let mut pvd = vec![0u8; 2048];
        disc.read_sector(&mut pvd, self.base_fad + 16, 1, 2048);

        let actual_data_len;
        if &pvd[1..8] == b"\x01CD001\x01" {
            println!("reios: iso9660 PVD found");
            let lba = read_u32bi(&pvd[156..164]);
            let len = read_u32bi(&pvd[156 + 8..164 + 8]);

            actual_data_len = ((len + 2047) / 2048) * 2048;
            println!("reios: iso9660 root_directory, FAD: {}, len: {}", 150 + lba, actual_data_len);

            let sectors = (actual_data_len.min(data_len as u32) / 2048) as u32;
            disc.read_sector(&mut temp[..sectors as usize * 2048], 150 + lba, sectors, 2048);
        } else {
            actual_data_len = data_len as u32;
            let sectors = (data_len / 2048) as u32;
            disc.read_sector(&mut temp, self.base_fad + 16, sectors, 2048);
        }

        // Search for bootfile
        let bootfile_bytes = bootfile.as_bytes();
        for i in 0..(actual_data_len as usize - 20) {
            if temp[i..].starts_with(bootfile_bytes) {
                println!("Found {} at {:06X}", bootfile, i);

                // Read file location info (ISO9660 directory entry)
                if i < 33 {
                    continue;
                }

                let lba = read_u32bi(&temp[i - 33 + 2..i - 33 + 10]);
                let len = read_u32bi(&temp[i - 33 + 10..i - 33 + 18]);

                println!("file LBA: {}", lba);
                println!("file LEN: {}", len);

                // Load file to 0x8c010000
                if self.descrambl {
                    // Read to temp buffer first
                    let file_sectors = ((len + 2047) / 2048) as u32;
                    let mut temp_file = vec![0u8; (file_sectors * 2048) as usize];
                    disc.read_sector(&mut temp_file, lba + 150, file_sectors, 2048);

                    // Descramble
                    let mut dst = vec![0u8; len as usize];
                    crate::descrambl::descrambl_buffer(&temp_file, &mut dst, len as usize);
                    mem.write_mem_block(0x8c010000, &dst);
                } else {
                    // Direct read
                    let file_sectors = ((len + 2047) / 2048) as u32;
                    let mut file_data = vec![0u8; (file_sectors * 2048) as usize];
                    disc.read_sector(&mut file_data, lba + 150, file_sectors, 2048);
                    mem.write_mem_block(0x8c010000, &file_data[..len as usize]);
                }

                self.bootfile_inited = true;
                return true;
            }
        }

        false
    }

    /// Setup system state for Dreamcast boot
    fn setup_state(&self, ctx: &mut dyn ReiosSh4Context, boot_addr: u32) {
        // Setup registers to imitate normal BIOS boot
        ctx.set_r(15, 0x8d000000); // Stack pointer

        ctx.set_gbr(0x8c000000);
        ctx.set_ssr(0x40000001);
        ctx.set_spc(0x8c000776);
        ctx.set_sgr(0x8d000000);
        ctx.set_dbr(0x8c000010);
        ctx.set_vbr(0x8c000000);
        ctx.set_pr(0xac00043c);
        ctx.set_fpul(0x00000000);
        ctx.set_current_pc(boot_addr);
        ctx.set_next_pc(boot_addr.wrapping_add(2));

        ctx.set_sr_status(0x400000f0);
        ctx.set_sr_t(true);

        ctx.set_fpscr_full(0x00040001);
    }

    /// Setup Naomi-specific state
    #[allow(dead_code)]
    fn setup_naomi(&self, ctx: &mut dyn ReiosSh4Context, boot_addr: u32) {
        // Setup registers for Naomi boot
        ctx.set_r(0, 0x0c021000);
        ctx.set_r(1, 0x0c01f820);
        ctx.set_r(2, 0xa0710004);
        ctx.set_r(3, 0x0c01f130);
        ctx.set_r(4, 0x5bfccd08);
        ctx.set_r(5, 0xa05f7000);
        ctx.set_r(6, 0xa05f7008);
        ctx.set_r(7, 0x00000007);
        ctx.set_r(8, 0x00000000);
        ctx.set_r(9, 0x00002000);
        ctx.set_r(10, 0xffffffff);
        ctx.set_r(11, 0x0c0e0000);
        ctx.set_r(12, 0x00000000);
        ctx.set_r(13, 0x00000000);
        ctx.set_r(14, 0x00000000);
        ctx.set_r(15, 0x0cc00000);

        ctx.set_gbr(0x0c2abcc0);
        ctx.set_ssr(0x60000000);
        ctx.set_spc(0x0c041738);
        ctx.set_sgr(0x0cbfffb0);
        ctx.set_dbr(0x00000fff);
        ctx.set_vbr(0x0c000000);
        ctx.set_pr(0xac0195ee);
        ctx.set_fpul(0x000001e0);
        ctx.set_current_pc(boot_addr);
        ctx.set_next_pc(boot_addr.wrapping_add(2));

        ctx.set_sr_status(0x60000000);
        ctx.set_sr_t(true);

        ctx.set_fpscr_full(0x00040001);
    }

    /// Boot the system
    pub fn boot(
        &mut self,
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
    ) {
        println!("-----------------");
        println!("REIOS: Booting up");
        println!("-----------------");

        // Clear memory at 0x8C000000
        let zeros = vec![0xFFu8; 64 * 1024];
        mem.write_mem_block(0x8C000000, &zeros);

        // Setup syscalls
        Self::setup_syscall(mem, self.hook_addr(1).unwrap(), DC_BIOS_SYSCALL_SYSTEM);
        Self::setup_syscall(mem, self.hook_addr(2).unwrap(), DC_BIOS_SYSCALL_FONT);
        Self::setup_syscall(mem, self.hook_addr(3).unwrap(), DC_BIOS_SYSCALL_FLASHROM);
        Self::setup_syscall(mem, self.hook_addr(4).unwrap(), DC_BIOS_SYSCALL_GD);
        Self::setup_syscall(mem, self.hook_addr(5).unwrap(), DC_BIOS_SYSCALL_MISC);

        mem.write_mem16(self.hook_addr(6).unwrap(), REIOS_OPCODE); // exit

        mem.write_mem32(DC_BIOS_ENTRYPOINT_GD_DO_BIOSCALL, REIOS_OPCODE as u32);

        // Infinite loop for ARM (0xEAFFFFFE = b .)
        mem.write_mem32(0x80800000, 0xEAFFFFFE);

        // Setup some hardware register
        mem.write_mem32(0xffa00000 + 0x40, 0x8001);

        // Try to locate and load bootfile
        if !self.bootfile_inited {
            if !self.locate_bootfile(mem, disc, "1ST_READ.BIN") {
                println!("reios: Failed to locate bootfile");
            }
        }

        // Setup boot state
        if self.bootfile_inited {
            self.setup_state(ctx, 0xac008300);
        } else {
            // Fallback boot address
            self.setup_state(ctx, 0x8c010000);
        }
    }

    fn setup_syscall(mem: &mut dyn ReiosSh4Memory, hook_addr: u32, syscall_addr: u32) {
        mem.write_mem32(syscall_addr, hook_addr);
        mem.write_mem16(hook_addr, REIOS_OPCODE);
        println!("reios: Patching syscall vector {:08X}, points to {:08X}", syscall_addr, hook_addr);
    }

    /// Handle REIOS trap/syscall using stored context pointers
    /// This method directly uses the stored pointers
    /// SAFETY: Must only be called after set_context_ptrs with valid pointers
    pub unsafe fn trap_self(&mut self, op: u16, pc: u32) {
        assert_eq!(op, REIOS_OPCODE, "Invalid REIOS opcode");

        // Ensure pointers were set
        if self.mem_ptr.is_none() || self.ctx_ptr.is_none() || self.disc_ptr.is_none() {
            println!("reios: trap_self called before set_context_ptrs");
            return;
        }

        // Trap handling logic (same as trap() but using stored pointers)
        let pc_adjusted = pc;
        let mapped = syscall_addr_map(pc_adjusted);

        // Return to PR using the stored ctx pointer
        {
            let ctx = &mut *self.ctx_ptr.unwrap();
            ctx.set_next_pc(ctx.get_pr());
        }

        // Dispatch to hook handler
        if let Some(&hook_id) = self.hooks.get(&mapped) {
            // Use raw pointer to split the borrows
            let self_ptr = self as *mut Self;
            let mem = &mut *self.mem_ptr.unwrap();
            let ctx = &mut *self.ctx_ptr.unwrap();
            let disc = &*self.disc_ptr.unwrap();

            // Inline dispatch - call directly on self_ptr
            match hook_id {
                0 => (*self_ptr).boot(mem, ctx, disc),
                1 => Self::sys_system(mem, ctx),
                2 => Self::sys_font(),
                3 => Self::sys_flashrom(mem, ctx),
                4 => Self::sys_gd(mem, ctx, disc),
                5 => Self::sys_misc(ctx),
                6 => Self::sys_exit(ctx),
                7 => Self::gd_do_bioscall(mem, ctx, disc),
                _ => println!("reios: Unknown hook ID {}", hook_id),
            }
        } else {
            println!("reios: Unknown hook at PC=0x{:08X}", pc_adjusted);
        }
    }

    /// Dispatch to appropriate hook handler (using dynamic trait objects)
    fn dispatch_hook_dyn(
        &mut self,
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
        hook_id: usize,
    ) {
        match hook_id {
            0 => self.boot(mem, ctx, disc),
            1 => Self::sys_system(mem, ctx),
            2 => Self::sys_font(),
            3 => Self::sys_flashrom(mem, ctx),
            4 => Self::sys_gd(mem, ctx, disc),
            5 => Self::sys_misc(ctx),
            6 => Self::sys_exit(ctx),
            7 => Self::gd_do_bioscall(mem, ctx, disc),
            _ => println!("reios: Unknown hook ID {}", hook_id),
        }
    }

    /// Dispatch to appropriate hook handler
    fn dispatch_hook(
        &mut self,
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
        hook_id: usize,
    ) {
        match hook_id {
            0 => self.boot(mem, ctx, disc),
            1 => Self::sys_system(mem, ctx),
            2 => Self::sys_font(),
            3 => Self::sys_flashrom(mem, ctx),
            4 => Self::sys_gd(mem, ctx, disc),
            5 => Self::sys_misc(ctx),
            6 => Self::sys_exit(ctx),
            7 => Self::gd_do_bioscall(mem, ctx, disc),
            _ => println!("reios: Unknown hook ID {}", hook_id),
        }
    }

    /// System syscall handler
    fn sys_system(mem: &mut dyn ReiosSh4Memory, ctx: &mut dyn ReiosSh4Context) {
        let cmd = ctx.get_r(7);

        match cmd {
            0 => {
                // SYSINFO_INIT
                ctx.set_r(0, 0);
            }
            2 => {
                // SYSINFO_ICON
                println!("SYSINFO_ICON");
                ctx.set_r(0, 704);
            }
            3 => {
                // SYSINFO_ID
                mem.write_mem32(SYSINFO_ID_ADDR + 0, 0xe1e2e3e4);
                mem.write_mem32(SYSINFO_ID_ADDR + 4, 0xe5e6e7e8);
                ctx.set_r(0, SYSINFO_ID_ADDR);
            }
            _ => {
                println!("unhandled: reios_sys_system cmd={}", cmd);
            }
        }
    }

    /// Font syscall handler
    fn sys_font() {
        println!("reios_sys_font");
    }

    /// Flash ROM syscall handler
    fn sys_flashrom(mem: &mut dyn ReiosSh4Memory, ctx: &mut dyn ReiosSh4Context) {
        let cmd = ctx.get_r(7);

        match cmd {
            0 => {
                // FLASHROM_INFO
                let part = ctx.get_r(4) as usize;
                let dest = ctx.get_r(5);

                if part <= 4 {
                    mem.write_mem32(dest + 0, FLASHROM_INFO[part].0);
                    mem.write_mem32(dest + 4, FLASHROM_INFO[part].1);
                    ctx.set_r(0, 0); // SUCCESS
                } else {
                    ctx.set_r(0, 0xFFFFFFFF); // -1 FAILURE
                }
            }
            1 => {
                // FLASHROM_READ
                let offs = ctx.get_r(4);
                let dest = ctx.get_r(5);
                let size = ctx.get_r(6);

                // Read from flash ROM (stub - would need flash ROM pointer)
                // For now, just return zeros
                println!("reios_sys_flashrom: READ stub - offs={:08X}, dest={:08X}, size={}", offs, dest, size);
                ctx.set_r(0, size);
            }
            2 => {
                // FLASHROM_WRITE
                println!("reios_sys_flashrom: WRITE stub");
                // Stub - would need flash ROM pointer
            }
            3 => {
                // FLASHROM_DELETE
                println!("reios_sys_flashrom: DELETE stub");
                // Stub - would need flash ROM pointer
            }
            _ => {
                println!("reios_sys_flashrom: not handled, {}", cmd);
            }
        }
    }

    /// GD-ROM syscall handler
    fn sys_gd(
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
    ) {
        crate::gdrom_hle::gdrom_hle_op(mem, ctx, disc);
    }

    /// Misc syscall handler
    fn sys_misc(ctx: &mut dyn ReiosSh4Context) {
        println!(
            "reios_sys_misc - r7: 0x{:08X}, r4 0x{:08X}, r5 0x{:08X}, r6 0x{:08X}",
            ctx.get_r(7),
            ctx.get_r(4),
            ctx.get_r(5),
            ctx.get_r(6)
        );
        ctx.set_r(0, 0);
    }

    /// Exit handler
    fn sys_exit(ctx: &mut dyn ReiosSh4Context) {
        if ctx.is_running() {
            println!("-----------------");
            println!("REIOS: Exit");
            println!("-----------------");
            ctx.stop();
        }
    }

    /// GD-ROM bioscall handler
    fn gd_do_bioscall(
        mem: &mut dyn ReiosSh4Memory,
        ctx: &mut dyn ReiosSh4Context,
        disc: &dyn ReiosDisc,
    ) {
        crate::gdrom_hle::gdrom_hle_op(mem, ctx, disc);
    }
}

/// Map syscall address (mask to common address space)
#[inline]
fn syscall_addr_map(addr: u32) -> u32 {
    (addr & 0x1FFFFFFF) | 0x80000000
}

/// Read 32-bit bi-endian integer (big-endian bytes as used by DC BIOS)
#[inline]
fn read_u32bi(ptr: &[u8]) -> u32 {
    assert!(ptr.len() >= 8);
    ((ptr[4] as u32) << 24)
        | ((ptr[5] as u32) << 16)
        | ((ptr[6] as u32) << 8)
        | (ptr[7] as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_addr_map() {
        assert_eq!(syscall_addr_map(0xA0000000), 0xA0000000);
        assert_eq!(syscall_addr_map(0x8C000000), 0x8C000000);
    }

    #[test]
    fn test_read_u32bi() {
        let data = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        assert_eq!(read_u32bi(&data), 0x04050607);
    }

    #[test]
    fn test_reios_context_creation() {
        let ctx = ReiosContext::new();
        assert_eq!(ctx.base_fad, 45150);
        assert!(!ctx.descrambl);
    }
}
