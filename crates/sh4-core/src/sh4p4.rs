#![allow(non_camel_case_types)]
#![allow(dead_code)]

use std::cell::UnsafeCell;
use std::ptr::{self, addr_of, addr_of_mut};
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::Sh4Ctx;

struct Global<T>(UnsafeCell<T>);

unsafe impl<T: Send> Sync for Global<T> {}

impl<T> Global<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    unsafe fn get(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct Reg8(u8);
    impl Debug;

    pub u8, full, set_full: 7, 0;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct Reg16(u16);
    impl Debug;

    pub u16, full, set_full: 15, 0;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct Reg32(u32);
    impl Debug;

    pub u32, full, set_full: 31, 0;
}

#[derive(Copy, Clone)]
struct TmuChannelRuntime {
    accum: u32,
    prescale: u32,
    running: bool,
}

impl TmuChannelRuntime {
    const fn new() -> Self {
        Self {
            accum: 0,
            prescale: 4,
            running: false,
        }
    }
}

struct TmuRuntime {
    channels: [TmuChannelRuntime; 3],
}

impl TmuRuntime {
    const fn new() -> Self {
        Self {
            channels: [
                TmuChannelRuntime::new(),
                TmuChannelRuntime::new(),
                TmuChannelRuntime::new(),
            ],
        }
    }
}

////// Addresses
//
// ==== CCN ====
pub const CCN_PTEH_ADDR: u32 = 0x1F000000;
pub const CCN_PTEL_ADDR: u32 = 0x1F000004;
pub const CCN_TTB_ADDR: u32 = 0x1F000008;
pub const CCN_TEA_ADDR: u32 = 0x1F00000C;
pub const CCN_MMUCR_ADDR: u32 = 0x1F000010;
pub const CCN_BASRA_ADDR: u32 = 0x1F000014;
pub const CCN_BASRB_ADDR: u32 = 0x1F000018;
pub const CCN_CCR_ADDR: u32 = 0x1F00001C;
pub const CCN_TRA_ADDR: u32 = 0x1F000020;
pub const CCN_EXPEVT_ADDR: u32 = 0x1F000024;
pub const CCN_INTEVT_ADDR: u32 = 0x1F000028;
pub const CCN_CPU_VERSION_ADDR: u32 = 0x1F000030;
pub const CCN_PTEA_ADDR: u32 = 0x1F000034;
pub const CCN_QACR0_ADDR: u32 = 0x1F000038;
pub const CCN_QACR1_ADDR: u32 = 0x1F00003C;
pub const CCN_PRR_ADDR: u32 = 0x1F000044;

//
// ==== UBC ====
pub const UBC_BARA_ADDR: u32 = 0x1F200000;
pub const UBC_BAMRA_ADDR: u32 = 0x1F200004;
pub const UBC_BBRA_ADDR: u32 = 0x1F200008;
pub const UBC_BARB_ADDR: u32 = 0x1F20000C;
pub const UBC_BAMRB_ADDR: u32 = 0x1F200010;
pub const UBC_BBRB_ADDR: u32 = 0x1F200014;
pub const UBC_BDRB_ADDR: u32 = 0x1F200018;
pub const UBC_BDMRB_ADDR: u32 = 0x1F20001C;
pub const UBC_BRCR_ADDR: u32 = 0x1F200020;

//
// ==== BSC ====
pub const BSC_BCR1_ADDR: u32 = 0x1F800000;
pub const BSC_BCR2_ADDR: u32 = 0x1F800004;
pub const BSC_WCR1_ADDR: u32 = 0x1F800008;
pub const BSC_WCR2_ADDR: u32 = 0x1F80000C;
pub const BSC_WCR3_ADDR: u32 = 0x1F800010;
pub const BSC_MCR_ADDR: u32 = 0x1F800014;
pub const BSC_PCR_ADDR: u32 = 0x1F800018;
pub const BSC_RTCSR_ADDR: u32 = 0x1F80001C;
pub const BSC_RTCNT_ADDR: u32 = 0x1F800020;
pub const BSC_RTCOR_ADDR: u32 = 0x1F800024;
pub const BSC_RFCR_ADDR: u32 = 0x1F800028;
pub const BSC_PCTRA_ADDR: u32 = 0x1F80002C;
pub const BSC_PDTRA_ADDR: u32 = 0x1F800030;
pub const BSC_PCTRB_ADDR: u32 = 0x1F800040;
pub const BSC_PDTRB_ADDR: u32 = 0x1F800044;
pub const BSC_GPIOIC_ADDR: u32 = 0x1F800048;
pub const BSC_SDMR2_ADDR: u32 = 0x1F900000;
pub const BSC_SDMR3_ADDR: u32 = 0x1F940000;

//
// ==== DMAC ====
pub const DMAC_SAR0_ADDR: u32 = 0x1FA00000;
pub const DMAC_DAR0_ADDR: u32 = 0x1FA00004;
pub const DMAC_DMATCR0_ADDR: u32 = 0x1FA00008;
pub const DMAC_CHCR0_ADDR: u32 = 0x1FA0000C;
pub const DMAC_SAR1_ADDR: u32 = 0x1FA00010;
pub const DMAC_DAR1_ADDR: u32 = 0x1FA00014;
pub const DMAC_DMATCR1_ADDR: u32 = 0x1FA00018;
pub const DMAC_CHCR1_ADDR: u32 = 0x1FA0001C;
pub const DMAC_SAR2_ADDR: u32 = 0x1FA00020;
pub const DMAC_DAR2_ADDR: u32 = 0x1FA00024;
pub const DMAC_DMATCR2_ADDR: u32 = 0x1FA00028;
pub const DMAC_CHCR2_ADDR: u32 = 0x1FA0002C;
pub const DMAC_SAR3_ADDR: u32 = 0x1FA00030;
pub const DMAC_DAR3_ADDR: u32 = 0x1FA00034;
pub const DMAC_DMATCR3_ADDR: u32 = 0x1FA00038;
pub const DMAC_CHCR3_ADDR: u32 = 0x1FA0003C;
pub const DMAC_DMAOR_ADDR: u32 = 0x1FA00040;

//
// ==== CPG ====
pub const CPG_FRQCR_ADDR: u32 = 0x1FC00000;
pub const CPG_STBCR_ADDR: u32 = 0x1FC00004;
pub const CPG_WTCNT_ADDR: u32 = 0x1FC00008;
pub const CPG_WTCSR_ADDR: u32 = 0x1FC0000C;
pub const CPG_STBCR2_ADDR: u32 = 0x1FC00010;

//
// ==== RTC ====
pub const RTC_R64CNT_ADDR: u32 = 0x1FC80000;
pub const RTC_RSECCNT_ADDR: u32 = 0x1FC80004;
pub const RTC_RMINCNT_ADDR: u32 = 0x1FC80008;
pub const RTC_RHRCNT_ADDR: u32 = 0x1FC8000C;
pub const RTC_RWKCNT_ADDR: u32 = 0x1FC80010;
pub const RTC_RDAYCNT_ADDR: u32 = 0x1FC80014;
pub const RTC_RMONCNT_ADDR: u32 = 0x1FC80018;
pub const RTC_RYRCNT_ADDR: u32 = 0x1FC8001C;
pub const RTC_RSECAR_ADDR: u32 = 0x1FC80020;
pub const RTC_RMINAR_ADDR: u32 = 0x1FC80024;
pub const RTC_RHRAR_ADDR: u32 = 0x1FC80028;
pub const RTC_RWKAR_ADDR: u32 = 0x1FC8002C;
pub const RTC_RDAYAR_ADDR: u32 = 0x1FC80030;
pub const RTC_RMONAR_ADDR: u32 = 0x1FC80034;
pub const RTC_RCR1_ADDR: u32 = 0x1FC80038;
pub const RTC_RCR2_ADDR: u32 = 0x1FC8003C;

//
// ==== INTC ====
pub const INTC_ICR_ADDR: u32 = 0x1FD00000;
pub const INTC_IPRA_ADDR: u32 = 0x1FD00004;
pub const INTC_IPRB_ADDR: u32 = 0x1FD00008;
pub const INTC_IPRC_ADDR: u32 = 0x1FD0000C;
pub const INTC_IPRD_ADDR: u32 = 0x1FD00010;

//
// ==== TMU ====
pub const TMU_TOCR_ADDR: u32 = 0x1FD80000;
pub const TMU_TSTR_ADDR: u32 = 0x1FD80004;
pub const TMU_TCOR0_ADDR: u32 = 0x1FD80008;
pub const TMU_TCNT0_ADDR: u32 = 0x1FD8000C;
pub const TMU_TCR0_ADDR: u32 = 0x1FD80010;
pub const TMU_TCOR1_ADDR: u32 = 0x1FD80014;
pub const TMU_TCNT1_ADDR: u32 = 0x1FD80018;
pub const TMU_TCR1_ADDR: u32 = 0x1FD8001C;
pub const TMU_TCOR2_ADDR: u32 = 0x1FD80020;
pub const TMU_TCNT2_ADDR: u32 = 0x1FD80024;
pub const TMU_TCR2_ADDR: u32 = 0x1FD80028;
pub const TMU_TCPR2_ADDR: u32 = 0x1FD8002C;

//
// ==== SCI ====
pub const SCI_SCSMR1_ADDR: u32 = 0x1FE00000;
pub const SCI_SCBRR1_ADDR: u32 = 0x1FE00004;
pub const SCI_SCSCR1_ADDR: u32 = 0x1FE00008;
pub const SCI_SCTDR1_ADDR: u32 = 0x1FE0000C;
pub const SCI_SCSSR1_ADDR: u32 = 0x1FE00010;
pub const SCI_SCRDR1_ADDR: u32 = 0x1FE00014;
pub const SCI_SCSCMR1_ADDR: u32 = 0x1FE00018;
pub const SCI_SCSPTR1_ADDR: u32 = 0x1FE0001C;

//
// ==== SCIF ====
pub const SCIF_SCSMR2_ADDR: u32 = 0x1FE80000;
pub const SCIF_SCBRR2_ADDR: u32 = 0x1FE80004;
pub const SCIF_SCSCR2_ADDR: u32 = 0x1FE80008;
pub const SCIF_SCFTDR2_ADDR: u32 = 0x1FE8000C;
pub const SCIF_SCFSR2_ADDR: u32 = 0x1FE80010;
pub const SCIF_SCFRDR2_ADDR: u32 = 0x1FE80014;
pub const SCIF_SCFCR2_ADDR: u32 = 0x1FE80018;
pub const SCIF_SCFDR2_ADDR: u32 = 0x1FE8001C;
pub const SCIF_SCSPTR2_ADDR: u32 = 0x1FE80020;
pub const SCIF_SCLSR2_ADDR: u32 = 0x1FE80024;

//
// ==== UDI ====
pub const UDI_SDIR_ADDR: u32 = 0x1FF00000;
pub const UDI_SDDR_ADDR: u32 = 0x1FF00008;

use bitfield::bitfield;

