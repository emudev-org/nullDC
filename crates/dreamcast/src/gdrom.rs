#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(clippy::upper_case_acronyms)]

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::asic;


use std::cmp::{max, min};
use std::mem::{size_of};
use std::ptr;

const GD_BASE: u32 = 0x005F_7000;
const GD_END: u32 = 0x005F_70FF;

const REPLY_A1: [u16; 40] = [
	0x2020,0x0020,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
	0x4553,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,0x2020,
	0x4443,0x522d,0x4d4f,0x4420,0x4952,0x4556,0x2020,0x2020,
	0x2e36,0x3334,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,
	0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000,0x0000
];

const REPLY_11: [u16; 16] = [
	0x0000,0x0000,0xb400,0x0019,0x0800,0x4553,0x2020,0x2020,
	0x2020,0x6552,0x2076,0x2e36,0x3334,0x3939,0x3430,0x3830
];

const REPLY_71: [u16; 506] = [
    0x0b96,0xf045,0xff7e,0x063d,0x7d4d,0xbf10,0x0007,0xcf73,0x009c,0x0cbc,0xaf1c,0x301c,0xa7e7,0xa803,0x0098,0x0fbd,0x5bbd,0x50aa,0x3923,
		0x1031,0x690e,0xe513,0xd200,0x660d,0xbf54,0xfd5f,0x7437,0x5bf4,0x0022,0x09c6,0xca0f,0xe893,0xaba4,0x6100,0x2e0e,0x4be1,0x8b76,0xa56a,
		0xe69c,0xc423,0x4b00,0x1b06,0x0191,0xe200,0xcf0d,0x38ca,0xb93a,0x91e7,0xefe5,0x004b,0x09d6,0x68d3,0xc43e,0x2daf,0x2a00,0xf90d,0x78fc,
		0xaeed,0xb399,0x5a32,0x00e7,0x0a4c,0x9722,0x825b,0x7a06,0x004c,0x0e42,0x7857,0xf546,0xfc20,0xcb6b,0x5b01,0x0086,0x0ee4,0x26b2,0x71cd,
		0xa5e3,0x0633,0x9a8e,0x0050,0x0707,0x34f5,0xe6ef,0x3200,0x130f,0x5941,0x0f56,0x3802,0x642a,0x072a,0x003e,0x1152,0x1d2a,0x765f,0xa066,
		0x2fb2,0xc797,0x6e5e,0xe252,0x5800,0xca09,0xa589,0x0adf,0x00de,0x0650,0xb849,0x00b4,0x0577,0xe824,0xbb00,0x910c,0xa289,0x628b,0x6ade,
		0x60c6,0xe700,0x0f0f,0x9611,0xd255,0xe6bf,0x0b48,0xab5c,0x00dc,0x0aba,0xd730,0x0e48,0x6378,0x000c,0x0dd2,0x8afb,0xfea3,0x3af8,0x88dd,
		0x4ba9,0xa200,0x750a,0x0d5d,0x2437,0x9dc5,0xf700,0x250b,0xdbef,0xe041,0x3e52,0x004e,0x03b7,0xe500,0xb911,0x5ade,0xcf57,0x1ab9,0x7ffc,
		0xee26,0xcd7b,0x002b,0x084b,0x09b8,0x6a70,0x009f,0x114b,0x158c,0xa387,0x4f05,0x8e37,0xde63,0x39ef,0x4bfc,0xab00,0x0b10,0xaa91,0xe10f,
		0xaee9,0x3a69,0x03f8,0xd269,0xe200,0xc107,0x3d5c,0x0082,0x08a9,0xc468,0x2ead,0x00d1,0x0ef7,0x47c6,0xcdc8,0x7c8e,0x5c00,0xb995,0x00f4,
		0x04e3,0x005b,0x0774,0xc765,0x8e84,0xc600,0x6107,0x4480,0x003f,0x0ec8,0x7872,0xd347,0x4dc2,0xc0af,0x1354,0x0031,0x0df7,0xd848,0x92e2,
		0x7f9f,0x442f,0x3368,0x0d00,0xab10,0xeafe,0x198e,0xf881,0x7c6f,0xe1de,0x06b3,0x4d00,0x6611,0x4cae,0xb7f9,0xee2f,0x8eb0,0xe17e,0x958d,
		0x006f,0x0df4,0x9d88,0xe3ca,0xb2c4,0xbb47,0x69a0,0xf300,0x480b,0x4117,0xa064,0x710e,0x0082,0x1e34,0x4d18,0x8085,0xa94c,0x660b,0x759b,
		0x6113,0x2770,0x7a81,0xcd02,0xab57,0x02df,0x5293,0xdf83,0xa848,0x9ea6,0x6f74,0x0389,0x2528,0x9652,0x67ff,0xd87a,0xb13c,0x462c,0xef84,
		0xc1e1,0xc9c6,0x96dc,0xa9aa,0x82c4,0x2758,0x7557,0x3467,0x3bfb,0xbf25,0x3bfb,0x13f6,0x96ec,0x16e5,0xfd26,0xdaa8,0xc61b,0x7f50,0xff47,
		0x5508,0xed08,0x9300,0xc49b,0x6771,0xa6ec,0x16cc,0x8720,0x0747,0x00a6,0x5d79,0xab4f,0x6fa1,0x6b7a,0xc427,0xa3da,0x94c3,0x7f4f,0xe5f3,
		0x6f1b,0xe5cc,0xe5f0,0xc99d,0xfdae,0xac39,0xe54c,0x8358,0x6525,0x7492,0x819e,0xb6a0,0x02a9,0x079b,0xe7b6,0x5779,0x4ad9,0xface,0x94b4,
		0xcc05,0x3c86,0x06dd,0xa6cd,0x2424,0xc1fa,0x48f9,0x0cc9,0xc46c,0x8296,0xf617,0x0931,0xe2c4,0xfd77,0x46cf,0xb218,0x015f,0xd16b,0x567b,
		0x94b8,0xe54a,0x196c,0xc0f0,0x70b6,0xf793,0xd1d3,0x6e2b,0x537c,0x856d,0x0cd1,0x778b,0x90ee,0x15da,0xe055,0x0958,0xfc56,0x9f31,0x46af,
		0xc3cb,0x718d,0xf275,0xc32c,0xa1bb,0xcfc4,0x5627,0x9b7c,0xaffe,0x4e3e,0xcdb4,0xaa6a,0xf3f5,0x22e3,0xe182,0x68a5,0xdbb3,0x9e8f,0x7b5e,
		0xf090,0x3f79,0x8c52,0x8861,0xae76,0x6314,0x0f19,0xce1d,0x63a1,0xb210,0xd7e2,0xb194,0xcb33,0x8528,0x9b7d,0xf4f5,0x5025,0xdb9b,0xa535,
		0x9cb0,0x9209,0x31e3,0xab40,0xf44d,0xe835,0x0ab3,0xc321,0x9c86,0x29cb,0x77a4,0xbc57,0xdad8,0x82a5,0xe880,0x72cf,0xad81,0x282e,0xd8ff,
		0xd1b6,0x972b,0xff00,0x06e1,0x3944,0x4b1c,0x19ab,0x4d5b,0x3ed6,0x5c1b,0xbb64,0x6832,0x7cf5,0x9ec9,0xb4e8,0x1b29,0x4d7f,0x8080,0x8b7e,
		0x0a1c,0x9ae6,0x49bf,0xc51e,0x67b6,0x057d,0x90e4,0x4b40,0x9baf,0xde52,0x8017,0x5681,0x3aea,0x8253,0x628c,0x96fb,0x6f97,0x16c1,0xd478,
		0xe77b,0x5ab9,0xeb2a,0x6887,0xd333,0x4531,0xfefa,0x1cf4,0x8690,0x7773,0xa9d9,0x4ad1,0xcf4a,0x23ae,0xf9db,0xd809,0xdc18,0x0d6a,0x19e4,
		0x658c,0x64c6,0xdcc7,0xe3a9,0xb191,0xc84c,0x9ec1,0x7f3b,0xa3cb,0xddcf,0x1df0,0x6e07,0xcedc,0xcd0d,0x1e7e,0x1155,0xdf8b,0xab3a,0x3bb6,
		0x526e,0xa77f,0xd100,0xbe33,0x9bf2,0x4afc,0x9dcf,0xc68f,0x7bc4,0xe7da,0x1c2a,0x6e26
];


