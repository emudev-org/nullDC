#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use std::ptr::NonNull;

const SEND_LEVEL: [u32; 16] = [
    255,
    14 << 3,
    13 << 3,
    12 << 3,
    11 << 3,
    10 << 3,
    9 << 3,
    8 << 3,
    7 << 3,
    6 << 3,
    5 << 3,
    4 << 3,
    3 << 3,
    2 << 3,
    1 << 3,
    0 << 3,
];

const AEG_ATTACK_TIME: [f64; 64] = [
    -1.0, -1.0, 8100.0, 6900.0, 6000.0, 4800.0, 4000.0, 3400.0, 3000.0, 2400.0, 2000.0, 1700.0,
    1500.0, 1200.0, 1000.0, 860.0, 760.0, 600.0, 500.0, 430.0, 380.0, 300.0, 250.0, 220.0, 190.0,
    150.0, 130.0, 110.0, 95.0, 76.0, 63.0, 55.0, 47.0, 38.0, 31.0, 27.0, 24.0, 19.0, 15.0, 13.0,
    12.0, 9.4, 7.9, 6.8, 6.0, 4.7, 3.8, 3.4, 3.0, 2.4, 2.0, 1.8, 1.6, 1.3, 1.1, 0.93, 0.85, 0.65,
    0.53, 0.44, 0.40, 0.35, 0.0, 0.0,
];

const AEG_DSR_TIME: [f64; 64] = [
    -1.0, -1.0, 118200.0, 101300.0, 88600.0, 70900.0, 59100.0, 50700.0, 44300.0, 35500.0, 29600.0,
    25300.0, 22200.0, 17700.0, 14800.0, 12700.0, 11100.0, 8900.0, 7400.0, 6300.0, 5500.0, 4400.0,
    3700.0, 3200.0, 2800.0, 2200.0, 1800.0, 1600.0, 1400.0, 1100.0, 920.0, 790.0, 690.0, 550.0,
    460.0, 390.0, 340.0, 270.0, 230.0, 200.0, 170.0, 140.0, 110.0, 98.0, 85.0, 68.0, 57.0, 49.0,
    43.0, 34.0, 28.0, 25.0, 22.0, 18.0, 14.0, 12.0, 11.0, 8.5, 7.1, 6.1, 5.4, 4.3, 3.6, 3.1,
];

const NUM_CHANNELS: usize = 64;
const CHANNEL_REG_STRIDE: usize = 0x80;
const DSP_OUT_VOL_OFFSET: usize = 0x2000;
const COMMON_REG_OFFSET: usize = 0x2800;
const DSP_DATA_OFFSET: usize = 0x3000;

pub trait AudioStream {
    fn write_sample(&mut self, right: i16, left: i16);
}

#[derive(Debug, Default, Clone)]
pub struct AicaSettings {
    pub no_batch: bool,
    pub cdda_mute: u32,
    pub dsp_enabled: u32,
    pub no_sound: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Settings {
    pub aica: AicaSettings,
}

pub static mut SETTINGS: Settings = Settings {
    aica: AicaSettings {
        no_batch: false,
        cdda_mute: 0,
        dsp_enabled: 0,
        no_sound: false,
    },
};

#[derive(Clone, Copy)]
pub enum EgState {
    Attack = 0,
    Decay1 = 1,
    Decay2 = 2,
    Release = 3,
}

impl Default for EgState {
    fn default() -> Self {
        EgState::Attack
    }
}

#[derive(Clone, Default)]
struct EnvelopeGenerator {
    value: i32,
    state: EgState,
    attack_rate: u32,
    decay1_rate: u32,
    decay2_value: u32,
    decay2_rate: u32,
    release_rate: u32,
}

impl EnvelopeGenerator {
    fn reset(&mut self) {
        self.value = 0x3FF << AEG_STEP_BITS;
        self.state = EgState::Attack;
    }

