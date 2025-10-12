use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::Mutex;

use crate::asic;

const GD_BASE: u32 = 0x005F_7000;
const GD_END: u32 = 0x005F_70FF;

const REG_ALT_STATUS: u32 = 0x18;
const REG_DEV_CTRL: u32 = 0x18;
const REG_DATA: u32 = 0x80;
const REG_ERROR: u32 = 0x84;
const REG_FEATURES: u32 = 0x84;
const REG_INT_REASON: u32 = 0x88;
const REG_SEC_COUNT: u32 = 0x88;
const REG_SEC_NUMBER: u32 = 0x8C;
const REG_BYTE_COUNT_LOW: u32 = 0x90;
const REG_BYTE_COUNT_HIGH: u32 = 0x94;
const REG_DRV_SEL: u32 = 0x98;
const REG_STATUS: u32 = 0x9C;
const REG_COMMAND: u32 = 0x9C;

const STATUS_CHECK: u8 = 0x01;
const STATUS_DRQ: u8 = 0x08;
const STATUS_DSC: u8 = 0x10;
const STATUS_DRDY: u8 = 0x40;
const STATUS_BSY: u8 = 0x80;

const INT_REASON_COD: u8 = 0x01;
const INT_REASON_IO: u8 = 0x02;

const GDROM_EXT_BIT: u8 = 0;

const MAX_TRANSFER_BYTES: usize = 2048;

struct Registers {
    status: u8,
    alt_status: u8,
    error: u8,
    features: u8,
    int_reason: u8,
    sector_count: u8,
    sector_number: u8,
    byte_count: u16,
    drv_sel: u8,
    command: u8,
}

impl Registers {
    fn reset(&mut self) {
        self.status = STATUS_DRDY;
        self.alt_status = self.status;
        self.error = 0;
        self.features = 0;
        self.int_reason = 0;
        self.sector_count = 0;
        self.sector_number = 0;
        self.byte_count = 0;
        self.drv_sel = 0;
        self.command = 0;
    }
}

impl Default for Registers {
    fn default() -> Self {
        let mut regs = Registers {
            status: 0,
            alt_status: 0,
            error: 0,
            features: 0,
            int_reason: 0,
            sector_count: 0,
            sector_number: 0,
            byte_count: 0,
            drv_sel: 0,
            command: 0,
        };
        regs.reset();
        regs
    }
}

#[derive(Default)]
struct StubImageReader;

impl StubImageReader {
    fn disc_present(&self) -> bool {
        false
    }

    fn read_sector(&self, _sector: u32, _buffer: &mut [u8]) {
        // no-op for now
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    ExpectPacket,
    DataIn,
    DataOut,
}

struct Gdrom {
    regs: Registers,
    data_fifo: VecDeque<u16>,
    image: StubImageReader,
    phase: Phase,
    packet: [u8; 12],
    packet_index: usize,
    sense_key: u8,
    sense_code: u8,
    sense_qual: u8,
}

impl Default for Gdrom {
    fn default() -> Self {
        Gdrom {
            regs: Registers::default(),
            data_fifo: VecDeque::new(),
            image: StubImageReader::default(),
            phase: Phase::Idle,
            packet: [0; 12],
            packet_index: 0,
            sense_key: 0,
            sense_code: 0,
            sense_qual: 0,
        }
    }
}

impl Gdrom {
    fn reset(&mut self) {
        self.regs.reset();
        self.data_fifo.clear();
        self.phase = Phase::Idle;
        self.packet_index = 0;
        self.sense_key = 0;
        self.sense_code = 0;
        self.sense_qual = 0;
    }

    fn handle_read(&mut self, offset: u32, size: usize) -> u32 {
        let mut value = match offset {
            REG_ALT_STATUS => self.regs.alt_status as u32,
            REG_DATA => self.read_data_register(),
            REG_ERROR => self.regs.error as u32,
            REG_INT_REASON => self.regs.int_reason as u32,
            REG_SEC_NUMBER => self.regs.sector_number as u32,
            REG_BYTE_COUNT_LOW => (self.regs.byte_count & 0xFF) as u32,
            REG_BYTE_COUNT_HIGH => (self.regs.byte_count >> 8) as u32,
            REG_DRV_SEL => self.regs.drv_sel as u32,
            REG_STATUS => self.current_status() as u32,
            _ => 0,
        };

        if size == 2 {
            value &= 0xFFFF;
        } else if size == 1 {
            value &= 0xFF;
        }

        value
    }

    fn handle_write(&mut self, offset: u32, size: usize, value: u32) {
        let narrowed = match size {
            1 => value as u8 as u32,
            2 => value as u16 as u32,
            _ => value,
        };

        match offset {
            REG_DEV_CTRL => self.write_dev_ctrl(narrowed as u8),
            REG_DATA => self.write_data_register(narrowed as u16),
            REG_FEATURES => self.regs.features = narrowed as u8,
            REG_SEC_COUNT => self.regs.sector_count = narrowed as u8,
            REG_SEC_NUMBER => self.regs.sector_number = narrowed as u8,
            REG_BYTE_COUNT_LOW => {
                self.regs.byte_count = (self.regs.byte_count & 0xFF00) | ((narrowed as u8) as u16);
            }
            REG_BYTE_COUNT_HIGH => {
                self.regs.byte_count =
                    (self.regs.byte_count & 0x00FF) | (((narrowed as u8) as u16) << 8);
            }
            REG_DRV_SEL => self.regs.drv_sel = narrowed as u8,
            REG_COMMAND => self.write_command(narrowed as u8),
            _ => {}
        }
    }

