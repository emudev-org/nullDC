// Tile-based rasterization and pixel pipeline
// Implements buffer operations, rasterization, and texture/shading pipeline

use crate::types::*;
use crate::pvr_mem::*;
use crate::pvr_regs::*;
use crate::lists_types::*;
use crate::lists::{TaRect, DrawParametersEx};
use crate::tex_utils::*;
use std::f32;

pub const MAX_RENDER_PIXELS: usize = 1024;
pub const STRIDE_PIXEL_OFFSET: usize = 32;

// Buffer indices
pub const TAG_BUFFER_A: usize = 0;
pub const TAG_BUFFER_B: usize = 1;
pub const DEPTH_BUFFER_A: usize = 0;
pub const DEPTH_BUFFER_B: usize = 1;
pub const DEPTH_BUFFER_C: usize = 2;

pub const PARAMETER_TAG_SORT_MASK: u32 = 0xFFFFFFC0;

pub type ZType = f32;
pub type StencilType = u8;

#[derive(Copy, Clone, Default)]
pub struct TagState {
    pub valid: bool,
    pub rendered: bool,
}

// Global render buffers
pub static mut TAG_STATUS: [TagState; MAX_RENDER_PIXELS] = [TagState { valid: false, rendered: false }; MAX_RENDER_PIXELS];
pub static mut TAG_BUFFER: [[u32; MAX_RENDER_PIXELS]; 2] = [[0; MAX_RENDER_PIXELS]; 2];
pub static mut STENCIL_BUFFER: [StencilType; MAX_RENDER_PIXELS] = [0; MAX_RENDER_PIXELS];
pub static mut COLOR_BUFFER_1: [u32; MAX_RENDER_PIXELS] = [0; MAX_RENDER_PIXELS];
pub static mut COLOR_BUFFER_2: [u32; MAX_RENDER_PIXELS] = [0; MAX_RENDER_PIXELS];
pub static mut DEPTH_BUFFER: [[ZType; MAX_RENDER_PIXELS]; 3] = [[0.0; MAX_RENDER_PIXELS]; 3];

pub static mut MORE_TO_DRAW: bool = false;

#[inline(always)]
fn mmin(a: f32, b: f32, c: f32, d: f32) -> f32 {
    let mut rv = a.min(b);
    rv = c.min(rv);
    d.max(rv)
}

#[inline(always)]
fn mmax(a: f32, b: f32, c: f32, d: f32) -> f32 {
    let mut rv = a.max(b);
    rv = c.max(rv);
    d.min(rv)
}

#[inline(always)]
fn mask_w(w: f32) -> f32 {
    w
}

// Buffer operations
pub unsafe fn clear_buffers(param_value: u32, depth_value: f32, stencil_value: u8) {
    unsafe {
        let zb = &mut DEPTH_BUFFER[DEPTH_BUFFER_A];
        let stencil = &mut STENCIL_BUFFER;
        let pb = &mut TAG_BUFFER[TAG_BUFFER_A];
        let ts = &mut TAG_STATUS;

        for i in 0..MAX_RENDER_PIXELS {
            zb[i] = mask_w(depth_value);
            stencil[i] = stencil_value;
            pb[i] = param_value;
            ts[i] = TagState { valid: true, rendered: false };
        }
    }
}

pub unsafe fn clear_param_status_buffer() {
    unsafe {
        let ts = &mut TAG_STATUS;
        for i in 0..MAX_RENDER_PIXELS {
            ts[i] = TagState { valid: false, rendered: false };
        }
    }
}

pub unsafe fn peel_buffers_pt_initial(depth_value: f32) {
    unsafe {
        DEPTH_BUFFER[DEPTH_BUFFER_C].copy_from_slice(&DEPTH_BUFFER[DEPTH_BUFFER_A]);
        let ts = &mut TAG_STATUS;
        let stencil = &mut STENCIL_BUFFER;

        for i in 0..MAX_RENDER_PIXELS {
            ts[i] = TagState { valid: false, rendered: false };
            stencil[i] = 0;
        }
    }
}

pub unsafe fn peel_buffers_pt() {
    unsafe {
        DEPTH_BUFFER[DEPTH_BUFFER_B].copy_from_slice(&DEPTH_BUFFER[DEPTH_BUFFER_A]);
        TAG_BUFFER[TAG_BUFFER_B].copy_from_slice(&TAG_BUFFER[TAG_BUFFER_A]);
    }
}

pub unsafe fn set_tag_to_max() {
    unsafe {
        TAG_BUFFER[TAG_BUFFER_A].fill(0xFFFFFFFF);
    }
}

pub unsafe fn peel_buffers(depth_value: f32, stencil_value: u8) {
    unsafe {
        DEPTH_BUFFER[DEPTH_BUFFER_B].copy_from_slice(&DEPTH_BUFFER[DEPTH_BUFFER_A]);
        TAG_BUFFER[TAG_BUFFER_B].copy_from_slice(&TAG_BUFFER[TAG_BUFFER_A]);

        let zb = &mut DEPTH_BUFFER[DEPTH_BUFFER_A];
        let stencil = &mut STENCIL_BUFFER;
        let ts = &mut TAG_STATUS;

        for i in 0..MAX_RENDER_PIXELS {
            zb[i] = mask_w(depth_value);
            ts[i] = TagState { valid: false, rendered: false };
            stencil[i] = stencil_value;
        }
    }
}

pub unsafe fn summarize_stencil_or() {
    unsafe {
        let stencil = &mut STENCIL_BUFFER;
        for i in 0..MAX_RENDER_PIXELS {
            if (stencil[i] & 0b100) != 0 {
                stencil[i] |= stencil[i] >> 1;
                stencil[i] &= 0b001;
            }
        }
    }
}

pub unsafe fn summarize_stencil_and() {
    unsafe {
        let stencil = &mut STENCIL_BUFFER;
        for i in 0..MAX_RENDER_PIXELS {
            if (stencil[i] & 0b100) != 0 {
                stencil[i] &= stencil[i] >> 1;
                stencil[i] &= 0b001;
            }
        }
    }
}

pub unsafe fn clear_more_to_draw() {
    unsafe {
        MORE_TO_DRAW = false;
    }
}

pub unsafe fn get_more_to_draw() -> bool {
    unsafe { MORE_TO_DRAW }
}

pub unsafe fn get_color_output_buffer() -> *mut u8 {
    unsafe { COLOR_BUFFER_1.as_mut_ptr() as *mut u8 }
}

// Helper structs for plane stepping
#[derive(Copy, Clone, Default)]
pub struct PlaneStepper3 {
    pub ddx: f32,
    pub ddy: f32,
    pub c: f32,
}

impl PlaneStepper3 {
    pub fn setup(area: &TaRect, v1: &Vertex, v2: &Vertex, v3: &Vertex, val1: f32, val2: f32, val3: f32) -> Self {
        let dx1 = v2.x - v1.x;
        let dx2 = v3.x - v1.x;
        let dy1 = v2.y - v1.y;
        let dy2 = v3.y - v1.y;
        let dv1 = val2 - val1;
        let dv2 = val3 - val1;

        let det = dx1 * dy2 - dx2 * dy1;
        let det_inv = if det.abs() < 0.0001 { 0.0 } else { 1.0 / det };

        let ddx = (dv1 * dy2 - dv2 * dy1) * det_inv;
        let ddy = (dv2 * dx1 - dv1 * dx2) * det_inv;

        let c = val1 - ddx * (v1.x - area.left as f32) - ddy * (v1.y - area.top as f32);

        PlaneStepper3 { ddx, ddy, c }
    }