//
// ==== BSC ===
//

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_BCR1(u32);
    impl Debug;

    pub a56pcm, set_a56pcm: 0;
    pub res_0, _: 1;
    pub dramtp0, set_dramtp0: 2;
    pub dramtp1, set_dramtp1: 3;
    pub dramtp2, set_dramtp2: 4;
    pub a6bst0, set_a6bst0: 5;
    pub a6bst1, set_a6bst1: 6;
    pub a6bst2, set_a6bst2: 7;

    pub a5bst0, set_a5bst0: 8;
    pub a5bst1, set_a5bst1: 9;
    pub a5bst2, set_a5bst2: 10;
    pub a0bst0, set_a0bst0: 11;
    pub a0bst1, set_a0bst1: 12;
    pub a0bst2, set_a0bst2: 13;
    pub hizcnt, set_hizcnt: 14;
    pub hizmem, set_hizmem: 15;

    pub res_1, _: 16;
    pub memmpx, set_memmpx: 17;
    pub pshr, set_pshr: 18;
    pub breqen, set_breqen: 19;
    pub a4mbc, set_a4mbc: 20;
    pub a1mbc, set_a1mbc: 21;
    pub res_2, _: 22;
    pub res_3, _: 23;

    pub opup, set_opup: 24;
    pub ipup, set_ipup: 25;
    pub res_4, _: 26;
    pub res_5, _: 27;
    pub res_6, _: 28;
    pub a0mpx, set_a0mpx: 29;
    pub master, set_master: 30;
    pub endian, set_endian: 31;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_BCR2(u16);
    impl Debug;

    pub porten, set_porten: 0;
    pub res_0, _: 1;
    pub a0sz0, _: 2;     // read-only
    pub a1sz1, set_a1sz1: 3;
    pub a2sz0, set_a2sz0: 4;
    pub a2sz1, set_a2sz1: 5;
    pub a3sz0, set_a3sz0: 6;
    pub a3sz1, set_a3sz1: 7;
    pub a4sz0, set_a4sz0: 8;
    pub a4sz1, set_a4sz1: 9;
    pub a5sz0, set_a5sz0: 10;
    pub a5sz1, set_a5sz1: 11;
    pub a6sz0, set_a6sz0: 12;
    pub a6sz1, set_a6sz1: 13;
    pub a0sz0_inp, _: 14;
    pub a0sz1_inp, _: 15;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_WCR1(u32);
    impl Debug;

    pub a0iw0, set_a0iw0: 0;
    pub a0iw1, set_a0iw1: 1;
    pub a0iw2, set_a0iw2: 2;
    pub res_0, _: 3;
    pub a1iw0, set_a1iw0: 4;
    pub a1iw1, set_a1iw1: 5;
    pub a1iw2, set_a1iw2: 6;
    pub res_1, _: 7;
    pub a2iw0, set_a2iw0: 8;
    pub a2iw1, set_a2iw1: 9;
    pub a2iw2, set_a2iw2: 10;
    pub res_2, _: 11;
    pub a3iw0, set_a3iw0: 12;
    pub a3iw1, set_a3iw1: 13;
    pub a3iw2, set_a3iw2: 14;
    pub res_3, _: 15;
    pub a4iw0, set_a4iw0: 16;
    pub a4iw1, set_a4iw1: 17;
    pub a4iw2, set_a4iw2: 18;
    pub res_4, _: 19;
    pub a5iw0, set_a5iw0: 20;
    pub a5iw1, set_a5iw1: 21;
    pub a5iw2, set_a5iw2: 22;
    pub res_5, _: 23;
    pub a6iw0, set_a6iw0: 24;
    pub a6iw1, set_a6iw1: 25;
    pub a6iw2, set_a6iw2: 26;
    pub res_6, _: 27;
    pub dmaiw0, set_dmaiw0: 28;
    pub dmaiw1, set_dmaiw1: 29;
    pub dmaiw2, set_dmaiw2: 30;
    pub res_7, _: 31;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_WCR2(u32);
    impl Debug;

    pub a0b0, set_a0b0: 0;
    pub a0b1, set_a0b1: 1;
    pub a0b2, set_a0b2: 2;
    pub a0w0, set_a0w0: 3;
    pub a0w1, set_a0w1: 4;
    pub a0w2, set_a0w2: 5;
    pub a1w0, set_a1w0: 6;
    pub a1w1, set_a1w1: 7;
    pub a1w2, set_a1w2: 8;
    pub a2w0, set_a2w0: 9;
    pub a2w1, set_a2w1: 10;
    pub a2w2, set_a2w2: 11;
    pub res_0, _: 12;
    pub a3w0, set_a3w0: 13;
    pub a3w1, set_a3w1: 14;
    pub a3w2, set_a3w2: 15;
    pub res_1, _: 16;
    pub a4w0, set_a4w0: 17;
    pub a4w1, set_a4w1: 18;
    pub a4w2, set_a4w2: 19;
    pub a5b0, set_a5b0: 20;
    pub a5b1, set_a5b1: 21;
    pub a5b2, set_a5b2: 22;
    pub a5w0, set_a5w0: 23;
    pub a5w1, set_a5w1: 24;
    pub a5w2, set_a5w2: 25;
    pub a6b0, set_a6b0: 26;
    pub a6b1, set_a6b1: 27;
    pub a6b2, set_a6b2: 28;
    pub a6w0, set_a6w0: 29;
    pub a6w1, set_a6w1: 30;
    pub a6w2, set_a6w2: 31;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_WCR3(u32);
    impl Debug;

    pub a0h0, set_a0h0: 0;
    pub a0h1, set_a0h1: 1;
    pub a0s0, set_a0s0: 2;
    pub res_0, _: 3;
    pub a1h0, set_a1h0: 4;
    pub a1h1, set_a1h1: 5;
    pub a1s0, set_a1s0: 6;
    pub res_1, _: 7;
    pub a2h0, set_a2h0: 8;
    pub a2h1, set_a2h1: 9;
    pub a2s0, set_a2s0: 10;
    pub res_2, _: 11;
    pub a3h0, set_a3h0: 12;
    pub a3h1, set_a3h1: 13;
    pub a3s0, set_a3s0: 14;
    pub res_3, _: 15;
    pub a4h0, set_a4h0: 16;
    pub a4h1, set_a4h1: 17;
    pub a4s0, set_a4s0: 18;
    pub res_4, _: 19;
    pub a5h0, set_a5h0: 20;
    pub a5h1, set_a5h1: 21;
    pub a5s0, set_a5s0: 22;
    pub res_5, _: 23;
    pub a6h0, set_a6h0: 24;
    pub a6h1, set_a6h1: 25;
    pub a6s0, set_a6s0: 26;
    pub res_6, _: 31, 27;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_MCR(u32);
    impl Debug;

    pub edo_mode, set_edo_mode: 0;
    pub rmode, set_rmode: 1;
    pub rfsh, set_rfsh: 2;
    pub amx0, set_amx0: 3;
    pub amx1, set_amx1: 4;
    pub amx2, set_amx2: 5;
    pub amxext, set_amxext: 6;
    pub sz0, set_sz0: 7;
    pub sz1, set_sz1: 8;
    pub be, set_be: 9;
    pub tras, set_tras: 12, 10;
    pub trwl, set_trwl: 14, 13;
    pub rcd, set_rcd: 16, 15;
    pub tpc, set_tpc: 21, 19;
    pub tcas, set_tcas: 23;
    pub trc, set_trc: 26, 24;
    pub mrset, set_mrset: 27;
    pub rasd, set_rasd: 28;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct DMAC_CHCR(u32);
    impl Debug;

    pub de, set_de: 0;
    pub te, set_te: 1;
    pub ie, set_ie: 2;
    pub res0, _: 3;
    pub ts, set_ts: 6, 4;
    pub tm, set_tm: 7;
    pub rs, set_rs: 11, 8;
    pub sm, set_sm: 13, 12;
    pub dm, set_dm: 15, 14;
    pub al, set_al: 16;
    pub am, set_am: 17;
    pub rl, set_rl: 18;
    pub ds, set_ds: 19;
    pub res1, _: 23, 20;
    pub dtc, set_dtc: 24;
    pub dsa, set_dsa: 27, 25;
    pub stc, set_stc: 28;
    pub ssa, set_ssa: 31, 29;
}

