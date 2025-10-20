/*
    REIOS integration for Dreamcast emulator
    Implements the trait interfaces required by the reios crate
*/

use crate::Dreamcast;
use sh4_core::Sh4Ctx;

// Dummy disc implementation for ELF loading (no disc inserted)
pub(crate) struct DummyDisc;

// Static instance for use with REIOS trap handling
pub(crate) static mut DUMMY_DISC_INSTANCE: DummyDisc = DummyDisc;

impl reios::ReiosDisc for DummyDisc {
    fn read_sector(&self, _buffer: &mut [u8], _fad: u32, _sector_count: u32, _sector_size: u32) -> bool {
        // No disc - fill with zeros
        false
    }

    fn get_toc(&self, buffer: &mut [u32], _session: u32) {
        // Empty TOC
        for entry in buffer.iter_mut() {
            *entry = 0;
        }
    }

    fn get_session_info(&self, buffer: &mut [u8], _session: u8) {
        // No session info
        for byte in buffer.iter_mut() {
            *byte = 0;
        }
    }

    fn get_disc_type(&self) -> u32 {
        0x80 // GDROM type
    }
}

// Wrapper for Sh4Ctx to implement ReiosSh4Context and ReiosSh4Memory
pub(crate) struct Sh4ContextWrapper<'a> {
    pub ctx: &'a mut Sh4Ctx,
    pub running: &'a mut bool,
    pub dreamcast: &'a mut Dreamcast,
}

impl<'a> reios::ReiosSh4Context for Sh4ContextWrapper<'a> {
    fn get_r(&self, reg: usize) -> u32 {
        self.ctx.r[reg]
    }

    fn set_r(&mut self, reg: usize, value: u32) {
        self.ctx.r[reg] = value;
    }

    fn get_pc(&self) -> u32 {
        self.ctx.pc0
    }

    fn set_current_pc(&mut self, value: u32) {
        self.ctx.pc0 = value;
    }
    
    fn set_next_pc(&mut self, value: u32) {
        self.ctx.pc1 = value.wrapping_add(0);
        self.ctx.pc2 = value.wrapping_add(2);
    }

    fn get_pr(&self) -> u32 {
        self.ctx.pr
    }

    fn set_pr(&mut self, value: u32) {
        self.ctx.pr = value;
    }

    fn get_gbr(&self) -> u32 {
        self.ctx.gbr
    }

    fn set_gbr(&mut self, value: u32) {
        self.ctx.gbr = value;
    }

    fn get_vbr(&self) -> u32 {
        self.ctx.vbr
    }

    fn set_vbr(&mut self, value: u32) {
        self.ctx.vbr = value;
    }

    fn get_ssr(&self) -> u32 {
        self.ctx.ssr
    }

    fn set_ssr(&mut self, value: u32) {
        self.ctx.ssr = value;
    }

    fn get_spc(&self) -> u32 {
        self.ctx.spc
    }

    fn set_spc(&mut self, value: u32) {
        self.ctx.spc = value;
    }

    fn get_sgr(&self) -> u32 {
        self.ctx.sgr
    }

    fn set_sgr(&mut self, value: u32) {
        self.ctx.sgr = value;
    }

    fn get_dbr(&self) -> u32 {
        self.ctx.dbr
    }

    fn set_dbr(&mut self, value: u32) {
        self.ctx.dbr = value;
    }

    fn get_fpul(&self) -> u32 {
        self.ctx.fpul
    }

    fn set_fpul(&mut self, value: u32) {
        self.ctx.fpul = value;
    }

    fn get_sr_status(&self) -> u32 {
        self.ctx.sr.full()
    }

    fn set_sr_status(&mut self, value: u32) {
        self.ctx.sr.set_full(value);
    }

    fn get_sr_t(&self) -> bool {
        self.ctx.sr_t != 0
    }

    fn set_sr_t(&mut self, value: bool) {
        self.ctx.sr_t = if value { 1 } else { 0 };
    }

    fn get_fpscr_full(&self) -> u32 {
        self.ctx.fpscr.full()
    }

    fn set_fpscr_full(&mut self, value: u32) {
        self.ctx.fpscr.set_full(value);
    }