// ===================================================
// Logging control
// ===================================================
const ENABLE_LOG_ATA: bool = false;
const ENABLE_LOG_SPI: bool = false;
const ENABLE_LOG_SPICMD: bool = false;
const ENABLE_LOG_RM: bool = false;
const ENABLE_LOG_SUBCODE: bool = false;

macro_rules! println_ata {
    ($($arg:tt)*) => { if ENABLE_LOG_ATA { println!($($arg)*); } };
}
macro_rules! println_spi {
    ($($arg:tt)*) => { if ENABLE_LOG_SPI { println!($($arg)*); } };
}
macro_rules! println_spicmd {
    ($($arg:tt)*) => { if ENABLE_LOG_SPICMD { println!($($arg)*); } };
}
macro_rules! println_rm {
    ($($arg:tt)*) => { if ENABLE_LOG_RM { println!($($arg)*); } };
}
macro_rules! println_subcode {
    ($($arg:tt)*) => { if ENABLE_LOG_SUBCODE { println!($($arg)*); } };
}

macro_rules! verify {
    ($cond:expr) => {
        if !$cond { panic!("verify!({}) failed", stringify!($cond)); }
    };
}
macro_rules! die {
    ($($arg:tt)*) => { panic!("Fatal: {}", format!($($arg)*)); };
}

// ===================================================
// Enumerations & Disc type
// ===================================================
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum gd_states {
    gds_waitcmd,
    gds_procata,
    gds_waitpacket,
    gds_procpacket,
    gds_pio_send_data,
    gds_pio_get_data,
    gds_pio_end,
    gds_procpacketdone,
    gds_readsector_pio,
    gds_readsector_dma,
    gds_process_set_mode,
}

impl Default for gd_states {
    fn default() -> Self {
        gd_states::gds_waitcmd
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DiscType {
    NoDisk = 0,
    Open = 1,
    Busy = 2,
    GdRom = 3,
    CdRom_XA = 4,
}

// ===================================================
// Bitfields
// ===================================================
bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_StatusT(u8);
    impl Debug;
    pub CHECK, set_CHECK: 0;
    pub res, _: 1;
    pub CORR, set_CORR: 2;
    pub DRQ, set_DRQ: 3;
    pub DSC, set_DSC: 4;
    pub DF, set_DF: 5;
    pub DRDY, set_DRDY: 6;
    pub BSY, set_BSY: 7;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_ErrRegT(u8);
    impl Debug;
    pub ILI, set_ILI: 0;
    pub EOM, set_EOM: 1;
    pub ABRT, set_ABRT: 2;
    pub MCR, set_MCR: 3;
    pub Sense, set_Sense: 7,4;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_FeaturesT(u8);
    impl Debug;
    pub DMA, set_DMA: 0;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_InterruptReasonT(u8);
    impl Debug;
    pub CoD, set_CoD: 0;
    pub IO, set_IO: 1;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_SecCountT(u8);
    impl Debug;
    pub ModeVal, set_ModeVal: 3,0;
    pub TransMode, set_TransMode: 7,4;
}

bitfield::bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct GD_SecNumbT(u8);
    impl Debug;
    pub Status, set_Status: 3,0;
    pub DiscFormat, set_DiscFormat: 7,4;
}