    fn set_state(&mut self, new_state: EgState) {
        self.state = new_state;
    }

    fn value(&self) -> u32 {
        (self.value >> AEG_STEP_BITS) as u32
    }
}

#[derive(Clone, Default)]
struct LfoState {
    counter: u32,
    start_value: u32,
    state: u8,
    alfo: u32,
    alfo_shift: u8,
    plfo: u32,
    plfo_shift: u8,
}

#[derive(Clone, Default)]
struct LoopInfo {
    lsa: u32,
    lea: u32,
    looped: bool,
}

#[derive(Clone, Default)]
struct AdpcmState {
    last_quant: i32,
}

#[derive(Clone, Default)]
struct VolumeMix {
    dl_att: u32,
    dr_att: u32,
    dsp_att: u32,
    dsp_out: usize,
}

#[derive(Clone, Default)]
struct ChannelState {
    ca: u32,
    step: Fp2210,
    update_rate: u32,
    s0: i32,
    s1: i32,
    loop_info: LoopInfo,
    adpcm: AdpcmState,
    noise_state: u32,
    vol_mix: VolumeMix,
    aeg: EnvelopeGenerator,
    lfo: LfoState,
    enabled: bool,
    format: i32,
    lpctl: bool,
    lpslnk: bool,
    ssctl: bool,
    sa_addr: u32,
    prev_kyonb: bool,
}

impl ChannelState {
    fn reset(&mut self) {
        self.ca = 0;
        self.step = Fp2210 { full: 0 };
        self.update_rate = 0;
        self.s0 = 0;
        self.s1 = 0;
        self.loop_info = LoopInfo::default();
        self.adpcm.last_quant = 127;
        self.noise_state = 0;
        self.enabled = false;
        self.aeg.reset();
        self.format = 0;
        self.lpctl = false;
        self.lpslnk = false;
        self.ssctl = false;
        self.sa_addr = 0;
        self.prev_kyonb = false;
    }

    fn set_aeg_state(&mut self, new_state: EgState) {
        self.aeg.set_state(new_state);
        if matches!(new_state, EgState::Release) {
            self.enabled = false;
        }
    }

    fn key_on(&mut self) {
        self.enabled = true;
        self.set_aeg_state(EgState::Attack);
        self.aeg.value = 0x3FF << AEG_STEP_BITS;
        self.ca = 0;
        self.step = Fp2210 { full: 0 };
        self.loop_info.looped = false;
        self.adpcm.last_quant = 127;
        self.s0 = 0;
        self.s1 = 0;
    }

    fn key_off(&mut self) {
        self.set_aeg_state(EgState::Release);
    }
}

struct ChannelRegs {
    base: NonNull<u8>,
    channel_index: usize,
}

impl ChannelRegs {
    fn new(base: NonNull<u8>, channel_index: usize) -> Self {
        Self {
            base,
            channel_index,
        }
    }

    unsafe fn ptr(&self, offset: usize) -> *mut u8 {
        self.base
            .as_ptr()
            .add(self.channel_index * CHANNEL_REG_STRIDE + offset)
    }

    unsafe fn read_u8(&self, offset: usize) -> u8 {
        *self.ptr(offset)
    }

    unsafe fn write_u8(&self, offset: usize, value: u8) {
        *self.ptr(offset) = value;
    }

    unsafe fn read_u16(&self, offset: usize) -> u16 {
        let ptr = self.ptr(offset);
        u16::from_le_bytes([*ptr, *ptr.add(1)])
    }

    unsafe fn write_u16(&self, offset: usize, value: u16) {
        let ptr = self.ptr(offset);
        let bytes = value.to_le_bytes();
        *ptr = bytes[0];
        *ptr.add(1) = bytes[1];
    }

    unsafe fn read_bit_range(&self, offset: usize, shift: u8, width: u8) -> u32 {
        let mask = ((1u32 << width) - 1) << shift;
        ((self.read_u16(offset) as u32) & mask) >> shift
    }

