/*
    This file is part of libswirl
*/
// #include "license/bsd"

// PVR register definitions from pvr_regs.h

use bitfield::bitfield;

pub static mut EMU_REGS: *const u32 = std::ptr::null();

pub const PVR_REG_SIZE: u32 = 0x8000;
pub const PVR_REG_MASK: u32 = PVR_REG_SIZE - 1;

// Macro to access PVR registers
macro_rules! pvr_reg {
    ($addr:expr, $type:ty) => {
        unsafe { &*(EMU_REGS.offset(((($addr / 4) & PVR_REG_MASK) as usize) as isize) as *const $type) }
    };
}

// Register addresses
pub const ID_ADDR: u32 = 0x00000000;
pub const REVISION_ADDR: u32 = 0x00000004;
pub const SOFTRESET_ADDR: u32 = 0x00000008;

pub const STARTRENDER_ADDR: u32 = 0x00000014;
pub const TEST_SELECT_ADDR: u32 = 0x00000018;

pub const PARAM_BASE_ADDR: u32 = 0x00000020;

pub const REGION_BASE_ADDR: u32 = 0x0000002C;
pub const SPAN_SORT_CFG_ADDR: u32 = 0x00000030;

pub const VO_BORDER_COL_ADDR: u32 = 0x00000040;
pub const FB_R_CTRL_ADDR: u32 = 0x00000044;
pub const FB_W_CTRL_ADDR: u32 = 0x00000048;
pub const FB_W_LINESTRIDE_ADDR: u32 = 0x0000004C;
pub const FB_R_SOF1_ADDR: u32 = 0x00000050;
pub const FB_R_SOF2_ADDR: u32 = 0x00000054;

pub const FB_R_SIZE_ADDR: u32 = 0x0000005C;
pub const FB_W_SOF1_ADDR: u32 = 0x00000060;
pub const FB_W_SOF2_ADDR: u32 = 0x00000064;
pub const FB_X_CLIP_ADDR: u32 = 0x00000068;
pub const FB_Y_CLIP_ADDR: u32 = 0x0000006C;

pub const FPU_SHAD_SCALE_ADDR: u32 = 0x00000074;
pub const FPU_CULL_VAL_ADDR: u32 = 0x00000078;
pub const FPU_PARAM_CFG_ADDR: u32 = 0x0000007C;
pub const HALF_OFFSET_ADDR: u32 = 0x00000080;
pub const FPU_PERP_VAL_ADDR: u32 = 0x00000084;
pub const ISP_BACKGND_D_ADDR: u32 = 0x00000088;
pub const ISP_BACKGND_T_ADDR: u32 = 0x0000008C;

pub const ISP_FEED_CFG_ADDR: u32 = 0x00000098;

pub const SDRAM_REFRESH_ADDR: u32 = 0x000000A0;
pub const SDRAM_ARB_CFG_ADDR: u32 = 0x000000A4;
pub const SDRAM_CFG_ADDR: u32 = 0x000000A8;

pub const FOG_COL_RAM_ADDR: u32 = 0x000000B0;
pub const FOG_COL_VERT_ADDR: u32 = 0x000000B4;
pub const FOG_DENSITY_ADDR: u32 = 0x000000B8;
pub const FOG_CLAMP_MAX_ADDR: u32 = 0x000000BC;
pub const FOG_CLAMP_MIN_ADDR: u32 = 0x000000C0;
pub const SPG_TRIGGER_POS_ADDR: u32 = 0x000000C4;
pub const SPG_HBLANK_INT_ADDR: u32 = 0x000000C8;
pub const SPG_VBLANK_INT_ADDR: u32 = 0x000000CC;
pub const SPG_CONTROL_ADDR: u32 = 0x000000D0;
pub const SPG_HBLANK_ADDR: u32 = 0x000000D4;
pub const SPG_LOAD_ADDR: u32 = 0x000000D8;
pub const SPG_VBLANK_ADDR: u32 = 0x000000DC;
pub const SPG_WIDTH_ADDR: u32 = 0x000000E0;
pub const TEXT_CONTROL_ADDR: u32 = 0x000000E4;
pub const VO_CONTROL_ADDR: u32 = 0x000000E8;
pub const VO_STARTX_ADDR: u32 = 0x000000EC;
pub const VO_STARTY_ADDR: u32 = 0x000000F0;
pub const SCALER_CTL_ADDR: u32 = 0x000000F4;
pub const PAL_RAM_CTRL_ADDR: u32 = 0x00000108;
pub const SPG_STATUS_ADDR: u32 = 0x0000010C;
pub const FB_BURSTCTRL_ADDR: u32 = 0x00000110;
pub const FB_C_SOF_ADDR: u32 = 0x00000114;
pub const Y_COEFF_ADDR: u32 = 0x00000118;

