#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
use bitfield::bitfield;

// =============================================================================
// PVR Register Constants
// =============================================================================

pub const PVR_REG_SIZE: usize = 0x8000;
pub const PVR_REG_MASK: usize = PVR_REG_SIZE - 1;

// PVR base register definitions
pub const ID_ADDR: u32 = 0x00000000; // R   Device ID
pub const REVISION_ADDR: u32 = 0x00000004; // R   Revision number
pub const SOFTRESET_ADDR: u32 = 0x00000008; // RW  CORE & TA software reset

pub const STARTRENDER_ADDR: u32 = 0x00000014; // RW  Drawing start
pub const TEST_SELECT_ADDR: u32 = 0x00000018; // RW  Test (writing this register is prohibited)

pub const PARAM_BASE_ADDR: u32 = 0x00000020; // RW  Base address for ISP parameters
pub const REGION_BASE_ADDR: u32 = 0x0000002C; // RW  Base address for Region Array
pub const SPAN_SORT_CFG_ADDR: u32 = 0x00000030; // RW  Span Sorter control

pub const VO_BORDER_COL_ADDR: u32 = 0x00000040; // RW  Border area color
pub const FB_R_CTRL_ADDR: u32 = 0x00000044; // RW  Frame buffer read control
pub const FB_W_CTRL_ADDR: u32 = 0x00000048; // RW  Frame buffer write control
pub const FB_W_LINESTRIDE_ADDR: u32 = 0x0000004C; // RW  Frame buffer line stride
pub const FB_R_SOF1_ADDR: u32 = 0x00000050; // RW  Read start address for field - 1/strip - 1
pub const FB_R_SOF2_ADDR: u32 = 0x00000054; // RW  Read start address for field - 2/strip - 2
pub const FB_R_SIZE_ADDR: u32 = 0x0000005C; // RW  Frame buffer XY size
pub const FB_W_SOF1_ADDR: u32 = 0x00000060; // RW  Write start address for field - 1/strip - 1
pub const FB_W_SOF2_ADDR: u32 = 0x00000064; // RW  Write start address for field - 2/strip - 2
pub const FB_X_CLIP_ADDR: u32 = 0x00000068; // RW  Pixel clip X coordinate
pub const FB_Y_CLIP_ADDR: u32 = 0x0000006C; // RW  Pixel clip Y coordinate