    fn stop(&mut self) {
        *self.running = false;
    }

    fn is_running(&self) -> bool {
        *self.running
    }
}

// Implement ReiosSh4Memory for Sh4ContextWrapper
impl<'a> reios::ReiosSh4Memory for Sh4ContextWrapper<'a> {
    fn read_mem32(&self, addr: u32) -> u32 {
        let mut value: u32 = 0;
        sh4_core::sh4mem::read_mem(self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem32(&mut self, addr: u32, value: u32) {
        sh4_core::sh4mem::write_mem(self.ctx as *mut _, addr, value);
    }

    fn read_mem16(&self, addr: u32) -> u16 {
        let mut value: u16 = 0;
        sh4_core::sh4mem::read_mem(self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem16(&mut self, addr: u32, value: u16) {
        sh4_core::sh4mem::write_mem(self.ctx as *mut _, addr, value);
    }

    fn read_mem8(&self, addr: u32) -> u8 {
        let mut value: u8 = 0;
        sh4_core::sh4mem::read_mem(self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem8(&mut self, addr: u32, value: u8) {
        sh4_core::sh4mem::write_mem(self.ctx as *mut _, addr, value);
    }

    fn get_mem_ptr(&mut self, addr: u32, size: u32) -> Option<*mut u8> {
        // Try to get direct pointer to memory regions
        let addr_masked = addr & 0x1FFFFFFF;

        // System RAM: 0x0C000000-0x0CFFFFFF (16MB)
        if addr_masked >= 0x0C000000 && addr_masked < 0x0D000000 {
            let offset = (addr_masked - 0x0C000000) as usize;
            if offset + size as usize <= self.dreamcast.sys_ram.len() {
                return Some(self.dreamcast.sys_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        // VRAM: 0x04000000-0x047FFFFF (8MB)
        if addr_masked >= 0x04000000 && addr_masked < 0x04800000 {
            let offset = (addr_masked - 0x04000000) as usize;
            if offset + size as usize <= self.dreamcast.video_ram.len() {
                return Some(self.dreamcast.video_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        // Audio RAM: 0x00800000-0x009FFFFF (2MB)
        if addr_masked >= 0x00800000 && addr_masked < 0x00A00000 {
            let offset = (addr_masked - 0x00800000) as usize;
            if offset + size as usize <= self.dreamcast.audio_ram.len() {
                return Some(self.dreamcast.audio_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        None
    }

    fn read_mem_block(&self, addr: u32, data: &mut [u8]) {
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = self.read_mem8(addr + i as u32) as u8;
        }
    }

    fn write_mem_block(&mut self, addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write_mem8(addr + i as u32, byte);
        }
    }
}

// Implement ReiosSh4Memory for Dreamcast
impl reios::ReiosSh4Memory for Dreamcast {
    fn read_mem32(&self, addr: u32) -> u32 {
        let mut value: u32 = 0;
        sh4_core::sh4mem::read_mem(&self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem32(&mut self, addr: u32, value: u32) {
        sh4_core::sh4mem::write_mem(&mut self.ctx as *mut _, addr, value);
    }

    fn read_mem16(&self, addr: u32) -> u16 {
        let mut value: u16 = 0;
        sh4_core::sh4mem::read_mem(&self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem16(&mut self, addr: u32, value: u16) {
        sh4_core::sh4mem::write_mem(&mut self.ctx as *mut _, addr, value);
    }

    fn read_mem8(&self, addr: u32) -> u8 {
        let mut value: u8 = 0;
        sh4_core::sh4mem::read_mem(&self.ctx as *const _ as *mut _, addr, &mut value);
        value
    }

    fn write_mem8(&mut self, addr: u32, value: u8) {
        sh4_core::sh4mem::write_mem(&mut self.ctx as *mut _, addr, value);
    }

    fn get_mem_ptr(&mut self, addr: u32, size: u32) -> Option<*mut u8> {
        // Try to get direct pointer to memory regions
        let addr_masked = addr & 0x1FFFFFFF;

        // System RAM: 0x0C000000-0x0CFFFFFF (16MB)
        if addr_masked >= 0x0C000000 && addr_masked < 0x0D000000 {
            let offset = (addr_masked - 0x0C000000) as usize;
            if offset + size as usize <= self.sys_ram.len() {
                return Some(self.sys_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        // VRAM: 0x04000000-0x047FFFFF (8MB)
        if addr_masked >= 0x04000000 && addr_masked < 0x04800000 {
            let offset = (addr_masked - 0x04000000) as usize;
            if offset + size as usize <= self.video_ram.len() {
                return Some(self.video_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        // Audio RAM: 0x00800000-0x009FFFFF (2MB)
        if addr_masked >= 0x00800000 && addr_masked < 0x00A00000 {
            let offset = (addr_masked - 0x00800000) as usize;
            if offset + size as usize <= self.audio_ram.len() {
                return Some(self.audio_ram.as_mut_ptr().wrapping_add(offset));
            }
        }

        None
    }

    fn read_mem_block(&self, addr: u32, data: &mut [u8]) {
        for (i, byte) in data.iter_mut().enumerate() {
            sh4_core::sh4mem::read_mem(
                &self.ctx as *const _ as *mut _,
                addr + i as u32,
                byte
            );
        }
    }

    fn write_mem_block(&mut self, addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            sh4_core::sh4mem::write_mem(
                &mut self.ctx as *mut _,
                addr + i as u32,
                byte
            );
        }
    }
}

impl Dreamcast {
    /// Load an ELF file from disk
    fn load_elf(&mut self, elf_path: &str) -> Result<(), String> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(elf_path)
            .map_err(|e| format!("Failed to open ELF file: {}", e))?;

        let mut elf_bytes = Vec::new();
        file.read_to_end(&mut elf_bytes)
            .map_err(|e| format!("Failed to read ELF file: {}", e))?;

        // Use existing init_dreamcast_with_elf function
        crate::init_dreamcast_with_elf(self as *mut Dreamcast, &elf_bytes)
    }

    /// Boot with an ELF file using REIOS
    pub fn boot_with_elf(&mut self, elf_path: &str) {
        println!("Dreamcast: Booting with ELF using REIOS");

        // Load ELF file (this also resets the dreamcast)
        if let Err(e) = self.load_elf(elf_path) {
            panic!("Failed to load ELF {}: {:?}", elf_path, e);
        }

        // Need to split borrowing - use unsafe for controlled access
        let self_ptr = self as *mut Dreamcast;
        let mut reios_ctx = unsafe {
            // Allocate ctx_wrapper on heap so pointers remain valid
            let ctx_wrapper = Box::new(Sh4ContextWrapper {
                ctx: &mut (*self_ptr).ctx,
                running: &mut (*self_ptr).running,
                dreamcast: &mut *self_ptr,
            });

            // Leak the box to get a stable pointer with 'static lifetime
            let wrapper_ptr = Box::leak(ctx_wrapper) as *mut Sh4ContextWrapper;

            // Create trait object pointers from wrapper
            let mem_ptr: *mut dyn reios::ReiosSh4Memory = wrapper_ptr;
            let ctx_ptr: *mut dyn reios::ReiosSh4Context = wrapper_ptr;
            let disc_ptr: *const dyn reios::ReiosDisc = &DUMMY_DISC_INSTANCE;

            // Initialize REIOS with pointers
            let mut reios_ctx = reios::ReiosContext::new(mem_ptr, ctx_ptr, disc_ptr);

            let mem = &mut *self_ptr;
            reios_ctx.init(mem);

            // Boot with REIOS
            let mem_ref = &mut *wrapper_ptr as &mut dyn reios::ReiosSh4Memory;
            let ctx_ref = &mut *wrapper_ptr as &mut dyn reios::ReiosSh4Context;
            reios_ctx.boot(mem_ref, ctx_ref, &DUMMY_DISC_INSTANCE);

            reios_ctx
        };

        // Store REIOS context in SH4 context
        let pc = self.ctx.pc0;
        self.ctx.reios_ctx = Some(reios_ctx);

        println!("Dreamcast: REIOS boot complete, PC = 0x{:08X}", pc);
    }
}
