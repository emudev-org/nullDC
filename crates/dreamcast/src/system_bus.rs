use crate::pvr;

// Function pointer types
pub type RegReadAddrFP = fn(ctx: *mut u32, addr: u32) -> u32;
pub type RegWriteAddrFP = fn(ctx: *mut u32, addr: u32, data: u32);

// Access flag constants
pub const REG_ACCESS_32: u32 = 4;
pub const REG_RF: u32 = 8;
pub const REG_WF: u32 = 16;
pub const REG_RO: u32 = 32;
pub const REG_WO: u32 = 64;
pub const REG_CONST: u32 = 128;
pub const REG_NO_ACCESS: u32 = REG_RO | REG_WO;

// RIO flags
pub const RIO_DATA: u32 = 0;
pub const RIO_WF: u32 = REG_WF;
pub const RIO_FUNC: u32 = REG_WF | REG_RF;
pub const RIO_RO: u32 = REG_RO | REG_WF;
pub const RIO_RO_FUNC: u32 = REG_RO | REG_RF | REG_WF;
pub const RIO_CONST: u32 = REG_RO | REG_WF;
pub const RIO_WO_FUNC: u32 = REG_WF | REG_RF | REG_WO;
pub const RIO_NO_ACCESS: u32 = REG_WF | REG_RF | REG_NO_ACCESS;

// Register entry
#[derive(Clone, Copy)]
pub struct RegisterStruct {
    pub data32: u32,
    pub read_function_addr: Option<RegReadAddrFP>,
    pub write_function_addr: Option<RegWriteAddrFP>,
    pub context: *mut u32,
    pub flags: u32,
}

impl Default for RegisterStruct {
    fn default() -> Self {
        Self {
            data32: 0,
            read_function_addr: None,
            write_function_addr: None,
            context: std::ptr::null_mut(),
            flags: 0,
        }
    }
}

pub struct SystemBus {
    pub sb_regs: Vec<RegisterStruct>,
    pub sb_ffst_rc: u32,
    pub sb_ffst: u32,
}

pub const SB_BASE: u32 = 0x005F6800;

//0x005F6800    SB_C2DSTAT  RW  ch2-DMA destination address
pub const SB_C2DSTAT_ADDR: u32 = 0x005F6800;
//0x005F6804    SB_C2DLEN   RW  ch2-DMA length
pub const SB_C2DLEN_ADDR: u32 = 0x005F6804;
//0x005F6808    SB_C2DST    RW  ch2-DMA start
pub const SB_C2DST_ADDR: u32 = 0x005F6808;

//0x005F6810    SB_SDSTAW   RW  Sort-DMA start link table address
pub const SB_SDSTAW_ADDR: u32 = 0x005F6810;
//0x005F6814    SB_SDBAAW   RW  Sort-DMA link base address
pub const SB_SDBAAW_ADDR: u32 = 0x005F6814;
//0x005F6818    SB_SDWLT    RW  Sort-DMA link address bit width
pub const SB_SDWLT_ADDR: u32 = 0x005F6818;
//0x005F681C    SB_SDLAS    RW  Sort-DMA link address shift control
pub const SB_SDLAS_ADDR: u32 = 0x005F681C;
//0x005F6820    SB_SDST RW  Sort-DMA start
pub const SB_SDST_ADDR: u32 = 0x005F6820;
//0x005F6860 SB_SDDIV R(?) Sort-DMA LAT index (guess)
pub const SB_SDDIV_ADDR: u32 = 0x005F6860;

//0x005F6840    SB_DBREQM   RW  DBREQ# signal mask control
pub const SB_DBREQM_ADDR: u32 = 0x005F6840;
//0x005F6844    SB_BAVLWC   RW  BAVL# signal wait count
pub const SB_BAVLWC_ADDR: u32 = 0x005F6844;
//0x005F6848    SB_C2DPRYC  RW  DMA (TA/Root Bus) priority count
pub const SB_C2DPRYC_ADDR: u32 = 0x005F6848;
//0x005F684C    SB_C2DMAXL  RW  ch2-DMA maximum burst length
pub const SB_C2DMAXL_ADDR: u32 = 0x005F684C;

//0x005F6880    SB_TFREM    R   TA FIFO remaining amount
pub const SB_TFREM_ADDR: u32 = 0x005F6880;
//0x005F6884    SB_LMMODE0  RW  Via TA texture memory bus select 0
pub const SB_LMMODE0_ADDR: u32 = 0x005F6884;
//0x005F6888    SB_LMMODE1  RW  Via TA texture memory bus select 1
pub const SB_LMMODE1_ADDR: u32 = 0x005F6888;
//0x005F688C    SB_FFST R   FIFO status
pub const SB_FFST_ADDR: u32 = 0x005F688C;
//0x005F6890    SB_SFRES    W   System reset
pub const SB_SFRES_ADDR: u32 = 0x005F6890;