//
// ==== BSC Peripheral Control and Timing ===
//

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_PCR(u16);
    impl Debug;

    pub a6teh0, set_a6teh0: 0;
    pub a6teh1, set_a6teh1: 1;
    pub a6teh2, set_a6teh2: 2;
    pub a5teh0, set_a5teh0: 3;
    pub a5teh1, set_a5teh1: 4;
    pub a5teh2, set_a5teh2: 5;
    pub a6ted0, set_a6ted0: 6;
    pub a6ted1, set_a6ted1: 7;
    pub a6ted2, set_a6ted2: 8;
    pub a5ted0, set_a5ted0: 9;
    pub a5ted1, set_a5ted1: 10;
    pub a5ted2, set_a5ted2: 11;
    pub a6pcw0, set_a6pcw0: 12;
    pub a6pcw1, set_a6pcw1: 13;
    pub a5pcw0, set_a5pcw0: 14;
    pub a5pcw1, set_a5pcw1: 15;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_RTCSR(u16);
    impl Debug;

    pub lmts, set_lmts: 0;
    pub ovie, set_ovie: 1;
    pub ovf, set_ovf: 2;
    pub cks, set_cks: 5, 3;
    pub cmie, set_cmie: 6;
    pub cmf, set_cmf: 7;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_RTCNT(u16);
    impl Debug;

    pub value, set_value: 7, 0;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_RTCOR(u16);
    impl Debug;

    pub value, set_value: 7, 0;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_RFCR(u16);
    impl Debug;

    pub value, set_value: 9, 0;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_PCTRA(u32);
    impl Debug;

    pub pb0io, set_pb0io: 0;
    pub pb0pup, set_pb0pup: 1;
    pub pb1io, set_pb1io: 2;
    pub pb1pup, set_pb1pup: 3;
    pub pb2io, set_pb2io: 4;
    pub pb2pup, set_pb2pup: 5;
    pub pb3io, set_pb3io: 6;
    pub pb3pup, set_pb3pup: 7;
    pub pb4io, set_pb4io: 8;
    pub pb4pup, set_pb4pup: 9;
    pub pb5io, set_pb5io: 10;
    pub pb5pup, set_pb5pup: 11;
    pub pb6io, set_pb6io: 12;
    pub pb6pup, set_pb6pup: 13;
    pub pb7io, set_pb7io: 14;
    pub pb7pup, set_pb7pup: 15;
    pub pb8io, set_pb8io: 16;
    pub pb8pup, set_pb8pup: 17;
    pub pb9io, set_pb9io: 18;
    pub pb9pup, set_pb9pup: 19;
    pub pb10io, set_pb10io: 20;
    pub pb10pup, set_pb10pup: 21;
    pub pb11io, set_pb11io: 22;
    pub pb11pup, set_pb11pup: 23;
    pub pb12io, set_pb12io: 24;
    pub pb12pup, set_pb12pup: 25;
    pub pb13io, set_pb13io: 26;
    pub pb13pup, set_pb13pup: 27;
    pub pb14io, set_pb14io: 28;
    pub pb14pup, set_pb14pup: 29;
    pub pb15io, set_pb15io: 30;
    pub pb15pup, set_pb15pup: 31;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_PCTRB(u32);
    impl Debug;

    pub pb16io, set_pb16io: 0;
    pub pb16pup, set_pb16pup: 1;
    pub pb17io, set_pb17io: 2;
    pub pb17pup, set_pb17pup: 3;
    pub pb18io, set_pb18io: 4;
    pub pb18pup, set_pb18pup: 5;
    pub pb19io, set_pb19io: 6;
    pub pb19pup, set_pb19pup: 7;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_PDTRA(u16);
    impl Debug;

    pub pb0dt, set_pb0dt: 0;
    pub pb1dt, set_pb1dt: 1;
    pub pb2dt, set_pb2dt: 2;
    pub pb3dt, set_pb3dt: 3;
    pub pb4dt, set_pb4dt: 4;
    pub pb5dt, set_pb5dt: 5;
    pub pb6dt, set_pb6dt: 6;
    pub pb7dt, set_pb7dt: 7;
    pub pb8dt, set_pb8dt: 8;
    pub pb9dt, set_pb9dt: 9;
    pub pb10dt, set_pb10dt: 10;
    pub pb11dt, set_pb11dt: 11;
    pub pb12dt, set_pb12dt: 12;
    pub pb13dt, set_pb13dt: 13;
    pub pb14dt, set_pb14dt: 14;
    pub pb15dt, set_pb15dt: 15;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_PDTRB(u16);
    impl Debug;

    pub pb16dt, set_pb16dt: 0;
    pub pb17dt, set_pb17dt: 1;
    pub pb18dt, set_pb18dt: 2;
    pub pb19dt, set_pb19dt: 3;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct BSC_GPIOIC(u16);
    impl Debug;

    pub ptiren0, set_ptiren0: 0;
    pub ptiren1, set_ptiren1: 1;
    pub ptiren2, set_ptiren2: 2;
    pub ptiren3, set_ptiren3: 3;
    pub ptiren4, set_ptiren4: 4;
    pub ptiren5, set_ptiren5: 5;
    pub ptiren6, set_ptiren6: 6;
    pub ptiren7, set_ptiren7: 7;
    pub ptiren8, set_ptiren8: 8;
    pub ptiren9, set_ptiren9: 9;
    pub ptiren10, set_ptiren10: 10;
    pub ptiren11, set_ptiren11: 11;
    pub ptiren12, set_ptiren12: 12;
    pub ptiren13, set_ptiren13: 13;
    pub ptiren14, set_ptiren14: 14;
    pub ptiren15, set_ptiren15: 15;
}

//
// ==== CCN and MMU ===
//

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_PTEH(u32);
    impl Debug;

    pub asid, set_asid: 7, 0;
    pub vpn, set_vpn: 31, 10;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_PTEL(u32);
    impl Debug;

    pub wt, set_wt: 0;
    pub sh, set_sh: 1;
    pub d, set_d: 2;
    pub c, set_c: 3;
    pub sz0, set_sz0: 4;
    pub pr, set_pr: 6, 5;
    pub sz1, set_sz1: 7;
    pub v, set_v: 8;
    pub ppn, set_ppn: 28, 10;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_PTEA(u32);
    impl Debug;

    pub sa, set_sa: 2, 0;
    pub tc, set_tc: 3;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_MMUCR(u32);
    impl Debug;

    pub at, set_at: 0;
    pub ti, set_ti: 2;
    pub sv, set_sv: 8;
    pub sqmd, set_sqmd: 9;
    pub urc, set_urc: 15, 10;
    pub urb, set_urb: 21, 16;
    pub lrui, set_lrui: 27, 22;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_CCR(u32);
    impl Debug;

    pub oce, set_oce: 0;
    pub wt, set_wt: 1;
    pub cb, set_cb: 2;
    pub oci, set_oci: 3;
    pub ora, set_ora: 5;
    pub oix, set_oix: 7;
    pub ice, set_ice: 8;
    pub ici, set_ici: 11;
    pub iix, set_iix: 15;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct CCN_QACR(u32);
    impl Debug;

    pub area, set_area: 4, 2;
}

unsafe impl Sync for CCN_QACR {}

//
// ==== INTC ===
//

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct INTC_ICR(u16);
    impl Debug;

    pub irlm, set_irlm: 7;
    pub nmie, set_nmie: 8;
    pub nmib, set_nmib: 9;
    pub mai, set_mai: 14;
    pub nmil, set_nmil: 15;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct INTC_IPRA(u16);
    impl Debug;

    pub rtc, set_rtc: 3, 0;
    pub tmu2, set_tmu2: 7, 4;
    pub tmu1, set_tmu1: 11, 8;
    pub tmu0, set_tmu0: 15, 12;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct INTC_IPRB(u16);
    impl Debug;

    pub sci1, set_sci1: 7, 4;
    pub ref_, set_ref: 11, 8;
    pub wdt, set_wdt: 15, 12;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct INTC_IPRC(u16);
    impl Debug;

    pub hitachi_udi, set_hitachi_udi: 3, 0;
    pub scif, set_scif: 7, 4;
    pub dmac, set_dmac: 11, 8;
    pub gpio, set_gpio: 15, 12;
}

//
// ==== SCIF ===
//

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCSMR2(u16);
    impl Debug;

    pub cks, set_cks: 1, 0;
    pub stop, set_stop: 3;
    pub paritymode, set_paritymode: 4;
    pub pe, set_pe: 5;
    pub chr, set_chr: 6;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCSCR2(u16);
    impl Debug;

    pub cke1, set_cke1: 1;
    pub reie, set_reie: 3;
    pub re, set_re: 4;
    pub te, set_te: 5;
    pub rie, set_rie: 6;
    pub tie, set_tie: 7;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCFSR2(u16);
    impl Debug;

    pub dr, set_dr: 0;
    pub rdf, set_rdf: 1;
    pub per, set_per: 2;
    pub fer, set_fer: 3;
    pub brk, set_brk: 4;
    pub tdfe, set_tdfe: 5;
    pub tend, set_tend: 6;
    pub er, set_er: 7;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCFCR2(u16);
    impl Debug;

    pub loopback, set_loopback: 0;
    pub rfrst, set_rfrst: 1;
    pub tfrst, set_tfrst: 2;
    pub mce, set_mce: 3;
    pub ttrg, set_ttrg: 5, 4;
    pub rtrg, set_rtrg: 7, 6;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCFDR2(u16);
    impl Debug;

    pub r, set_r: 4, 0;
    pub t, set_t: 12, 8;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCSPTR2(u16);
    impl Debug;

    pub spb2dt, set_spb2dt: 0;
    pub spb2io, set_spb2io: 1;
    pub ctsdt, set_ctsdt: 4;
    pub ctsio, set_ctsio: 5;
    pub rtsdt, set_rtsdt: 6;
    pub rtsio, set_rtsio: 7;
}

bitfield! {
    #[derive(Copy, Clone, Default)]
    pub struct SCIF_SCLSR2(u16);
    impl Debug;

    pub orer, set_orer: 0;
}

// dataz

// INTC
static mut INTC_ICR_DATA: INTC_ICR = INTC_ICR(0);
static mut INTC_IPRA_DATA: INTC_IPRA = INTC_IPRA(0);
static mut INTC_IPRB_DATA: INTC_IPRB = INTC_IPRB(0);
static mut INTC_IPRC_DATA: INTC_IPRC = INTC_IPRC(0);
static mut INTC_IPRD_DATA: Reg16 = Reg16(0);

static IRL_PRIORITY: Global<u16> = Global::new(0x0246);

const NUM_INTERRUPTS: usize = 28;
const ALL_INTERRUPTS_MASK: u32 = (1 << NUM_INTERRUPTS) - 1;

#[derive(Copy, Clone)]
enum PriorityRegister {
    Ipra,
    Iprb,
    Iprc,
    Irl,
}

#[derive(Copy, Clone)]
struct PriorityRef {
    register: PriorityRegister,
    nibble: u8,
}

impl PriorityRef {
    const fn ipra(nibble: u8) -> Self {
        Self {
            register: PriorityRegister::Ipra,
            nibble,
        }
    }

    const fn iprb(nibble: u8) -> Self {
        Self {
            register: PriorityRegister::Iprb,
            nibble,
        }
    }

    const fn iprc(nibble: u8) -> Self {
        Self {
            register: PriorityRegister::Iprc,
            nibble,
        }
    }

    const fn irl(nibble: u8) -> Self {
        Self {
            register: PriorityRegister::Irl,
            nibble,
        }
    }

    unsafe fn read(&self) -> u8 {
        match self.register {
            PriorityRegister::Ipra => unsafe {
                ((INTC_IPRA_DATA.0 >> (self.nibble as u32 * 4)) & 0xF) as u8
            },
            PriorityRegister::Iprb => unsafe {
                ((INTC_IPRB_DATA.0 >> (self.nibble as u32 * 4)) & 0xF) as u8
            },
            PriorityRegister::Iprc => unsafe {
                ((INTC_IPRC_DATA.0 >> (self.nibble as u32 * 4)) & 0xF) as u8
            },
            PriorityRegister::Irl => unsafe {
                ((*IRL_PRIORITY.get() >> (self.nibble as u32 * 4)) & 0xF) as u8
            },
        }
    }
}

#[derive(Copy, Clone)]
struct InterruptSource {
    event_code: u32,
    priority_ref: PriorityRef,
    cached_priority: u8,
}

impl InterruptSource {
    const fn empty() -> Self {
        Self {
            event_code: 0,
            priority_ref: PriorityRef::ipra(0),
            cached_priority: 0,
        }
    }