pub const PT_ALPHA_REF_ADDR: u32 = 0x0000011C;

// TA REGS
pub const TA_OL_BASE_ADDR: u32 = 0x00000124;
pub const TA_ISP_BASE_ADDR: u32 = 0x00000128;
pub const TA_OL_LIMIT_ADDR: u32 = 0x0000012C;
pub const TA_ISP_LIMIT_ADDR: u32 = 0x00000130;
pub const TA_NEXT_OPB_ADDR: u32 = 0x00000134;
pub const TA_ISP_CURRENT_ADDR: u32 = 0x00000138;
pub const TA_GLOB_TILE_CLIP_ADDR: u32 = 0x0000013C;
pub const TA_ALLOC_CTRL_ADDR: u32 = 0x00000140;
pub const TA_LIST_INIT_ADDR: u32 = 0x00000144;
pub const TA_YUV_TEX_BASE_ADDR: u32 = 0x00000148;
pub const TA_YUV_TEX_CTRL_ADDR: u32 = 0x0000014C;
pub const TA_YUV_TEX_CNT_ADDR: u32 = 0x00000150;

pub const TA_LIST_CONT_ADDR: u32 = 0x00000160;
pub const TA_NEXT_OPB_INIT_ADDR: u32 = 0x00000164;

pub const FOG_TABLE_START_ADDR: u32 = 0x00000200;
pub const FOG_TABLE_END_ADDR: u32 = 0x000003FC;

pub const TA_OL_POINTERS_START_ADDR: u32 = 0x00000600;
pub const TA_OL_POINTERS_END_ADDR: u32 = 0x00000F5C;

pub const PALETTE_RAM_START_ADDR: u32 = 0x00001000;
pub const PALETTE_RAM_END_ADDR: u32 = 0x00001FFC;

// Register type definitions using bitfield

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_R_CTRL_type(u32);
    impl Debug;

    pub fb_enable, set_fb_enable: 0;
    pub fb_line_double, set_fb_line_double: 1;
    pub fb_depth, set_fb_depth: 3, 2;
    pub fb_concat, set_fb_concat: 6, 4;
    pub u8, fb_chroma_threshold, set_fb_chroma_threshold: 15, 8;
    pub fb_stripsize, set_fb_stripsize: 21, 16;
    pub fb_strip_buf_en, set_fb_strip_buf_en: 22;
    pub vclk_div, set_vclk_div: 23;
}

impl FB_R_CTRL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum FbDepthEnum {
    Fbde0555 = 0,
    Fbde565 = 1,
    Fbde888 = 2,
    FbdeC888 = 3,
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_R_SIZE_type(u32);
    impl Debug;

    pub fb_x_size, set_fb_x_size: 9, 0;
    pub fb_y_size, set_fb_y_size: 19, 10;
    pub fb_modulus, set_fb_modulus: 29, 20;
    pub fb_res, set_fb_res: 31, 30;
}

impl FB_R_SIZE_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct VO_BORDER_COL_type(u32);
    impl Debug;

    pub u8, blue, set_blue: 7, 0;
    pub u8, green, set_green: 15, 8;
    pub u8, red, set_red: 23, 16;
    pub chroma, set_chroma: 24;
}

