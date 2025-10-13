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

        let bg_tag = pvr_regs::isp_backgnd_t().full();

        crate::tile::clear_fpu_cache();

        // Tile needs clear?
        if !entry.control.z_keep() {
            rendlog!("ZCLEAR");
            // Clear Param + Z + stencil buffers
            let bg_depth = unsafe { pvr_regs::isp_backgnd_d().f };
            crate::tile::clear_buffers(bg_tag, bg_depth, 0);
        } else {
            rendlog!("ZKEEP");
            crate::tile::clear_param_status_buffer();
        }

        // Render OPAQ to TAGS
        if !entry.opaque.empty() {
            rendlog!("OPAQ");
            render_object_list(RenderMode::Opaque, entry.opaque.ptr_in_words() * 4, &rect);

            if !entry.opaque_mod.empty() {
                rendlog!("OPAQ_MOD");
                render_object_list(RenderMode::Modifier, entry.opaque_mod.ptr_in_words() * 4, &rect);
            }
        }

        rendlog!("OP_PARAMS");
        // Render TAGS to ACCUM
        crate::tile::render_param_tags(RenderMode::Opaque as u8, rect.left, rect.top);

        // Render PT to TAGS
        if !entry.puncht.empty() {
            rendlog!("PT");

            crate::tile::peel_buffers_pt_initial(f32::MAX);
            crate::tile::clear_more_to_draw();

            // Render to TAGS
            render_object_list(RenderMode::PunchthroughPass0, entry.puncht.ptr_in_words() * 4, &rect);

            // Keep reference Z buffer
            crate::tile::peel_buffers_pt();

            rendlog!("PT_PARAMS");
            // Render TAGS to ACCUM, making Z holes as-needed
            crate::tile::render_param_tags(RenderMode::PunchthroughPass0 as u8, rect.left, rect.top);

            while crate::tile::get_more_to_draw() {
                rendlog!("PT_N");
                crate::tile::clear_more_to_draw();

                // Render to TAGS
                render_object_list(RenderMode::PunchthroughPassN, entry.puncht.ptr_in_words() * 4, &rect);

                if !crate::tile::get_more_to_draw() {
                    break;
                }

                crate::tile::clear_more_to_draw();
                // Keep reference Z buffer
                crate::tile::peel_buffers_pt();

                rendlog!("PT_N_PARAMS");
                // Render TAGS to ACCUM, making Z holes as-needed
                crate::tile::render_param_tags(RenderMode::PunchthroughPass0 as u8, rect.left, rect.top);
            }

            if !entry.opaque_mod.empty() {
                rendlog!("PT_MOD");
                render_object_list(RenderMode::Modifier, entry.opaque_mod.ptr_in_words() * 4, &rect);
                rendlog!("PT_MOD_PARAMS");
                crate::tile::render_param_tags(RenderMode::PunchthroughMv as u8, rect.left, rect.top);
            }
        }

        // Layer peeling rendering
        if !entry.trans.empty() {
            if entry.control.pre_sort() {
                rendlog!("TR_PS");
                // Clear the param buffer
                crate::tile::clear_param_status_buffer();

                // Render to TAGS
                render_object_list(RenderMode::TranslucentPresort, entry.trans.ptr_in_words() * 4, &rect);
            } else {
                rendlog!("TR_AS");
                crate::tile::set_tag_to_max();
                loop {
                    rendlog!("TR_AS_N");
                    // Prepare for a new pass
                    crate::tile::clear_more_to_draw();

                    // Copy depth test to depth reference buffer, clear depth test buffer, clear stencil
                    crate::tile::peel_buffers(f32::MAX, 0);

                    // Render to TAGS
                    render_object_list(RenderMode::TranslucentAutosort, entry.trans.ptr_in_words() * 4, &rect);

                    if !entry.trans_mod.empty() {
                        render_object_list(RenderMode::Modifier, entry.trans_mod.ptr_in_words() * 4, &rect);
                    }

                    rendlog!("TR_PARAMS");
                    // Render TAGS to ACCUM
                    crate::tile::render_param_tags(RenderMode::TranslucentAutosort as u8, rect.left, rect.top);

                    if !crate::tile::get_more_to_draw() {
                        break;
                    }
                }
            }
        }

        // Copy to VRAM
        if !entry.control.no_writeout() {
            unsafe {
                write_tile_to_vram(&entry, &rect);
            }
        }

        if entry.control.last_region() {
            break;
        }
    }
}