    unsafe fn write_bit_range(&self, offset: usize, shift: u8, width: u8, value: u32) {
        let mask = ((1u32 << width) - 1) << shift;
        let mut reg = self.read_u16(offset) as u32;
        reg = (reg & !mask) | ((value << shift) & mask);
        self.write_u16(offset, reg as u16);
    }

    unsafe fn sa_low(&self) -> u32 {
        self.read_u16(0x04) as u32
    }

    unsafe fn sa_hi(&self) -> u32 {
        self.read_bit_range(0x00, 0, 7)
    }

    unsafe fn pcms(&self) -> u32 {
        self.read_bit_range(0x00, 7, 2)
    }

    unsafe fn ssctl(&self) -> u32 {
        self.read_bit_range(0x00, 10, 1)
    }

    unsafe fn lpctl(&self) -> u32 {
        self.read_bit_range(0x00, 9, 1)
    }

    unsafe fn kyonb(&self) -> bool {
        self.read_bit_range(0x00, 14, 1) != 0
    }

    unsafe fn set_kyonb(&self, value: bool) {
        self.write_bit_range(0x00, 14, 1, value as u32);
    }

    unsafe fn kyonex(&self) -> bool {
        self.read_bit_range(0x00, 15, 1) != 0
    }

    unsafe fn clear_kyonex(&self) {
        self.write_bit_range(0x00, 15, 1, 0);
    }

    unsafe fn lsa(&self) -> u32 {
        self.read_u16(0x08) as u32
    }

    unsafe fn lea(&self) -> u32 {
        self.read_u16(0x0C) as u32
    }

    unsafe fn ar(&self) -> u32 {
        self.read_bit_range(0x10, 0, 5)
    }

    unsafe fn d1r(&self) -> u32 {
        self.read_bit_range(0x10, 6, 5)
    }

    unsafe fn d2r(&self) -> u32 {
        self.read_bit_range(0x10, 11, 5)
    }

    unsafe fn rr(&self) -> u32 {
        self.read_bit_range(0x14, 0, 5)
    }

    unsafe fn dl(&self) -> u32 {
        self.read_bit_range(0x14, 5, 5)
    }

    unsafe fn krs(&self) -> u32 {
        self.read_bit_range(0x14, 10, 4)
    }

    unsafe fn lpslnk(&self) -> bool {
        self.read_bit_range(0x14, 14, 1) != 0
    }

    unsafe fn fns(&self) -> u32 {
        self.read_bit_range(0x18, 0, 10)
    }

    unsafe fn oct(&self) -> u32 {
        self.read_bit_range(0x18, 11, 4)
    }

    unsafe fn alfos(&self) -> u32 {
        self.read_bit_range(0x1C, 0, 3)
    }

    unsafe fn alfows(&self) -> u32 {
        self.read_bit_range(0x1C, 3, 2)
    }

    unsafe fn plfos(&self) -> u32 {
        self.read_bit_range(0x1C, 6, 3)
    }

    unsafe fn plfows(&self) -> u32 {
        self.read_bit_range(0x1C, 9, 2)
    }

    unsafe fn lfof(&self) -> u32 {
        self.read_bit_range(0x1C, 11, 5)
    }

    unsafe fn lfore(&self) -> bool {
        self.read_bit_range(0x1C, 15, 1) != 0
    }

    unsafe fn clear_lfore(&self) {
        self.write_bit_range(0x1C, 15, 1, 0);
    }

    unsafe fn isel(&self) -> u32 {
        self.read_bit_range(0x20, 0, 4)
    }

    unsafe fn imxl(&self) -> u32 {
        self.read_bit_range(0x20, 4, 4)
    }

    unsafe fn dipan(&self) -> u32 {
        self.read_bit_range(0x24, 0, 5)
    }

    unsafe fn disdl(&self) -> u32 {
        self.read_bit_range(0x24, 8, 4)
    }