pub const FPU_SHAD_SCALE_ADDR: u32 = 0x00000074; // RW  Intensity Volume mode
pub const FPU_CULL_VAL_ADDR: u32 = 0x00000078; // RW  Comparison value for culling
pub const FPU_PARAM_CFG_ADDR: u32 = 0x0000007C; // RW  Parameter read control
pub const HALF_OFFSET_ADDR: u32 = 0x00000080; // RW  Pixel sampling control
pub const FPU_PERP_VAL_ADDR: u32 = 0x00000084; // RW  Comparison value for perpendicular polygons
pub const ISP_BACKGND_D_ADDR: u32 = 0x00000088; // RW  Background surface depth
pub const ISP_BACKGND_T_ADDR: u32 = 0x0000008C; // RW  Background surface tag
pub const ISP_FEED_CFG_ADDR: u32 = 0x00000098; // RW  Translucent polygon sort mode
pub const SDRAM_REFRESH_ADDR: u32 = 0x000000A0; // RW  Texture memory refresh counter
pub const SDRAM_ARB_CFG_ADDR: u32 = 0x000000A4; // RW  Texture memory arbiter control
pub const SDRAM_CFG_ADDR: u32 = 0x000000A8; // RW  Texture memory control
pub const FOG_COL_RAM_ADDR: u32 = 0x000000B0; // RW  Color for Look Up table Fog
pub const FOG_COL_VERT_ADDR: u32 = 0x000000B4; // RW  Color for vertex Fog
pub const FOG_DENSITY_ADDR: u32 = 0x000000B8; // RW  Fog scale value
pub const FOG_CLAMP_MAX_ADDR: u32 = 0x000000BC; // RW  Color clamping maximum value
pub const FOG_CLAMP_MIN_ADDR: u32 = 0x000000C0; // RW  Color clamping minimum value
pub const SPG_TRIGGER_POS_ADDR: u32 = 0x000000C4; // RW  External trigger signal HV counter value
pub const SPG_HBLANK_INT_ADDR: u32 = 0x000000C8; // RW  H-blank interrupt control
pub const SPG_VBLANK_INT_ADDR: u32 = 0x000000CC; // RW  V-blank interrupt control
pub const SPG_CONTROL_ADDR: u32 = 0x000000D0; // RW  Sync pulse generator control
pub const SPG_HBLANK_ADDR: u32 = 0x000000D4; // RW  H-blank control
pub const SPG_LOAD_ADDR: u32 = 0x000000D8; // RW  HV counter load value
pub const SPG_VBLANK_ADDR: u32 = 0x000000DC; // RW  V-blank control
pub const SPG_WIDTH_ADDR: u32 = 0x000000E0; // RW  Sync width control
pub const TEXT_CONTROL_ADDR: u32 = 0x000000E4; // RW  Texturing control
pub const VO_CONTROL_ADDR: u32 = 0x000000E8; // RW  Video output control
pub const VO_STARTX_ADDR: u32 = 0x000000EC; // RW  Video output start X position
pub const VO_STARTY_ADDR: u32 = 0x000000F0; // RW  Video output start Y position
pub const SCALER_CTL_ADDR: u32 = 0x000000F4; // RW  X & Y scaler control
pub const PAL_RAM_CTRL_ADDR: u32 = 0x00000108; // RW  Palette RAM control
pub const SPG_STATUS_ADDR: u32 = 0x0000010C; // R   Sync pulse generator status
pub const FB_BURSTCTRL_ADDR: u32 = 0x00000110; // RW  Frame buffer burst control
pub const FB_C_SOF_ADDR: u32 = 0x00000114; // R   Current frame buffer start address
pub const Y_COEFF_ADDR: u32 = 0x00000118; // RW  Y scaling coefficient
pub const PT_ALPHA_REF_ADDR: u32 = 0x0000011C; // RW  Alpha value for Punch Through polygon comparison

// TA REGS
pub const TA_OL_BASE_ADDR: u32 = 0x00000124; // RW  Object list write start address
pub const TA_ISP_BASE_ADDR: u32 = 0x00000128; // RW  ISP/TSP Parameter write start address
pub const TA_OL_LIMIT_ADDR: u32 = 0x0000012C; // RW  Start address of next Object Pointer Block
pub const TA_ISP_LIMIT_ADDR: u32 = 0x00000130; // RW  Current ISP/TSP Parameter write address
pub const TA_NEXT_OPB_ADDR: u32 = 0x00000134; // R   Global Tile clip control
pub const TA_ISP_CURRENT_ADDR: u32 = 0x00000138; // R   Current ISP/TSP Parameter write address
pub const TA_GLOB_TILE_CLIP_ADDR: u32 = 0x0000013C; // RW  Global Tile clip control
pub const TA_ALLOC_CTRL_ADDR: u32 = 0x00000140; // RW  Object list control
pub const TA_LIST_INIT_ADDR: u32 = 0x00000144; // RW  TA initialization
pub const TA_YUV_TEX_BASE_ADDR: u32 = 0x00000148; // RW  YUV422 texture write start address
pub const TA_YUV_TEX_CTRL_ADDR: u32 = 0x0000014C; // RW  YUV converter control
pub const TA_YUV_TEX_CNT_ADDR: u32 = 0x00000150; // R   YUV converter macro block counter value
pub const TA_LIST_CONT_ADDR: u32 = 0x00000160; // RW  TA continuation processing
pub const TA_NEXT_OPB_INIT_ADDR: u32 = 0x00000164; // RW  Additional OPB starting address
pub const FOG_TABLE_START_ADDR: u32 = 0x00000200; // RW  Look-up table Fog data
pub const FOG_TABLE_END_ADDR: u32 = 0x000003FC;
pub const TA_OL_POINTERS_START_ADDR: u32 = 0x00000600; // R   TA object List Pointer data
pub const TA_OL_POINTERS_END_ADDR: u32 = 0x00000F5C;
pub const PALETTE_RAM_START_ADDR: u32 = 0x00001000; // RW  Palette RAM
pub const PALETTE_RAM_END_ADDR: u32 = 0x00001FFC;