// ===================================================
// Simple structs
// ===================================================
#[repr(C)]
pub union SpiCommandInfo {
    pub CommandCode: u8,
    pub CommandData: [u8; 12],
    pub CommandData_16: [u16; 6],
}

#[derive(Copy, Clone, Default)]
pub struct read_params_t {
    pub start_sector: u32,
    pub remaining_sectors: u32,
    pub sector_type: u32,
}

#[derive(Copy, Clone, Default)]
pub struct packet_cmd_t {
    pub index: u32,
    pub data_16: [u16; 6],
    pub data_8: [u8; 12],
}

#[derive(Default)]
pub struct read_buff_t {
    pub cache_index: u32,
    pub cache_size: u32,
    pub cache: Vec<u8>,
}

#[derive(Default)]
pub struct pio_buff_t {
    pub next_state: gd_states,
    pub index: u32,
    pub size: u32,
    pub data: Vec<u16>,
}

#[derive(Default)]
pub struct ata_cmd_t {
    pub command: u8,
}

#[derive(Default, Copy, Clone)]
pub struct cdda_t {
    pub playing: bool,
    pub repeats: u32,
    pub CurrAddrFAD: u32,
    pub EndAddrFAD: u32,
    pub StartAddrFAD: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ByteCount_t {
    pub parts: ByteCountParts,
    pub full: u16,
}

impl Default for ByteCount_t {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct ByteCountParts {
    pub low: u8,
    pub hi: u8,
}

// ===================================================
// Constants / registers / ATA / SPI
// ===================================================
pub const GDROM_IRQ_EXT_BIT: u8 = 0;

pub const GD_BUSY: u8 = 0x00;
pub const GD_PAUSE: u8 = 0x01;
pub const GD_STANDBY: u8 = 0x02;
pub const GD_PLAY: u8 = 0x03;
pub const GD_SEEK: u8 = 0x04;
pub const GD_SCAN: u8 = 0x05;
pub const GD_OPEN: u8 = 0x06;
pub const GD_NODISC: u8 = 0x07;
pub const GD_RETRY: u8 = 0x08;
pub const GD_ERROR: u8 = 0x09;

// ATA
pub const ATA_NOP: u8 = 0x00;
pub const ATA_SOFT_RESET: u8 = 0x08;
pub const ATA_EXEC_DIAG: u8 = 0x90;
pub const ATA_SPI_PACKET: u8 = 0xA0;
pub const ATA_IDENTIFY_DEV: u8 = 0xA1;
pub const ATA_IDENTIFY: u8 = 0xEC;
pub const ATA_SET_FEATURES: u8 = 0xEF;

// SPI
pub const SPI_TEST_UNIT: u8 = 0x00;
pub const SPI_REQ_STAT: u8 = 0x10;
pub const SPI_REQ_MODE: u8 = 0x11;
pub const SPI_SET_MODE: u8 = 0x12;
pub const SPI_REQ_ERROR: u8 = 0x13;
pub const SPI_GET_TOC: u8 = 0x14;
pub const SPI_REQ_SES: u8 = 0x15;
pub const SPI_CD_OPEN: u8 = 0x16;
pub const SPI_CD_PLAY: u8 = 0x20;
pub const SPI_CD_SEEK: u8 = 0x21;
pub const SPI_CD_SCAN: u8 = 0x22;
pub const SPI_CD_READ: u8 = 0x30;
pub const SPI_CD_READ2: u8 = 0x31;
pub const SPI_GET_SCD: u8 = 0x40;

// Dummy GD-ROM I/O addresses
pub const GD_STATUS_Read: u32 = 0x005F709C;
pub const GD_ALTSTAT_Read: u32 = 0x005F7018;
pub const GD_BYCTLLO: u32 = 0x005F7090;
pub const GD_BYCTLHI: u32 = 0x005F7094;
pub const GD_DATA: u32 = 0x005F7080;
pub const GD_DRVSEL: u32 = 0x005F7098;
pub const GD_ERROR_Read: u32 = 0x005F7084;
pub const GD_IREASON_Read: u32 = 0x005F7088;
pub const GD_SECTNUM: u32 = 0x005F708C;
pub const GD_DEVCTRL_Write: u32 = 0x005F7018;
pub const GD_FEATURES_Write: u32 = 0x005F7084;
pub const GD_SECTCNT_Write: u32 = 0x005F7088;
pub const GD_COMMAND_Write: u32 = 0x005F709C;

// ===================================================
// External stubs
// ===================================================
pub struct GDRDisc;
impl GDRDisc {
    pub fn ReadSector(&self, _dst: *mut u8, _fad: u32, _count: u32, _sz: u32) { println!("ReadSector stub"); }
    pub fn GetDiscType(&self) -> u32 { 3 } // gdrom
    pub fn GetToc(&self, _dst: *mut u32, _sel: u8) { println!("GetToc stub"); }
    pub fn GetSessionInfo(&self, _dst: *mut u8, _sel: u8) { println!("GetSessionInfo stub"); }
    pub fn ReadSubChannel(&self, _dst: *mut u8, _off: u32, _len: u32) { println!("ReadSubChannel stub"); }
}
static mut g_GDRDisc: Option<GDRDisc> = None;
// static mut g_GDRDisc: Option<GDRDisc> = Some(GDRDisc);

// ===================================================
// GDRomV3Impl skeleton
// ===================================================
pub struct GDRomV3Impl {
    pub gdrom_schid: i32,
    pub sns_asc: i32,
    pub sns_ascq: i32,
    pub sns_key: i32,
    pub set_mode_offset: u32,
    pub read_params: read_params_t,
    pub packet_cmd: packet_cmd_t,
    pub read_buff: read_buff_t,
    pub pio_buff: pio_buff_t,
    pub ata_cmd: ata_cmd_t,
    pub cdda: cdda_t,
    pub gd_state: gd_states,
    pub gd_disk_type: DiscType,
    pub data_write_mode: u32,
    pub DriveSel: u32,
    pub Error: GD_ErrRegT,
    pub IntReason: GD_InterruptReasonT,
    pub Features: GD_FeaturesT,
    pub SecCount: GD_SecCountT,
    pub SecNumber: GD_SecNumbT,
    pub GDStatus: GD_StatusT,
    pub ByteCount: ByteCount_t,
    pub reply_11: [u16; 16],
}

impl GDRomV3Impl {
    pub fn new() -> Self {
        Self {
            gdrom_schid: 0,
            sns_asc: 0, sns_ascq: 0, sns_key: 0,
            set_mode_offset: 0,
            read_params: Default::default(),
            packet_cmd: Default::default(),
            read_buff: read_buff_t { cache: vec![0; 2352*32], ..Default::default() },
            pio_buff: pio_buff_t { data: vec![0; 0x10000>>1], ..Default::default() },
            ata_cmd: Default::default(),
            cdda: Default::default(),
            gd_state: gd_states::gds_waitcmd,
            gd_disk_type: DiscType::NoDisk,
            data_write_mode: 0, DriveSel: 0,
            Error: Default::default(),
            IntReason: Default::default(),
            Features: Default::default(),
            SecCount: Default::default(),
            SecNumber: Default::default(),
            GDStatus: Default::default(),
            ByteCount: Default::default(),
            reply_11: REPLY_11
        }
    }

