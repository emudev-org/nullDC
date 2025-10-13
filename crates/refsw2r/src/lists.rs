/*
    This file is part of libswirl
*/
// #include "license/bsd"

// List rendering from refsw_lists.cc

use crate::types::{Vertex, ISP_TSP, TSP, TCW};
use crate::pvr_mem::{vri, vrf, EMU_VRAM};
use crate::pvr_regs::{self, ISP_BACKGND_T_type};
use crate::lists_types::*;
use crate::rendlog;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TaRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Opaque = 0,
    PunchthroughPass0 = 1,
    PunchthroughPassN = 2,
    PunchthroughMv = 3,
    TranslucentAutosort = 4,
    TranslucentPresort = 5,
    Modifier = 6,
}

#[derive(Copy, Clone)]
pub struct DrawParametersEx {
    pub isp: ISP_TSP,
    pub tsp: [TSP; 2],
    pub tcw: [TCW; 2],
}

impl Default for DrawParametersEx {
    fn default() -> Self {
        Self {
            isp: ISP_TSP(0),
            tsp: [TSP(0), TSP(0)],
            tcw: [TCW(0), TCW(0)],
        }
    }
}

// Vertex color unpacking macro
macro_rules! vert_packed_color {
    ($to:expr, $src:expr) => {
        {
            let mut t = $src;
            $to[0] = (t & 0xFF) as u8; t >>= 8;
            $to[1] = (t & 0xFF) as u8; t >>= 8;
            $to[2] = (t & 0xFF) as u8; t >>= 8;
            $to[3] = (t & 0xFF) as u8;
        }
    };
}

pub unsafe fn core_tag_from_desc(
    cache_bypass: u32,
    shadow: u32,
    skip: u32,
    param_offs_in_words: u32,
    tag_offset: u32
) -> ISP_BACKGND_T_type {
    let mut rv = ISP_BACKGND_T_type(0);
    rv.set_tag_offset(tag_offset);
    rv.set_param_offs_in_words(param_offs_in_words);
    rv.set_skip(skip);
    rv.set_shadow(shadow != 0);
    rv.set_cache_bypass(cache_bypass != 0);
    rv
}

#[inline]
pub fn f16(v: u16) -> f32 {
    let z = (v as u32) << 16;
    f32::from_bits(z)
}

// Decode a vertex in the native PVR format
pub unsafe fn decode_pvr_vertex(
    params: &DrawParametersEx,
    mut ptr: u32,
    cv: &mut Vertex,
    two_volumes: bool
) {
    let vram = unsafe { EMU_VRAM };

    // XYZ are always there
    cv.x = vrf(vram, ptr); ptr += 4;
    cv.y = vrf(vram, ptr); ptr += 4;
    cv.z = vrf(vram, ptr); ptr += 4;

    if params.isp.texture() {
        if params.isp.uv_16b() {
            let uv = vri(vram, ptr);
            cv.u = f16((uv >> 16) as u16);
            cv.v = f16((uv & 0xFFFF) as u16);
            ptr += 4;
        } else {
            cv.u = vrf(vram, ptr); ptr += 4;
            cv.v = vrf(vram, ptr); ptr += 4;
        }
    }

    // Color
    let col = vri(vram, ptr); ptr += 4;
    vert_packed_color!(cv.col, col);

    if params.isp.offset() {
        let spc = vri(vram, ptr); ptr += 4;
        vert_packed_color!(cv.spc, spc);
    }

    if two_volumes {
        if params.isp.texture() {
            if params.isp.uv_16b() {
                let uv = vri(vram, ptr);
                cv.u1 = f16((uv >> 16) as u16);
                cv.v1 = f16((uv & 0xFFFF) as u16);
                ptr += 4;
            } else {
                cv.u1 = vrf(vram, ptr); ptr += 4;
                cv.v1 = vrf(vram, ptr); ptr += 4;
            }
        }

        let col1 = vri(vram, ptr); ptr += 4;
        vert_packed_color!(cv.col1, col1);

        if params.isp.offset() {
            let spc1 = vri(vram, ptr); ptr += 4;
            vert_packed_color!(cv.spc1, spc1);
        }
    }
}

// Decode an object (params + vertices)
pub unsafe fn decode_pvr_vertices(
    params: &mut DrawParametersEx,
    mut base: u32,
    skip: u32,
    two_volumes: bool,
    vtx: &mut [Vertex],
    offset: usize
) -> u32 {
    let vram = unsafe { EMU_VRAM };

    params.isp.set_full(vri(vram, base));
    params.tsp[0].set_full(vri(vram, base + 4));
    params.tcw[0].set_full(vri(vram, base + 8));

    base += 12;
    if two_volumes {
        params.tsp[1].set_full(vri(vram, base));
        params.tcw[1].set_full(vri(vram, base + 4));
        base += 8;
    }

    // Skip offset vertices
    for _ in 0..offset {
        base += (3 + skip * (if two_volumes { 2 } else { 1 })) * 4;
    }

    // Decode requested vertices
    for i in 0..vtx.len() {
        unsafe { decode_pvr_vertex(params, base, &mut vtx[i], two_volumes); }
        base += (3 + skip * (if two_volumes { 2 } else { 1 })) * 4;
    }

    base
}