    const fn new(event_code: u32, priority_ref: PriorityRef) -> Self {
        Self {
            event_code,
            priority_ref,
            cached_priority: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct InterruptEvent {
    index: usize,
    event_code: u32,
}

struct InterruptController {
    pending: u32,
    enabled: u32,
    sources: [InterruptSource; NUM_INTERRUPTS],
    order: [usize; NUM_INTERRUPTS],
}

impl InterruptController {
    const fn new() -> Self {
        Self {
            pending: 0,
            enabled: ALL_INTERRUPTS_MASK,
            sources: [InterruptSource::empty(); NUM_INTERRUPTS],
            order: [0; NUM_INTERRUPTS],
        }
    }

    fn init(&mut self) {
        self.sources = [
            InterruptSource::new(0x0320, PriorityRef::irl(0)), // IRL_9
            InterruptSource::new(0x0360, PriorityRef::irl(1)), // IRL_11
            InterruptSource::new(0x03A0, PriorityRef::irl(2)), // IRL_13
            InterruptSource::new(0x0600, PriorityRef::iprc(0)), // HUDI
            InterruptSource::new(0x0620, PriorityRef::iprc(3)), // GPIO
            InterruptSource::new(0x0640, PriorityRef::iprc(2)), // DMTE0
            InterruptSource::new(0x0660, PriorityRef::iprc(2)), // DMTE1
            InterruptSource::new(0x0680, PriorityRef::iprc(2)), // DMTE2
            InterruptSource::new(0x06A0, PriorityRef::iprc(2)), // DMTE3
            InterruptSource::new(0x06C0, PriorityRef::iprc(2)), // DMAE
            InterruptSource::new(0x0400, PriorityRef::ipra(3)), // TMU0
            InterruptSource::new(0x0420, PriorityRef::ipra(2)), // TMU1
            InterruptSource::new(0x0440, PriorityRef::ipra(1)), // TMU2 underflow
            InterruptSource::new(0x0460, PriorityRef::ipra(1)), // TMU2 TICPI2
            InterruptSource::new(0x0480, PriorityRef::ipra(0)), // RTC alarm
            InterruptSource::new(0x04A0, PriorityRef::ipra(0)), // RTC periodic
            InterruptSource::new(0x04C0, PriorityRef::ipra(0)), // RTC carry-up
            InterruptSource::new(0x04E0, PriorityRef::iprb(1)), // SCI1 ERI
            InterruptSource::new(0x0500, PriorityRef::iprb(1)), // SCI1 RXI
            InterruptSource::new(0x0520, PriorityRef::iprb(1)), // SCI1 TXI
            InterruptSource::new(0x0540, PriorityRef::iprb(1)), // SCI1 TEI
            InterruptSource::new(0x0700, PriorityRef::iprc(1)), // SCIF ERI
            InterruptSource::new(0x0720, PriorityRef::iprc(1)), // SCIF RXI
            InterruptSource::new(0x0740, PriorityRef::iprc(1)), // SCIF BRI
            InterruptSource::new(0x0760, PriorityRef::iprc(1)), // SCIF TXI
            InterruptSource::new(0x0560, PriorityRef::iprb(3)), // WDT
            InterruptSource::new(0x0580, PriorityRef::iprb(2)), // REF RCMI
            InterruptSource::new(0x05A0, PriorityRef::ipra(2)), // REF ROVI
        ];

        for i in 0..NUM_INTERRUPTS {
            self.order[i] = i;
        }

        self.pending = 0;
        self.enabled = ALL_INTERRUPTS_MASK;
        self.update_priorities();
    }

    fn update_priorities(&mut self) {
        for i in 0..NUM_INTERRUPTS {
            self.sources[i].cached_priority = unsafe { self.sources[i].priority_ref.read() };
        }
        self.sort_sources();
    }

    fn sort_sources(&mut self) {
        for i in 1..NUM_INTERRUPTS {
            let mut j = i;
            while j > 0 {
                let prev = self.order[j - 1];
                let curr = self.order[j];
                if self.sources[prev].cached_priority <= self.sources[curr].cached_priority {
                    break;
                }
                self.order.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    #[allow(dead_code)]
    fn set_pending(&mut self, index: usize) {
        self.pending |= 1u32 << index;
    }

    #[allow(dead_code)]
    fn clear_pending(&mut self, index: usize) {
        self.pending &= !(1u32 << index);
    }

    #[allow(dead_code)]
    fn enable(&mut self, index: usize) {
        self.enabled |= 1u32 << index;
    }

    #[allow(dead_code)]
    fn disable(&mut self, index: usize) {
        self.enabled &= !(1u32 << index);
    }

    fn next_event(&mut self, sr: &crate::SrStatus) -> Option<InterruptEvent> {
        if sr.bl() {
            return None;
        }

        let mask_level = sr.imask() as u8;

        for &idx in &self.order {
            if idx >= NUM_INTERRUPTS {
                continue;
            }
            let bit = 1u32 << idx;
            if (self.pending & bit) == 0 {
                continue;
            }
            if (self.enabled & bit) == 0 {
                continue;
            }
            let priority = self.sources[idx].cached_priority;
            if priority <= mask_level {
                continue;
            }
            return Some(InterruptEvent {
                index: idx,
                event_code: self.sources[idx].event_code,
            });
        }

        None
    }
}

static INTERRUPT_CONTROLLER: Global<InterruptController> = Global::new(InterruptController::new());

type PeripheralHook = fn(*mut Sh4Ctx, u32);
static PERIPHERAL_HOOK: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

pub fn register_peripheral_hook(hook: Option<PeripheralHook>) {
    let ptr = hook.map(|f| f as *mut ()).unwrap_or(std::ptr::null_mut());
    PERIPHERAL_HOOK.store(ptr, Ordering::SeqCst);
}

fn with_controller<F, R>(f: F) -> R
where
    F: FnOnce(&mut InterruptController) -> R,
{
    unsafe {
        let controller = INTERRUPT_CONTROLLER.get();
        f(controller)
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InterruptSourceId {
    Irl9,
    Irl11,
    Irl13,
    HudiUnderflow,
    Gpio,
    DmacDmte0,
    DmacDmte1,
    DmacDmte2,
    DmacDmte3,
    DmacDmae,
    Tmu0Underflow,
    Tmu1Underflow,
    Tmu2Underflow,
    Tmu2InputCapture,
    RtcAlarm,
    RtcPeriodic,
    RtcCarry,
    Sci1Eri,
    Sci1Rxi,
    Sci1Txi,
    Sci1Tei,
    ScifEri,
    ScifRxi,
    ScifBri,
    ScifTxi,
    WdtIti,
    RefRcmi,
    RefRovi,
}

impl InterruptSourceId {
    const fn index(self) -> usize {
        self as usize
    }
}

const TMU_CHANNEL_IDS: [InterruptSourceId; 3] = [
    InterruptSourceId::Tmu0Underflow,
    InterruptSourceId::Tmu1Underflow,
    InterruptSourceId::Tmu2Underflow,
];

fn tmu_interrupt_id(ch: usize) -> InterruptSourceId {
    TMU_CHANNEL_IDS[ch]
}

pub fn intc_raise_interrupt(source: InterruptSourceId) {
    with_controller(|ctrl| ctrl.set_pending(source.index()));
}

pub fn intc_clear_interrupt(source: InterruptSourceId) {
    with_controller(|ctrl| ctrl.clear_pending(source.index()));
}

pub fn intc_enable_interrupt(source: InterruptSourceId) {
    with_controller(|ctrl| ctrl.enable(source.index()));
}

pub fn intc_disable_interrupt(source: InterruptSourceId) {
    with_controller(|ctrl| ctrl.disable(source.index()));
}

pub fn intc_priorities_changed() {
    with_controller(|ctrl| ctrl.update_priorities());
}

pub fn intc_initialize() {
    with_controller(|ctrl| ctrl.init());
}

fn compose_full_sr(ctx: *mut crate::Sh4Ctx) -> u32 {
    unsafe { (*ctx).sr.0 | (*ctx).sr_t }
}

pub(crate) unsafe fn intc_try_service(ctx: *mut crate::Sh4Ctx) -> bool {
    let event = with_controller(|ctrl| {
        let sr_ref = unsafe { &(*ctx).sr };
        let evt = ctrl.next_event(sr_ref);
        if let Some(ref evt) = evt {
            ctrl.clear_pending(evt.index);
        }
        evt
    });

    if let Some(event) = event {
        unsafe {
            ptr::write(addr_of_mut!(CCN_INTEVT_DATA), Reg32(event.event_code));

            (*ctx).ssr = compose_full_sr(ctx);
            (*ctx).spc = (*ctx).pc0;
            (*ctx).sgr = (*ctx).r[15];

            (*ctx).sr.set_bl(true);
            (*ctx).sr.set_md(true);
            let old_rb = (*ctx).sr.rb();
            (*ctx).sr.set_rb(true);
            if !old_rb {
                crate::backend_ipr::sh4_rbank_switch(
                    addr_of_mut!((*ctx).r[0]),
                    addr_of_mut!((*ctx).r_bank[0]),
                );
            }

            let vector = (*ctx).vbr.wrapping_add(0x0000_0600);
            (*ctx).pc0 = vector;
            (*ctx).pc1 = vector.wrapping_add(2);
            (*ctx).pc2 = vector.wrapping_add(4);

            (*ctx).is_delayslot0 = 0;
            (*ctx).is_delayslot1 = 0;
        }

        true
    } else {
        false
    }
}

pub(crate) fn peripherals_step(ctx: *mut crate::Sh4Ctx, cycles: u32) {
    tmu_step(ctx, cycles);
    let hook_ptr = PERIPHERAL_HOOK.load(Ordering::Relaxed);
    if !hook_ptr.is_null() {
        let hook: PeripheralHook = unsafe { std::mem::transmute(hook_ptr) };
        hook(ctx, cycles);
    }
}

// RTC
static mut RTC_R64CNT_DATA: Reg8 = Reg8(0);
static mut RTC_RSECCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RMINCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RHRCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RWKCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RDAYCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RMONCNT_DATA: Reg8 = Reg8(0);
static mut RTC_RYRCNT_DATA: Reg16 = Reg16(0);
static mut RTC_RSECAR_DATA: Reg8 = Reg8(0);
static mut RTC_RMINAR_DATA: Reg8 = Reg8(0);
static mut RTC_RHRAR_DATA: Reg8 = Reg8(0);
static mut RTC_RWKAR_DATA: Reg8 = Reg8(0);
static mut RTC_RDAYAR_DATA: Reg8 = Reg8(0);
static mut RTC_RMONAR_DATA: Reg8 = Reg8(0);
static mut RTC_RCR1_DATA: Reg8 = Reg8(0);
static mut RTC_RCR2_DATA: Reg8 = Reg8(0);

// BSC
static mut BSC_BCR1_DATA: BSC_BCR1 = BSC_BCR1(0);
static mut BSC_BCR2_DATA: BSC_BCR2 = BSC_BCR2(0);
static mut BSC_WCR1_DATA: BSC_WCR1 = BSC_WCR1(0);
static mut BSC_WCR2_DATA: BSC_WCR2 = BSC_WCR2(0);
static mut BSC_WCR3_DATA: BSC_WCR3 = BSC_WCR3(0);
static mut BSC_MCR_DATA: BSC_MCR = BSC_MCR(0);
static mut BSC_PCR_DATA: BSC_PCR = BSC_PCR(0);
static mut BSC_RTCSR_DATA: BSC_RTCSR = BSC_RTCSR(0);
static mut BSC_RTCNT_DATA: BSC_RTCNT = BSC_RTCNT(0);
static mut BSC_RTCOR_DATA: BSC_RTCOR = BSC_RTCOR(0);
static mut BSC_RFCR_DATA: BSC_RFCR = BSC_RFCR(0);
static mut BSC_PCTRA_DATA: BSC_PCTRA = BSC_PCTRA(0);
static mut BSC_PDTRA_DATA: BSC_PDTRA = BSC_PDTRA(0);
static mut BSC_PCTRB_DATA: BSC_PCTRB = BSC_PCTRB(0);
static mut BSC_PDTRB_DATA: BSC_PDTRB = BSC_PDTRB(0);
static mut BSC_GPIOIC_DATA: BSC_GPIOIC = BSC_GPIOIC(0);

// UBC
static mut UBC_BARA_DATA: Reg32 = Reg32(0);
static mut UBC_BAMRA_DATA: Reg8 = Reg8(0);
static mut UBC_BBRA_DATA: Reg16 = Reg16(0);
static mut UBC_BARB_DATA: Reg32 = Reg32(0);
static mut UBC_BAMRB_DATA: Reg8 = Reg8(0);
static mut UBC_BBRB_DATA: Reg16 = Reg16(0);
static mut UBC_BDRB_DATA: Reg32 = Reg32(0);
static mut UBC_BDMRB_DATA: Reg32 = Reg32(0);
static mut UBC_BRCR_DATA: Reg16 = Reg16(0);

// SCIF
static mut SCIF_SCSMR2_DATA: SCIF_SCSMR2 = SCIF_SCSMR2(0);
static mut SCIF_SCBRR2_DATA: Reg8 = Reg8(0);
static mut SCIF_SCSCR2_DATA: SCIF_SCSCR2 = SCIF_SCSCR2(0);
static mut SCIF_SCFTDR2_DATA: Reg8 = Reg8(0);
static mut SCIF_SCFSR2_DATA: SCIF_SCFSR2 = SCIF_SCFSR2(0);
static mut SCIF_SCFRDR2_DATA: Reg8 = Reg8(0);
static mut SCIF_SCFCR2_DATA: SCIF_SCFCR2 = SCIF_SCFCR2(0);
static mut SCIF_SCFDR2_DATA: SCIF_SCFDR2 = SCIF_SCFDR2(0);
static mut SCIF_SCSPTR2_DATA: SCIF_SCSPTR2 = SCIF_SCSPTR2(0);
static mut SCIF_SCLSR2_DATA: SCIF_SCLSR2 = SCIF_SCLSR2(0);

// TMU
static mut TMU_TOCR_DATA: Reg8 = Reg8(0);
static mut TMU_TSTR_DATA: Reg8 = Reg8(0);
static mut TMU_TCOR0_DATA: Reg32 = Reg32(0);
static mut TMU_TCNT0_DATA: Reg32 = Reg32(0);
static mut TMU_TCR0_DATA: Reg16 = Reg16(0);
static mut TMU_TCOR1_DATA: Reg32 = Reg32(0);
static mut TMU_TCNT1_DATA: Reg32 = Reg32(0);
static mut TMU_TCR1_DATA: Reg16 = Reg16(0);
static mut TMU_TCOR2_DATA: Reg32 = Reg32(0);
static mut TMU_TCNT2_DATA: Reg32 = Reg32(0);
static mut TMU_TCR2_DATA: Reg16 = Reg16(0);
static mut TMU_TCPR2_DATA: Reg32 = Reg32(0);

static TMU_RUNTIME: Global<TmuRuntime> = Global::new(TmuRuntime::new());

const TMU_UNDERFLOW: u16 = 0x0100;
const TMU_UNIE: u16 = 0x0020;
const TMU_INVALID_PRESCALE: u32 = 0;

unsafe fn tmu_get_tcor(ch: usize) -> u32 {
    match ch {
        0 => unsafe { ptr::read(addr_of!(TMU_TCOR0_DATA)) }.full(),
        1 => unsafe { ptr::read(addr_of!(TMU_TCOR1_DATA)) }.full(),
        2 => unsafe { ptr::read(addr_of!(TMU_TCOR2_DATA)) }.full(),
        _ => unreachable!("invalid TMU channel"),
    }
}

unsafe fn tmu_get_tcnt(ch: usize) -> u32 {
    match ch {
        0 => unsafe { ptr::read(addr_of!(TMU_TCNT0_DATA)) }.full(),
        1 => unsafe { ptr::read(addr_of!(TMU_TCNT1_DATA)) }.full(),
        2 => unsafe { ptr::read(addr_of!(TMU_TCNT2_DATA)) }.full(),
        _ => unreachable!("invalid TMU channel"),
    }
}

unsafe fn tmu_set_tcnt(ch: usize, value: u32) {
    match ch {
        0 => unsafe { ptr::write(addr_of_mut!(TMU_TCNT0_DATA), Reg32(value)) },
        1 => unsafe { ptr::write(addr_of_mut!(TMU_TCNT1_DATA), Reg32(value)) },
        2 => unsafe { ptr::write(addr_of_mut!(TMU_TCNT2_DATA), Reg32(value)) },
        _ => unreachable!("invalid TMU channel"),
    }
}

unsafe fn tmu_get_tcr(ch: usize) -> u16 {
    match ch {
        0 => unsafe { ptr::read(addr_of!(TMU_TCR0_DATA)) }.full(),
        1 => unsafe { ptr::read(addr_of!(TMU_TCR1_DATA)) }.full(),
        2 => unsafe { ptr::read(addr_of!(TMU_TCR2_DATA)) }.full(),
        _ => unreachable!("invalid TMU channel"),
    }
}

unsafe fn tmu_store_tcr(ch: usize, value: u16) {
    match ch {
        0 => unsafe { ptr::write(addr_of_mut!(TMU_TCR0_DATA), Reg16(value)) },
        1 => unsafe { ptr::write(addr_of_mut!(TMU_TCR1_DATA), Reg16(value)) },
        2 => unsafe { ptr::write(addr_of_mut!(TMU_TCR2_DATA), Reg16(value)) },
        _ => unreachable!("invalid TMU channel"),
    }
}

unsafe fn tmu_get_tstr() -> u8 {
    unsafe { ptr::read(addr_of!(TMU_TSTR_DATA)) }.full()
}

unsafe fn tmu_set_tstr(value: u8) {
    unsafe { ptr::write(addr_of_mut!(TMU_TSTR_DATA), Reg8(value & 0x07)) };
}

fn tmu_prescale_from_mode(mode: u16) -> u32 {
    match mode & 0x7 {
        0 => 4,
        1 => 16,
        2 => 64,
        3 => 256,
        4 => 1024,
        _ => TMU_INVALID_PRESCALE,
    }
}

fn tmu_runtime_mut() -> &'static mut TmuRuntime {
    unsafe { TMU_RUNTIME.get() }
}

fn tmu_channel_from_tcnt_ctx(ctx: *mut u8) -> usize {
    if ctx == addr_of_mut!(TMU_TCNT0_DATA) as *mut u8 {
        0
    } else if ctx == addr_of_mut!(TMU_TCNT1_DATA) as *mut u8 {
        1
    } else if ctx == addr_of_mut!(TMU_TCNT2_DATA) as *mut u8 {
        2
    } else {
        unreachable!("Invalid TMU TCNT context pointer");
    }
}

fn tmu_channel_from_tcr_ctx(ctx: *mut u8) -> usize {
    if ctx == addr_of_mut!(TMU_TCR0_DATA) as *mut u8 {
        0
    } else if ctx == addr_of_mut!(TMU_TCR1_DATA) as *mut u8 {
        1
    } else if ctx == addr_of_mut!(TMU_TCR2_DATA) as *mut u8 {
        2
    } else {
        unreachable!("Invalid TMU TCR context pointer");
    }
}

fn tmu_update_prescale(ch: usize) {
    let runtime = tmu_runtime_mut();
    let mode = unsafe { tmu_get_tcr(ch) };
    runtime.channels[ch].prescale = tmu_prescale_from_mode(mode);
    runtime.channels[ch].accum = 0;
}

fn tmu_set_running(ch: usize, running: bool) {
    let runtime = tmu_runtime_mut();
    runtime.channels[ch].running = running;
    runtime.channels[ch].accum = 0;
}

fn tmu_update_running_from_tstr() {
    let tstr = unsafe { tmu_get_tstr() };
    for ch in 0..3 {
        let running = (tstr & (1 << ch)) != 0;
        tmu_set_running(ch, running);
    }
}

fn tmu_update_interrupt_mask(ch: usize) {
    let tcr = unsafe { tmu_get_tcr(ch) };
    let id = tmu_interrupt_id(ch);
    if (tcr & TMU_UNIE) != 0 {
        intc_enable_interrupt(id);
    } else {
        intc_disable_interrupt(id);
        intc_clear_interrupt(id);
    }
}

fn tmu_handle_underflow(_ctx: *mut crate::Sh4Ctx, runtime: &mut TmuRuntime, ch: usize) {
    unsafe {
        let mut tcr = tmu_get_tcr(ch);
        tcr |= TMU_UNDERFLOW;
        tmu_store_tcr(ch, tcr);
        if (tcr & TMU_UNIE) != 0 {
            intc_raise_interrupt(tmu_interrupt_id(ch));
        }
        let reload = tmu_get_tcor(ch);
        tmu_set_tcnt(ch, reload);
        runtime.channels[ch].accum = 0;
    }
}

fn tmu_step(ctx: *mut crate::Sh4Ctx, cycles: u32) {
    if cycles == 0 {
        return;
    }

    let runtime = tmu_runtime_mut();
    for ch in 0..3 {
        if !runtime.channels[ch].running {
            continue;
        }

        let prescale = runtime.channels[ch].prescale;
        if prescale == 0 {
            continue;
        }

        runtime.channels[ch].accum = runtime.channels[ch].accum.saturating_add(cycles);
        let mut steps = runtime.channels[ch].accum / prescale;
        runtime.channels[ch].accum %= prescale;

        while steps > 0 {
            unsafe {
                let current = tmu_get_tcnt(ch);
                if current == 0 {
                    tmu_handle_underflow(ctx, runtime, ch);
                } else {
                    tmu_set_tcnt(ch, current.wrapping_sub(1));
                }
            }
            steps -= 1;
        }
    }
}

// DMAC
static mut DMAC_SAR0_DATA: Reg32 = Reg32(0);
static mut DMAC_DAR0_DATA: Reg32 = Reg32(0);
static mut DMAC_DMATCR0_DATA: Reg32 = Reg32(0);
static mut DMAC_CHCR0_DATA: DMAC_CHCR = DMAC_CHCR(0);
static mut DMAC_SAR1_DATA: Reg32 = Reg32(0);
static mut DMAC_DAR1_DATA: Reg32 = Reg32(0);
static mut DMAC_DMATCR1_DATA: Reg32 = Reg32(0);
static mut DMAC_CHCR1_DATA: DMAC_CHCR = DMAC_CHCR(0);
static mut DMAC_SAR2_DATA: Reg32 = Reg32(0);
static mut DMAC_DAR2_DATA: Reg32 = Reg32(0);
static mut DMAC_DMATCR2_DATA: Reg32 = Reg32(0);
static mut DMAC_CHCR2_DATA: DMAC_CHCR = DMAC_CHCR(0);
static mut DMAC_SAR3_DATA: Reg32 = Reg32(0);
static mut DMAC_DAR3_DATA: Reg32 = Reg32(0);
static mut DMAC_DMATCR3_DATA: Reg32 = Reg32(0);
static mut DMAC_CHCR3_DATA: DMAC_CHCR = DMAC_CHCR(0);
static mut DMAC_DMAOR_DATA: Reg32 = Reg32(0);

#[inline]
pub fn dmac_get_dmaor() -> u32 {
    unsafe { DMAC_DMAOR_DATA.0 }
}

#[inline]
pub fn dmac_get_sar(channel: usize) -> u32 {
    unsafe {
        match channel {
            0 => DMAC_SAR0_DATA.0,
            1 => DMAC_SAR1_DATA.0,
            2 => DMAC_SAR2_DATA.0,
            3 => DMAC_SAR3_DATA.0,
            _ => panic!("Invalid DMAC channel {}", channel),
        }
    }
}

#[inline]
pub fn dmac_set_sar(channel: usize, value: u32) {
    unsafe {
        match channel {
            0 => DMAC_SAR0_DATA = Reg32(value),
            1 => DMAC_SAR1_DATA = Reg32(value),
            2 => DMAC_SAR2_DATA = Reg32(value),
            3 => DMAC_SAR3_DATA = Reg32(value),
            _ => panic!("Invalid DMAC channel {}", channel),
        }
    }
}

#[inline]
pub fn dmac_get_chcr(channel: usize) -> u32 {
    unsafe {
        match channel {
            0 => DMAC_CHCR0_DATA.0,
            1 => DMAC_CHCR1_DATA.0,
            2 => DMAC_CHCR2_DATA.0,
            3 => DMAC_CHCR3_DATA.0,
            _ => panic!("Invalid DMAC channel {}", channel),
        }
    }
}

#[inline]
pub fn dmac_set_chcr(channel: usize, value: u32) {
    unsafe {
        match channel {
            0 => DMAC_CHCR0_DATA = DMAC_CHCR(value),
            1 => DMAC_CHCR1_DATA = DMAC_CHCR(value),
            2 => DMAC_CHCR2_DATA = DMAC_CHCR(value),
            3 => DMAC_CHCR3_DATA = DMAC_CHCR(value),
            _ => panic!("Invalid DMAC channel {}", channel),
        }
    }
}

#[inline]
pub fn dmac_set_dmatcr(channel: usize, value: u32) {
    unsafe {
        match channel {
            0 => DMAC_DMATCR0_DATA = Reg32(value),
            1 => DMAC_DMATCR1_DATA = Reg32(value),
            2 => DMAC_DMATCR2_DATA = Reg32(value),
            3 => DMAC_DMATCR3_DATA = Reg32(value),
            _ => panic!("Invalid DMAC channel {}", channel),
        }
    }
}

// CPG
static mut CPG_FRQCR_DATA: Reg32 = Reg32(0);
static mut CPG_STBCR_DATA: Reg8 = Reg8(0);
static mut CPG_WTCNT_DATA: Reg16 = Reg16(0);
static mut CPG_WTCSR_DATA: Reg16 = Reg16(0);
static mut CPG_STBCR2_DATA: Reg8 = Reg8(0);

// CCN
static mut CCN_PTEH_DATA: CCN_PTEH = CCN_PTEH(0);
static mut CCN_PTEL_DATA: CCN_PTEL = CCN_PTEL(0);
static mut CCN_TTB_DATA: Reg32 = Reg32(0);
static mut CCN_TEA_DATA: Reg32 = Reg32(0);
static mut CCN_MMUCR_DATA: CCN_MMUCR = CCN_MMUCR(0);
static mut CCN_BASRA_DATA: Reg8 = Reg8(0);
static mut CCN_BASRB_DATA: Reg8 = Reg8(0);
static mut CCN_CCR_DATA: CCN_CCR = CCN_CCR(0);
static mut CCN_TRA_DATA: Reg32 = Reg32(0);
static mut CCN_EXPEVT_DATA: Reg32 = Reg32(0);
static mut CCN_INTEVT_DATA: Reg32 = Reg32(0);
static mut CCN_CPU_VERSION_DATA: Reg32 = Reg32(0);
static mut CCN_PTEA_DATA: CCN_PTEA = CCN_PTEA(0);
static mut CCN_QACR0_DATA: CCN_QACR = CCN_QACR(0);
static mut CCN_QACR1_DATA: CCN_QACR = CCN_QACR(0);
static mut CCN_PRR_DATA: Reg32 = Reg32(0);

pub struct P4Register {
    pub read: fn(ctx: *mut u8, addr: u32) -> u32,
    pub write: fn(ctx: *mut u8, addr: u32, data: u32),
    pub size: u8, // in bytes
    ctx: *mut u8, // context pointer, usually the register data
}

unsafe impl Sync for P4Register {}

fn area7_unreachable_read(_ctx: *mut u8, addr: u32) -> u32 {
    panic!("Unreachable area7 read: {:08X}", addr);
}

fn area7_unreachable_write(_ctx: *mut u8, addr: u32, data: u32) {
    panic!("Unreachable area7 write: {:08X} data = {:08X}", addr, data);
}

const P4REGISTER_UNREACHABLE: P4Register = P4Register {
    read: area7_unreachable_read,
    write: area7_unreachable_write,
    size: 0,
    ctx: std::ptr::null_mut(),
};

fn area7_dram_cfg_read(_ctx: *mut u8, addr: u32) -> u32 {
    panic!("Unreachable area7_dram_cfg_read read: {:08X}", addr);
}

fn area7_dram_cfg_write(_ctx: *mut u8, addr: u32, data: u32) {
    println!(
        "area7_dram_cfg_write write: {:08X} data = {:08X}",
        addr, data
    );
}

const P4REGISTER_DRAM_CFG: P4Register = P4Register {
    read: area7_dram_cfg_read,
    write: area7_dram_cfg_write,
    size: 0,
    ctx: std::ptr::null_mut(),
};

static mut RIO_CCN: [P4Register; 18] = [P4REGISTER_UNREACHABLE; 18];
static mut RIO_UBC: [P4Register; 9] = [P4REGISTER_UNREACHABLE; 9];
static mut RIO_BSC: [P4Register; 19] = [P4REGISTER_UNREACHABLE; 19];
static mut RIO_DMAC: [P4Register; 17] = [P4REGISTER_UNREACHABLE; 17];
static mut RIO_CPG: [P4Register; 5] = [P4REGISTER_UNREACHABLE; 5];
static mut RIO_RTC: [P4Register; 16] = [P4REGISTER_UNREACHABLE; 16];
static mut RIO_INTC: [P4Register; 5] = [P4REGISTER_UNREACHABLE; 5];
static mut RIO_TMU: [P4Register; 12] = [P4REGISTER_UNREACHABLE; 12];
static mut RIO_SCI: [P4Register; 8] = [P4REGISTER_UNREACHABLE; 8];
static mut RIO_SCIF: [P4Register; 10] = [P4REGISTER_UNREACHABLE; 10];

pub fn area7_router(mut addr: u32) -> &'static P4Register {
    addr &= 0x1FFFFFFF;
    let idx: usize = ((addr / 4) & 0x3F) as usize;
    unsafe {
        match addr {
            0x1F000000..=0x1F000044 => &RIO_CCN[idx],
            0x1F200000..=0x1F200020 => &RIO_UBC[idx],
            0x1F800000..=0x1F800048 => &RIO_BSC[idx],
            0x1F900000..=0x1F90FFFF => &P4REGISTER_DRAM_CFG, // DRAM Settings 2
            0x1F940000..=0x1F94FFFF => &P4REGISTER_DRAM_CFG, // DRAM Settings 3
            0x1FA00000..=0x1FA00040 => &RIO_DMAC[idx],
            0x1FC00000..=0x1FC00010 => &RIO_CPG[idx],
            0x1FC80000..=0x1FC8003C => &RIO_RTC[idx],
            0x1FD00000..=0x1FD00010 => &RIO_INTC[idx],

            0x1FD80000..=0x1FD8002C => &RIO_TMU[idx],
            0x1FE00000..=0x1FE0001C => &RIO_SCI[idx],
            0x1FE80000..=0x1FE80024 => &RIO_SCIF[idx],

            _ => &P4REGISTER_UNREACHABLE,
        }
    }
}

pub fn p4_read<T: crate::sh4mem::MemoryData>(_ctx: *mut u8, addr: u32) -> T {
    // Bits [31:24] select the area within P4 space
    let area = (addr >> 24) & 0xFF;

    match area {
        // Store queue — unimplemented
        0xE0..=0xE3 => {
            println!("Unhandled p4 read [Store queue] 0x{:08X}", addr);
            T::default()
        }

        // F0–F1 areas — reserved / dummy reads
        0xF0 | 0xF1 => T::default(),

        // ITLB Address + Data.V
        0xF2 => {
            println!("Unhandled p4 read [ITLB Address + Data.V] 0x{:08X}", addr);
            T::default()
        }

        // ITLB Data
        0xF3 => {
            println!("Unhandled p4 read [ITLB Data] 0x{:08X}", addr);
            T::default()
        }

        // Operand cache address array (unimplemented)
        0xF4 => {
            println!(
                "Unhandled p4 read [Operand cache address array] 0x{:08X}",
                addr
            );
            T::default()
        }

        0xF5 => T::default(),

        // UTLB Address + flags
        0xF6 => {
            println!("Unhandled p4 read [UTLB Address + flags] 0x{:08X}", addr);
            T::default()
        }

        // UTLB Data
        0xF7 => {
            println!("Unhandled p4 read [UTLB Data] 0x{:08X}", addr);
            T::default()
        }

        0xFF => {
            let handler = area7_router(addr);
            if handler.size as usize != std::mem::size_of::<T>() {
                panic!(
                    "p4_read::<u{}> {:x} size mismatch, handler size = {}",
                    std::mem::size_of::<T>(),
                    addr,
                    handler.size
                );
            }
            let raw_value = (handler.read)(handler.ctx, addr);
            T::from_u32(raw_value)
        }

        _ => {
            println!("Unhandled p4 read [Reserved] 0x{:08X}", addr);
            T::default()
        }
    }
}

fn p4_write<T: crate::sh4mem::MemoryData>(_ctx: *mut u8, addr: u32, data: T) {
    // Bits [31:24] select the area within P4 space
    let area = (addr >> 24) & 0xFF;

    match area {
        // Store queue — unimplemented
        0xE0..=0xE3 => {
            println!(
                "Unhandled p4_write::<u{}> [Store queue] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }

        // F0–F1 areas — reserved / dummy reads
        0xF0 | 0xF1 => {
            // println!("Unhandled p4_write::<u{}> [reserved] {:x} data = {:x}", std::mem::size_of::<T>(), addr, vadatalue);
        }

        // ITLB Address + Data.V
        0xF2 => {
            println!(
                "Unhandled p4_write::<u{}> [ITLB Address + Data.V] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }

        // ITLB Data
        0xF3 => {
            println!(
                "Unhandled p4_write::<u{}> [ITLB Data] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }

        // Operand cache address array (unimplemented)
        0xF4 => {
            // println!(
            //     "Unhandled p4_write::<u{}> [Operand cache address array] {:x} data = {:x}",
            //     std::mem::size_of::<T>(),
            //     addr,
            //     data
            // );
        }

        0xF5 => {
            // println!("Unhandled p4_write::<u{}> [Reserved] {:x} data = {:x}", std::mem::size_of::<T>(), addr, data);
        }

        // UTLB Address + flags
        0xF6 => {
            println!(
                "Unhandled p4_write::<u{}> [UTLB Address + flags] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }

        // UTLB Data
        0xF7 => {
            println!(
                "Unhandled p4_write::<u{}> [UTLB Data] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }

        0xFF => {
            let handler = area7_router(addr);
            if handler.size != 0 && handler.size as usize != std::mem::size_of::<T>() {
                panic!(
                    "p4_write::<u{}> {:x} data = {:x} size mismatch, handler size = {}",
                    std::mem::size_of::<T>(),
                    addr,
                    data,
                    handler.size
                );
            }
            (handler.write)(handler.ctx, addr, data.to_u32());
        }

        _ => {
            println!(
                "Unhandled p4_write::<u{}> [Reserved] {:x} data = {:x}",
                std::mem::size_of::<T>(),
                addr,
                data
            );
        }
    }
}

pub const P4_HANDLERS: crate::MemHandlers = crate::MemHandlers {
    read8: p4_read::<u8>,
    read16: p4_read::<u16>,
    read32: p4_read::<u32>,
    read64: p4_read::<u64>,

    write8: p4_write::<u8>,
    write16: p4_write::<u16>,
    write32: p4_write::<u32>,
    write64: p4_write::<u64>,
};

fn sq_read_invalid<T: crate::sh4mem::MemoryData>(_ctx: *mut u8, addr: u32) -> T {
    println!("sq_read: 0x{:08X}, reads are not supported", addr);
    T::default()
}
fn sq_write_invalid<T: crate::sh4mem::MemoryData>(_ctx: *mut u8, addr: u32, data: T) {
    println!(
        "sq_write: 0x{:08X}, writes are not supported for this size, data = 0x{:X}",
        addr,
        data.to_u32()
    );
}

fn sq_write32(ctx: *mut u8, addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data;
    }
}

fn sq_write64(ctx: *mut u8, addr: u32, data: u64) {
    unsafe {
        *(ctx as *mut u64) = data;
    }
}

pub const SQ_HANDLERS: crate::MemHandlers = crate::MemHandlers {
    read8: sq_read_invalid::<u8>,
    read16: sq_read_invalid::<u16>,
    read32: sq_read_invalid::<u32>,
    read64: sq_read_invalid::<u64>,

    write8: sq_write_invalid::<u8>,
    write16: sq_write_invalid::<u16>,
    write32: sq_write32,
    write64: sq_write64,
};

fn read_data_8(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u8) as u32 }
}
fn read_data_16(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u16) as u32 }
}
fn read_data_32(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u32) }
}
fn write_data_8(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u8) = data as u8;
    }
}
fn write_data_16(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u16) = data as u16;
    }
}
fn write_data_32(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data;
    }
}

fn area7_read_only(_ctx: *mut u8, addr: u32, _data: u32) {
    #[cfg(debug_assertions)]
    eprintln!("Ignoring write to read-only area7 register {:08X}", addr);
}

fn write_intc_ipr(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        let reg = ctx as *mut u16;
        let value = data as u16;
        if *reg != value {
            *reg = value;
            intc_priorities_changed();
        }
    }
}

fn read_intc_iprd(_ctx: *mut u8, _addr: u32) -> u32 {
    0
}

fn write_bsc_pctra(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut BSC_PCTRA) = BSC_PCTRA(data);
    }
}

fn dreamcast_cable_setting() -> u32 {
    // TODO: plumb real settings once configuration is available
    3
}

fn read_bsc_pdtra(_ctx: *mut u8, _addr: u32) -> u32 {
    unsafe {
        let pctra = BSC_PCTRA_DATA.0;
        let pdtra = BSC_PDTRA_DATA.0 as u32;

        let mut tfinal = match pctra & 0xF {
            0x8 | 0xB => 3,
            0xC if (pdtra & 0xF) == 2 => 3,
            _ => 0,
        };

        if (pctra & 0xF) == 0xB && (pdtra & 0xF) == 2 {
            tfinal = 0;
        }

        tfinal |= dreamcast_cable_setting() << 8;
        tfinal
    }
}

fn write_bsc_pdtra(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut BSC_PDTRA) = BSC_PDTRA(data as u16);
    }
}