    unsafe fn q(&self) -> u32 {
        self.read_bit_range(0x28, 0, 5)
    }

    unsafe fn tl(&self) -> u32 {
        self.read_u8(0x29) as u32
    }
}

struct CommonRegs {
    base: NonNull<u8>,
}

impl CommonRegs {
    fn new(base: NonNull<u8>) -> Self {
        Self { base }
    }

    unsafe fn ptr(&self, offset: usize) -> *mut u8 {
        self.base.as_ptr().add(COMMON_REG_OFFSET + offset)
    }

    unsafe fn read_u8(&self, offset: usize) -> u8 {
        *self.ptr(offset)
    }

    unsafe fn write_u8(&self, offset: usize, value: u8) {
        *self.ptr(offset) = value;
    }

    unsafe fn read_u16(&self, offset: usize) -> u16 {
        let ptr = self.ptr(offset);
        u16::from_le_bytes([*ptr, *ptr.add(1)])
    }

    unsafe fn write_u16(&self, offset: usize, value: u16) {
        let bytes = value.to_le_bytes();
        let ptr = self.ptr(offset);
        *ptr = bytes[0];
        *ptr.add(1) = bytes[1];
    }

    unsafe fn mvol(&self) -> u32 {
        (self.read_u16(0x00) & 0xF) as u32
    }

    unsafe fn dac18b(&self) -> bool {
        (self.read_u16(0x00) & (1 << 8)) != 0
    }

    unsafe fn mono(&self) -> bool {
        (self.read_u16(0x00) & (1 << 15)) != 0
    }

    unsafe fn set_miemp(&self, value: bool) {
        let mut reg = self.read_u16(0x08);
        if value {
            reg |= 1 << 8;
        } else {
            reg &= !(1 << 8);
        }
        self.write_u16(0x08, reg);
    }

    unsafe fn set_moemp(&self, value: bool) {
        let mut reg = self.read_u16(0x08);
        if value {
            reg |= 1 << 11;
        } else {
            reg &= !(1 << 11);
        }
        self.write_u16(0x08, reg);
    }

    unsafe fn mslc(&self) -> u32 {
        (self.read_u16(0x0C) & 0x3F) as u32
    }

    unsafe fn afset(&self) -> bool {
        (self.read_u16(0x0C) & (1 << 14)) != 0
    }

    unsafe fn set_lp(&self, value: bool) {
        let mut reg = self.read_u16(0x10);
        if value {
            reg |= 1 << 15;
        } else {
            reg &= !(1 << 15);
        }
        self.write_u16(0x10, reg);
    }

    unsafe fn set_eg(&self, value: u32) {
        let mut reg = self.read_u16(0x10) as u32;
        reg &= !0x1FFF;
        reg |= value & 0x1FFF;
        self.write_u16(0x10, reg as u16);
    }

    unsafe fn set_sgc_state(&self, value: EgState) {
        let mut reg = self.read_u16(0x10) as u32;
        reg &= !(0b11 << 13);
        reg |= (value as u32 & 0b11) << 13;
        self.write_u16(0x10, reg as u16);
    }

    unsafe fn set_ca(&self, value: u32) {
        self.write_u16(0x14, value as u16);
    }

    unsafe fn rbl(&self) -> u32 {
        ((self.read_u16(0x04) >> 13) & 0x3) as u32
    }

    unsafe fn rbp(&self) -> u32 {
        (self.read_u16(0x04) & 0x0FFF) as u32
    }
}

struct DspOutVolRegs {
    base: NonNull<u8>,
}

impl DspOutVolRegs {
    fn new(base: NonNull<u8>) -> Self {
        Self { base }
    }

    unsafe fn ptr(&self, index: usize) -> *mut u8 {
        self.base.as_ptr().add(DSP_OUT_VOL_OFFSET + index * 4)
    }

