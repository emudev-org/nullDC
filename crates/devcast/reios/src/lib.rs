/*
    REIOS - Dreamcast BIOS Replacement
    Ported from reference/devcast/libswirl/reios

    Extremely primitive BIOS replacement for Dreamcast emulation.
    Many thanks to Lars Olsson (jlo@ludd.luth.se) for BIOS decompile work.
*/

pub mod descrambl;
pub mod gdrom_hle;
pub mod reios;
pub mod traits;

// Re-export main types and functions
pub use reios::{ReiosContext, IpBinMetadata, REIOS_OPCODE};
pub use gdrom_hle::{GdromHleState, gdrom_hle_op};
pub use traits::{ReiosSh4Memory, ReiosSh4Context, ReiosDisc};

// Export constants
pub use gdrom_hle::{
    SYSCALL_GDROM,
    GDROM_SEND_COMMAND, GDROM_CHECK_COMMAND, GDROM_MAIN,
    GDROM_INIT, GDROM_CHECK_DRIVE, GDROM_ABORT_COMMAND,
    GDROM_RESET, GDROM_SECTOR_MODE,
    GDCC_PIOREAD, GDCC_DMAREAD, GDCC_GETTOC, GDCC_GETTOC2,
    GDCC_PLAY, GDCC_PLAY_SECTOR, GDCC_PAUSE, GDCC_RELEASE,
    GDCC_INIT, GDCC_SEEK, GDCC_READ, GDCC_STOP, GDCC_GETSCD, GDCC_GETSES,
};