fn scif_write_transmit(ctx: *mut u8, _addr: u32, data: u32) {
    let byte = (data & 0xFF) as u8;
    unsafe {
        *(ctx as *mut u8) = byte;
    }
    #[cfg(debug_assertions)]
    eprint!("{}", byte as char);
}

fn scif_read_status(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe {
        let status = *(ctx as *mut u16) as u32;
        // Bit mask mirrors the reference implementation's ready flags (0x60 base + optional RX flag)
        status | 0x60
    }
}

fn scif_write_status(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u16) = data as u16;
    }
}

fn scif_read_data(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u8) as u32 }
}

fn scif_read_fifo_depth(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u16) as u32 }
}

fn write_tmu_tstr(_ctx: *mut u8, _addr: u32, data: u32) {
    let value = (data & 0x07) as u8;
    unsafe {
        tmu_set_tstr(value);
    }
    tmu_update_running_from_tstr();
}

fn read_tmu_tcnt(ctx: *mut u8, _addr: u32) -> u32 {
    let ch = tmu_channel_from_tcnt_ctx(ctx);
    unsafe { tmu_get_tcnt(ch) }
}

fn write_tmu_tcnt(ctx: *mut u8, _addr: u32, data: u32) {
    let ch = tmu_channel_from_tcnt_ctx(ctx);
    unsafe {
        tmu_set_tcnt(ch, data);
        let mut tcr = tmu_get_tcr(ch);
        tcr &= !TMU_UNDERFLOW;
        tmu_store_tcr(ch, tcr);
    }
    intc_clear_interrupt(tmu_interrupt_id(ch));
    let runtime = tmu_runtime_mut();
    runtime.channels[ch].accum = 0;
}