// Copy tile color buffer to VRAM
unsafe fn write_tile_to_vram(entry: &RegionArrayEntry, rect: &TaRect) {
    // Precomputed "threshold biases" = bias4[bayer4[i][j]]
    const BAYER_BIAS: [[u8; 4]; 4] = [
        [  8, 136,  40, 168 ],  // 0→8, 8→136, 2→40, 10→168
        [200,  72, 232, 104 ],  //12→200,4→72, 14→232,6→104
        [ 56, 184,  24, 152 ],  // 3→56,11→184,1→24, 9→152
        [248, 120, 216,  88 ]   //15→248,7→120,13→216,5→88
    ];

    let copy = crate::tile::get_color_output_buffer();

    let scaler_ctl = pvr_regs::scaler_ctl();
    let field = scaler_ctl.fieldselect();
    let interlace = scaler_ctl.interlace();

    let fb_w_sof1 = pvr_regs::fb_w_sof1();
    let fb_w_sof2 = pvr_regs::fb_w_sof2();
    let base = if interlace && field { fb_w_sof2 } else { fb_w_sof1 };

    let fb_w_ctrl = pvr_regs::fb_w_ctrl();
    let fb_packmode = fb_w_ctrl.fb_packmode();

    let bpp = if fb_packmode == 0x1 { 2 } else { 4 };
    let fb_w_linestride = pvr_regs::fb_w_linestride();
    let offset_bytes = entry.control.tilex() * 32 * bpp + entry.control.tiley() * 32 * fb_w_linestride.stride() * 8;

    let vram = crate::pvr_mem::EMU_VRAM;

    for y in 0..32 {
        let dst = base + offset_bytes + y * fb_w_linestride.stride() * 8;

        for x in 0..32 {
            let src_idx = ((y * 32 + x) * 4) as usize;

            if fb_packmode == 0x1 {
                // RGB565
                let r8 = (*copy.offset(src_idx as isize + 0)) as i32;
                let g8 = (*copy.offset(src_idx as isize + 1)) as i32;
                let b8 = (*copy.offset(src_idx as isize + 2)) as i32;

                let t = BAYER_BIAS[(y & 3) as usize][(x & 3) as usize] as i32;

                // Integer quantize
                let mut r5 = (r8 * 31 + t) / 255;
                let mut g6 = (g8 * 63 + t) / 255;
                let mut b5 = (b8 * 31 + t) / 255;

                // Clamp
                if r5 < 0 { r5 = 0; } else if r5 > 31 { r5 = 31; }
                if g6 < 0 { g6 = 0; } else if g6 > 63 { g6 = 63; }
                if b5 < 0 { b5 = 0; } else if b5 > 31 { b5 = 31; }

                let pixel = (r5 << 0) | (g6 << 5) | (b5 << 11);
                crate::pvr_mem::pvr_write_area1_16(vram, dst + x * bpp, pixel as u16);
            } else {
                // RGB888/ARGB8888
                let pixel =
                    (*copy.offset(src_idx as isize + 0)) as u32 |
                    ((*copy.offset(src_idx as isize + 1)) as u32) << 8 |
                    ((*copy.offset(src_idx as isize + 2)) as u32) << 16 |
                    ((*copy.offset(src_idx as isize + 3)) as u32) << 24;
                crate::pvr_mem::pvr_write_area1_32(vram, dst + x * bpp, pixel);
            }
        }
    }
}