pub unsafe fn read_region_array_entry(base: u32, entry: &mut RegionArrayEntry) -> u32 {
    let vram = unsafe { EMU_VRAM };
    let fmt_v1 = !pvr_regs::fpu_param_cfg().region_header_type();

    entry.control.set_full(vri(vram, base));
    entry.opaque.set_full(vri(vram, base + 4));
    entry.opaque_mod.set_full(vri(vram, base + 8));
    entry.trans.set_full(vri(vram, base + 12));
    entry.trans_mod.set_full(vri(vram, base + 16));

    let rv;
    if fmt_v1 {
        entry.control.set_pre_sort(pvr_regs::isp_feed_cfg().pre_sort());
        entry.puncht.set_full(0x80000000);
        rv = 5 * 4;
    } else {
        entry.puncht.set_full(vri(vram, base + 20));
        rv = 6 * 4;
    }

    rv
}

// Render a triangle using the tile rasterizer
pub unsafe fn render_triangle(
    render_mode: RenderMode,
    params: &DrawParametersEx,
    tag: u32,
    v1: &Vertex,
    v2: &Vertex,
    v3: &Vertex,
    v4: Option<&Vertex>,
    area: &TaRect
) {
    unsafe {
        crate::tile::rasterize_triangle(render_mode as u8, params, tag, v1, v2, v3, v4, area);
    }
}

pub unsafe fn render_triangle_strip(
    render_mode: RenderMode,
    obj: ObjectListEntry,
    rect: &TaRect
) {
    let mut vtx = [Vertex::default(); 8];
    let mut params = DrawParametersEx::default();

    let param_base = pvr_regs::param_base() & 0xF00000;
    let obj_tstrip = obj.as_tstrip();

    let tag_address = param_base + obj_tstrip.param_offs_in_words() * 4;

    let shad_scale = pvr_regs::fpu_shad_scale();
    let two_volumes = obj_tstrip.shadow() && !shad_scale.intensity_shadow();

    decode_pvr_vertices(&mut params, tag_address, obj_tstrip.skip(), two_volumes, &mut vtx, 0);

    for i in 0..6 {
        if (obj_tstrip.mask() & (1 << (5 - i))) != 0 {
            let tag = core_tag_from_desc(
                params.isp.cache_bypass() as u32,
                obj_tstrip.shadow() as u32,
                obj_tstrip.skip(),
                obj_tstrip.param_offs_in_words(),
                i
            );

            let not_even = (i & 1) as usize;
            let even = not_even ^ 1;

            rendlog!("STRIP: {:08X} {} {} {} {} {} {} {} {} {} {}",
                tag.full(), vtx[i as usize +not_even].x, vtx[i as usize +not_even].y, vtx[i as usize +not_even].z,
                vtx[i as usize +even].x, vtx[i as usize +even].y, vtx[i as usize +even].z,
                vtx[i as usize +2].x, vtx[i as usize +2].y, vtx[i as usize +2].z, i);

            render_triangle(
                render_mode,
                &params,
                tag.full(),
                &vtx[i as usize + not_even],
                &vtx[i as usize + even],
                &vtx[i as usize + 2],
                None,
                rect
            );
        }
    }
}

pub unsafe fn render_triangle_array(
    render_mode: RenderMode,
    obj: ObjectListEntry,
    rect: &TaRect
) {
    let obj_tarray = obj.as_tarray();
    let triangles = obj_tarray.prims() + 1;
    let param_base = pvr_regs::param_base() & 0xF00000;

    let mut param_ptr = param_base + obj_tarray.param_offs_in_words() * 4;
    let shad_scale = pvr_regs::fpu_shad_scale();
    let two_volumes = obj_tarray.shadow() && !shad_scale.intensity_shadow();

    for i in 0..triangles {
        let mut params = DrawParametersEx::default();
        let mut vtx = [Vertex::default(); 3];

        let tag_address = param_ptr;
        param_ptr = decode_pvr_vertices(&mut params, tag_address, obj_tarray.skip(), two_volumes, &mut vtx, 0);

        let tag = core_tag_from_desc(
            params.isp.cache_bypass() as u32,
            obj_tarray.shadow() as u32,
            obj_tarray.skip(),
            (tag_address - param_base) / 4,
            0
        );

        rendlog!("TARR: {:08X} {} {} {} {} {} {} {} {} {} {}",
            tag.full(), vtx[0].x, vtx[0].y, vtx[0].z,
            vtx[1].x, vtx[1].y, vtx[1].z,
            vtx[2].x, vtx[2].y, vtx[2].z, i);

        render_triangle(render_mode, &params, tag.full(), &vtx[0], &vtx[1], &vtx[2], None, rect);
    }
}