impl VO_BORDER_COL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_STATUS_type(u32);
    impl Debug;

    pub scanline, set_scanline: 9, 0;
    pub fieldnum, set_fieldnum: 10;
    pub blank, set_blank: 11;
    pub hsync, set_hsync: 12;
    pub vsync, set_vsync: 13;
}

impl SPG_STATUS_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_HBLANK_INT_type(u32);
    impl Debug;

    pub line_comp_val, set_line_comp_val: 9, 0;
    pub hblank_int_mode, set_hblank_int_mode: 13, 12;
    pub hblank_in_interrupt, set_hblank_in_interrupt: 25, 16;
}

impl SPG_HBLANK_INT_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_VBLANK_INT_type(u32);
    impl Debug;

    pub vblank_in_interrupt_line_number, set_vblank_in_interrupt_line_number: 9, 0;
    pub vblank_out_interrupt_line_number, set_vblank_out_interrupt_line_number: 25, 16;
}

impl SPG_VBLANK_INT_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_CONTROL_type(u32);
    impl Debug;

    pub mhsync_pol, set_mhsync_pol: 0;
    pub mvsync_pol, set_mvsync_pol: 1;
    pub mcsync_pol, set_mcsync_pol: 2;
    pub spg_lock, set_spg_lock: 3;
    pub interlace, set_interlace: 4;
    pub force_field2, set_force_field2: 5;
    pub ntsc, set_ntsc: 6;
    pub pal, set_pal: 7;
    pub sync_direction, set_sync_direction: 8;
    pub csync_on_h, set_csync_on_h: 9;
}

impl SPG_CONTROL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_HBLANK_type(u32);
    impl Debug;

    pub hstart, set_hstart: 9, 0;
    pub hbend, set_hbend: 25, 16;
}

impl SPG_HBLANK_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_LOAD_type(u32);
    impl Debug;

    pub hcount, set_hcount: 9, 0;
    pub vcount, set_vcount: 25, 16;
}

impl SPG_LOAD_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_VBLANK_type(u32);
    impl Debug;

    pub vstart, set_vstart: 9, 0;
    pub vbend, set_vbend: 25, 16;
}

impl SPG_VBLANK_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SPG_WIDTH_type(u32);
    impl Debug;

    pub hswidth, set_hswidth: 6, 0;
    pub vswidth, set_vswidth: 11, 8;
    pub bpwidth, set_bpwidth: 21, 12;
    pub eqwidth, set_eqwidth: 31, 22;
}

impl SPG_WIDTH_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct SCALER_CTL_type(u32);
    impl Debug;

    pub vscalefactor, set_vscalefactor: 15, 0;
    pub hscale, set_hscale: 16;
    pub interlace, set_interlace: 17;
    pub fieldselect, set_fieldselect: 18;
}

impl SCALER_CTL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_X_CLIP_type(u32);
    impl Debug;

    pub min, set_min: 10, 0;
    pub max, set_max: 26, 16;
}

impl FB_X_CLIP_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_Y_CLIP_type(u32);
    impl Debug;

    pub min, set_min: 9, 0;
    pub max, set_max: 25, 16;
}

impl FB_Y_CLIP_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct VO_CONTROL_type(u32);
    impl Debug;

    pub hsync_pol, set_hsync_pol: 0;
    pub vsync_pol, set_vsync_pol: 1;
    pub blank_pol, set_blank_pol: 2;
    pub blank_video, set_blank_video: 3;
    pub field_mode, set_field_mode: 7, 4;
    pub pixel_double, set_pixel_double: 8;
    pub pclk_delay, set_pclk_delay: 21, 16;
}

impl VO_CONTROL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct VO_STARTX_type(u32);
    impl Debug;

    pub h_start, set_h_start: 9, 0;
}

impl VO_STARTX_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct VO_STARTY_type(u32);
    impl Debug;

    pub v_start_field1, set_v_start_field1: 9, 0;
    pub v_start_field2, set_v_start_field2: 25, 16;
}