fn write_tmu_tcr(ctx: *mut u8, _addr: u32, data: u32) {
    let ch = tmu_channel_from_tcr_ctx(ctx);
    let value = (data as u16) & 0x03FF;
    unsafe {
        tmu_store_tcr(ch, value);
    }
    tmu_update_prescale(ch);
    tmu_update_interrupt_mask(ch);
    if (value & TMU_UNDERFLOW) == 0 {
        intc_clear_interrupt(tmu_interrupt_id(ch));
    }
}

fn read_tmu_tcpr2(ctx: *mut u8, _addr: u32) -> u32 {
    unsafe { *(ctx as *mut u32) }
}

fn write_tmu_tcpr2(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data;
    }
}

fn write_dmac_control(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data;
    }
}

fn write_ccn_mmucr(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data;
    }
}

fn write_ccn_ccr(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        *(ctx as *mut u32) = data & !0x300;
    }
}

fn read_ccn_qacr<const IDX: usize>(_ctx: *mut u8, _addr: u32) -> u32 {
    unsafe {
        match IDX {
            0 => CCN_QACR0_DATA.0,
            1 => CCN_QACR1_DATA.0,
            _ => unreachable!("Invalid CCN QACR index: {}", IDX),
        }
    }
}

fn write_ccn_qacr<const IDX: usize>(ctx: *mut u8, _addr: u32, data: u32) {
    unsafe {
        let sh4ctx = ctx as *mut crate::Sh4Ctx;
        let qacr = CCN_QACR(data);

        match IDX {
            0 => {
                (*sh4ctx).qacr0_base = qacr.area() << 26;
                CCN_QACR0_DATA = qacr;
            }
            1 => {
                (*sh4ctx).qacr1_base = qacr.area() << 26;
                CCN_QACR1_DATA = qacr;
            }
            _ => unreachable!("Invalid CCN QACR index: {}", IDX),
        }
    }
}

