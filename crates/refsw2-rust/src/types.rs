/*
    This file is part of libswirl
*/
// #include "license/bsd"

// Core structures from core_structs.h ported to Rust

use bitfield::bitfield;

// Bits that affect drawing (for caching params)
pub const PCW_DRAW_MASK: u32 = 0x000000CE;

bitfield! {
    /// Parameter Control Word - 4 bytes
    #[derive(Copy, Clone)]
    pub struct PCW(u32);
    impl Debug;

    // Obj Control (affects drawing)
    pub uv_16bit, set_uv_16bit: 0;        // 0
    pub gouraud, set_gouraud: 1;          // 1
    pub offset, set_offset: 2;            // 1
    pub texture, set_texture: 3;          // 1
    pub col_type, set_col_type: 5, 4;     // 00
    pub volume, set_volume: 6;            // 1
    pub shadow, set_shadow: 7;            // 1

    // Reserved (bits 8-15)

    // Group Control
    pub user_clip, set_user_clip: 17, 16;
    pub strip_len, set_strip_len: 19, 18;
    pub res_2, set_res_2: 22, 20;
    pub group_en, set_group_en: 23;

    // Para Control
    pub list_type, set_list_type: 26, 24;
    pub res_1, set_res_1: 27;
    pub end_of_strip, set_end_of_strip: 28;
    pub para_type, set_para_type: 31, 29;

    // obj_ctrl access
    pub u8, obj_ctrl, set_obj_ctrl: 7, 0;

    // Additional fields for TA preprocessing
    pub s6x, set_s6x: 8;
    pub u8, pteos, set_pteos: 31, 28;
}

impl PCW {
    #[inline]
    pub const fn full(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn set_full(&mut self, val: u32) {
        self.0 = val;
    }
}

bitfield! {
    /// ISP/TSP Instruction Word
    #[derive(Copy, Clone)]
    #[allow(non_camel_case_types)]
    pub struct ISP_TSP(u32);
    impl Debug;

    // Standard fields
    pub dcalc_ctrl, set_dcalc_ctrl: 20;
    pub cache_bypass, set_cache_bypass: 21;
    pub uv_16b, set_uv_16b: 22;      // Replaced by PCW in TA
    pub gouraud, set_gouraud: 23;     // Replaced by PCW in TA
    pub offset, set_offset: 24;       // Replaced by PCW in TA
    pub texture, set_texture: 25;     // Replaced by PCW in TA
    pub z_write_dis, set_z_write_dis: 26;
    pub cull_mode, set_cull_mode: 28, 27;
    pub depth_mode, set_depth_mode: 31, 29;
}

impl ISP_TSP {
    #[inline]
    pub const fn full(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn set_full(&mut self, val: u32) {
        self.0 = val;
    }
}

bitfield! {
    /// ISP Modifier Volume
    #[derive(Copy, Clone)]
    #[allow(non_camel_case_types)]
    pub struct ISP_Modvol(u32);
    impl Debug;

    pub id, set_id: 25, 0;
    pub volume_last, set_volume_last: 26;
    pub cull_mode, set_cull_mode: 28, 27;
    pub depth_mode, set_depth_mode: 31, 29;
}

impl ISP_Modvol {
    #[inline]
    pub const fn full(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn set_full(&mut self, val: u32) {
        self.0 = val;
    }
}

bitfield! {
    /// TSP Instruction Word
    #[derive(Copy, Clone)]
    pub struct TSP(u32);
    impl Debug;

    pub tex_v, set_tex_v: 2, 0;
    pub tex_u, set_tex_u: 5, 3;
    pub shad_instr, set_shad_instr: 7, 6;
    pub mip_map_d, set_mip_map_d: 11, 8;
    pub sup_sample, set_sup_sample: 12;
    pub filter_mode, set_filter_mode: 14, 13;
    pub clamp_v, set_clamp_v: 15;
    pub clamp_u, set_clamp_u: 16;
    pub flip_v, set_flip_v: 17;
    pub flip_u, set_flip_u: 18;
    pub ignore_tex_a, set_ignore_tex_a: 19;
    pub use_alpha, set_use_alpha: 20;
    pub color_clamp, set_color_clamp: 21;
    pub fog_ctrl, set_fog_ctrl: 23, 22;
    pub dst_select, set_dst_select: 24;  // Secondary Accum
    pub src_select, set_src_select: 25;  // Primary Accum
    pub dst_instr, set_dst_instr: 28, 26;
    pub src_instr, set_src_instr: 31, 29;
}

impl TSP {
    #[inline]
    pub const fn full(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn set_full(&mut self, val: u32) {
        self.0 = val;
    }
}

bitfield! {
    /// Texture Control Word
    #[derive(Copy, Clone)]
    pub struct TCW(u32);
    impl Debug;

    pub tex_addr, set_tex_addr: 20, 0;
    // Reserved: 24, 21
    pub stride_sel, set_stride_sel: 25;
    pub scan_order, set_scan_order: 26;
    pub pixel_fmt, set_pixel_fmt: 29, 27;
    pub vq_comp, set_vq_comp: 30;
    pub mip_mapped, set_mip_mapped: 31;

    // For paletted textures
    pub pal_select, set_pal_select: 26, 21;
}

impl TCW {
    #[inline]
    pub const fn full(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn set_full(&mut self, val: u32) {
        self.0 = val;
    }
}

/// Generic vertex storage type
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,

    pub col: [u8; 4],
    pub spc: [u8; 4],

    pub u: f32,
    pub v: f32,

    // Two volumes format
    pub col1: [u8; 4],
    pub spc1: [u8; 4],

    pub u1: f32,
    pub v1: f32,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            col: [0; 4],
            spc: [0; 4],
            u: 0.0,
            v: 0.0,
            col1: [0; 4],
            spc1: [0; 4],
            u1: 0.0,
            v1: 0.0,
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    Pixel1555 = 0,
    Pixel565 = 1,
    Pixel4444 = 2,
    PixelYUV = 3,
    PixelBumpMap = 4,
    PixelPal4 = 5,
    PixelPal8 = 6,
    PixelReserved = 7,
}