    unsafe fn read_u16(&self, index: usize) -> u16 {
        let ptr = self.ptr(index);
        u16::from_le_bytes([*ptr, *ptr.add(1)])
    }

    unsafe fn efpan(&self, index: usize) -> u32 {
        (self.read_u16(index) & 0x1F) as u32
    }

    unsafe fn efsdl(&self, index: usize) -> u32 {
        ((self.read_u16(index) >> 8) & 0xF) as u32
    }
}

struct DspDataRegs {
    base: NonNull<u8>,
}

impl DspDataRegs {
    fn new(base: NonNull<u8>) -> Self {
        Self { base }
    }

    unsafe fn efreg_ptr(&self, index: usize) -> *mut u8 {
        self.base.as_ptr().add(DSP_DATA_OFFSET + 0x1580 + index * 4)
    }

    unsafe fn read_efreg(&self, index: usize) -> i16 {
        let ptr = self.efreg_ptr(index);
        i16::from_le_bytes([*ptr, *ptr.add(1)])
    }

    unsafe fn exts_ptr(&self, index: usize) -> *mut u8 {
        self.base.as_ptr().add(DSP_DATA_OFFSET + 0x15C0 + index * 4)
    }

    unsafe fn write_exts(&self, index: usize, value: i32) {
        let ptr = self.exts_ptr(index);
        let bytes = (value as u32).to_le_bytes();
        *ptr = bytes[0];
        *ptr.add(1) = bytes[1];
        *ptr.add(2) = bytes[2];
        *ptr.add(3) = bytes[3];
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Fp2210 {
    full: u32,
}

impl Fp2210 {
    fn fractional(self) -> u32 {
        self.full & 0x3FF
    }

    fn integral(self) -> u32 {
        self.full >> 10
    }
}

const AEG_STEP_BITS: u32 = 16;

pub struct Sgc {
    audio_stream: Box<dyn AudioStream>,
    aica_reg: NonNull<u8>,
    aica_ram: NonNull<u8>,
    aica_ram_mask: u32,
    dsp: Box<DspContext>,
    channels: Vec<ChannelState>,
    volume_lut: [i32; 16],
    tl_lut: [i32; 1024],
    aeg_att_sps: [u32; 64],
    aeg_dsr_sps: [u32; 64],
}

impl Sgc {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        audio_stream: Box<dyn AudioStream>,
        aica_reg: NonNull<u8>,
        dsp: Box<DspContext>,
        aica_ram: NonNull<u8>,
        aram_size: u32,
    ) -> Self {
        let mut volume_lut = [0; 16];
        for (i, item) in volume_lut.iter_mut().enumerate() {
            if i == 0 {
                *item = 0;
            } else {
                *item = ((1 << 15) as f64 / 2f64.powf((15 - i) as f64 / 2.0)).round() as i32;
            }
        }

        let mut tl_lut = [0; 1024];
        for i in 0..256 {
            tl_lut[i] = ((1 << 15) as f64 / 2f64.powf(i as f64 / 16.0)).round() as i32;
        }

        let mut aeg_att_sps = [0; 64];
        let mut aeg_dsr_sps = [0; 64];
        for i in 0..64 {
            aeg_att_sps[i] = calc_aeg_steps(AEG_ATTACK_TIME[i]);
            aeg_dsr_sps[i] = calc_aeg_steps(AEG_DSR_TIME[i]);
        }

        let sgc = Self {
            audio_stream,
            aica_reg,
            aica_ram,
            aica_ram_mask: aram_size - 1,
            dsp,
            channels: vec![ChannelState::default(); NUM_CHANNELS],
            volume_lut,
            tl_lut,
            aeg_att_sps,
            aeg_dsr_sps,
        };

        sgc
    }

    fn step_channel(&mut self, channel_index: usize) -> (i32, i32) {
        let aica_ram = self.aica_ram;
        let aica_mask = self.aica_ram_mask;
        let tl_lut = &self.tl_lut;

        let regs = ChannelRegs::new(self.aica_reg, channel_index);
        let channel = &mut self.channels[channel_index];

        let (kyonb, lpctl, lpslnk, ssctl, pcms, tl, dipan, disdl, fns, oct);
        let mut lsa;
        let mut lea;
        let mut sa_addr;

        unsafe {
            kyonb = regs.kyonb();
            lpctl = regs.lpctl();
            lpslnk = regs.lpslnk();
            ssctl = regs.ssctl();
            pcms = regs.pcms();
            lsa = regs.lsa();
            lea = regs.lea();
            tl = regs.tl();
            dipan = regs.dipan();
            disdl = regs.disdl();
            fns = regs.fns();
            oct = regs.oct();

            sa_addr = (regs.sa_hi() << 16) | regs.sa_low();
            if pcms == 0 {
                sa_addr &= !1;
            }
        }

        if kyonb && !channel.prev_kyonb {
            channel.reset();
            channel.key_on();
        } else if !kyonb && channel.prev_kyonb {
            channel.key_off();
        }

        channel.prev_kyonb = kyonb;

        if !channel.enabled {
            return (0, 0);
        }

        let lpctl_flag = lpctl != 0;
        let ssctl_flag = ssctl != 0;

        channel.lpctl = lpctl_flag;
        channel.lpslnk = lpslnk;
        channel.ssctl = ssctl_flag;
        channel.format = if ssctl_flag { 4 } else { pcms as i32 };
        if lea == 0 {
            lea = u32::MAX;
        }
        if lsa >= lea {
            lsa = 0;
        }

        channel.loop_info.lsa = lsa;
        channel.loop_info.lea = lea;
        channel.sa_addr = sa_addr;
        channel.update_rate = Self::compute_update_rate(oct, fns);

        let sample = Self::fetch_sample(aica_ram, aica_mask, channel);
        let (mut left, mut right) = Self::apply_channel_volume(sample, tl, dipan, disdl, tl_lut);

        Self::advance_channel(channel);

        if !channel.enabled {
            left = 0;
            right = 0;
        }

        (left, right)
    }

    fn apply_master_volume(&self, mixl: &mut i32, mixr: &mut i32, common: &CommonRegs) {
        let mvol = unsafe { common.mvol().min(15) as usize };
        let vol = self.volume_lut[mvol];
        *mixl = ((i64::from(*mixl) * i64::from(vol)) >> 15) as i32;
        *mixr = ((i64::from(*mixr) * i64::from(vol)) >> 15) as i32;
    }

    fn apply_dac_18b(&self, mixl: &mut i32, mixr: &mut i32, common: &CommonRegs) {
        if unsafe { common.dac18b() } {
            *mixl >>= 2;
            *mixr >>= 2;
        }
    }

    fn clip_sample(sample: i32) -> i16 {
        sample.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }

    fn zero_dsp_mix(&mut self) {
        self.dsp.mixs.fill(0);
    }

    fn compute_update_rate(oct: u32, fns: u32) -> u32 {
        let mut update_rate = 1024 | (fns & 0x3FF);
        if (oct & 0x8) != 0 {
            let shift = (16 - (oct & 0xF)).clamp(0, 15);
            update_rate >>= shift;
        } else {
            update_rate <<= oct & 0x7;
        }
        if update_rate == 0 {
            1
        } else {
            update_rate
        }
    }

    unsafe fn read_ram_byte(aica_ram: NonNull<u8>, mask: u32, addr: u32) -> u8 {
        let masked = (addr & mask) as usize;
        *aica_ram.as_ptr().add(masked)
    }

    fn fetch_sample(aica_ram: NonNull<u8>, mask: u32, channel: &ChannelState) -> i32 {
        match channel.format {
            0 => {
                let base = channel.sa_addr.wrapping_add(channel.ca.wrapping_mul(2)) & mask;
                let lo = unsafe { Self::read_ram_byte(aica_ram, mask, base) };
                let hi = unsafe { Self::read_ram_byte(aica_ram, mask, base.wrapping_add(1)) };
                i16::from_le_bytes([lo, hi]) as i32
            }
            1 => {
                let base = channel.sa_addr.wrapping_add(channel.ca) & mask;
                let byte = unsafe { Self::read_ram_byte(aica_ram, mask, base) } as i8;
                (byte as i32) << 8
            }
            _ => 0,
        }
    }

    fn apply_channel_volume(
        sample: i32,
        tl: u32,
        dipan: u32,
        disdl: u32,
        tl_lut: &[i32; 1024],
    ) -> (i32, i32) {
        let disdl_idx = (disdl & 0xF) as usize;
        let pan_idx = ((!dipan) & 0xF) as usize;

        let att_full = tl as usize + SEND_LEVEL[disdl_idx] as usize;
        let att_pan = att_full + SEND_LEVEL[pan_idx] as usize;

        let (att_left, att_right) = if (dipan & 0x10) != 0 {
            (att_full.min(1023), att_pan.min(1023))
        } else {
            (att_pan.min(1023), att_full.min(1023))
        };

        let gain_left = tl_lut[att_left];
        let gain_right = tl_lut[att_right];

        let left = ((i64::from(sample) * i64::from(gain_left)) >> 15) as i32;
        let right = ((i64::from(sample) * i64::from(gain_right)) >> 15) as i32;

        (left, right)
    }

    fn advance_channel(channel: &mut ChannelState) {
        channel.step.full = channel.step.full.wrapping_add(channel.update_rate);
        let mut advance = channel.step.full >> 10;
        channel.step.full &= 0x3FF;

        if advance == 0 {
            advance = 1;
        }

        channel.ca = channel.ca.wrapping_add(advance);

        if channel.loop_info.lea > channel.loop_info.lsa && channel.ca >= channel.loop_info.lea {
            channel.loop_info.looped = true;
            if channel.lpctl {
                channel.ca = channel.loop_info.lsa;
            } else {
                channel.enabled = false;
                channel.ca = channel.loop_info.lea;
            }
        }
    }

    pub fn aica_sample(&mut self) {
        unsafe {
            if SETTINGS.aica.no_sound {
                return;
            }
        }

        self.zero_dsp_mix();
        let dsp_data = DspDataRegs::new(self.aica_reg);
        unsafe {
            dsp_data.write_exts(0, 0);
            dsp_data.write_exts(1, 0);
        }

        let mut mixl: i32 = 0;
        let mut mixr: i32 = 0;

        for channel_index in 0..NUM_CHANNELS {
            let (chl, chr) = self.step_channel(channel_index);
            mixl += chl;
            mixr += chr;
        }

        // CDDA input currently not implemented; mix remains unchanged.

        let common = CommonRegs::new(self.aica_reg);
        unsafe {
            if common.mono() {
                mixl += mixr;
                mixr = mixl;
            }
        }

        self.apply_master_volume(&mut mixl, &mut mixr, &common);
        self.apply_dac_18b(&mut mixl, &mut mixr, &common);

        let left = Self::clip_sample(mixl);
        let right = Self::clip_sample(mixr);

        self.audio_stream.write_sample(right, left);
    }
}

fn calc_aeg_steps(t: f64) -> u32 {
    const AEG_ALL_STEPS: f64 = 1024.0 * (1u64 << AEG_STEP_BITS) as f64 - 1.0;

    if t < 0.0 {
        return 0;
    }
    if t == 0.0 {
        return AEG_ALL_STEPS as u32;
    }

    let scnt = 44.1 * t;
    let steps = AEG_ALL_STEPS / scnt;
    (steps + 0.5) as u32
}

#[derive(Default)]
pub struct DspContext {
    pub mixs: [i32; 16],
    pub rbl: u32,
    pub rbp: u32,
    pub dyndirty: bool,
}