//0x005F689C    SB_SBREV    R   System bus revision number
pub const SB_SBREV_ADDR: u32 = 0x005F689C;
//0x005F68A0    SB_RBSPLT   RW  SH4 Root Bus split enable
pub const SB_RBSPLT_ADDR: u32 = 0x005F68A0;

//0x005F6900    SB_ISTNRM   RW  Normal interrupt status
pub const SB_ISTNRM_ADDR: u32 = 0x005F6900;
//0x005F6904    SB_ISTEXT   R   External interrupt status
pub const SB_ISTEXT_ADDR: u32 = 0x005F6904;
//0x005F6908    SB_ISTERR   RW  Error interrupt status
pub const SB_ISTERR_ADDR: u32 = 0x005F6908;

//0x005F6910    SB_IML2NRM  RW  Level 2 normal interrupt mask
pub const SB_IML2NRM_ADDR: u32 = 0x005F6910;
//0x005F6914    SB_IML2EXT  RW  Level 2 external interrupt mask
pub const SB_IML2EXT_ADDR: u32 = 0x005F6914;
//0x005F6918    SB_IML2ERR  RW  Level 2 error interrupt mask
pub const SB_IML2ERR_ADDR: u32 = 0x005F6918;

//0x005F6920    SB_IML4NRM  RW  Level 4 normal interrupt mask
pub const SB_IML4NRM_ADDR: u32 = 0x005F6920;
//0x005F6924    SB_IML4EXT  RW  Level 4 external interrupt mask
pub const SB_IML4EXT_ADDR: u32 = 0x005F6924;
//0x005F6928    SB_IML4ERR  RW  Level 4 error interrupt mask
pub const SB_IML4ERR_ADDR: u32 = 0x005F6928;

//0x005F6930    SB_IML6NRM  RW  Level 6 normal interrupt mask
pub const SB_IML6NRM_ADDR: u32 = 0x005F6930;
//0x005F6934    SB_IML6EXT  RW  Level 6 external interrupt mask
pub const SB_IML6EXT_ADDR: u32 = 0x005F6934;
//0x005F6938    SB_IML6ERR  RW  Level 6 error interrupt mask
pub const SB_IML6ERR_ADDR: u32 = 0x005F6938;

//0x005F6940    SB_PDTNRM   RW  Normal interrupt PVR-DMA startup mask
pub const SB_PDTNRM_ADDR: u32 = 0x005F6940;
//0x005F6944    SB_PDTEXT   RW  External interrupt PVR-DMA startup mask
pub const SB_PDTEXT_ADDR: u32 = 0x005F6944;

//0x005F6950    SB_G2DTNRM  RW  Normal interrupt G2-DMA startup mask
pub const SB_G2DTNRM_ADDR: u32 = 0x005F6950;
//0x005F6954    SB_G2DTEXT  RW  External interrupt G2-DMA startup mask
pub const SB_G2DTEXT_ADDR: u32 = 0x005F6954;

//0x005F6C04    SB_MDSTAR   RW  Maple-DMA command table address
pub const SB_MDSTAR_ADDR: u32 = 0x005F6C04;

//0x005F6C10    SB_MDTSEL   RW  Maple-DMA trigger select
pub const SB_MDTSEL_ADDR: u32 = 0x005F6C10;
//0x005F6C14    SB_MDEN RW  Maple-DMA enable
pub const SB_MDEN_ADDR: u32 = 0x005F6C14;
//0x005F6C18    SB_MDST RW  Maple-DMA start
pub const SB_MDST_ADDR: u32 = 0x005F6C18;

//0x005F6C80    SB_MSYS RW  Maple system control
pub const SB_MSYS_ADDR: u32 = 0x005F6C80;
//0x005F6C84    SB_MST  R   Maple status
pub const SB_MST_ADDR: u32 = 0x005F6C84;
//0x005F6C88    SB_MSHTCL   W   Maple-DMA hard trigger clear
pub const SB_MSHTCL_ADDR: u32 = 0x005F6C88;
//0x005F6C8C    SB_MDAPRO   W   Maple-DMA address range
pub const SB_MDAPRO_ADDR: u32 = 0x005F6C8C;

//0x005F6CE8    SB_MMSEL    RW  Maple MSB selection
pub const SB_MMSEL_ADDR: u32 = 0x005F6CE8;

//0x005F6CF4    SB_MTXDAD   R   Maple Txd address counter
pub const SB_MTXDAD_ADDR: u32 = 0x005F6CF4;
//0x005F6CF8    SB_MRXDAD   R   Maple Rxd address counter
pub const SB_MRXDAD_ADDR: u32 = 0x005F6CF8;
//0x005F6CFC    SB_MRXDBD   R   Maple Rxd base address
pub const SB_MRXDBD_ADDR: u32 = 0x005F6CFC;