    fn read_data_register(&mut self) -> u32 {
        if let Some(word) = self.data_fifo.pop_front() {
            if self.data_fifo.is_empty() {
                self.complete_data_phase();
            }
            word as u32
        } else {
            0
        }
    }

    fn write_data_register(&mut self, value: u16) {
        match self.phase {
            Phase::ExpectPacket => self.push_packet_word(value),
            Phase::DataOut => {
                // Consume and ignore for now
            }
            _ => {
                if self.data_fifo.len() < 32 {
                    self.data_fifo.push_back(value);
                }
            }
        }
    }

    fn write_dev_ctrl(&mut self, value: u8) {
        let nien = (value >> 1) & 1;
        if nien != 0 {
            self.regs.status &= !STATUS_DRQ;
        }
        if (value & 0x04) != 0 {
            self.reset();
        }
    }

    fn write_command(&mut self, value: u8) {
        println!("[GDROM] ATA command 0x{:02X}", value);
        self.regs.command = value;
        self.regs.status |= STATUS_BSY;
        match value {
            0x00 | 0x08 => {
                // NOP / soft reset
                self.complete_success();
            }
            0xA0 => {
                self.enter_packet_phase();
            }
            0xA1 | 0xEC => {
                self.prepare_identify_data();
            }
            0xEF => {
                println!("[GDROM] SET FEATURES (ignored)");
                self.complete_success();
            }
            _ => {
                println!("[GDROM] Unhandled ATA command 0x{:02X}", value);
                self.complete_success();
            }
        }
    }

    fn current_status(&self) -> u8 {
        let mut status = self.regs.status;
        if self.sense_key != 0 || !self.image.disc_present() {
            status |= STATUS_CHECK;
        }
        status
    }

    fn prepare_identify_data(&mut self) {
        let mut data = [0u8; 512];
        // Word 0 : ATAPI device
        data[0] = 0x85;
        // Word 1 : logical cylinders (dummy)
        data[2] = 0x00;
        data[3] = 0x02;
        // Word 49 : capabilities (supports IORDY)
        data[98] = 0x00;
        data[99] = 0x2F;
        // Word 83 : command set supported (identify ATAPI)
        data[166] = 0x00;
        data[167] = 0x04;
        self.start_pio_transfer(&data);
        self.phase = Phase::DataIn;
    }

    fn enter_packet_phase(&mut self) {
        self.phase = Phase::ExpectPacket;
        self.packet_index = 0;
        self.packet.fill(0);
        self.data_fifo.clear();
        self.regs.byte_count = 12;
        self.regs.int_reason = INT_REASON_COD;
        self.regs.status &= !STATUS_BSY;
        self.regs.status |= STATUS_DRDY | STATUS_DRQ;
        self.regs.alt_status = self.regs.status;
        self.signal_interrupt();
    }

    fn push_packet_word(&mut self, word: u16) {
        if self.packet_index < self.packet.len() {
            self.packet[self.packet_index] = (word & 0xFF) as u8;
            self.packet_index += 1;
        }
        if self.packet_index < self.packet.len() {
            self.packet[self.packet_index] = (word >> 8) as u8;
            self.packet_index += 1;
        }

        if self.packet_index >= self.packet.len() {
            self.regs.status |= STATUS_BSY;
            self.regs.status &= !STATUS_DRQ;
            self.phase = Phase::Idle;
            self.process_packet();
        }
    }

    fn process_packet(&mut self) {
        let command = self.packet[0];
        println!("[GDROM] Packet command 0x{:02X}", command);
        match command {
            0x00 => self.cmd_test_unit_ready(),
            0x10 => self.cmd_request_status(),
            0x11 => self.cmd_request_mode(),
            0x12 => self.cmd_set_mode(),
            0x13 => self.cmd_request_error(),
            0x14 => self.cmd_get_toc(),
            0x15 => self.cmd_request_session(),
            0x20 | 0x21 | 0x22 => self.cmd_simple_ok(command),
            0x30 | 0x31 => self.cmd_read(),
            0x40 => self.cmd_request_subcode(),
            _ => {
                println!("[GDROM] Unhandled packet command 0x{:02X}", command);
                self.complete_error(0x05, 0x20, 0x00);
            }
        }
        self.packet_index = 0;
    }

    fn cmd_test_unit_ready(&mut self) {
        if self.image.disc_present() {
            self.complete_success();
        } else {
            self.complete_error(0x02, 0x3A, 0x00);
        }
    }