// =============================================================================
// Bitfield Definitions
// =============================================================================

bitfield! {
    pub struct FB_R_CTRL(u32);
    impl Debug;
    pub fb_enable, set_fb_enable: 0;
    pub fb_line_double, set_fb_line_double: 1;
    pub fb_depth, set_fb_depth: 3, 2;
    pub fb_concat, set_fb_concat: 6, 4;
    pub R, set_R: 7;
    pub fb_chroma_threshold, set_fb_chroma_threshold: 15, 8;
    pub fb_stripsize, set_fb_stripsize: 21, 16;
    pub fb_strip_buf_en, set_fb_strip_buf_en: 22;
    pub vclk_div, set_vclk_div: 23;
}

pub enum FBDepth {
    FBDE_0555 = 0,
    FBDE_565 = 1,
    FBDE_888 = 2,
    FBDE_C888 = 3,
}

bitfield! {
    pub struct FB_R_SIZE(u32);
    impl Debug;
    pub fb_x_size, set_fb_x_size: 9, 0;
    pub fb_y_size, set_fb_y_size: 19, 10;
    pub fb_modulus, set_fb_modulus: 29, 20;
    pub fb_res, set_fb_res: 31, 30;
}

bitfield! {
    pub struct VO_BORDER_COL(u32);
    impl Debug;
    pub blue, set_blue: 7, 0;
    pub green, set_green: 15, 8;
    pub red, set_red: 23, 16;
    pub chroma, set_chroma: 24;
}

bitfield! {
    pub struct SPG_STATUS(u32);
    impl Debug;
    pub scanline, set_scanline: 9, 0;
    pub fieldnum, set_fieldnum: 10;
    pub blank, set_blank: 11;
    pub hsync, set_hsync: 12;
    pub vsync, set_vsync: 13;
}

bitfield! {
    pub struct SPG_HBLANK_INT(u32);
    impl Debug;
    pub line_comp_val, set_line_comp_val: 9, 0;
    pub hblank_int_mode, set_hblank_int_mode: 13, 12;
    pub hblank_in_interrupt, set_hblank_in_interrupt: 25, 16;
}

bitfield! {
    pub struct SPG_VBLANK_INT(u32);
    impl Debug;
    pub vblank_in_interrupt_line_number, set_vblank_in_interrupt_line_number: 9, 0;
    pub vblank_out_interrupt_line_number, set_vblank_out_interrupt_line_number: 25, 16;
}