//0x005F7404    SB_GDSTAR   RW  GD-DMA start address
pub const SB_GDSTAR_ADDR: u32 = 0x005F7404;
//0x005F7408    SB_GDLEN    RW  GD-DMA length
pub const SB_GDLEN_ADDR: u32 = 0x005F7408;
//0x005F740C    SB_GDDIR    RW  GD-DMA direction
pub const SB_GDDIR_ADDR: u32 = 0x005F740C;

//0x005F7414    SB_GDEN RW  GD-DMA enable
pub const SB_GDEN_ADDR: u32 = 0x005F7414;
//0x005F7418    SB_GDST RW  GD-DMA start
pub const SB_GDST_ADDR: u32 = 0x005F7418;

//0x005F7480    SB_G1RRC    W   System ROM read access timing
pub const SB_G1RRC_ADDR: u32 = 0x005F7480;
//0x005F7484    SB_G1RWC    W   System ROM write access timing
pub const SB_G1RWC_ADDR: u32 = 0x005F7484;
//0x005F7488    SB_G1FRC    W   Flash ROM read access timing
pub const SB_G1FRC_ADDR: u32 = 0x005F7488;
//0x005F748C    SB_G1FWC    W   Flash ROM write access timing
pub const SB_G1FWC_ADDR: u32 = 0x005F748C;
//0x005F7490    SB_G1CRC    W   GD PIO read access timing
pub const SB_G1CRC_ADDR: u32 = 0x005F7490;
//0x005F7494    SB_G1CWC    W   GD PIO write access timing
pub const SB_G1CWC_ADDR: u32 = 0x005F7494;

//0x005F74A0    SB_G1GDRC   W   GD-DMA read access timing
pub const SB_G1GDRC_ADDR: u32 = 0x005F74A0;
//0x005F74A4    SB_G1GDWC   W   GD-DMA write access timing
pub const SB_G1GDWC_ADDR: u32 = 0x005F74A4;

//0x005F74B0    SB_G1SYSM   R   System mode
pub const SB_G1SYSM_ADDR: u32 = 0x005F74B0;
//0x005F74B4    SB_G1CRDYC  W   G1IORDY signal control
pub const SB_G1CRDYC_ADDR: u32 = 0x005F74B4;
//0x005F74B8    SB_GDAPRO   W   GD-DMA address range
pub const SB_GDAPRO_ADDR: u32 = 0x005F74B8;

//0x005F74F4    SB_GDSTARD  R   GD-DMA address count (on Root Bus)
pub const SB_GDSTARD_ADDR: u32 = 0x005F74F4;
//0x005F74F8    SB_GDLEND   R   GD-DMA transfer counter
pub const SB_GDLEND_ADDR: u32 = 0x005F74F8;

//0x005F7800    SB_ADSTAG   RW  AICA:G2-DMA G2 start address
pub const SB_ADSTAG_ADDR: u32 = 0x005F7800;
//0x005F7804    SB_ADSTAR   RW  AICA:G2-DMA system memory start address
pub const SB_ADSTAR_ADDR: u32 = 0x005F7804;
//0x005F7808    SB_ADLEN    RW  AICA:G2-DMA length
pub const SB_ADLEN_ADDR: u32 = 0x005F7808;
//0x005F780C    SB_ADDIR    RW  AICA:G2-DMA direction
pub const SB_ADDIR_ADDR: u32 = 0x005F780C;
//0x005F7810    SB_ADTSEL   RW  AICA:G2-DMA trigger select
pub const SB_ADTSEL_ADDR: u32 = 0x005F7810;
//0x005F7814    SB_ADEN RW  AICA:G2-DMA enable
pub const SB_ADEN_ADDR: u32 = 0x005F7814;

//0x005F7818    SB_ADST RW  AICA:G2-DMA start
pub const SB_ADST_ADDR: u32 = 0x005F7818;
//0x005F781C    SB_ADSUSP   RW  AICA:G2-DMA suspend
pub const SB_ADSUSP_ADDR: u32 = 0x005F781C;

//0x005F7820    SB_E1STAG   RW  Ext1:G2-DMA G2 start address
pub const SB_E1STAG_ADDR: u32 = 0x005F7820;
//0x005F7824    SB_E1STAR   RW  Ext1:G2-DMA system memory start address
pub const SB_E1STAR_ADDR: u32 = 0x005F7824;
//0x005F7828    SB_E1LEN    RW  Ext1:G2-DMA length
pub const SB_E1LEN_ADDR: u32 = 0x005F7828;
//0x005F782C    SB_E1DIR    RW  Ext1:G2-DMA direction
pub const SB_E1DIR_ADDR: u32 = 0x005F782C;
//0x005F7830    SB_E1TSEL   RW  Ext1:G2-DMA trigger select
pub const SB_E1TSEL_ADDR: u32 = 0x005F7830;
//0x005F7834    SB_E1EN RW  Ext1:G2-DMA enable
pub const SB_E1EN_ADDR: u32 = 0x005F7834;
//0x005F7838    SB_E1ST RW  Ext1:G2-DMA start
pub const SB_E1ST_ADDR: u32 = 0x005F7838;
//0x005F783C    SB_E1SUSP   RW  Ext1: G2-DMA suspend
pub const SB_E1SUSP_ADDR: u32 = 0x005F783C;