    pub fn FillReadBuffer(&mut self) {
        self.read_buff.cache_index = 0;
        let mut count = self.read_params.remaining_sectors;
        let mut hint = 0;

        if count > 32 {
            hint = max(count - 32, 32);
            count = 32;
        }

        self.read_buff.cache_size = count * self.read_params.sector_type;

        unsafe {
            if let Some(ref disc) = g_GDRDisc {
                disc.ReadSector(
                    self.read_buff.cache.as_mut_ptr(),
                    self.read_params.start_sector,
                    count,
                    self.read_params.sector_type,
                );
            }
        }

        self.read_params.start_sector += count;
        self.read_params.remaining_sectors =
            self.read_params.remaining_sectors.saturating_sub(count);
    }

    // ===================================================
    // gd_set_state
    // ===================================================
    pub fn gd_set_state(&mut self, state: gd_states) {
        let prev = self.gd_state;
        self.gd_state = state;

        match state {
            gd_states::gds_waitcmd => {
                self.GDStatus.set_DRDY(true);
                self.GDStatus.set_BSY(false);
            }

            gd_states::gds_procata => {
                self.GDStatus.set_DRDY(false);
                self.GDStatus.set_BSY(true);
                self.gd_process_ata_cmd();
            }

            gd_states::gds_waitpacket => {
                verify!(prev == gd_states::gds_procata);
                self.packet_cmd.index = 0;
                self.IntReason.set_CoD(true);
                self.IntReason.set_IO(false);
                self.GDStatus.set_BSY(false);
                self.GDStatus.set_DRQ(true);
            }

            gd_states::gds_procpacket => {
                verify!(prev == gd_states::gds_waitpacket);
                self.GDStatus.set_DRQ(false);
                self.GDStatus.set_BSY(true);
                self.gd_process_spi_cmd();
            }

            gd_states::gds_pio_send_data | gd_states::gds_pio_get_data => {
                unsafe { self.ByteCount.full = (self.pio_buff.size << 1) as u16 };
                self.IntReason.set_IO(true);
                self.IntReason.set_CoD(false);
                self.GDStatus.set_DRQ(true);
                self.GDStatus.set_BSY(false);
                asic::raise_external(GDROM_IRQ_EXT_BIT);
            }

            gd_states::gds_readsector_pio => {
                self.GDStatus.set_BSY(true);

                let mut sector_count = self.read_params.remaining_sectors;
                let mut next_state = gd_states::gds_pio_end;

                if sector_count > 27 {
                    sector_count = 27;
                    next_state = gd_states::gds_readsector_pio;
                }

                unsafe {
                    if let Some(ref disc) = g_GDRDisc {
                        disc.ReadSector(
                            self.pio_buff.data.as_mut_ptr() as *mut u8,
                            self.read_params.start_sector,
                            sector_count,
                            self.read_params.sector_type,
                        );
                    }
                }

                self.read_params.start_sector += sector_count;
                self.read_params.remaining_sectors =
                    self.read_params.remaining_sectors.saturating_sub(sector_count);

                self.gd_spi_pio_end(None, sector_count * self.read_params.sector_type, next_state);
            }

            gd_states::gds_readsector_dma => {
                self.FillReadBuffer();
            }

            gd_states::gds_pio_end => {
                self.GDStatus.set_DRQ(false);
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            gd_states::gds_procpacketdone => {
                self.GDStatus.set_DRDY(true);
                self.IntReason.set_CoD(true);
                self.IntReason.set_IO(true);
                self.GDStatus.set_DRQ(false);
                self.GDStatus.set_BSY(false);
                asic::raise_external(GDROM_IRQ_EXT_BIT);
                self.gd_set_state(gd_states::gds_waitcmd);
            }

            gd_states::gds_process_set_mode => {
                let off = self.set_mode_offset as usize;
                let sz = (self.pio_buff.size << 1) as usize;
                let slice = &self.pio_buff.data[..self.pio_buff.size as usize];
                unsafe {
                    let dest = &mut *(self.reply_11.as_mut_ptr().add(off) as *mut [u16; REPLY_11.len()]);
                    dest[..sz / 2].copy_from_slice(&slice[..sz / 2]);
                }
                self.gd_set_state(gd_states::gds_pio_end);
            }

            _ => {
                die!("Unhandled GDROM state {:?}", state);
            }
        }
    }

    // ===================================================
    // gd_setdisc
    // ===================================================
    pub fn gd_setdisc(&mut self) {
        self.cdda.playing = false;
        let mut newd = DiscType::NoDisk;

        unsafe {
            if let Some(ref disc) = g_GDRDisc {
                newd = match disc.GetDiscType() {
                    1 => DiscType::Open,
                    2 => DiscType::Busy,
                    3 => DiscType::GdRom,
                    4 => DiscType::CdRom_XA,
                    _ => DiscType::NoDisk,
                };
            }
        }

        match newd {
            DiscType::NoDisk => {
                self.sns_asc = 0x29;
                self.sns_ascq = 0x00;
                self.sns_key = 0x6;
                self.SecNumber.set_Status(GD_NODISC);
            }
            DiscType::Open => {
                self.sns_asc = 0x28;
                self.sns_ascq = 0x00;
                self.sns_key = 0x6;
                self.SecNumber.set_Status(GD_OPEN);
            }
            DiscType::Busy => {
                self.SecNumber.set_Status(GD_BUSY);
                self.GDStatus.set_BSY(true);
                self.GDStatus.set_DRDY(false);
            }
            _ => {
                if self.SecNumber.Status() == GD_BUSY {
                    self.SecNumber.set_Status(GD_PAUSE);
                } else {
                    self.SecNumber.set_Status(GD_STANDBY);
                }
            }
        }

        if self.gd_disk_type == DiscType::Busy && newd != DiscType::Busy {
            self.GDStatus.set_BSY(false);
            self.GDStatus.set_DRDY(true);
        }

        self.gd_disk_type = newd;
        self.SecNumber.set_DiscFormat((self.gd_disk_type as u8) >> 4);
    }

    // ===================================================
    // gd_reset
    // ===================================================
    pub fn gd_reset(&mut self) {
        self.gd_setdisc();
        self.gd_set_state(gd_states::gds_waitcmd);
    }

    // ===================================================
    // GetFAD
    // ===================================================
    pub fn GetFAD(&self, data: &[u8], msf: bool) -> u32 {
        if msf {
            println!("GDROM: MSF FORMAT");
            (data[0] as u32 * 60 * 75) + (data[1] as u32 * 75) + data[2] as u32
        } else {
            ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32)
        }
    }