    #[inline(always)]
    pub fn ip(&self, x: f32, y: f32) -> f32 {
        self.c + self.ddx * x + self.ddy * y
    }

    #[inline(always)]
    pub fn ip_u8(&self, x: f32, y: f32, w: f32) -> u8 {
        let val = (self.c + self.ddx * x + self.ddy * y) * w;
        val.max(0.0).min(255.0) as u8
    }
}

#[derive(Copy, Clone)]
pub struct InterpolatedParameters {
    pub inv_w: PlaneStepper3,
    pub col: [[PlaneStepper3; 4]; 2],
    pub ofs: [[PlaneStepper3; 4]; 2],
    pub u: [PlaneStepper3; 2],
    pub v: [PlaneStepper3; 2],
}

impl Default for InterpolatedParameters {
    fn default() -> Self {
        InterpolatedParameters {
            inv_w: PlaneStepper3::default(),
            col: [[PlaneStepper3::default(); 4]; 2],
            ofs: [[PlaneStepper3::default(); 4]; 2],
            u: [PlaneStepper3::default(); 2],
            v: [PlaneStepper3::default(); 2],
        }
    }
}

impl InterpolatedParameters {
    pub unsafe fn setup(
        &mut self,
        area: &TaRect,
        params: &DrawParametersEx,
        v1: Vertex,
        v2: Vertex,
        v3: Vertex,
        two_volumes: bool
    ) {
        // Setup inverse W (depth)
        self.inv_w = PlaneStepper3::setup(area, &v1, &v2, &v3, v1.z, v2.z, v3.z);

        // Setup U/V for volume 0
        self.u[0] = PlaneStepper3::setup(area, &v1, &v2, &v3, v1.u * v1.z, v2.u * v2.z, v3.u * v3.z);
        self.v[0] = PlaneStepper3::setup(area, &v1, &v2, &v3, v1.v * v1.z, v2.v * v2.z, v3.v * v3.z);

        // Setup colors for volume 0
        if params.isp.gouraud() {
            // Gouraud shading - interpolate colors from each vertex
            for i in 0..4 {
                self.col[0][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                    v1.col[i] as f32 * v1.z,
                    v2.col[i] as f32 * v2.z,
                    v3.col[i] as f32 * v3.z);
            }

            for i in 0..4 {
                self.ofs[0][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                    v1.spc[i] as f32 * v1.z,
                    v2.spc[i] as f32 * v2.z,
                    v3.spc[i] as f32 * v3.z);
            }
        } else {
            // Flat shading - use v3 color for all vertices
            for i in 0..4 {
                self.col[0][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                    v3.col[i] as f32 * v1.z,
                    v3.col[i] as f32 * v2.z,
                    v3.col[i] as f32 * v3.z);
            }

            for i in 0..4 {
                self.ofs[0][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                    v3.spc[i] as f32 * v1.z,
                    v3.spc[i] as f32 * v2.z,
                    v3.spc[i] as f32 * v3.z);
            }
        }

        // Setup volume 1 if two volumes (for shadows)
        if two_volumes {
            self.u[1] = PlaneStepper3::setup(area, &v1, &v2, &v3, v1.u1 * v1.z, v2.u1 * v2.z, v3.u1 * v3.z);
            self.v[1] = PlaneStepper3::setup(area, &v1, &v2, &v3, v1.v1 * v1.z, v2.v1 * v2.z, v3.v1 * v3.z);

            if params.isp.gouraud() {
                for i in 0..4 {
                    self.col[1][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                        v1.col1[i] as f32 * v1.z,
                        v2.col1[i] as f32 * v2.z,
                        v3.col1[i] as f32 * v3.z);
                }

                for i in 0..4 {
                    self.ofs[1][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                        v1.spc1[i] as f32 * v1.z,
                        v2.spc1[i] as f32 * v2.z,
                        v3.spc1[i] as f32 * v3.z);
                }
            } else {
                for i in 0..4 {
                    self.col[1][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                        v3.col1[i] as f32 * v1.z,
                        v3.col1[i] as f32 * v2.z,
                        v3.col1[i] as f32 * v3.z);
                }

                for i in 0..4 {
                    self.ofs[1][i] = PlaneStepper3::setup(area, &v1, &v2, &v3,
                        v3.spc1[i] as f32 * v1.z,
                        v3.spc1[i] as f32 * v2.z,
                        v3.spc1[i] as f32 * v3.z);
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct FpuEntry {
    pub params: DrawParametersEx,
    pub ips: InterpolatedParameters,
}

impl Default for FpuEntry {
    fn default() -> Self {
        FpuEntry {
            params: DrawParametersEx::default(),
            ips: InterpolatedParameters::default(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct FpuCacheEntry {
    pub entry: FpuEntry,
    pub tag: u32,
}

impl Default for FpuCacheEntry {
    fn default() -> Self {
        FpuCacheEntry {
            entry: FpuEntry::default(),
            tag: 0,
        }
    }
}

const EMPTY_FPU_ENTRY: FpuCacheEntry = FpuCacheEntry {
    entry: FpuEntry {
        params: DrawParametersEx {
            isp: ISP_TSP(0),
            tsp: [TSP(0), TSP(0)],
            tcw: [TCW(0), TCW(0)],
        },
        ips: InterpolatedParameters {
            inv_w: PlaneStepper3 { ddx: 0.0, ddy: 0.0, c: 0.0 },
            col: [[PlaneStepper3 { ddx: 0.0, ddy: 0.0, c: 0.0 }; 4]; 2],
            ofs: [[PlaneStepper3 { ddx: 0.0, ddy: 0.0, c: 0.0 }; 4]; 2],
            u: [PlaneStepper3 { ddx: 0.0, ddy: 0.0, c: 0.0 }; 2],
            v: [PlaneStepper3 { ddx: 0.0, ddy: 0.0, c: 0.0 }; 2],
        },
    },
    tag: 0,
};

pub static mut FPU_CACHE: [FpuCacheEntry; 32] = [EMPTY_FPU_ENTRY; 32];

pub unsafe fn clear_fpu_cache() {
    unsafe {
        for entry in &mut FPU_CACHE {
            entry.tag = 0;
        }
    }
}

// Get or decode FPU parameter entry from tag
pub unsafe fn get_fpu_entry(
    rect: &TaRect,
    render_mode: u8,
    core_tag: ISP_BACKGND_T_type
) -> &'static FpuEntry {
    unsafe {
        let cache_index = (core_tag.param_offs_in_words() & 31) as usize;

        // Check if cached
        if FPU_CACHE[cache_index].tag == core_tag.full() {
            return &FPU_CACHE[cache_index].entry;
        }

        // Decode parameters and vertices
        let param_base = crate::pvr_regs::param_base();
        let param_addr = param_base + core_tag.param_offs_in_words() * 4;
        let skip = core_tag.skip();
        let fpu_shad_scale = crate::pvr_regs::fpu_shad_scale();
        let two_volumes = core_tag.shadow() && !fpu_shad_scale.intensity_shadow();

        let mut vtx = [Vertex::default(); 3];
        let tag_offset = core_tag.tag_offset() as usize;

        crate::lists::decode_pvr_vertices(
            &mut FPU_CACHE[cache_index].entry.params,
            param_addr,
            skip,
            two_volumes,
            &mut vtx,
            tag_offset
        );

        // Setup interpolation parameters
        FPU_CACHE[cache_index].entry.ips.setup(
            rect,
            &FPU_CACHE[cache_index].entry.params,
            vtx[0],
            vtx[1],
            vtx[2],
            two_volumes
        );

        FPU_CACHE[cache_index].tag = core_tag.full();

        &FPU_CACHE[cache_index].entry
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Color {
    pub b: u8,
    pub g: u8,
    pub r: u8,
    pub a: u8,
}

impl Color {
    pub fn from_raw(raw: u32) -> Self {
        Color {
            b: (raw & 0xFF) as u8,
            g: ((raw >> 8) & 0xFF) as u8,
            r: ((raw >> 16) & 0xFF) as u8,
            a: ((raw >> 24) & 0xFF) as u8,
        }
    }

    pub fn to_raw(&self) -> u32 {
        self.b as u32 | ((self.g as u32) << 8) | ((self.r as u32) << 16) | ((self.a as u32) << 24)
    }

    pub fn bgra(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    pub fn bgra_mut(&mut self) -> [&mut u8; 4] {
        [&mut self.b, &mut self.g, &mut self.r, &mut self.a]
    }
}

// Helper for top-left edge rule
#[inline(always)]
fn is_top_left(x: f32, y: f32) -> bool {
    let is_top = y == 0.0 && x > 0.0;
    let is_left = y < 0.0;
    is_top || is_left
}

// Flush NaN values to zero
#[inline(always)]
fn flush_nan(a: f32) -> f32 {
    if a.is_nan() { 0.0 } else { a }
}

// Mipmap offset lookup table
const MIP_POINT: [u32; 11] = [
    0x00003,          // 1
    0x00001 * 4,      // 2
    0x00002 * 4,      // 4
    0x00006 * 4,      // 8
    0x00016 * 4,      // 16
    0x00056 * 4,      // 32
    0x00156 * 4,      // 64
    0x00556 * 4,      // 128
    0x01556 * 4,      // 256
    0x05556 * 4,      // 512
    0x15556 * 4,      // 1024
];

// Clamp and flip a texture coordinate
#[inline(always)]
fn clamp_flip(coord: i32, size: i32, pp_clamp: bool, pp_flip: bool) -> i32 {
    if pp_clamp {
        // Clamp mode
        if coord < 0 {
            0
        } else if coord >= size {
            size - 1
        } else {
            coord
        }
    } else if pp_flip {
        // Flip mode
        let mut c = coord & (size * 2 - 1);
        if (c & size) != 0 {
            c ^= size * 2 - 1;
        }
        c
    } else {
        // Wrap mode
        coord & (size - 1)
    }
}

// Convert u8 to 256 scale (adds half bit for rounding)
#[inline(always)]
fn to_u8_256(v: u8) -> u32 {
    v as u32 + ((v as u32) >> 7)
}

// Depth processing for a pixel
pub unsafe fn pixel_flush_isp(
    render_mode: u8,
    depth_mode: u32,
    z_write_dis: bool,
    x: f32,
    y: f32,
    inv_w: f32,
    index: usize,
    tag: u32
) {
    unsafe {
        let pb = &mut TAG_BUFFER[TAG_BUFFER_A][index];
        let ts = &mut TAG_STATUS[index];
        let pb2 = TAG_BUFFER[TAG_BUFFER_B][index];
        let zb = &mut DEPTH_BUFFER[DEPTH_BUFFER_A][index];
        let zb2 = DEPTH_BUFFER[DEPTH_BUFFER_B][index];
        let stencil = &mut STENCIL_BUFFER[index];

        let mut mode = depth_mode;

        // Adjust depth mode based on render mode
        match render_mode {
            1 | 2 => mode = 6, // PUNCHTHROUGH_PASS0, PUNCHTHROUGH_PASSN
            4 => mode = 3,     // TRANSLUCENT_AUTOSORT
            6 => mode = 6,     // MODIFIER
            _ => {}
        }

        // Depth test
        let depth_passed = match mode {
            0 => return, // never
            1 => inv_w < *zb, // less
            2 => inv_w == *zb, // equal
            3 => {
                if inv_w > *zb {
                    if render_mode == 4 { // TRANSLUCENT_AUTOSORT
                        MORE_TO_DRAW = true;
                    }
                    return;
                }
                true
            },
            4 => inv_w > *zb, // greater
            5 => inv_w != *zb, // not equal
            6 => inv_w >= *zb, // greater or equal
            7 => true, // always
            _ => return,
        };

        if !depth_passed {
            return;
        }

        // Handle different render modes
        match render_mode {
            0 => { // OPAQUE
                if !z_write_dis {
                    *zb = mask_w(inv_w);
                }
                *pb = tag;
                ts.valid = true;
            },
            6 => { // MODIFIER
                *stencil ^= 0b0010;
                *stencil |= 0b100;
            },
            1 => { // PUNCHTHROUGH_PASS0
                *zb = mask_w(inv_w);
                *pb = tag;
                ts.valid = true;
            },
            2 => { // PUNCHTHROUGH_PASSN
                if ts.rendered {
                    return;
                }
                if inv_w > zb2 {
                    return;
                }
                if inv_w == zb2 || inv_w == *zb {
                    let tag_rendered = pb2;
                    if (tag & PARAMETER_TAG_SORT_MASK) <= (tag_rendered & PARAMETER_TAG_SORT_MASK) {
                        return;
                    }
                }
                MORE_TO_DRAW = true;
                *zb = mask_w(inv_w);
                *pb = tag;
            },
            5 => { // TRANSLUCENT_PRESORT
                if !z_write_dis {
                    *zb = mask_w(inv_w);
                }
                *pb = tag;
                ts.valid = true;
            },
            4 => { // TRANSLUCENT_AUTOSORT
                if inv_w < zb2 {
                    return;
                }
                if inv_w == zb2 {
                    let tag_rendered = pb2;
                    if (tag & PARAMETER_TAG_SORT_MASK) <= (tag_rendered & PARAMETER_TAG_SORT_MASK) && tag_rendered != 0xFFFFFFFF {
                        return;
                    }
                }
                if inv_w == *zb {
                    let tag_rendered = pb2;
                    if (tag & PARAMETER_TAG_SORT_MASK) <= (tag_rendered & PARAMETER_TAG_SORT_MASK) && tag_rendered != 0xFFFFFFFF {
                        return;
                    }
                    if ts.valid {
                        let tag_pending = *pb;
                        if (tag & PARAMETER_TAG_SORT_MASK) > (tag_pending & PARAMETER_TAG_SORT_MASK) {
                            MORE_TO_DRAW = true;
                            return;
                        }
                    }
                }
                *zb = mask_w(inv_w);
                if ts.valid {
                    MORE_TO_DRAW = true;
                }
                ts.valid = true;
                *pb = tag;
            },
            _ => {}
        }
    }
}

// Rasterize a single triangle
pub unsafe fn rasterize_triangle(
    render_mode: u8,
    params: &DrawParametersEx,
    tag: u32,
    v1: &Vertex,
    v2: &Vertex,
    v3: &Vertex,
    v4: Option<&Vertex>,
    area: &TaRect
) {
    let y1 = flush_nan(v1.y);
    let y2 = flush_nan(v2.y);
    let y3 = flush_nan(v3.y);
    let y4 = if let Some(v) = v4 { flush_nan(v.y) } else { 0.0 };

    let x1 = flush_nan(v1.x);
    let x2 = flush_nan(v2.x);
    let x3 = flush_nan(v3.x);
    let x4 = if let Some(v) = v4 { flush_nan(v.x) } else { 0.0 };

    let mut sgn = 1.0;
    let tri_area = (x1 - x3) * (y2 - y3) - (y1 - y3) * (x2 - x3);

    if tri_area > 0.0 {
        sgn = -1.0;
    }

    // Cull based on area and mode
    let cull_mode = params.isp.cull_mode();
    if cull_mode != 0 {
        let abs_area = tri_area.abs();

        let fpu_cull_val = unsafe { crate::pvr_regs::fpu_cull_val() };
        if abs_area < fpu_cull_val {
            return;
        }

        if cull_mode >= 2 {
            let mode = cull_mode & 1;
            if (mode == 0 && tri_area < 0.0) || (mode == 1 && tri_area > 0.0) {
                return;
            }
        }
    }

    // Half-edge constants
    let dx12 = sgn * (x1 - x2);
    let dx23 = sgn * (x2 - x3);
    let dx31 = if v4.is_some() { sgn * (x3 - x4) } else { sgn * (x3 - x1) };
    let dx41 = if v4.is_some() { sgn * (x4 - x1) } else { 0.0 };

    let dy12 = sgn * (y1 - y2);
    let dy23 = sgn * (y2 - y3);
    let dy31 = if v4.is_some() { sgn * (y3 - y4) } else { sgn * (y3 - y1) };
    let dy41 = if v4.is_some() { sgn * (y4 - y1) } else { 0.0 };

    let c1 = dy12 * (x1 - area.left as f32) - dx12 * (y1 - area.top as f32);
    let c2 = dy23 * (x2 - area.left as f32) - dx23 * (y2 - area.top as f32);
    let c3 = dy31 * (x3 - area.left as f32) - dx31 * (y3 - area.top as f32);
    let c4 = if v4.is_some() { dy41 * (x4 - area.left as f32) - dx41 * (y4 - area.top as f32) } else { 1.0 };

    let t1 = is_top_left(x2 - x1, y2 - y1);
    let t2 = is_top_left(x3 - x2, y3 - y2);
    let (t3, t4) = if v4.is_none() {
        (is_top_left(x1 - x3, y1 - y3), true)
    } else {
        (is_top_left(x4 - x3, y4 - y3), is_top_left(x1 - x4, y1 - y4))
    };

    // Setup depth interpolation
    let z_stepper = PlaneStepper3::setup(area, v1, v2, v3, v1.z, v2.z, v3.z);

    let half_offset = unsafe { crate::pvr_regs::half_offset() };
    let halfpixel = if half_offset.fpu_pixel_half_offset() { 0.5 } else { 0.0 };
    let mut y_ps = halfpixel;
    let minx_ps = halfpixel;

    // Loop through all pixels in the tile
    for _y in 0..32 {
        let mut x_ps = minx_ps;

        // Early reject for entire scanline
        let kxhs12 = c1 + dx12 * y_ps - dy12 * 0.0;
        let kxhs23 = c2 + dx23 * y_ps - dy23 * 0.0;
        let kxhs31 = c3 + dx31 * y_ps - dy31 * 0.0;
        let kxhs41 = c4 + dx41 * y_ps - dy41 * 0.0;
        let zxhs12 = c1 + dx12 * y_ps - dy12 * 32.5;
        let zxhs23 = c2 + dx23 * y_ps - dy23 * 32.5;
        let zxhs31 = c3 + dx31 * y_ps - dy31 * 32.5;
        let zxhs41 = c4 + dx41 * y_ps - dy41 * 32.5;

        if (kxhs12 < 0.0 && zxhs12 < 0.0) || (kxhs23 < 0.0 && zxhs23 < 0.0) ||
           (kxhs31 < 0.0 && zxhs31 < 0.0) || (kxhs41 < 0.0 && zxhs41 < 0.0) {
            y_ps += 1.0;
            continue;
        }

        for x in 0..32 {
            let xhs12 = c1 + dx12 * y_ps - dy12 * x_ps;
            let xhs23 = c2 + dx23 * y_ps - dy23 * x_ps;
            let xhs31 = c3 + dx31 * y_ps - dy31 * x_ps;
            let xhs41 = c4 + dx41 * y_ps - dy41 * x_ps;

            let in_triangle = (xhs12 > 0.0 || (t1 && xhs12 == 0.0)) &&
                             (xhs23 > 0.0 || (t2 && xhs23 == 0.0)) &&
                             (xhs31 > 0.0 || (t3 && xhs31 == 0.0)) &&
                             (xhs41 > 0.0 || (t4 && xhs41 == 0.0));

            if in_triangle {
                let index = (_y * 32 + x) as usize;
                let inv_w = z_stepper.ip(x_ps, y_ps);
                unsafe {
                    pixel_flush_isp(
                        render_mode,
                        params.isp.depth_mode(),
                        params.isp.z_write_dis(),
                        x_ps,
                        y_ps,
                        inv_w,
                        index,
                        tag
                    );
                }
            }

            x_ps += 1.0;
        }

        y_ps += 1.0;
    }
}

// Interpolate base color for a pixel
#[inline(always)]
unsafe fn interpolate_base(col: &[PlaneStepper3; 4], x: f32, y: f32, w: f32, use_alpha: bool, cheap_shadows: bool, in_volume: bool) -> Color {
    let mut mult = 256;

    if cheap_shadows && in_volume {
        let fpu_shad_scale = unsafe { crate::pvr_regs::fpu_shad_scale() };
        mult = to_u8_256(fpu_shad_scale.scale_factor());
    }

    let mut rv = Color {
        b: (0.5 + col[0].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        g: (0.5 + col[1].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        r: (0.5 + col[2].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        a: (0.5 + col[3].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
    };

    if !use_alpha {
        rv.a = 255;
    }

    rv
}

// Interpolate offset color for a pixel
#[inline(always)]
unsafe fn interpolate_offs(ofs: &[PlaneStepper3; 4], x: f32, y: f32, w: f32, cheap_shadows: bool, in_volume: bool) -> Color {
    let mut mult = 256;

    if cheap_shadows && in_volume {
        let fpu_shad_scale = unsafe { crate::pvr_regs::fpu_shad_scale() };
        mult = to_u8_256(fpu_shad_scale.scale_factor());
    }

    Color {
        b: (0.5 + ofs[0].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        g: (0.5 + ofs[1].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        r: (0.5 + ofs[2].ip_u8(x, y, w) as f32 * mult as f32 / 256.0) as u8,
        a: (0.5 + ofs[3].ip_u8(x, y, w) as f32) as u8,
    }
}

// Color combiner - combine base, texture, and offset colors
#[inline(always)]
fn color_combiner(base: Color, textel: Color, offset: Color, pp_texture: bool, pp_offset: bool, pp_shad_instr: u32) -> Color {
    let mut rv = base;

    if pp_texture {
        match pp_shad_instr {
            0 => {
                // Replace with texture
                rv = textel;
            }
            1 => {
                // Modulate RGB, texture alpha
                rv.b = (textel.b as u32 * to_u8_256(base.b) / 256) as u8;
                rv.g = (textel.g as u32 * to_u8_256(base.g) / 256) as u8;
                rv.r = (textel.r as u32 * to_u8_256(base.r) / 256) as u8;
                rv.a = textel.a;
            }
            2 => {
                // Alpha blend with texture
                let tb = to_u8_256(textel.a);
                let cb = 256 - tb;
                rv.b = ((textel.b as u32 * tb + base.b as u32 * cb) / 256) as u8;
                rv.g = ((textel.g as u32 * tb + base.g as u32 * cb) / 256) as u8;
                rv.r = ((textel.r as u32 * tb + base.r as u32 * cb) / 256) as u8;
                rv.a = base.a;
            }
            3 => {
                // Modulate all channels
                rv.b = (textel.b as u32 * to_u8_256(base.b) / 256) as u8;
                rv.g = (textel.g as u32 * to_u8_256(base.g) / 256) as u8;
                rv.r = (textel.r as u32 * to_u8_256(base.r) / 256) as u8;
                rv.a = (textel.a as u32 * to_u8_256(base.a) / 256) as u8;
            }
            _ => {}
        }

        if pp_offset {
            // Add offset color (saturate)
            rv.b = rv.b.saturating_add(offset.b);
            rv.g = rv.g.saturating_add(offset.g);
            rv.r = rv.r.saturating_add(offset.r);
        }
    }

    rv
}

// Blending coefficient calculation
#[inline(always)]
fn blend_coefs(src: Color, dst: Color, pp_alpha_inst: u32, src_other: bool) -> Color {
    let mut rv = Color { b: 0, g: 0, r: 0, a: 0 };

    match pp_alpha_inst >> 1 {
        0 => {}, // zero - already initialized
        1 => rv = if src_other { src } else { dst }, // other color
        2 => { // src alpha
            rv.b = src.a;
            rv.g = src.a;
            rv.r = src.a;
            rv.a = src.a;
        }
        3 => { // dst alpha
            rv.b = dst.a;
            rv.g = dst.a;
            rv.r = dst.a;
            rv.a = dst.a;
        }
        _ => {}
    }

    if (pp_alpha_inst & 1) != 0 {
        rv.b = 255 - rv.b;
        rv.g = 255 - rv.g;
        rv.r = 255 - rv.r;
        rv.a = 255 - rv.a;
    }

    rv
}

// Blending unit - performs alpha blending
#[inline(always)]
unsafe fn blending_unit(
    index: usize,
    col: Color,
    pp_src_sel: u32,
    pp_dst_sel: u32,
    pp_src_inst: u32,
    pp_dst_inst: u32,
    pp_alpha_test: bool
) -> bool {
    unsafe {
        let mut at = true;

        let mut final_col = col;
        if pp_alpha_test {
            let pt_alpha_ref = crate::pvr_regs::pt_alpha_ref() as u8;
            if col.a < pt_alpha_ref {
                final_col.a = 0;
                at = false;
            } else {
                final_col.a = 255;
            }
        }

        let src = if pp_src_sel != 0 {
            Color::from_raw(COLOR_BUFFER_2[index])
        } else {
            final_col
        };

        let dst = if pp_dst_sel != 0 {
            Color::from_raw(COLOR_BUFFER_2[index])
        } else {
            Color::from_raw(COLOR_BUFFER_1[index])
        };

        let src_blend = blend_coefs(src, dst, pp_src_inst, false);
        let dst_blend = blend_coefs(src, dst, pp_dst_inst, true);

        let mut rv = Color { b: 0, g: 0, r: 0, a: 0 };
        rv.b = ((src.b as u32 * to_u8_256(src_blend.b) + dst.b as u32 * to_u8_256(dst_blend.b)) >> 8).min(255) as u8;
        rv.g = ((src.g as u32 * to_u8_256(src_blend.g) + dst.g as u32 * to_u8_256(dst_blend.g)) >> 8).min(255) as u8;
        rv.r = ((src.r as u32 * to_u8_256(src_blend.r) + dst.r as u32 * to_u8_256(dst_blend.r)) >> 8).min(255) as u8;
        rv.a = ((src.a as u32 * to_u8_256(src_blend.a) + dst.a as u32 * to_u8_256(dst_blend.a)) >> 8).min(255) as u8;

        if pp_dst_sel != 0 {
            COLOR_BUFFER_2[index] = rv.to_raw();
        } else {
            COLOR_BUFFER_1[index] = rv.to_raw();
        }

        at
    }
}

// Static offset color that persists across invocations (for bump maps)
static mut OFFS: Color = Color { b: 0x80, g: 0x40, r: 0x00, a: 0x20 };

// ============================================================================
// Texture Fetching Infrastructure
// ============================================================================

// Pixel format constants (from types.rs)
const PIXEL_1555: u32 = 0;
const PIXEL_565: u32 = 1;
const PIXEL_4444: u32 = 2;
const PIXEL_YUV: u32 = 3;
const PIXEL_BUMPMAP: u32 = 4;
const PIXEL_PAL4: u32 = 5;
const PIXEL_PAL8: u32 = 6;
const PIXEL_RESERVED: u32 = 7;

// Calculate texture base address (with VQ codebook offset)
#[inline(always)]
unsafe fn tex_address_gen(vq_comp: bool, tcw: TCW) -> u32 {
    let mut base_address = tcw.tex_addr() << 3;
    if vq_comp {
        base_address += 256 * 4 * 2;
    }
    base_address
}

// Calculate texture offset for u,v coordinates
#[inline(always)]
unsafe fn tex_offset_gen(vq_comp: bool, mip_mapped: bool, scan_order: bool, tsp: TSP, u: i32, v: i32, stride: u32, mip_level: u32) -> u32 {
    let mip_offset = if mip_mapped {
        MIP_POINT[(3 + tsp.tex_u() - mip_level) as usize]
    } else {
        0
    };

    if vq_comp || !scan_order {
        if mip_mapped {
            mip_offset + twop(u as u32, v as u32, tsp.tex_u() - mip_level, tsp.tex_u() - mip_level)
        } else {
            mip_offset + twop(u as u32, v as u32, tsp.tex_u(), tsp.tex_v())
        }
    } else {
        mip_offset + u as u32 + stride * v as u32
    }
}

// Calculate bits per pixel (in 4.1 format for VQ)
#[inline(always)]
const fn bits_per_pixel(vq_comp: bool, pixel_fmt: u32) -> u32 {
    let rv = if pixel_fmt == PIXEL_PAL8 {
        8
    } else if pixel_fmt == PIXEL_PAL4 {
        4
    } else {
        16
    };

    if vq_comp {
        8 * 2 / (64 / rv) // 8 bpp / (pixels per 64 bits)
    } else {
        rv * 2
    }
}

// Calculate texture stride
#[inline(always)]
unsafe fn tex_stride(stride_sel: bool, scan_order: bool, tex_u: u32, mip_level: u32) -> u32 {
    if stride_sel && scan_order {
        (crate::pvr_regs::text_control() & 31) * 32
    } else {
        (8u32 << tex_u) >> mip_level
    }
}

// VQ codebook lookup
#[inline(always)]
unsafe fn vq_lookup(start_address: u32, memtel: u64, offset: u32) -> u64 {
    let memtel8 = &memtel as *const u64 as *const u8;
    let vq_book = EMU_VRAM.add((start_address & (VRAM_MASK - 7)) as usize) as *const u64;
    let index = *memtel8.add((offset & 7) as usize);
    *vq_book.add(index as usize)
}

// Decode textel from memory to raw color value
#[inline(always)]
unsafe fn decode_textel(pixel_fmt: u32, pal_select: u32, memtel: u64, offset: u32) -> u32 {
    let memtel_32 = &memtel as *const u64 as *const u32;
    let memtel_16 = &memtel as *const u64 as *const u16;
    let memtel_8 = &memtel as *const u64 as *const u8;

    match pixel_fmt {
        PIXEL_RESERVED | PIXEL_1555 | PIXEL_565 | PIXEL_4444 | PIXEL_BUMPMAP => {
            *memtel_16.add((offset & 3) as usize) as u32
        }
        PIXEL_YUV => {
            let memtel_yuv = *memtel_32.add((offset & 1) as usize);
            let memtel_yuv8 = &memtel_yuv as *const u32 as *const u8;
            let y = *memtel_yuv8.add((1 + (offset & 2)) as usize) as i32;
            let u = *memtel_yuv8.add(0) as i32;
            let v = *memtel_yuv8.add(2) as i32;
            yuv422(y, u, v)
        }
        PIXEL_PAL4 => {
            let local_idx = (memtel >> ((offset & 15) * 4)) & 15;
            let idx = pal_select * 16 | (local_idx as u32);
            let palette_ram = crate::pvr_regs::palette_ram();
            *palette_ram.add(idx as usize)
        }
        PIXEL_PAL8 => {
            let local_idx = *memtel_8.add((offset & 7) as usize);
            let idx = (pal_select / 16) * 256 | (local_idx as u32);
            let palette_ram = crate::pvr_regs::palette_ram();
            *palette_ram.add(idx as usize)
        }
        _ => 0xDEADBEEF
    }
}

// Get expansion format for pixel format
#[inline(always)]
unsafe fn get_expand_format(pixel_fmt: u32) -> u32 {
    if pixel_fmt == PIXEL_PAL4 || pixel_fmt == PIXEL_PAL8 {
        crate::pvr_regs::pal_ram_ctrl() & 3
    } else if pixel_fmt == PIXEL_BUMPMAP || pixel_fmt == PIXEL_YUV {
        3
    } else {
        pixel_fmt & 3
    }
}

// Expand color to ARGB8888
#[inline(always)]
fn expand_to_argb8888(scan_order: bool, color: u32, mode: u32) -> u32 {
    match mode {
        0 => argb1555_32(color as u16),
        1 => argb565_32(color as u16),
        2 => argb4444_32(color as u16),
        3 => argb8888_32(color),
        _ => 0xDEADBEEF
    }
}

// Main texture fetch function
#[inline(always)]
unsafe fn texture_fetch(
    vq_comp: bool,
    mip_mapped: bool,
    scan_order_: bool,
    stride_sel_: bool,
    pixel_fmt: u32,
    tsp: TSP,
    tcw: TCW,
    u: i32,
    v: i32,
    mip_level: u32
) -> Color {
    // YUV fallback for smallest mip level
    if mip_level == (tsp.tex_u() + 3) && pixel_fmt == PIXEL_YUV {
        return texture_fetch(vq_comp, mip_mapped, scan_order_, stride_sel_, PIXEL_565, tsp, tcw, u, v, mip_level);
    }

    // These are fixed to zero for pal4/pal8
    let scan_order = scan_order_ && !(pixel_fmt == PIXEL_PAL4 || pixel_fmt == PIXEL_PAL8);
    let stride_sel = stride_sel_ && !(pixel_fmt == PIXEL_PAL4 || pixel_fmt == PIXEL_PAL8);

    let stride = tex_stride(stride_sel, scan_order, tsp.tex_u(), mip_level);
    let start_address = tcw.tex_addr() << 3;
    let fbpp = bits_per_pixel(vq_comp, pixel_fmt);
    let base_address = tex_address_gen(vq_comp, tcw);
    let offset = tex_offset_gen(vq_comp, mip_mapped, scan_order, tsp, u, v, stride, mip_level);

    let mut memtel = *(EMU_VRAM.add(((base_address + offset * fbpp / 16) & (VRAM_MASK - 7)) as usize) as *const u64);

    if vq_comp {
        memtel = vq_lookup(start_address, memtel, offset * fbpp / 16);
    }

    let textel = decode_textel(pixel_fmt, tcw.pal_select(), memtel, offset);
    let expand_format = get_expand_format(pixel_fmt);
    let textel = expand_to_argb8888(scan_order, textel, expand_format);

    Color::from_raw(textel)
}

// Texture filtering - point sampling or bilinear interpolation
#[inline(always)]
unsafe fn texture_filter(
    pp_ignore_tex_a: bool,
    pp_clamp_u: bool,
    pp_clamp_v: bool,
    pp_flip_u: bool,
    pp_flip_v: bool,
    pp_filter_mode: u32,
    vq_comp: bool,
    mip_mapped: bool,
    scan_order: bool,
    stride_sel: bool,
    pixel_fmt: u32,
    tsp: TSP,
    tcw: TCW,
    u: f32,
    v: f32,
    mip_level: u32
) -> Color {
    let half_offset = crate::pvr_regs::half_offset();
    let halfpixel = if half_offset.texure_pixel_half_offset() { 0 } else { 127 };

    let mut mip_level = mip_level;
    if mip_level >= (tsp.tex_u() + 3) {
        mip_level = tsp.tex_u() + 3;
    }

    let (size_u, size_v) = if tcw.mip_mapped() {
        let size = (8 << tsp.tex_u()) >> mip_level;
        (size, size)
    } else {
        (8 << tsp.tex_u(), 8 << tsp.tex_v())
    };

    let ui = (u * size_u as f32 * 256.0) as i32 + halfpixel;
    let vi = (v * size_v as f32 * 256.0) as i32 + halfpixel;

    // Fetch 4 texture samples for filtering
    let offset00 = texture_fetch(vq_comp, mip_mapped, scan_order, stride_sel, pixel_fmt, tsp, tcw,
        clamp_flip((ui >> 8) + 1, size_u, pp_clamp_u, pp_flip_u),
        clamp_flip((vi >> 8) + 1, size_v, pp_clamp_v, pp_flip_v),
        mip_level);
    let offset01 = texture_fetch(vq_comp, mip_mapped, scan_order, stride_sel, pixel_fmt, tsp, tcw,
        clamp_flip((ui >> 8) + 0, size_u, pp_clamp_u, pp_flip_u),
        clamp_flip((vi >> 8) + 1, size_v, pp_clamp_v, pp_flip_v),
        mip_level);
    let offset10 = texture_fetch(vq_comp, mip_mapped, scan_order, stride_sel, pixel_fmt, tsp, tcw,
        clamp_flip((ui >> 8) + 1, size_u, pp_clamp_u, pp_flip_u),
        clamp_flip((vi >> 8) + 0, size_v, pp_clamp_v, pp_flip_v),
        mip_level);
    let offset11 = texture_fetch(vq_comp, mip_mapped, scan_order, stride_sel, pixel_fmt, tsp, tcw,
        clamp_flip((ui >> 8) + 0, size_u, pp_clamp_u, pp_flip_u),
        clamp_flip((vi >> 8) + 0, size_v, pp_clamp_v, pp_flip_v),
        mip_level);

    let mut textel = if pp_filter_mode == 0 {
        // Point sampling - use nearest sample
        offset11
    } else if pp_filter_mode == 1 {
        // Bilinear filtering
        let ublend = to_u8_256((ui & 255) as u8);
        let vblend = to_u8_256((vi & 255) as u8);
        let nublend = 256 - ublend;
        let nvblend = 256 - vblend;

        Color {
            b: ((offset00.b as u32 * ublend * vblend +
                 offset01.b as u32 * nublend * vblend +
                 offset10.b as u32 * ublend * nvblend +
                 offset11.b as u32 * nublend * nvblend) / 65536) as u8,
            g: ((offset00.g as u32 * ublend * vblend +
                 offset01.g as u32 * nublend * vblend +
                 offset10.g as u32 * ublend * nvblend +
                 offset11.g as u32 * nublend * nvblend) / 65536) as u8,
            r: ((offset00.r as u32 * ublend * vblend +
                 offset01.r as u32 * nublend * vblend +
                 offset10.r as u32 * ublend * nvblend +
                 offset11.r as u32 * nublend * nvblend) / 65536) as u8,
            a: ((offset00.a as u32 * ublend * vblend +
                 offset01.a as u32 * nublend * vblend +
                 offset10.a as u32 * ublend * nvblend +
                 offset11.a as u32 * nublend * nvblend) / 65536) as u8,
        }
    } else {
        // Trilinear filtering (not implemented)
        Color { b: 0xAF, g: 0x67, r: 0x48, a: 0x39 }
    };

    if pp_ignore_tex_a {
        textel.a = 255;
    }

    textel
}

// ============================================================================
// Fog Unit
// ============================================================================

// Lookup fog alpha value from fog table
#[inline(always)]
unsafe fn lookup_fog_table(inv_w: f32) -> u8 {
    let fog_density = crate::pvr_regs::fog_density();
    let fog_density8 = &fog_density as *const u32 as *const u8;
    let fog_den_mant = (*fog_density8.add(1)) as f32 / 128.0;
    let fog_den_exp = (*fog_density8.add(0)) as i8;

    let fog_den = fog_den_mant * 2.0f32.powi(fog_den_exp as i32);
    let mut fog_w = fog_den * inv_w;

    fog_w = fog_w.max(1.0).min(255.999985);

    // Extract float fields for table lookup
    let fog_w_bits = fog_w.to_bits();
    let m = fog_w_bits & 0x7FFFFF;
    let e = (fog_w_bits >> 23) & 0xFF;

    let index = (((e + 1) & 7) << 4) | ((m >> 19) & 15);
    let blend_factor = ((m >> 11) & 255) as u8;
    let blend_inv = 255 ^ blend_factor;

    let fog_table = crate::pvr_regs::fog_table();
    let fog_entry = fog_table.add(index as usize) as *const u8;
    let fog_alpha = ((*fog_entry.add(0) as u32 * to_u8_256(blend_factor) +
                      *fog_entry.add(1) as u32 * to_u8_256(blend_inv)) >> 8) as u8;

    fog_alpha
}

// Fog unit - apply color clamping and fog
#[inline(always)]
unsafe fn fog_unit(
    pp_offset: bool,
    pp_color_clamp: bool,
    pp_fog_ctrl: u32,
    mut col: Color,
    inv_w: f32,
    offs_a: u8
) -> Color {
    // Color clamping
    if pp_color_clamp {
        let clamp_max = Color::from_raw(crate::pvr_regs::fog_clamp_max());
        let clamp_min = Color::from_raw(crate::pvr_regs::fog_clamp_min());

        col.b = col.b.min(clamp_max.b).max(clamp_min.b);
        col.g = col.g.min(clamp_max.g).max(clamp_min.g);
        col.r = col.r.min(clamp_max.r).max(clamp_min.r);
        col.a = col.a.min(clamp_max.a).max(clamp_min.a);
    }

    // Fog application
    match pp_fog_ctrl {
        0b00 | 0b11 => {
            // Lookup mode 1 or 2
            let fog_alpha = lookup_fog_table(inv_w);
            let fog_inv = 255 ^ fog_alpha;
            let col_ram = Color::from_raw(crate::pvr_regs::fog_col_ram());

            if pp_fog_ctrl == 0b00 {
                // Mode 1: blend fog color
                col.b = ((col.b as u32 * to_u8_256(fog_inv) + col_ram.b as u32 * to_u8_256(fog_alpha)) >> 8) as u8;
                col.g = ((col.g as u32 * to_u8_256(fog_inv) + col_ram.g as u32 * to_u8_256(fog_alpha)) >> 8) as u8;
                col.r = ((col.r as u32 * to_u8_256(fog_inv) + col_ram.r as u32 * to_u8_256(fog_alpha)) >> 8) as u8;
            } else {
                // Mode 2: replace with fog color
                col.b = col_ram.b;
                col.g = col_ram.g;
                col.r = col_ram.r;
                col.a = fog_alpha;
            }
        }
        0b01 => {
            // Per-vertex fog
            if pp_offset {
                let col_vert = Color::from_raw(crate::pvr_regs::fog_col_vert());
                let alpha = offs_a;
                let inv = 255 ^ alpha;

                col.b = ((col.b as u32 * to_u8_256(inv) + col_vert.b as u32 * to_u8_256(alpha)) >> 8) as u8;
                col.g = ((col.g as u32 * to_u8_256(inv) + col_vert.g as u32 * to_u8_256(alpha)) >> 8) as u8;
                col.r = ((col.r as u32 * to_u8_256(inv) + col_vert.r as u32 * to_u8_256(alpha)) >> 8) as u8;
            }
        }
        0b10 => {
            // No fog
        }
        _ => {}
    }

    col
}

// ============================================================================
// Bump Mapping
// ============================================================================

// Bump mapper - calculate intensity from bump map
#[inline(always)]
unsafe fn bump_mapper(textel: Color, offset: Color) -> Color {
    let k1 = offset.a;
    let k2 = offset.r;
    let k3 = offset.g;
    let q = offset.b;

    let r = textel.b;
    let s = textel.g;

    let bm_sin90 = &BM_SIN90;
    let bm_cos90 = &BM_COS90;
    let bm_cos360 = &BM_COS360;

    let mut i = (k1 as i32 * 127 * 127 +
                 k2 as i32 * bm_sin90[s as usize] as i32 * 127 +
                 k3 as i32 * bm_cos90[s as usize] as i32 * bm_cos360[((r as i32 - q as i32) & 255) as usize] as i32) / 127 / 127;

    if i < 0 {
        i = 0;
    } else if i > 255 {
        i = 255;
    }

    Color {
        b: 255,
        g: 255,
        r: 255,
        a: i as u8,
    }
}

// Pixel shader pipeline - implements full texture/shade pipeline for a pixel
unsafe fn pixel_flush_tsp(
    pp_alpha_test: bool,
    entry: &FpuEntry,
    x: f32,
    y: f32,
    index: usize,
    inv_w: f32,
    in_volume: bool,
    _core_tag: ISP_BACKGND_T_type
) -> bool {
    unsafe {
        let fpu_shad_scale = crate::pvr_regs::fpu_shad_scale();
        let pp_cheap_shadows = fpu_shad_scale.intensity_shadow();
        let two_volume_index = if in_volume && !pp_cheap_shadows { 1 } else { 0 };

        let pp_use_alpha = entry.params.tsp[two_volume_index].use_alpha();
        let pp_texture = entry.params.isp.texture();
        let pp_offset = entry.params.isp.offset();
        let w = 1.0 / inv_w;

        // Interpolate base color
        let base = interpolate_base(&entry.ips.col[two_volume_index], x, y, w, pp_use_alpha, pp_cheap_shadows, in_volume);

        // Texture fetching and filtering
        let mut textel = Color { b: 255, g: 255, r: 255, a: 255 };
        let mut mip_level = 0u32;

        if pp_texture {
            // Interpolate UV coordinates
            let u = entry.ips.u[two_volume_index].ip(x, y) * w;
            let v = entry.ips.v[two_volume_index].ip(x, y) * w;

            // Calculate mipmap level
            if entry.params.tcw[two_volume_index].mip_mapped() {
                let size_u = 8 << entry.params.tsp[two_volume_index].tex_u();
                // Faux mipmap calculation (doesn't follow hardware exactly)
                let ddx = (entry.ips.u[two_volume_index].ddx + entry.ips.v[two_volume_index].ddx);
                let ddy = (entry.ips.u[two_volume_index].ddy + entry.ips.v[two_volume_index].ddy);

                let d_mip = ddx.abs().min(ddy.abs()) * w * size_u as f32 * entry.params.tsp[two_volume_index].mip_map_d() as f32 / 4.0;

                mip_level = 0; // biggest
                let mut d = d_mip;
                while d > 1.5 && mip_level < 11 {
                    mip_level += 1;
                    d = d / 2.0;
                }
            }

            // Call texture filter which calls texture fetch
            textel = texture_filter(
                entry.params.tsp[two_volume_index].ignore_tex_a(),
                entry.params.tsp[two_volume_index].clamp_u(),
                entry.params.tsp[two_volume_index].clamp_v(),
                entry.params.tsp[two_volume_index].flip_u(),
                entry.params.tsp[two_volume_index].flip_v(),
                entry.params.tsp[two_volume_index].filter_mode(),
                entry.params.tcw[two_volume_index].vq_comp(),
                entry.params.tcw[two_volume_index].mip_mapped(),
                entry.params.tcw[two_volume_index].scan_order(),
                entry.params.tcw[two_volume_index].stride_sel(),
                entry.params.tcw[two_volume_index].pixel_fmt(),
                entry.params.tsp[two_volume_index],
                entry.params.tcw[two_volume_index],
                u,
                v,
                mip_level
            );

            // Update offset color if needed
            if pp_offset {
                OFFS = interpolate_offs(&entry.ips.ofs[two_volume_index], x, y, w, pp_cheap_shadows, in_volume);
            }
        }

        // Color combining (with bump mapping support)
        let pp_shad_instr = entry.params.tsp[two_volume_index].shad_instr();
        let mut col = if pp_texture && entry.params.tcw[two_volume_index].pixel_fmt() == PIXEL_BUMPMAP {
            bump_mapper(textel, OFFS)
        } else {
            color_combiner(base, textel, OFFS, pp_texture, pp_offset, pp_shad_instr)
        };

        // Fog unit
        let pp_color_clamp = entry.params.tsp[two_volume_index].color_clamp();
        let pp_fog_ctrl = entry.params.tsp[two_volume_index].fog_ctrl();
        col = fog_unit(pp_offset, pp_color_clamp, pp_fog_ctrl, col, inv_w, OFFS.a);

        // Blending unit
        let pp_src_sel = entry.params.tsp[two_volume_index].src_select() as u32;
        let pp_dst_sel = entry.params.tsp[two_volume_index].dst_select() as u32;
        let pp_src_inst = entry.params.tsp[two_volume_index].src_instr() as u32;
        let pp_dst_inst = entry.params.tsp[two_volume_index].dst_instr() as u32;

        blending_unit(index, col, pp_src_sel, pp_dst_sel, pp_src_inst, pp_dst_inst, pp_alpha_test)
    }
}

// Render from TAG buffer to ACCUM (color buffer)
// TAG holds references to triangles, ACCUM is the tile framebuffer
pub unsafe fn render_param_tags(render_mode: u8, tile_x: i32, tile_y: i32) {
    unsafe {
        let half_offset = crate::pvr_regs::half_offset();
        let halfpixel = if half_offset.tsp_pixel_half_offset() { 0.5 } else { 0.0 };

        let rect = TaRect {
            left: tile_x,
            top: tile_y,
            bottom: tile_y + 32,
            right: tile_x + 32,
        };

        for y in 0..32 {
            for x in 0..32 {
                let index = (y * 32 + x) as usize;
                let tag = TAG_BUFFER[TAG_BUFFER_A][index];
                let t = ISP_BACKGND_T_type(tag);
                let in_volume = (STENCIL_BUFFER[index] & 0b001) == 0b001 && t.shadow();
                let mut tag_valid = TAG_STATUS[index].valid;

                // RM_PUNCHTHROUGH_MV special case
                if render_mode == 3 { // PUNCHTHROUGH_MV
                    if !in_volume {
                        continue;
                    } else {
                        tag_valid = TAG_STATUS[index].rendered;
                    }
                }

                // PUNCHTHROUGH modes ignore volume
                if render_mode == 1 || render_mode == 2 {
                    // in_volume = false; // Not needed since we don't use it
                }

                if tag_valid {
                    let entry = get_fpu_entry(&rect, render_mode, t);
                    let inv_w = entry.ips.inv_w.ip(x as f32 + halfpixel, y as f32 + halfpixel);

                    let alpha_test_passed = pixel_flush_tsp(
                        render_mode == 1 || render_mode == 2, // is_punchthrough
                        entry,
                        x as f32 + halfpixel,
                        y as f32 + halfpixel,
                        index,
                        inv_w,
                        in_volume,
                        t
                    );

                    if render_mode == 1 || render_mode == 2 { // PUNCHTHROUGH
                        if !alpha_test_passed {
                            MORE_TO_DRAW = true;
                            // Feedback channel
                            DEPTH_BUFFER[DEPTH_BUFFER_A][index] = DEPTH_BUFFER[DEPTH_BUFFER_C][index];
                        } else {
                            TAG_STATUS[index].rendered = true;
                            TAG_STATUS[index].valid = false;
                        }
                    }

                    if render_mode == 5 { // TRANSLUCENT_PRESORT
                        TAG_STATUS[index].valid = false;
                    }
                }
            }
        }
    }
}