//0x005F7840    SB_E2STAG   RW  Ext2:G2-DMA G2 start address
pub const SB_E2STAG_ADDR: u32 = 0x005F7840;
//0x005F7844    SB_E2STAR   RW  Ext2:G2-DMA system memory start address
pub const SB_E2STAR_ADDR: u32 = 0x005F7844;
//0x005F7848    SB_E2LEN    RW  Ext2:G2-DMA length
pub const SB_E2LEN_ADDR: u32 = 0x005F7848;
//0x005F784C    SB_E2DIR    RW  Ext2:G2-DMA direction
pub const SB_E2DIR_ADDR: u32 = 0x005F784C;
//0x005F7850    SB_E2TSEL   RW  Ext2:G2-DMA trigger select
pub const SB_E2TSEL_ADDR: u32 = 0x005F7850;
//0x005F7854    SB_E2EN RW  Ext2:G2-DMA enable
pub const SB_E2EN_ADDR: u32 = 0x005F7854;
//0x005F7858    SB_E2ST RW  Ext2:G2-DMA start
pub const SB_E2ST_ADDR: u32 = 0x005F7858;
//0x005F785C    SB_E2SUSP   RW  Ext2: G2-DMA suspend
pub const SB_E2SUSP_ADDR: u32 = 0x005F785C;

//0x005F7860    SB_DDSTAG   RW  Dev:G2-DMA G2 start address
pub const SB_DDSTAG_ADDR: u32 = 0x005F7860;
//0x005F7864    SB_DDSTAR   RW  Dev:G2-DMA system memory start address
pub const SB_DDSTAR_ADDR: u32 = 0x005F7864;
//0x005F7868    SB_DDLEN    RW  Dev:G2-DMA length
pub const SB_DDLEN_ADDR: u32 = 0x005F7868;
//0x005F786C    SB_DDDIR    RW  Dev:G2-DMA direction
pub const SB_DDDIR_ADDR: u32 = 0x005F786C;
//0x005F7870    SB_DDTSEL   RW  Dev:G2-DMA trigger select
pub const SB_DDTSEL_ADDR: u32 = 0x005F7870;
//0x005F7874    SB_DDEN RW  Dev:G2-DMA enable
pub const SB_DDEN_ADDR: u32 = 0x005F7874;
//0x005F7878    SB_DDST RW  Dev:G2-DMA start
pub const SB_DDST_ADDR: u32 = 0x005F7878;
//0x005F787C    SB_DDSUSP   RW  Dev: G2-DMA suspend
pub const SB_DDSUSP_ADDR: u32 = 0x005F787C;

//0x005F7880    SB_G2ID R   G2 bus version
pub const SB_G2ID_ADDR: u32 = 0x005F7880;

//0x005F7890    SB_G2DSTO   RW  G2/DS timeout
pub const SB_G2DSTO_ADDR: u32 = 0x005F7890;
//0x005F7894 SB_G2TRTO RW G2/TR timeout
pub const SB_G2TRTO_ADDR: u32 = 0x005F7894;
//0x005F7898 SB_G2MDMTO RW Modem unit wait timeout
pub const SB_G2MDMTO_ADDR: u32 = 0x005F7898;
//0x005F789C SB_G2MDMW RW Modem unit wait time
pub const SB_G2MDMW_ADDR: u32 = 0x005F789C;

//0x005F78BC SB_G2APRO W G2-DMA address range
pub const SB_G2APRO_ADDR: u32 = 0x005F78BC;

//0x005F78C0 SB_ADSTAGD R AICA-DMA address counter (on AICA)
pub const SB_ADSTAGD_ADDR: u32 = 0x005F78C0;
//0x005F78C4 SB_ADSTARD R AICA-DMA address counter (on root bus)
pub const SB_ADSTARD_ADDR: u32 = 0x005F78C4;
//0x005F78C8 SB_ADLEND R AICA-DMA transfer counter
pub const SB_ADLEND_ADDR: u32 = 0x005F78C8;

//0x005F78D0 SB_E1STAGD R Ext-DMA1 address counter (on Ext)
pub const SB_E1STAGD_ADDR: u32 = 0x005F78D0;
//0x005F78D4 SB_E1STARD R Ext-DMA1 address counter (on root bus)
pub const SB_E1STARD_ADDR: u32 = 0x005F78D4;
//0x005F78D8 SB_E1LEND R Ext-DMA1 transfer counter
pub const SB_E1LEND_ADDR: u32 = 0x005F78D8;