fn read_ccn_prr(_ctx: *mut u8, _addr: u32) -> u32 {
    0
}

fn initialize_default_register_values() {
    unsafe {
        // BSC defaults follow the documented reset values
        BSC_BCR1_DATA = BSC_BCR1(0x0000_0000);
        BSC_BCR2_DATA = BSC_BCR2(0x3FFC);
        BSC_WCR1_DATA = BSC_WCR1(0x7777_7777);
        BSC_WCR2_DATA = BSC_WCR2(0xFFFE_EFFF);
        BSC_WCR3_DATA = BSC_WCR3(0x0777_7777);
        BSC_MCR_DATA = BSC_MCR(0x0000_0000);
        BSC_PCR_DATA = BSC_PCR(0x0000);
        BSC_RTCSR_DATA = BSC_RTCSR(0x0000);
        BSC_RTCNT_DATA = BSC_RTCNT(0x0000);
        BSC_RTCOR_DATA = BSC_RTCOR(0x0000);
        BSC_RFCR_DATA = BSC_RFCR(0x0000);
        BSC_PCTRA_DATA = BSC_PCTRA(0x0000_0000);
        BSC_PDTRA_DATA = BSC_PDTRA(0x0000);
        BSC_PCTRB_DATA = BSC_PCTRB(0x0000_0000);
        BSC_PDTRB_DATA = BSC_PDTRB(0x0000);
        BSC_GPIOIC_DATA = BSC_GPIOIC(0x0000);

        // CCN reset values (where documented)
        CCN_CCR_DATA = CCN_CCR(0x0000_0000);
        CCN_MMUCR_DATA = CCN_MMUCR(0x0000_0000);
        CCN_QACR0_DATA = CCN_QACR(0x0000_0000);
        CCN_QACR1_DATA = CCN_QACR(0x0000_0000);
        CCN_PTEH_DATA = CCN_PTEH(0x0000_0000);
        CCN_PTEL_DATA = CCN_PTEL(0x0000_0000);
        CCN_TTB_DATA = Reg32(0x0000_0000);
        CCN_TEA_DATA = Reg32(0x0000_0000);
        CCN_BASRA_DATA = Reg8(0x00);
        CCN_BASRB_DATA = Reg8(0x00);
        CCN_TRA_DATA = Reg32(0x0000_0000);
        CCN_EXPEVT_DATA = Reg32(0x0000_0000);
        CCN_INTEVT_DATA = Reg32(0x0000_0000);
        CCN_PTEA_DATA = CCN_PTEA(0x0000_0000);
        CCN_CPU_VERSION_DATA = Reg32(0x0402_05C1);
        CCN_PRR_DATA = Reg32(0x0000_0000);

        // Minimal defaults for modules that expect specific power-on state.
        DMAC_DMAOR_DATA = Reg32(0x0000_0000);
        TMU_TOCR_DATA = Reg8(0x00);
        TMU_TSTR_DATA = Reg8(0x00);
        TMU_TCOR0_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCOR1_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCOR2_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCNT0_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCNT1_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCNT2_DATA = Reg32(0xFFFF_FFFF);
        TMU_TCR0_DATA = Reg16(0x0000);
        TMU_TCR1_DATA = Reg16(0x0000);
        TMU_TCR2_DATA = Reg16(0x0000);
        TMU_TCPR2_DATA = Reg32(0x0000_0000);
        tmu_set_tstr(0);
        for ch in 0..3 {
            tmu_update_prescale(ch);
            tmu_update_interrupt_mask(ch);
            tmu_set_running(ch, false);
        }
        RTC_RCR1_DATA = Reg8(0x00);
        RTC_RCR2_DATA = Reg8(0x00);

        // Interrupt controller defaults
        INTC_ICR_DATA = INTC_ICR(0x0000);
        INTC_IPRA_DATA = INTC_IPRA(0x0000);
        INTC_IPRB_DATA = INTC_IPRB(0x0000);
        INTC_IPRC_DATA = INTC_IPRC(0x0000);
        INTC_IPRD_DATA = Reg16(0x0000);
        *IRL_PRIORITY.get() = 0x0246;
        intc_initialize();

        // CPG defaults
        CPG_FRQCR_DATA = Reg32(0x0000_0000);
        CPG_STBCR_DATA = Reg8(0x00);
        CPG_WTCNT_DATA = Reg16(0x0000);
        CPG_WTCSR_DATA = Reg16(0x0000);
        CPG_STBCR2_DATA = Reg8(0x00);
    }
}