    fn cmd_request_status(&mut self) {
        let mut data = [0u8; 8];
        data[0] = if self.image.disc_present() {
            0x00
        } else {
            0x02
        };
        data[1] = self.sense_key;
        data[2] = self.sense_code;
        data[3] = self.sense_qual;
        self.start_pio_transfer(&data);
    }

    fn cmd_request_mode(&mut self) {
        let length = usize::from(self.packet[4]).max(8);
        let length = length.min(MAX_TRANSFER_BYTES);
        let data = vec![0u8; length];
        self.start_pio_transfer(&data);
    }

    fn cmd_set_mode(&mut self) {
        println!("[GDROM] SET MODE (parameters ignored)");
        self.complete_success();
    }

    fn cmd_request_error(&mut self) {
        let mut data = [0u8; 18];
        data[0] = 0x70; // current errors, fixed
        data[2] = self.sense_key;
        data[7] = self.sense_code;
        data[8] = self.sense_qual;
        self.start_pio_transfer(&data);
    }

    fn cmd_get_toc(&mut self) {
        let length = (((self.packet[3] as usize) << 8) | self.packet[4] as usize).max(8);
        let length = length.min(MAX_TRANSFER_BYTES);
        let data = vec![0u8; length];
        self.start_pio_transfer(&data);
    }

    fn cmd_request_session(&mut self) {
        let length = (((self.packet[3] as usize) << 8) | self.packet[4] as usize).max(8);
        let length = length.min(MAX_TRANSFER_BYTES);
        let data = vec![0u8; length];
        self.start_pio_transfer(&data);
    }

    fn cmd_simple_ok(&mut self, code: u8) {
        println!("[GDROM] Command 0x{:02X} completed (stub)", code);
        self.complete_success();
    }

    fn cmd_request_subcode(&mut self) {
        let data = vec![0u8; 96];
        self.start_pio_transfer(&data);
    }

    fn cmd_read(&mut self) {
        let sector_count = ((self.packet[8] as u32) << 16)
            | ((self.packet[9] as u32) << 8)
            | (self.packet[10] as u32);
        let blocks = if sector_count == 0 {
            0x10000
        } else {
            sector_count
        };
        let bytes = (blocks as usize).saturating_mul(2048);
        let length = bytes.min(MAX_TRANSFER_BYTES).max(2048);
        println!(
            "[GDROM] READ command sectors={} returning {} bytes of zero data (stub)",
            blocks, length
        );
        let data = vec![0u8; length];
        self.start_pio_transfer(&data);
        self.phase = Phase::DataIn;
    }

    fn start_pio_transfer(&mut self, payload: &[u8]) {
        self.data_fifo.clear();
        for chunk in payload.chunks(2) {
            let word = chunk[0] as u16 | ((chunk.get(1).copied().unwrap_or(0) as u16) << 8);
            self.data_fifo.push_back(word);
        }
        self.regs.byte_count = payload.len() as u16;
        self.regs.status &= !STATUS_BSY;
        self.regs.status |= STATUS_DRDY | STATUS_DRQ;
        self.regs.int_reason = INT_REASON_IO;
        self.regs.alt_status = self.regs.status;
        self.phase = Phase::DataIn;
        self.signal_interrupt();
    }

    fn signal_interrupt(&self) {
        asic::raise_external(GDROM_EXT_BIT);
    }

    fn clear_sense(&mut self) {
        self.sense_key = 0;
        self.sense_code = 0;
        self.sense_qual = 0;
        self.regs.error = 0;
    }

    fn set_sense(&mut self, key: u8, asc: u8, ascq: u8) {
        self.sense_key = key;
        self.sense_code = asc;
        self.sense_qual = ascq;
        self.regs.error = (key & 0x0F) << 4;
    }

    fn complete_success(&mut self) {
        self.clear_sense();
        self.regs.status &= !(STATUS_BSY | STATUS_DRQ);
        self.regs.status |= STATUS_DSC | STATUS_DRDY;
        self.regs.alt_status = self.regs.status;
        self.phase = Phase::Idle;
        self.signal_interrupt();
    }

    fn complete_error(&mut self, key: u8, asc: u8, ascq: u8) {
        self.set_sense(key, asc, ascq);
        self.regs.status &= !(STATUS_BSY | STATUS_DRQ);
        self.regs.status |= STATUS_DSC | STATUS_DRDY | STATUS_CHECK;
        self.regs.alt_status = self.regs.status;
        self.phase = Phase::Idle;
        self.signal_interrupt();
    }

    fn complete_data_phase(&mut self) {
        self.phase = Phase::Idle;
        self.clear_sense();
        self.regs.status &= !(STATUS_DRQ | STATUS_BSY);
        self.regs.status |= STATUS_DSC | STATUS_DRDY;
        self.regs.alt_status = self.regs.status;
        self.signal_interrupt();
    }
}

static GDROM: Lazy<Mutex<Gdrom>> = Lazy::new(|| Mutex::new(Gdrom::default()));

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
        gd.handle_read(offset, size)
    } else {
        0
    }
}

pub fn write(addr: u32, size: usize, value: u32) {
    let offset = addr - GD_BASE;
    if let Ok(mut gd) = GDROM.lock() {
        gd.handle_write(offset, size, value);
    }
}