//0x005F78E0 SB_E2STAGD R Ext-DMA2 address counter (on Ext)
pub const SB_E2STAGD_ADDR: u32 = 0x005F78E0;
//0x005F78E4 SB_E2STARD R Ext-DMA2 address counter (on root bus)
pub const SB_E2STARD_ADDR: u32 = 0x005F78E4;
//0x005F78E8 SB_E2LEND R Ext-DMA2 transfer counter
pub const SB_E2LEND_ADDR: u32 = 0x005F78E8;

//0x005F78F0 SB_DDSTAGD R Dev-DMA address counter (on Ext)
pub const SB_DDSTAGD_ADDR: u32 = 0x005F78F0;
//0x005F78F4 SB_DDSTARD R Dev-DMA address counter (on root bus)
pub const SB_DDSTARD_ADDR: u32 = 0x005F78F4;
//0x005F78F8 SB_DDLEND R Dev-DMA transfer counter
pub const SB_DDLEND_ADDR: u32 = 0x005F78F8;

//0x005F7C00 SB_PDSTAP RW PVR-DMA PVR start address
pub const SB_PDSTAP_ADDR: u32 = 0x005F7C00;
//0x005F7C04 SB_PDSTAR RW PVR-DMA system memory start address
pub const SB_PDSTAR_ADDR: u32 = 0x005F7C04;
//0x005F7C08 SB_PDLEN RW PVR-DMA length
pub const SB_PDLEN_ADDR: u32 = 0x005F7C08;
//0x005F7C0C SB_PDDIR RW PVR-DMA direction
pub const SB_PDDIR_ADDR: u32 = 0x005F7C0C;
//0x005F7C10 SB_PDTSEL RW PVR-DMA trigger select
pub const SB_PDTSEL_ADDR: u32 = 0x005F7C10;
//0x005F7C14 SB_PDEN RW PVR-DMA enable
pub const SB_PDEN_ADDR: u32 = 0x005F7C14;
//0x005F7C18 SB_PDST RW PVR-DMA start
pub const SB_PDST_ADDR: u32 = 0x005F7C18;

//0x005F7C80 SB_PDAPRO W PVR-DMA address range
pub const SB_PDAPRO_ADDR: u32 = 0x005F7C80;

//0x005F7CF0 SB_PDSTAPD R PVR-DMA address counter (on Ext)
pub const SB_PDSTAPD_ADDR: u32 = 0x005F7CF0;
//0x005F7CF4 SB_PDSTARD R PVR-DMA address counter (on root bus)
pub const SB_PDSTARD_ADDR: u32 = 0x005F7CF4;
//0x005F7CF8 SB_PDLEND R PVR-DMA transfer counter
pub const SB_PDLEND_ADDR: u32 = 0x005F7CF8;

unsafe impl Sync for SystemBus {}

impl SystemBus {
    pub(crate) fn reg_index(addr: u32) -> usize {
        ((addr - SB_BASE) >> 2) as usize
    }

    pub(crate) fn load_reg(&self, addr: u32) -> u32 {
        let idx = Self::reg_index(addr);
        self.sb_regs[idx].data32
    }

    pub(crate) fn store_reg(&mut self, addr: u32, value: u32) {
        let idx = Self::reg_index(addr);
        self.sb_regs[idx].data32 = value;
    }

    pub fn regn32(&mut self, addr: u32) -> *mut u32 {
        let idx = ((addr - SB_BASE) / 4) as usize;
        &mut self.sb_regs[idx].data32
    }

    #[inline(always)]
    fn sbio_read_noacc(_ctx: *mut u32, _addr: u32) -> u32 {
        panic!("Invalid system bus read access");
    }

    #[inline(always)]
    fn sbio_write_noacc(_ctx: *mut u32, _addr: u32, _data: u32) {
        panic!("Invalid system bus write access");
    }

    #[inline(always)]
    fn sbio_write_const(_ctx: *mut u32, _addr: u32, _data: u32) {
        panic!("Attempted to write to const SB register");
    }

    #[inline(always)]
    fn sbio_write_zero(_ctx: *mut u32, _addr: u32, data: u32) {
        assert!(data == 0);
    }

    #[inline(always)]
    fn sbio_write_gdrom_unlock(_ctx: *mut u32, _addr: u32, data: u32) {
        assert!(
            data == 0 || data == 0x001fffff || data == 0x42fe || data == 0xa677 || data == 0x3ff
        );
    }

    #[inline(always)]
    fn sbio_writeonly(ctx: *mut u32, _addr: u32, data: u32) {
        unsafe {
            *(ctx) = data;
        }
    }

    #[inline(always)]
    fn sbio_writeonly_gdrom_protection(ctx: *mut u32, _addr: u32, data: u32) {
        unsafe {
            *(ctx) = data;
        }
    }

