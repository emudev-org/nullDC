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
        area: &TaRect,
        params: &DrawParametersEx,
        v1: Vertex,
        v2: Vertex,
        v3: Vertex,
        two_volumes: bool
    ) {
        // TODO: Implement full setup
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

        // Setup interpolation parameters (placeholder for now)
        // TODO: Implement full interpolation setup
        FPU_CACHE[cache_index].entry.ips.inv_w = PlaneStepper3::setup(
            rect,
            &vtx[0],
            &vtx[1],
            &vtx[2],
            vtx[0].z,
            vtx[1].z,
            vtx[2].z
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

                    // Placeholder for PixelFlush_tsp - will be implemented next
                    // let alpha_test_passed = pixel_flush_tsp(
                    //     render_mode == 1 || render_mode == 2, // is_punchthrough
                    //     entry,
                    //     x as f32 + halfpixel,
                    //     y as f32 + halfpixel,
                    //     index,
                    //     inv_w,
                    //     in_volume,
                    //     t
                    // );

                    let alpha_test_passed = true; // Placeholder

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