bitfield! {
    pub struct SPG_CONTROL(u32);
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

bitfield! {
    pub struct SPG_HBLANK(u32);
    impl Debug;
    pub hstart, set_hstart: 9, 0;
    pub hbend, set_hbend: 25, 16;
}

bitfield! {
    pub struct SPG_LOAD(u32);
    impl Debug;
    pub hcount, set_hcount: 9, 0;
    pub vcount, set_vcount: 25, 16;
}

bitfield! {
    pub struct SPG_VBLANK(u32);
    impl Debug;
    pub vstart, set_vstart: 9, 0;
    pub vbend, set_vbend: 25, 16;
}

bitfield! {
    pub struct SPG_WIDTH(u32);
    impl Debug;
    pub hswidth, set_hswidth: 6, 0;
    pub vswidth, set_vswidth: 11, 8;
    pub bpwidth, set_bpwidth: 21, 12;
    pub eqwidth, set_eqwidth: 31, 22;
}

bitfield! {
    pub struct SCALER_CTL(u32);
    impl Debug;
    pub vscalefactor, set_vscalefactor: 15, 0;
    pub hscale, set_hscale: 16;
    pub interlace, set_interlace: 17;
    pub fieldselect, set_fieldselect: 18;
}

bitfield! {
    pub struct FB_X_CLIP(u32);
    impl Debug;
    pub min, set_min: 10, 0;
    pub max, set_max: 26, 16;
}

bitfield! {
    pub struct FB_Y_CLIP(u32);
    impl Debug;
    pub min, set_min: 9, 0;
    pub max, set_max: 25, 16;
}

bitfield! {
    pub struct VO_CONTROL(u32);
    impl Debug;
    pub hsync_pol, set_hsync_pol: 0;
    pub vsync_pol, set_vsync_pol: 1;
    pub blank_pol, set_blank_pol: 2;
    pub blank_video, set_blank_video: 3;
    pub field_mode, set_field_mode: 7, 4;
    pub pixel_double, set_pixel_double: 8;
    pub pclk_delay, set_pclk_delay: 15, 10;
}

bitfield! {
    pub struct VO_STARTX(u32);
    impl Debug;
    pub hstart, set_hstart: 9, 0;
}

bitfield! {
    pub struct VO_STARTY(u32);
    impl Debug;
    pub vstart_field1, set_vstart_field1: 9, 0;
    pub vstart_field2, set_vstart_field2: 25, 16;
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union ISP_BACKGND_D {
    pub i: u32,
    pub f: f32,
}

bitfield! {
    pub struct ISP_BACKGND_T(u32);
    impl Debug;
    pub tag_offset, set_tag_offset: 2, 0;
    pub param_offs_in_words, set_param_offs_in_words: 23, 3;
    pub skip, set_skip: 26, 24;
    pub shadow, set_shadow: 27;
    pub cache_bypass, set_cache_bypass: 28;
}

bitfield! {
    pub struct ISP_FEED_CFG(u32);
    impl Debug;
    pub pre_sort, set_pre_sort: 0;
    pub discard_mode, set_discard_mode: 3;
    pub pt_chunk_size, set_pt_chunk_size: 13, 4;
    pub tr_cache_size, set_tr_cache_size: 23, 14;
}

bitfield! {
    pub struct FB_W_CTRL(u32);
    impl Debug;
    pub fb_packmode, set_fb_packmode: 2, 0;
    pub fb_dither, set_fb_dither: 3;
    pub fb_kval, set_fb_kval: 15, 8;
    pub fb_alpha_threshold, set_fb_alpha_threshold: 23, 16;
}

bitfield! {
    pub struct FB_W_LINESTRIDE(u32);
    impl Debug;
    pub stride, set_stride: 8, 0;
}

bitfield! {
    pub struct FPU_SHAD_SCALE(u32);
    impl Debug;
    pub scale_factor, set_scale_factor: 7, 0;
    pub intensity_shadow, set_intensity_shadow: 8;
}

bitfield! {
    pub struct FPU_PARAM_CFG(u32);
    impl Debug;
    pub pointer_first_burst, set_pointer_first_burst: 3, 0;
    pub pointer_burst, set_pointer_burst: 7, 4;
    pub isp_param_burst_threshold, set_isp_param_burst_threshold: 13, 8;
    pub tsp_param_burst_threshold, set_tsp_param_burst_threshold: 19, 14;
    pub region_header_type, set_region_header_type: 26;
}

bitfield! {
    pub struct HALF_OFFSET(u32);
    impl Debug;
    pub fpu_pixel_half_offset, set_fpu_pixel_half_offset: 0;
    pub tsp_pixel_half_offset, set_tsp_pixel_half_offset: 1;
    pub texture_pixel_half_offset, set_texture_pixel_half_offset: 2;
}

bitfield! {
    pub struct TA_GLOB_TILE_CLIP(u32);
    impl Debug;
    pub tile_x_num, set_tile_x_num: 5, 0;
    pub tile_y_num, set_tile_y_num: 19, 16;
}

bitfield! {
    pub struct TA_YUV_TEX_CTRL(u32);
    impl Debug;
    pub yuv_u_size, set_yuv_u_size: 5, 0;
    pub yuv_v_size, set_yuv_v_size: 13, 8;
    pub yuv_tex, set_yuv_tex: 16;
    pub yuv_form, set_yuv_form: 24;
}