    fn sb_ffst_read(ctx: *mut u32, _addr: u32) -> u32 {
        let sb = unsafe { &mut *(ctx as *mut SystemBus) };
        sb.sb_ffst_rc = sb.sb_ffst_rc.wrapping_add(1);
        if (sb.sb_ffst_rc & 0x8) != 0 {
            sb.sb_ffst ^= 31;
        }
        sb.sb_ffst
    }

    fn sb_sfres_write(ctx: *mut u32, _addr: u32, data: u32) {
        if (data & 0xFFFF) == 0x7611 {
            println!("SB/HOLLY: System reset requested");
            // virtualDreamcast.RequestReset();
        }
    }

    pub const fn default() -> Self {
        Self {
            sb_regs: Vec::new(),
            sb_ffst_rc: 0,
            sb_ffst: 0,
        }
    }

    pub fn setup(&mut self) {
        self.sb_regs = vec![RegisterStruct::default(); 0x540];

        for i in 0..self.sb_regs.len() {
            self.register_rio(
                std::ptr::null_mut(),
                SB_BASE + (i as u32 * 4),
                RIO_NO_ACCESS,
                None,
                None,
            );
        }

        // === Begin register table initialization ===

        let sb_ptr = self as *mut _ as *mut u32;

        macro_rules! rio {
            ($addr:ident, $mode:ident) => {
                self.register_rio(std::ptr::null_mut(), $addr, $mode, None, None);
            };
            ($addr:ident, $mode:ident, $rf:expr) => {
                self.register_rio(std::ptr::null_mut(), $addr, $mode, Some($rf), None);
            };
            ($addr:ident, $mode:ident, $rf:expr, $wf:expr) => {
                self.register_rio(std::ptr::null_mut(), $addr, $mode, Some($rf), Some($wf));
            };
        }

        // DMA / Sort-DMA
        rio!(SB_C2DSTAT_ADDR, RIO_DATA);
        rio!(SB_C2DLEN_ADDR, RIO_DATA);
        //rio!(SB_C2DST_ADDR, RIO_DATA);
        self.register_rio(
            sb_ptr,
            SB_C2DST_ADDR,
            RIO_WF,
            None,
            Some(pvr::sb_c2dst_write as _),
        );
        rio!(SB_SDSTAW_ADDR, RIO_DATA);
        rio!(SB_SDBAAW_ADDR, RIO_DATA);
        rio!(SB_SDWLT_ADDR, RIO_DATA);
        rio!(SB_SDLAS_ADDR, RIO_DATA);
        self.register_rio(
            sb_ptr,
            SB_SDST_ADDR,
            RIO_WF,
            None,
            Some(pvr::sb_sdst_write as _),
        );
        rio!(SB_SDDIV_ADDR, RIO_RO);
        rio!(SB_DBREQM_ADDR, RIO_DATA);
        rio!(SB_BAVLWC_ADDR, RIO_DATA);
        rio!(SB_C2DPRYC_ADDR, RIO_DATA);
        rio!(SB_C2DMAXL_ADDR, RIO_DATA);
        rio!(SB_TFREM_ADDR, RIO_RO);
        rio!(SB_LMMODE0_ADDR, RIO_DATA);
        rio!(SB_LMMODE1_ADDR, RIO_DATA);
        self.register_rio(
            sb_ptr,
            SB_FFST_ADDR,
            RIO_RO_FUNC,
            Some(Self::sb_ffst_read as _),
            None,
        );
        self.register_rio(
            sb_ptr,
            SB_SFRES_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sb_sfres_write as _),
        );
        rio!(SB_SBREV_ADDR, RIO_CONST);
        rio!(SB_RBSPLT_ADDR, RIO_DATA);
        rio!(SB_ISTNRM_ADDR, RIO_DATA);
        rio!(SB_ISTEXT_ADDR, RIO_RO);
        rio!(SB_ISTERR_ADDR, RIO_DATA);
        rio!(SB_IML2NRM_ADDR, RIO_DATA);
        rio!(SB_IML2EXT_ADDR, RIO_DATA);
        rio!(SB_IML2ERR_ADDR, RIO_DATA);
        rio!(SB_IML4NRM_ADDR, RIO_DATA);
        rio!(SB_IML4EXT_ADDR, RIO_DATA);
        rio!(SB_IML4ERR_ADDR, RIO_DATA);
        rio!(SB_IML6NRM_ADDR, RIO_DATA);
        rio!(SB_IML6EXT_ADDR, RIO_DATA);
        rio!(SB_IML6ERR_ADDR, RIO_DATA);
        rio!(SB_PDTNRM_ADDR, RIO_DATA);
        rio!(SB_PDTEXT_ADDR, RIO_DATA);
        rio!(SB_G2DTNRM_ADDR, RIO_DATA);
        rio!(SB_G2DTEXT_ADDR, RIO_DATA);
        rio!(SB_MDSTAR_ADDR, RIO_DATA);
        rio!(SB_MDTSEL_ADDR, RIO_DATA);
        rio!(SB_MDEN_ADDR, RIO_DATA);
        rio!(SB_MDST_ADDR, RIO_DATA);
        rio!(SB_MSYS_ADDR, RIO_DATA);
        rio!(SB_MST_ADDR, RIO_RO);
        let reg_ptr = self.regn32(SB_MSHTCL_ADDR);
        self.register_rio(
            reg_ptr,
            SB_MSHTCL_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_MDAPRO_ADDR);
        self.register_rio(
            reg_ptr,
            SB_MDAPRO_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        rio!(SB_MMSEL_ADDR, RIO_DATA);
        rio!(SB_MTXDAD_ADDR, RIO_RO);
        rio!(SB_MRXDAD_ADDR, RIO_RO);
        rio!(SB_MRXDBD_ADDR, RIO_RO);
        rio!(SB_GDSTAR_ADDR, RIO_DATA);
        rio!(SB_GDLEN_ADDR, RIO_DATA);
        rio!(SB_GDDIR_ADDR, RIO_DATA);
        rio!(SB_GDEN_ADDR, RIO_DATA);
        rio!(SB_GDST_ADDR, RIO_DATA);
        let reg_ptr = self.regn32(SB_MSHTCL_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1RRC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1RWC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1RWC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1FRC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1FRC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1FWC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1FWC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1CRC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1CRC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1CWC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1CWC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1GDRC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1GDRC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1GDWC_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G1GDWC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_G1SYSM_ADDR);
        rio!(SB_G1SYSM_ADDR, RIO_RO);
        self.register_rio(
            reg_ptr,
            SB_G1CRDYC_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        let reg_ptr = self.regn32(SB_GDAPRO_ADDR);
        self.register_rio(
            reg_ptr,
            SB_GDAPRO_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly_gdrom_protection as _),
        );
        rio!(SB_GDSTARD_ADDR, RIO_RO);
        rio!(SB_GDLEND_ADDR, RIO_RO);
        rio!(SB_ADSTAG_ADDR, RIO_DATA);
        rio!(SB_ADSTAR_ADDR, RIO_DATA);
        rio!(SB_ADLEN_ADDR, RIO_DATA);
        rio!(SB_ADDIR_ADDR, RIO_DATA);
        rio!(SB_ADTSEL_ADDR, RIO_DATA);
        rio!(SB_ADEN_ADDR, RIO_DATA);
        rio!(SB_ADST_ADDR, RIO_DATA);
        rio!(SB_ADSUSP_ADDR, RIO_DATA);
        rio!(SB_E1STAG_ADDR, RIO_DATA);
        rio!(SB_E1STAR_ADDR, RIO_DATA);
        rio!(SB_E1LEN_ADDR, RIO_DATA);
        rio!(SB_E1DIR_ADDR, RIO_DATA);
        rio!(SB_E1TSEL_ADDR, RIO_DATA);
        rio!(SB_E1EN_ADDR, RIO_DATA);
        rio!(SB_E1ST_ADDR, RIO_DATA);
        rio!(SB_E1SUSP_ADDR, RIO_DATA);
        rio!(SB_E2STAG_ADDR, RIO_DATA);
        rio!(SB_E2STAR_ADDR, RIO_DATA);
        rio!(SB_E2LEN_ADDR, RIO_DATA);
        rio!(SB_E2DIR_ADDR, RIO_DATA);
        rio!(SB_E2TSEL_ADDR, RIO_DATA);
        rio!(SB_E2EN_ADDR, RIO_DATA);
        rio!(SB_E2ST_ADDR, RIO_DATA);
        rio!(SB_E2SUSP_ADDR, RIO_DATA);
        rio!(SB_DDSTAG_ADDR, RIO_DATA);
        rio!(SB_DDSTAR_ADDR, RIO_DATA);
        rio!(SB_DDLEN_ADDR, RIO_DATA);
        rio!(SB_DDDIR_ADDR, RIO_DATA);
        rio!(SB_DDTSEL_ADDR, RIO_DATA);
        rio!(SB_DDEN_ADDR, RIO_DATA);
        rio!(SB_DDST_ADDR, RIO_DATA);
        rio!(SB_DDSUSP_ADDR, RIO_DATA);
        rio!(SB_G2ID_ADDR, RIO_CONST);
        rio!(SB_G2DSTO_ADDR, RIO_DATA);
        rio!(SB_G2TRTO_ADDR, RIO_DATA);
        rio!(SB_G2MDMTO_ADDR, RIO_DATA);
        rio!(SB_G2MDMW_ADDR, RIO_DATA);
        let reg_ptr = self.regn32(SB_G2APRO_ADDR);
        self.register_rio(
            reg_ptr,
            SB_G2APRO_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        rio!(SB_ADSTAGD_ADDR, RIO_RO);
        rio!(SB_ADSTARD_ADDR, RIO_RO);
        rio!(SB_ADLEND_ADDR, RIO_RO);
        rio!(SB_E1STAGD_ADDR, RIO_RO);
        rio!(SB_E1STARD_ADDR, RIO_RO);
        rio!(SB_E1LEND_ADDR, RIO_RO);
        rio!(SB_E2STAGD_ADDR, RIO_RO);
        rio!(SB_E2STARD_ADDR, RIO_RO);
        rio!(SB_E2LEND_ADDR, RIO_RO);
        rio!(SB_DDSTAGD_ADDR, RIO_RO);
        rio!(SB_DDSTARD_ADDR, RIO_RO);
        rio!(SB_DDLEND_ADDR, RIO_RO);
        rio!(SB_PDSTAP_ADDR, RIO_DATA);
        rio!(SB_PDSTAR_ADDR, RIO_DATA);
        rio!(SB_PDLEN_ADDR, RIO_DATA);
        rio!(SB_PDDIR_ADDR, RIO_DATA);
        rio!(SB_PDTSEL_ADDR, RIO_DATA);
        rio!(SB_PDEN_ADDR, RIO_DATA);
        self.register_rio(
            sb_ptr,
            SB_PDST_ADDR,
            RIO_WF,
            None,
            Some(pvr::sb_pdst_write as _),
        );
        let reg_ptr = self.regn32(SB_PDAPRO_ADDR);
        self.register_rio(
            reg_ptr,
            SB_PDAPRO_ADDR,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_writeonly as _),
        );
        rio!(SB_PDSTAPD_ADDR, RIO_RO);
        rio!(SB_PDSTARD_ADDR, RIO_RO);
        rio!(SB_PDLEND_ADDR, RIO_RO);

        // Special cases
        let reg_ptr = self.regn32(SB_PDAPRO_ADDR);
        self.register_rio(
            reg_ptr,
            0x005f74e4,
            RIO_WO_FUNC,
            None,
            Some(Self::sbio_write_gdrom_unlock),
        );

        for &a in &[
            0x005f68a4, 0x005f68ac, 0x005f78a0, 0x005f78a4, 0x005f78a8, 0x005f78ac, 0x005f78b0,
            0x005f78b4, 0x005f78b8,
        ] {
            let reg_ptr = self.regn32(a);
            self.register_rio(reg_ptr, a, RIO_WO_FUNC, None, Some(Self::sbio_write_zero));
        }
    }