macro_rules! rio {
    ($mod:ident, $reg:ident, $read:expr, $write:expr, $size:expr) => {
        paste::paste! {
            rio!(
                $mod,
                $reg,
                $read,
                $write,
                $size,
                std::ptr::addr_of_mut!([<$mod _ $reg _DATA>].0)
            );
        }
    };
    ($mod:ident, $reg:ident, $read:expr, $write:expr, $size:expr, $ctx:expr) => {
        paste::paste! {
            {
                let idx = (([<$mod _ $reg _ADDR>] as usize & 0xFF) / 4);
                [<RIO_ $mod>][idx] = P4Register {
                    read: $read,
                    write: $write,
                    size: $size / 8,
                    ctx: ($ctx) as *mut u8,
                };
            }
        }
    };
}

pub fn p4_init(sh4ctx: &mut crate::Sh4Ctx) {
    unsafe {
        initialize_default_register_values();
        let sh4_ctx_ptr = sh4ctx as *mut crate::Sh4Ctx;

        /* INTC */
        rio!(INTC, ICR, read_data_16, write_data_16, 16);
        rio!(INTC, IPRA, read_data_16, write_intc_ipr, 16);
        rio!(INTC, IPRB, read_data_16, write_intc_ipr, 16);
        rio!(INTC, IPRC, read_data_16, write_intc_ipr, 16);
        rio!(INTC, IPRD, read_intc_iprd, area7_read_only, 16);

        /* RTC */
        rio!(RTC, R64CNT, read_data_8, write_data_8, 8);
        rio!(RTC, RSECCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RMINCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RHRCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RWKCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RDAYCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RMONCNT, read_data_8, write_data_8, 8);
        rio!(RTC, RYRCNT, read_data_16, write_data_16, 16);
        rio!(RTC, RSECAR, read_data_8, write_data_8, 8);
        rio!(RTC, RMINAR, read_data_8, write_data_8, 8);
        rio!(RTC, RHRAR, read_data_8, write_data_8, 8);
        rio!(RTC, RWKAR, read_data_8, write_data_8, 8);
        rio!(RTC, RDAYAR, read_data_8, write_data_8, 8);
        rio!(RTC, RMONAR, read_data_8, write_data_8, 8);
        rio!(RTC, RCR1, read_data_8, write_data_8, 8);
        rio!(RTC, RCR2, read_data_8, write_data_8, 8);

        /* BSC */
        rio!(BSC, BCR1, read_data_32, write_data_32, 32);
        rio!(BSC, BCR2, read_data_16, write_data_16, 16);
        rio!(BSC, WCR1, read_data_32, write_data_32, 32);
        rio!(BSC, WCR2, read_data_32, write_data_32, 32);
        rio!(BSC, WCR3, read_data_32, write_data_32, 32);
        rio!(BSC, MCR, read_data_32, write_data_32, 32);
        rio!(BSC, PCR, read_data_16, write_data_16, 16);
        rio!(BSC, RTCSR, read_data_16, write_data_16, 16);
        rio!(BSC, RTCNT, read_data_16, write_data_16, 16);
        rio!(BSC, RTCOR, read_data_16, write_data_16, 16);
        rio!(BSC, RFCR, read_data_16, write_data_16, 16);
        rio!(BSC, PCTRA, read_data_32, write_bsc_pctra, 32);
        rio!(BSC, PDTRA, read_bsc_pdtra, write_bsc_pdtra, 16);
        rio!(BSC, PCTRB, read_data_32, write_data_32, 32);
        rio!(BSC, PDTRB, read_data_16, write_data_16, 16);
        rio!(BSC, GPIOIC, read_data_16, write_data_16, 16);

        /* UBC */
        rio!(UBC, BARA, read_data_32, write_data_32, 32);
        rio!(UBC, BAMRA, read_data_8, write_data_8, 8);
        rio!(UBC, BBRA, read_data_16, write_data_16, 16);
        rio!(UBC, BARB, read_data_32, write_data_32, 32);
        rio!(UBC, BAMRB, read_data_8, write_data_8, 8);
        rio!(UBC, BBRB, read_data_16, write_data_16, 16);
        rio!(UBC, BDRB, read_data_32, write_data_32, 32);
        rio!(UBC, BDMRB, read_data_32, write_data_32, 32);
        rio!(UBC, BRCR, read_data_16, write_data_16, 16);

        /* SCIF */
        rio!(SCIF, SCSMR2, read_data_16, write_data_16, 16);
        rio!(SCIF, SCBRR2, read_data_8, write_data_8, 8);
        rio!(SCIF, SCSCR2, read_data_16, write_data_16, 16);
        rio!(SCIF, SCFTDR2, read_data_8, scif_write_transmit, 8);
        rio!(SCIF, SCFSR2, scif_read_status, scif_write_status, 16);
        rio!(SCIF, SCFRDR2, scif_read_data, area7_read_only, 8);
        rio!(SCIF, SCFCR2, read_data_16, write_data_16, 16);
        rio!(SCIF, SCFDR2, scif_read_fifo_depth, area7_read_only, 16);
        rio!(SCIF, SCSPTR2, read_data_16, write_data_16, 16);
        rio!(SCIF, SCLSR2, read_data_16, write_data_16, 16);

        /* TMU */
        rio!(TMU, TOCR, read_data_8, write_data_8, 8);
        rio!(TMU, TSTR, read_data_8, write_tmu_tstr, 8);
        rio!(TMU, TCOR0, read_data_32, write_data_32, 32);
        rio!(TMU, TCNT0, read_tmu_tcnt, write_tmu_tcnt, 32);
        rio!(TMU, TCR0, read_data_16, write_tmu_tcr, 16);
        rio!(TMU, TCOR1, read_data_32, write_data_32, 32);
        rio!(TMU, TCNT1, read_tmu_tcnt, write_tmu_tcnt, 32);
        rio!(TMU, TCR1, read_data_16, write_tmu_tcr, 16);
        rio!(TMU, TCOR2, read_data_32, write_data_32, 32);
        rio!(TMU, TCNT2, read_tmu_tcnt, write_tmu_tcnt, 32);
        rio!(TMU, TCR2, read_data_16, write_tmu_tcr, 16);
        rio!(TMU, TCPR2, read_tmu_tcpr2, write_tmu_tcpr2, 32);

        /* DMAC */
        rio!(DMAC, SAR0, read_data_32, write_data_32, 32);
        rio!(DMAC, DAR0, read_data_32, write_data_32, 32);
        rio!(DMAC, DMATCR0, read_data_32, write_data_32, 32);
        rio!(DMAC, CHCR0, read_data_32, write_dmac_control, 32);
        rio!(DMAC, SAR1, read_data_32, write_data_32, 32);
        rio!(DMAC, DAR1, read_data_32, write_data_32, 32);
        rio!(DMAC, DMATCR1, read_data_32, write_data_32, 32);
        rio!(DMAC, CHCR1, read_data_32, write_dmac_control, 32);
        rio!(DMAC, SAR2, read_data_32, write_data_32, 32);
        rio!(DMAC, DAR2, read_data_32, write_data_32, 32);
        rio!(DMAC, DMATCR2, read_data_32, write_data_32, 32);
        rio!(DMAC, CHCR2, read_data_32, write_dmac_control, 32);
        rio!(DMAC, SAR3, read_data_32, write_data_32, 32);
        rio!(DMAC, DAR3, read_data_32, write_data_32, 32);
        rio!(DMAC, DMATCR3, read_data_32, write_data_32, 32);
        rio!(DMAC, CHCR3, read_data_32, write_dmac_control, 32);
        rio!(DMAC, DMAOR, read_data_32, write_dmac_control, 32);

        /* CPG */
        rio!(CPG, FRQCR, read_data_16, write_data_16, 16);
        rio!(CPG, STBCR, read_data_8, write_data_8, 8);
        rio!(CPG, WTCNT, read_data_16, write_data_16, 16);
        rio!(CPG, WTCSR, read_data_16, write_data_16, 16);
        rio!(CPG, STBCR2, read_data_8, write_data_8, 8);

        /* CCN */
        rio!(CCN, PTEH, read_data_32, write_data_32, 32);
        rio!(CCN, PTEL, read_data_32, write_data_32, 32);
        rio!(CCN, TTB, read_data_32, write_data_32, 32);
        rio!(CCN, TEA, read_data_32, write_data_32, 32);
        rio!(CCN, MMUCR, read_data_32, write_ccn_mmucr, 32);
        rio!(CCN, BASRA, read_data_8, write_data_8, 8);
        rio!(CCN, BASRB, read_data_8, write_data_8, 8);
        rio!(CCN, CCR, read_data_32, write_ccn_ccr, 32);
        rio!(CCN, TRA, read_data_32, write_data_32, 32);
        rio!(CCN, EXPEVT, read_data_32, write_data_32, 32);
        rio!(CCN, INTEVT, read_data_32, write_data_32, 32);
        rio!(CCN, CPU_VERSION, read_data_32, area7_read_only, 32);
        rio!(CCN, PTEA, read_data_32, write_data_32, 32);
        rio!(
            CCN,
            QACR0,
            read_ccn_qacr::<0>,
            write_ccn_qacr::<0>,
            32,
            sh4_ctx_ptr
        );
        rio!(
            CCN,
            QACR1,
            read_ccn_qacr::<1>,
            write_ccn_qacr::<1>,
            32,
            sh4_ctx_ptr
        );
        rio!(CCN, PRR, read_ccn_prr, area7_read_only, 32);
    }
}