impl VO_STARTY_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    #[allow(non_camel_case_types)]
    pub struct ISP_BACKGND_T_type(u32);
    impl Debug;

    pub tag_offset, set_tag_offset: 2, 0;
    pub param_offs_in_words, set_param_offs_in_words: 23, 3;
    pub skip, set_skip: 26, 24;
    pub shadow, set_shadow: 27;
    pub cache_bypass, set_cache_bypass: 28;
}

impl ISP_BACKGND_T_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ISP_BACKGND_D_type {
    pub i: u32,
    pub f: f32,
}

bitfield! {
    #[derive(Copy, Clone)]
    #[allow(non_camel_case_types)]
    pub struct ISP_FEED_CFG_type(u32);
    impl Debug;

    pub pre_sort, set_pre_sort: 0;
    pub discard_mode, set_discard_mode: 3;
    pub pt_chunk_size, set_pt_chunk_size: 13, 4;
    pub tr_cache_size, set_tr_cache_size: 23, 14;
}

impl ISP_FEED_CFG_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_W_CTRL_type(u32);
    impl Debug;

    pub fb_packmode, set_fb_packmode: 2, 0;
    pub fb_dither, set_fb_dither: 3;
    pub u8, fb_kval, set_fb_kval: 15, 8;
    pub u8, fb_alpha_threshold, set_fb_alpha_threshold: 23, 16;
}

impl FB_W_CTRL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FB_W_LINESTRIDE_type(u32);
    impl Debug;

    pub stride, set_stride: 8, 0;
}

impl FB_W_LINESTRIDE_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FPU_SHAD_SCALE_type(u32);
    impl Debug;

    pub u8, scale_factor, set_scale_factor: 7, 0;
    pub intensity_shadow, set_intensity_shadow: 8;
}

impl FPU_SHAD_SCALE_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct FPU_PARAM_CFG_type(u32);
    impl Debug;

    pub pointer_first_burst, set_pointer_first_burst: 3, 0;
    pub pointer_burst, set_pointer_burst: 7, 4;
    pub isp_param_burst_threshold, set_isp_param_burst_threshold: 13, 8;
    pub tsp_param_burst_threshold, set_tsp_param_burst_threshold: 19, 14;
    pub region_header_type, set_region_header_type: 21;
}

impl FPU_PARAM_CFG_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct HALF_OFFSET_type(u32);
    impl Debug;

    pub fpu_pixel_half_offset, set_fpu_pixel_half_offset: 0;
    pub tsp_pixel_half_offset, set_tsp_pixel_half_offset: 1;
    pub texure_pixel_half_offset, set_texure_pixel_half_offset: 2;
}

impl HALF_OFFSET_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct TA_GLOB_TILE_CLIP_type(u32);
    impl Debug;

    pub tile_x_num, set_tile_x_num: 5, 0;
    pub tile_y_num, set_tile_y_num: 19, 16;
}

impl TA_GLOB_TILE_CLIP_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct TA_YUV_TEX_CTRL_type(u32);
    impl Debug;

    pub yuv_u_size, set_yuv_u_size: 5, 0;
    pub yuv_v_size, set_yuv_v_size: 13, 8;
    pub yuv_tex, set_yuv_tex: 16;
    pub yuv_form, set_yuv_form: 24;
}