pub unsafe fn render_quad_array(
    render_mode: RenderMode,
    obj: ObjectListEntry,
    rect: &TaRect
) {
    let obj_qarray = obj.as_qarray();
    let quads = obj_qarray.prims() + 1;
    let param_base = pvr_regs::param_base() & 0xF00000;

    let mut param_ptr = param_base + obj_qarray.param_offs_in_words() * 4;
    let shad_scale = pvr_regs::fpu_shad_scale();
    let two_volumes = obj_qarray.shadow() && !shad_scale.intensity_shadow();

    for i in 0..quads {
        let mut params = DrawParametersEx::default();
        let mut vtx = [Vertex::default(); 4];

        let tag_address = param_ptr;
        param_ptr = decode_pvr_vertices(&mut params, tag_address, obj_qarray.skip(), two_volumes, &mut vtx, 0);

        let tag = core_tag_from_desc(
            params.isp.cache_bypass() as u32,
            obj_qarray.shadow() as u32,
            obj_qarray.skip(),
            (tag_address - param_base) / 4,
            0
        );

        rendlog!("QARR: {:08X} {} {} {} {} {} {} {} {} {} {} {} {} {}",
            tag.full(), vtx[0].x, vtx[0].y, vtx[0].z,
            vtx[1].x, vtx[1].y, vtx[1].z,
            vtx[2].x, vtx[2].y, vtx[2].z,
            vtx[3].x, vtx[3].y, vtx[3].z, i);

        render_triangle(render_mode, &params, tag.full(), &vtx[0], &vtx[1], &vtx[2], Some(&vtx[3]), rect);
    }
}

pub unsafe fn render_object_list(
    render_mode: RenderMode,
    mut base: u32,
    rect: &TaRect
) {
    let vram = unsafe { EMU_VRAM };

    loop {
        let obj = ObjectListEntry::new(vri(vram, base));
        rendlog!("OBJECT: {:08X} {:08X}", base, obj.full());
        base += 4;

        if !obj.is_not_triangle_strip() {
            render_triangle_strip(render_mode, obj, rect);
        } else {
            match obj.obj_type() {
                0b111 => { // link
                    let link = obj.as_link();
                    if link.end_of_list() {
                        return;
                    }
                    base = link.next_block_ptr_in_words() * 4;
                }
                0b100 => { // triangle array
                    render_triangle_array(render_mode, obj, rect);
                }
                0b101 => { // quad array
                    render_quad_array(render_mode, obj, rect);
                }
                _ => {
                    // println!("RenderObjectList: Not handled object type: {}", obj.obj_type());
                }
            }
        }
    }
}

// Main render entry point - called on START_RENDER write
pub unsafe fn render_core() {
    let mut base = pvr_regs::region_base();
    let mut entry = RegionArrayEntry::default();

    rendlog!("REFSW2LOG: 0");
    rendlog!("BGTAG: {:08X}", pvr_regs::isp_backgnd_t().full());

    // Parse region array
    loop {
        let step = read_region_array_entry(base, &mut entry);

        rendlog!("TILE: {:08X} {:08X} {:08X} {:08X} {:08X} {:08X} {:08X}",
            base, entry.control.full(), entry.opaque.full(), entry.opaque_mod.full(),
            entry.trans.full(), entry.trans_mod.full(), entry.puncht.full());

        base += step;

        let rect = TaRect {
            top: (entry.control.tiley() * 32) as i32,
            left: (entry.control.tilex() * 32) as i32,
            bottom: (entry.control.tiley() * 32 + 32) as i32,
            right: (entry.control.tilex() * 32 + 32) as i32,
        };

        // Render opaque
        if !entry.opaque.empty() {
            rendlog!("OPAQ");
            render_object_list(RenderMode::Opaque, entry.opaque.ptr_in_words() * 4, &rect);

            if !entry.opaque_mod.empty() {
                rendlog!("OPAQ_MOD");
                render_object_list(RenderMode::Modifier, entry.opaque_mod.ptr_in_words() * 4, &rect);
            }
        }

        // Render punch-through
        if !entry.puncht.empty() {
            rendlog!("PT");
            render_object_list(RenderMode::PunchthroughPass0, entry.puncht.ptr_in_words() * 4, &rect);
        }

        // Render translucent
        if !entry.trans.empty() {
            if entry.control.pre_sort() {
                rendlog!("TR_PS");
                render_object_list(RenderMode::TranslucentPresort, entry.trans.ptr_in_words() * 4, &rect);
            } else {
                rendlog!("TR_AS");
                render_object_list(RenderMode::TranslucentAutosort, entry.trans.ptr_in_words() * 4, &rect);
            }
        }

        if entry.control.last_region() {
            break;
        }
    }
}