    pub fn register_rio(
        &mut self,
        context: *mut u32,
        reg_addr: u32,
        flags: u32,
        rf: Option<RegReadAddrFP>,
        wf: Option<RegWriteAddrFP>,
    ) {
        let idx = ((reg_addr - SB_BASE) / 4) as usize;
        assert!(idx < self.sb_regs.len());
        let r = &mut self.sb_regs[idx];
        r.flags = flags | REG_ACCESS_32;
        r.context = context;

        if flags == RIO_NO_ACCESS {
            r.read_function_addr = Some(Self::sbio_read_noacc);
            r.write_function_addr = Some(Self::sbio_write_noacc);
        } else if flags == RIO_CONST {
            r.write_function_addr = Some(Self::sbio_write_const);
        } else {
            r.data32 = 0;
            if flags & REG_RF != 0 {
                r.read_function_addr = rf;
            }
            if flags & REG_WF != 0 {
                r.write_function_addr = Some(wf.unwrap_or(Self::sbio_write_noacc));
            }
        }
    }

    pub fn read(&self, addr: u32, sz: u32) -> u32 {
        let idx = ((addr - SB_BASE) >> 2) as usize;
        let r = &self.sb_regs[idx];
        if r.flags & (REG_RF | REG_WO) == 0 {
            match sz {
                4 => r.data32,
                2 => r.data32 & 0xFFFF,
                1 => r.data32 & 0xFF,
                _ => 0,
            }
        } else {
            let rf = r.read_function_addr.expect("read function missing");
            rf(r.context, addr)
        }
    }
    pub fn write(&mut self, addr: u32, data: u32, sz: u32) {
        let idx = ((addr - SB_BASE) >> 2) as usize;
        let r = &mut self.sb_regs[idx];
        if r.flags & REG_WF == 0 {
            match sz {
                4 => r.data32 = data,
                2 => r.data32 = data & 0xFFFF,
                1 => r.data32 = data & 0xFF,
                _ => {}
            }
        } else {
            let wf = r.write_function_addr.expect("write function missing");
            wf(r.context, addr, data);
        }
    }

    pub fn reset(&mut self) {
        self.sb_ffst_rc = 0;
        self.sb_ffst = 0;
    }
}