impl TA_YUV_TEX_CTRL_type {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

// Register accessors (read-only for now)
#[inline]
pub fn id() -> u32 {
    unsafe { *pvr_reg!(ID_ADDR, u32) }
}

#[inline]
pub fn param_base() -> u32 {
    unsafe { *pvr_reg!(PARAM_BASE_ADDR, u32) }
}

#[inline]
pub fn region_base() -> u32 {
    unsafe { *pvr_reg!(REGION_BASE_ADDR, u32) }
}

#[inline]
pub fn fb_w_sof1() -> u32 {
    unsafe { *pvr_reg!(FB_W_SOF1_ADDR, u32) }
}

#[inline]
pub fn fb_w_sof2() -> u32 {
    unsafe { *pvr_reg!(FB_W_SOF2_ADDR, u32) }
}

#[inline]
pub fn fb_w_ctrl() -> FB_W_CTRL_type {
    unsafe { *pvr_reg!(FB_W_CTRL_ADDR, FB_W_CTRL_type) }
}

#[inline]
pub fn fb_w_linestride() -> FB_W_LINESTRIDE_type {
    unsafe { *pvr_reg!(FB_W_LINESTRIDE_ADDR, FB_W_LINESTRIDE_type) }
}

#[inline]
pub fn scaler_ctl() -> SCALER_CTL_type {
    unsafe { *pvr_reg!(SCALER_CTL_ADDR, SCALER_CTL_type) }
}

#[inline]
pub fn fpu_param_cfg() -> FPU_PARAM_CFG_type {
    unsafe { *pvr_reg!(FPU_PARAM_CFG_ADDR, FPU_PARAM_CFG_type) }
}

#[inline]
pub fn isp_feed_cfg() -> ISP_FEED_CFG_type {
    unsafe { *pvr_reg!(ISP_FEED_CFG_ADDR, ISP_FEED_CFG_type) }
}

#[inline]
pub fn isp_backgnd_d() -> ISP_BACKGND_D_type {
    unsafe { *pvr_reg!(ISP_BACKGND_D_ADDR, ISP_BACKGND_D_type) }
}

#[inline]
pub fn isp_backgnd_t() -> ISP_BACKGND_T_type {
    unsafe { *pvr_reg!(ISP_BACKGND_T_ADDR, ISP_BACKGND_T_type) }
}

#[inline]
pub fn fpu_shad_scale() -> FPU_SHAD_SCALE_type {
    unsafe { *pvr_reg!(FPU_SHAD_SCALE_ADDR, FPU_SHAD_SCALE_type) }
}

#[inline]
pub fn fpu_cull_val() -> f32 {
    unsafe { *pvr_reg!(FPU_CULL_VAL_ADDR, f32) }
}

#[inline]
pub fn half_offset() -> HALF_OFFSET_type {
    unsafe { *pvr_reg!(HALF_OFFSET_ADDR, HALF_OFFSET_type) }
}

#[inline]
pub fn fog_density() -> u32 {
    unsafe { *pvr_reg!(FOG_DENSITY_ADDR, u32) }
}

#[inline]
pub fn fog_clamp_max() -> u32 {
    unsafe { *pvr_reg!(FOG_CLAMP_MAX_ADDR, u32) }
}

#[inline]
pub fn fog_clamp_min() -> u32 {
    unsafe { *pvr_reg!(FOG_CLAMP_MIN_ADDR, u32) }
}

#[inline]
pub fn fog_col_ram() -> u32 {
    unsafe { *pvr_reg!(FOG_COL_RAM_ADDR, u32) }
}

#[inline]
pub fn fog_col_vert() -> u32 {
    unsafe { *pvr_reg!(FOG_COL_VERT_ADDR, u32) }
}

#[inline]
pub fn pt_alpha_ref() -> u32 {
    unsafe { *pvr_reg!(PT_ALPHA_REF_ADDR, u32) }
}

#[inline]
pub fn text_control() -> u32 {
    unsafe { *pvr_reg!(TEXT_CONTROL_ADDR, u32) }
}

#[inline]
pub fn pal_ram_ctrl() -> u32 {
    unsafe { *pvr_reg!(PAL_RAM_CTRL_ADDR, u32) }
}

// Array accessors
#[inline]
pub fn fog_table() -> *const u32 {
    unsafe { pvr_reg!(FOG_TABLE_START_ADDR, u32) as *const u32 }
}

#[inline]
pub fn palette_ram() -> *const u32 {
    unsafe { pvr_reg!(PALETTE_RAM_START_ADDR, u32) as *const u32 }
}

// Context helpers
pub const TA_CURRENT_CTX_MASK: u32 = 0xF00000;
pub const CORE_CURRENT_CTX_MASK: u32 = 0xF00000;