    // ===================================================
    // gd_spi_pio_end
    // ===================================================
    pub fn gd_spi_pio_end(
        &mut self,
        buffer: Option<&[u8]>,
        len: u32,
        next_state: gd_states,
    ) {
        verify!(len < 0xFFFF);
        self.pio_buff.index = 0;
        self.pio_buff.size = len >> 1;
        self.pio_buff.next_state = next_state;

        if let Some(buf) = buffer {
            let n = len as usize;
            let words = n / 2;
            for i in 0..words {
                let w = (buf[i * 2] as u16) | ((buf[i * 2 + 1] as u16) << 8);
                if i < self.pio_buff.data.len() {
                    self.pio_buff.data[i] = w;
                }
            }
        }

        if len == 0 {
            self.gd_set_state(next_state);
        } else {
            self.gd_set_state(gd_states::gds_pio_send_data);
        }
    }

    // ===================================================
    // gd_spi_pio_read_end
    // ===================================================
    pub fn gd_spi_pio_read_end(&mut self, len: u32, next_state: gd_states) {
        verify!(len < 0xFFFF);
        self.pio_buff.index = 0;
        self.pio_buff.size = len >> 1;
        self.pio_buff.next_state = next_state;

        if len == 0 {
            self.gd_set_state(next_state);
        } else {
            self.gd_set_state(gd_states::gds_pio_get_data);
        }
    }

    // ===================================================
    // gd_process_ata_cmd
    // ===================================================
    pub fn gd_process_ata_cmd(&mut self) {
        self.Error.set_ABRT(false);

        if self.sns_key == 0x0 || self.sns_key == 0xB {
            self.GDStatus.set_CHECK(false);
        } else {
            self.GDStatus.set_CHECK(true);
        }

        match self.ata_cmd.command {
            ATA_NOP => {
                println_ata!("ATA_NOP");
                self.Error.set_ABRT(true);
                self.Error.set_Sense(self.sns_key as u8);
                self.GDStatus.set_BSY(false);
                self.GDStatus.set_CHECK(true);
                asic::raise_external(GDROM_IRQ_EXT_BIT);
                self.gd_set_state(gd_states::gds_waitcmd);
            }

            ATA_SOFT_RESET => {
                println_ata!("ATA_SOFT_RESET");
                self.gd_reset();
            }

            ATA_EXEC_DIAG => {
                println_ata!("ATA_EXEC_DIAG");
                println!("ATA_EXEC_DIAG -- not implemented");
            }

            ATA_SPI_PACKET => {
                println_ata!("ATA_SPI_PACKET");
                self.gd_set_state(gd_states::gds_waitpacket);
            }

            ATA_IDENTIFY_DEV => {
                println_ata!("ATA_IDENTIFY_DEV");
                let offset = (self.packet_cmd.data_8[2] >> 1) as usize;
                let size = self.packet_cmd.data_8[4] as u32;
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        REPLY_A1.as_ptr() as *const u8,
                        REPLY_A1.len() * std::mem::size_of::<u16>(),
                    )
                };
                self.gd_spi_pio_end(Some(bytes), size, gd_states::gds_pio_end);
            }

