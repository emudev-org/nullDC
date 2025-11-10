/*
    Trait interfaces for REIOS integration with emulator
*/

/// SH4 Memory access interface for REIOS
pub trait ReiosSh4Memory {
    /// Read a 32-bit value from memory
    fn read_mem32(&self, addr: u32) -> u32;

    /// Write a 32-bit value to memory
    fn write_mem32(&mut self, addr: u32, value: u32);

    /// Read a 16-bit value from memory
    fn read_mem16(&self, addr: u32) -> u16;

    /// Write a 16-bit value to memory
    fn write_mem16(&mut self, addr: u32, value: u16);

    /// Read an 8-bit value from memory
    fn read_mem8(&self, addr: u32) -> u8;

    /// Write an 8-bit value to memory
    fn write_mem8(&mut self, addr: u32, value: u8);

    /// Get a mutable pointer to memory at address
    /// Returns None if address is not directly accessible
    fn get_mem_ptr(&mut self, addr: u32, size: u32) -> Option<*mut u8>;

    /// Read a block of memory
    fn read_mem_block(&self, addr: u32, data: &mut [u8]);

    /// Write a block of memory
    fn write_mem_block(&mut self, addr: u32, data: &[u8]);
}

/// SH4 CPU context interface for REIOS
pub trait ReiosSh4Context {
    /// Get general purpose register value
    fn get_r(&self, reg: usize) -> u32;

    /// Set general purpose register value
    fn set_r(&mut self, reg: usize, value: u32);

    /// Get program counter
    fn get_pc(&self) -> u32;

    /// Set program counter
    fn set_current_pc(&mut self, value: u32);
    fn set_next_pc(&mut self, value: u32);

    /// Get procedure return register
    fn get_pr(&self) -> u32;

    /// Set procedure return register
    fn set_pr(&mut self, value: u32);

    /// Get other special registers
    fn get_gbr(&self) -> u32;
    fn set_gbr(&mut self, value: u32);
    fn get_vbr(&self) -> u32;
    fn set_vbr(&mut self, value: u32);
    fn get_ssr(&self) -> u32;
    fn set_ssr(&mut self, value: u32);
    fn get_spc(&self) -> u32;
    fn set_spc(&mut self, value: u32);
    fn get_sgr(&self) -> u32;
    fn set_sgr(&mut self, value: u32);
    fn get_dbr(&self) -> u32;
    fn set_dbr(&mut self, value: u32);
    fn get_fpul(&self) -> u32;
    fn set_fpul(&mut self, value: u32);

    /// Get/set SR status register
    fn get_sr_status(&self) -> u32;
    fn set_sr_status(&mut self, value: u32);
    fn get_sr_t(&self) -> bool;
    fn set_sr_t(&mut self, value: bool);

    /// Get/set FPSCR
    fn get_fpscr_full(&self) -> u32;
    fn set_fpscr_full(&mut self, value: u32);

    /// Stop CPU execution
    fn stop(&mut self);

    /// Check if CPU is running
    fn is_running(&self) -> bool;
}

/// GD-ROM disc interface for REIOS
pub trait ReiosDisc {
    /// Read sectors from disc
    /// Returns true on success
    fn read_sector(&self, buffer: &mut [u8], fad: u32, sector_count: u32, sector_size: u32) -> bool;

    /// Get table of contents
    fn get_toc(&self, buffer: &mut [u32], session: u32);

    /// Get session information
    fn get_session_info(&self, buffer: &mut [u8], session: u8);

    /// Get disc type (0 = CDROM, 0x80 = GDROM)
    fn get_disc_type(&self) -> u32;
}