            ATA_IDENTIFY => {
                println_ata!("ATA_IDENTIFY");
                self.DriveSel &= 0xf0;
                unsafe {
                    self.SecCount.0 = 1;
                    self.SecNumber.0 = 1;
                    self.ByteCount.parts.low = 0x14;
                    self.ByteCount.parts.hi = 0xeb;
                    self.Error.0 = 0x4;
                }
                self.GDStatus.0 = 0;
                self.GDStatus.set_DRDY(true);
                self.GDStatus.set_CHECK(true);
                asic::raise_external(GDROM_IRQ_EXT_BIT);
                self.gd_set_state(gd_states::gds_waitcmd);
            }

            ATA_SET_FEATURES => {
                println_ata!("ATA_SET_FEATURES");
                self.Error.set_ABRT(false);
                self.GDStatus.set_DSC(false);
                self.GDStatus.set_DF(false);
                self.GDStatus.set_CHECK(false);
                asic::raise_external(GDROM_IRQ_EXT_BIT);
                self.gd_set_state(gd_states::gds_waitcmd);
            }

            _ => {
                die!("Unknown ATA command {:02X}", self.ata_cmd.command);
            }
        }
    }

    // ===================================================
    // gd_process_spi_cmd
    // ===================================================
    pub fn gd_process_spi_cmd(&mut self) {
        println_spi!(
            "Sense: {:02x} {:02x} {:02x}",
            self.sns_asc,
            self.sns_ascq,
            self.sns_key
        );
        println_spi!(
            "SPI command {:02x}; Params: {:02x?}",
            self.packet_cmd.data_8[0],
            &self.packet_cmd.data_8
        );

        if self.sns_key == 0x0 || self.sns_key == 0xB {
            self.GDStatus.set_CHECK(false);
        } else {
            self.GDStatus.set_CHECK(true);
        }

        match self.packet_cmd.data_8[0] {
            SPI_TEST_UNIT => {
                println_spicmd!("SPI_TEST_UNIT");
                self.GDStatus.set_CHECK(self.SecNumber.Status() == GD_BUSY);
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_REQ_MODE => {
                println_spicmd!("SPI_REQ_MODE");
                let offset = (self.packet_cmd.data_8[2] >> 1) as usize;
                let size = self.packet_cmd.data_8[4] as u32;
                let dummy = vec![0u8; size as usize];
                self.gd_spi_pio_end(Some(&dummy), size, gd_states::gds_pio_end);
            }

            SPI_CD_READ => {
                println_spicmd!("SPI_CD_READ");
                let readcmd = &self.packet_cmd.data_8;
                let mut sector_type = 2048u32;
                let prmtype = (readcmd[1] & 1) != 0;
                let expdtype = (readcmd[1] >> 1) & 0x7;
                let other = (readcmd[1] >> 4) & 1;
                let data = (readcmd[1] >> 5) & 1;
                let subh = (readcmd[1] >> 6) & 1;
                let head = (readcmd[1] >> 7) & 1;
                if head == 1 && subh == 1 && data == 1 && expdtype == 3 && other == 0 {
                    sector_type = 2340;
                }

                let start_sector = self.GetFAD(&readcmd[2..5], prmtype);
                let sector_count =
                    ((readcmd[8] as u32) << 16) | ((readcmd[9] as u32) << 8) | readcmd[10] as u32;
                self.read_params.start_sector = start_sector;
                self.read_params.remaining_sectors = sector_count;
                self.read_params.sector_type = sector_type;

                println_spicmd!(
                    "SPI_CD_READ - Sector={} Size={}/{} DMA={}",
                    self.read_params.start_sector,
                    self.read_params.remaining_sectors,
                    self.read_params.sector_type,
                    self.Features.DMA()
                );

                if self.Features.DMA() {
                    self.gd_set_state(gd_states::gds_readsector_dma);
                } else {
                    self.gd_set_state(gd_states::gds_readsector_pio);
                }
            }

            SPI_GET_TOC => {
                println_spicmd!("SPI_GET_TOC");
                let size = ((self.packet_cmd.data_8[3] as u32) << 8)
                    | (self.packet_cmd.data_8[4] as u32);
                let toc = vec![0u8; size as usize];
                self.gd_spi_pio_end(Some(&toc), size, gd_states::gds_pio_end);
            }

            0x70 => {
                println_spicmd!("SPI unknown 0x70");
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            0x71 => {
                println_spicmd!("SPI unknown 0x71");
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        REPLY_A1.as_ptr() as *const u8,
                        REPLY_A1.len() * std::mem::size_of::<u16>(),
                    )
                };
                self.gd_spi_pio_end(Some(&bytes), bytes.len() as u32, gd_states::gds_pio_end);

                unsafe {
                    if let Some(ref disc) = g_GDRDisc {
                        match disc.GetDiscType() {
                            3 | 4 => self.SecNumber.set_Status(GD_PAUSE),
                            _ => self.SecNumber.set_Status(GD_STANDBY),
                        }
                    }
                }
            }

            SPI_SET_MODE => {
                println_spicmd!("SPI_SET_MODE");
                let offset = self.packet_cmd.data_8[2] as u32;
                let count = self.packet_cmd.data_8[4] as u32;
                verify!((offset + count) < 11);
                self.set_mode_offset = offset;
                self.gd_spi_pio_read_end(count, gd_states::gds_process_set_mode);
            }

            SPI_CD_READ2 => {
                println_spicmd!("SPI_CD_READ2");
                println!("Unhandled SPI_CD_READ2");
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_REQ_STAT => {
                println_spicmd!("SPI_REQ_STAT");
                let mut stat = [0u8; 10];
                stat[0] = self.SecNumber.Status();
                stat[1] = (self.SecNumber.DiscFormat() << 4) | (self.cdda.repeats as u8);
                stat[2] = 0x4;
                stat[3] = 2;
                stat[5] = (self.cdda.CurrAddrFAD >> 16) as u8;
                stat[6] = (self.cdda.CurrAddrFAD >> 8) as u8;
                stat[7] = (self.cdda.CurrAddrFAD & 0xFF) as u8;

                let offset = self.packet_cmd.data_8[2] as usize;
                let count = self.packet_cmd.data_8[4] as usize;
                verify!(offset + count < stat.len());
                self.gd_spi_pio_end(Some(&stat[offset..offset + count]), count as u32, gd_states::gds_pio_end);
            }

            SPI_REQ_ERROR => {
                println_spicmd!("SPI_REQ_ERROR");
                let mut resp = [0u8; 10];
                resp[0] = 0xF0;
                resp[2] = self.sns_key as u8;
                resp[8] = self.sns_asc as u8;
                resp[9] = self.sns_ascq as u8;
                self.gd_spi_pio_end(Some(&resp), self.packet_cmd.data_8[4] as u32, gd_states::gds_pio_end);
                self.sns_key = 0;
                self.sns_asc = 0;
                self.sns_ascq = 0;
            }

            SPI_REQ_SES => {
                println_spicmd!("SPI_REQ_SES");
                let mut ses = [0u8; 6];
                ses[0] = self.SecNumber.Status();
                self.gd_spi_pio_end(Some(&ses), self.packet_cmd.data_8[4] as u32, gd_states::gds_pio_end);
            }

            SPI_CD_OPEN => {
                println_spicmd!("SPI_CD_OPEN");
                println!("Unhandled SPI_CD_OPEN");
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_CD_PLAY => {
                println_spicmd!("SPI_CD_PLAY");
                self.cdda.playing = true;
                self.SecNumber.set_Status(GD_PLAY);
                let param_type = self.packet_cmd.data_8[1] & 0x7;
                match param_type {
                    1 => {
                        self.cdda.StartAddrFAD =
                            self.GetFAD(&self.packet_cmd.data_8[2..5], false);
                        self.cdda.EndAddrFAD = self.GetFAD(&self.packet_cmd.data_8[8..11], false);
                        self.cdda.CurrAddrFAD = self.cdda.StartAddrFAD;
                        self.GDStatus.set_DSC(true);
                    }
                    2 => {
                        self.cdda.StartAddrFAD =
                            self.GetFAD(&self.packet_cmd.data_8[2..5], true);
                        self.cdda.EndAddrFAD = self.GetFAD(&self.packet_cmd.data_8[8..11], true);
                        self.cdda.CurrAddrFAD = self.cdda.StartAddrFAD;
                        self.GDStatus.set_DSC(true);
                    }
                    7 => { /* resume */ }
                    _ => die!("SPI_CD_PLAY: unknown param_type"),
                }
                self.cdda.repeats = (self.packet_cmd.data_8[6] & 0xF) as u32;
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_CD_SEEK => {
                println_spicmd!("SPI_CD_SEEK");
                self.SecNumber.set_Status(GD_PAUSE);
                self.cdda.playing = false;
                let p = self.packet_cmd.data_8[1] & 0x7;
                match p {
                    1 => {
                        self.cdda.StartAddrFAD =
                            self.GetFAD(&self.packet_cmd.data_8[2..5], false);
                        self.cdda.CurrAddrFAD = self.cdda.StartAddrFAD;
                        self.GDStatus.set_DSC(true);
                    }
                    2 => {
                        self.cdda.StartAddrFAD =
                            self.GetFAD(&self.packet_cmd.data_8[2..5], true);
                        self.cdda.CurrAddrFAD = self.cdda.StartAddrFAD;
                        self.GDStatus.set_DSC(true);
                    }
                    3 => {
                        self.SecNumber.set_Status(GD_STANDBY);
                        self.cdda.StartAddrFAD = 150;
                        self.cdda.CurrAddrFAD = 150;
                        self.GDStatus.set_DSC(true);
                    }
                    4 => {}
                    _ => die!("SPI_CD_SEEK: unknown param"),
                }
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_CD_SCAN => {
                println_spicmd!("SPI_CD_SCAN");
                println!("Unhandled SPI_CD_SCAN");
                self.gd_set_state(gd_states::gds_procpacketdone);
            }

            SPI_GET_SCD => {
                println_spicmd!("SPI_GET_SCD");
                let mut subc_info = vec![0u8; 100];
                subc_info[0] = 0;
                subc_info[1] = 0x15;
                let format = self.packet_cmd.data_8[1] & 0xF;
                let sz: u32;
                if format == 0 {
                    sz = 100;
                } else {
                    sz = 0xE;
                }
                self.gd_spi_pio_end(Some(&subc_info), sz, gd_states::gds_pio_end);
            }

            _ => {
                println!(
                    "GDROM: Unhandled Sega SPI frame {:02X}",
                    self.packet_cmd.data_8[0]
                );
                self.gd_set_state(gd_states::gds_procpacketdone);
            }
        }
    }

     // ===================================================
    // MMIO Read
    // ===================================================
    pub fn Read(&mut self, Addr: u32, sz: u32) -> u32 {
        match Addr {
            GD_STATUS_Read => {
                asic::cancel_external(GDROM_IRQ_EXT_BIT);
                println_rm!("GDROM: STATUS [cancel int](v={:X})", self.GDStatus.0);
                (self.GDStatus.0 as u32) | (1 << 4)
            }
            GD_ALTSTAT_Read => {
                println_rm!("GDROM: Read From AltStatus (v={:X})", self.GDStatus.0);
                (self.GDStatus.0 as u32) | (1 << 4)
            }
            GD_BYCTLLO => {
                println_rm!("GDROM: Read From GD_BYCTLLO");
                unsafe { self.ByteCount.parts.low as u32 }
            }
            GD_BYCTLHI => {
                println_rm!("GDROM: Read From GD_BYCTLHI");
                unsafe { self.ByteCount.parts.hi as u32 }
            }
            GD_DATA => {
                if sz != 2 {
                    println!("GDROM: Bad size on DATA REG Read");
                }
                if self.pio_buff.index == self.pio_buff.size {
                    println!("GDROM: Illegal Read From DATA (underflow)");
                    return 0;
                }
                let rv = self.pio_buff.data[self.pio_buff.index as usize] as u32;
                self.pio_buff.index += 1;
                unsafe {
                    self.ByteCount.full = self.ByteCount.full.wrapping_sub(2);
                }
                if self.pio_buff.index == self.pio_buff.size {
                    verify!(self.pio_buff.next_state != gd_states::gds_pio_send_data);
                    self.gd_set_state(self.pio_buff.next_state);
                }
                rv
            }
            GD_DRVSEL => {
                println_rm!("GDROM: Read From DriveSel");
                self.DriveSel
            }
            GD_ERROR_Read => {
                println_rm!("GDROM: Read from ERROR Register");
                self.Error.set_Sense(self.sns_key as u8);
                self.Error.0 as u32
            }
            GD_IREASON_Read => {
                println_rm!("GDROM: Read from INTREASON Register");
                self.IntReason.0 as u32
            }
            GD_SECTNUM => {
                println_rm!(
                    "GDROM: Read from SecNumber Register (v={:X})",
                    self.SecNumber.0
                );
                self.SecNumber.0 as u32
            }
            _ => {
                println!(
                    "GDROM: Unhandled read from address {:08X}, Size:{:X}",
                    Addr, sz
                );
                0
            }
        }
    }

    // ===================================================
    // MMIO Write
    // ===================================================
    pub fn Write(&mut self, Addr: u32, data: u32, sz: u32) {
        match Addr {
            GD_BYCTLLO => {
                println_rm!("GDROM: Write to GD_BYCTLLO = {:X}, Size:{:X}", data, sz);
                unsafe {
                    self.ByteCount.parts.low = data as u8;
                }
            }
            GD_BYCTLHI => {
                println_rm!("GDROM: Write to GD_BYCTLHI = {:X}, Size:{:X}", data, sz);
                unsafe {
                    self.ByteCount.parts.hi = data as u8;
                }
            }
            GD_DATA => {
                if sz != 2 {
                    println!("GDROM: Bad size on DATA REG");
                }
                match self.gd_state {
                    gd_states::gds_waitpacket => {
                        if (self.packet_cmd.index as usize) < self.packet_cmd.data_16.len() {
                            self.packet_cmd.data_16[self.packet_cmd.index as usize] = data as u16;
                        }
                        self.packet_cmd.index += 1;
                        if self.packet_cmd.index == 6 {
                            self.gd_set_state(gd_states::gds_procpacket);
                        }
                    }
                    gd_states::gds_pio_get_data => {
                        if (self.pio_buff.index as usize) < self.pio_buff.data.len() {
                            self.pio_buff.data[self.pio_buff.index as usize] = data as u16;
                        }
                        self.pio_buff.index += 1;
                        if self.pio_buff.size == self.pio_buff.index {
                            verify!(self.pio_buff.next_state != gd_states::gds_pio_get_data);
                            self.gd_set_state(self.pio_buff.next_state);
                        }
                    }
                    _ => println!("GDROM: Illegal Write to DATA"),
                }
            }
            GD_DEVCTRL_Write => {
                println!("GDROM: Write GD_DEVCTRL (Not implemented on Dreamcast)");
            }
            GD_DRVSEL => {
                if data != 0 {
                    println!("GDROM: Write to GD_DRVSEL, !=0. Value is: {:02X}", data);
                }
                self.DriveSel = data;
            }
            GD_FEATURES_Write => {
                println_rm!("GDROM: Write to GD_FEATURES");
                self.Features.0 = data as u8;
            }
            GD_SECTCNT_Write => {
                println!("GDROM: Write to SecCount = {:X}", data);
                self.SecCount.0 = data as u8;
            }
            GD_SECTNUM => {
                println!("GDROM: Write to SecNum; not possible = {:X}", data);
            }
            GD_COMMAND_Write => {
                verify!(sz == 1);
                if (data != ATA_NOP as u32) && (data != ATA_SOFT_RESET as u32) {
                    verify!(self.gd_state == gd_states::gds_waitcmd);
                }
                self.ata_cmd.command = data as u8;
                self.gd_set_state(gd_states::gds_procata);
            }
            _ => {
                println!(
                    "GDROM: Unhandled write to address {:08X} <= {:08X}, Size:{:X}",
                    Addr, data, sz
                );
            }
        }
    }

    // ===================================================
    // Init / Reset
    // ===================================================
    pub fn init(&mut self) -> bool {
        self.gd_setdisc();
        println!("GDRomV3Impl::Init stubbed registration");
        true
    }

    pub fn reset(&mut self) {
        self.gd_reset();
    }
}


//static GDROM: Lazy<Mutex<Gdrom>> = Lazy::new(|| Mutex::new(Gdrom::default()));
static GDROM: Lazy<Mutex<GDRomV3Impl>> = Lazy::new(|| {
    Mutex::new(GDRomV3Impl::new())
});

pub fn reset() {
    if let Ok(mut gd) = GDROM.lock() {
        gd.reset();
    }
}

pub fn handles_address(addr: u32) -> bool {
    (addr & 0xFFFF_FF00) == GD_BASE
}

pub fn read(addr: u32, size: usize) -> u32 {
    let offset = addr - GD_BASE;
    if let Ok(mut gd) = GDROM.lock() {
        gd.Read(addr, size as u32)
        //gd.handle_read(offset, size)
    } else {
        0
    }
}

pub fn write(addr: u32, size: usize, value: u32) {
    let offset = addr - GD_BASE;
    if let Ok(mut gd) = GDROM.lock() {
        gd.Write(addr, value, size as u32);
        //gd.handle_write(offset, size, value);
    }
}
