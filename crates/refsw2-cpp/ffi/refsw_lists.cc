/*
	This file is part of libswirl
*/
//#include "license/bsd"

/*
    REFSW: Reference-style software rasterizer

    An attempt to model CLX2's CORE/SPG/RAMDAC at the lowest functional level

    Rasterizer structure
    ===

    Reads tile lists in CORE format, generated from a LLE TA implementation or software running from sh4,
    renders them in 32x32 tiles, reads out to VRAM and displays framebuffer from VRAM.

    CORE high level overview
    ===

    CORE Renders based on the REGION ARRAY, which is a flag-terminated list of tiles. Each RegionArrayEntry
    contains the TILE x/y position, control flags for Z clear/Write out/Presort and pointers to OBJECT LISTS.

    OBJECT LISTS are inline linked lists containing ObjectListEntries. Each ObjectListEntry has a small
    descriptor for the entry type and vertex size, and a pointer to the OBJECT DATA.

    OBJECT DATA contains the PARAMETERS for the OBJECT (ISP, TSP, TCW, optional TSP2 and TCW2) and vertixes.

    There are 3 OBJECT DATA TYPES
    - Triangle Strips (PARAMETERS, up to 8 VTXs) x 1
    - Triangle Arrays (PARAMETERS, 3 vtx) x Num_of_primitives
    - Quad Arrays (PARAMETERS, 4 vtx) x Num_of_primitives

    CORE renders the OBJECTS to its internal TILE BUFFERS, scales and filters the output (SCL)
    and writes out to VRAM.

    CORE Rendering details
    ===

    CORE has four main components, FPU (triangle setup) ISP (Rasterization, depth, stencil), TSP (Texutre + Shading)
    and SCL (tile writeout + scaling). There are three color rendering modes: DEPTH FIRST, DEPTH + COLOR and LAYER PEELING.

    OPAQUE OBJECTS are rendered using the DEPTH FIRST mode.
    PUNCH THROUGH OBJECTS are rendered using the DEPTH + COLOR mode.
    TRANSPARENT OBJECTS are rendered using either the DEPTH + COLOR mode or the LAYER PEELING mode.
    
    DEPTH FIRST mode
    ---
    OBJECTS are first rendered by ISP in the depth and tag buffers, 32 pixels (?) at a time. then the SPAN SORTER collects spans with the
    same tag and sends them to TSP for shading processing, one pixel at a time.

    DEPTH + COLOR mode
    ---
    OBJECTS are rendered by ISP and TSP at the same time, one pixel (?) at a time. ALPHA TEST feedback from TSP modifies the Z-write behavior.

    LAYER PEELING mode
    ---

    OBJECTS are first rendered by ISP in the depth and tag buffers, using a depth pass and a depth test buffer. SPAN SORTER collects spans with
    the same tag and sends them to TSP for shading processing. The process repeats itself until all layers have been indepedently rendered. On
    each pass, only the pixels with the lowest depth value that pass the depth pass buffer are rendered. In case of identical depth values, the
    tag buffer is used to sort the pixels by tag as well as depth in order to support co-planar polygons.
*/


#include "pvr_mem.h"
#include "TexUtils.h"

#include <cmath>
#include <float.h>

#include <memory>
#include <cstdio>
#include <cstring>
#include <algorithm>
#include <cassert>

#include "pvr_regs.h"

// #include <png.h>

#include "refsw_lists.h"

#include "refsw_tile.h"


extern uint8_t* emu_vram;
FILE* rendlog;

/*
    Main renderer class
*/
void RenderTriangle(RenderMode render_mode, DrawParameters* params, parameter_tag_t tag, const Vertex& v1, const Vertex& v2, const Vertex& v3, const Vertex* v4, taRECT* area)
{   
    RasterizeTriangle_table[render_mode](params, tag, v1, v2, v3, v4, area);

    if (render_mode == RM_TRANSLUCENT_PRESORT) {
        RenderParamTags<RM_TRANSLUCENT_PRESORT>(area->left, area->top);
    }

    if (render_mode == RM_MODIFIER)
    {
        // 0 normal polygon, 1 inside last, 2 outside last
        if (params->isp.modvol.VolumeMode == 1 ) 
        {
            RENDLOG("STENCIL_SUM_OR");
            SummarizeStencilOr();
        }
        else if (params->isp.modvol.VolumeMode == 2) 
        {
            RENDLOG("STENCIL_SUM_AND");
            SummarizeStencilAnd();
        }
    }
}

uint32_t ReadRegionArrayEntry(uint32_t base, RegionArrayEntry* entry) 
{
    bool fmt_v1 = FPU_PARAM_CFG.region_header_type == 0;

    entry->control.full     = vri(emu_vram, base);
    entry->opaque.full      = vri(emu_vram, base + 4);
    entry->opaque_mod.full  = vri(emu_vram, base + 8);
    entry->trans.full       = vri(emu_vram, base + 12);
    entry->trans_mod.full   = vri(emu_vram, base + 16);


    uint32_t rv;
    if (fmt_v1)
    {
        entry->control.pre_sort = ISP_FEED_CFG.pre_sort;
        entry->puncht.full = 0x80000000;
        rv = 5 * 4;
    }
    else
    {
        entry->puncht.full = vri(emu_vram, base + 20);
        rv = 6 * 4;
    }

    return rv;
}

#define vert_packed_color_(to,src) \
	{ \
	uint32_t t=src; \
	to[0] = (uint8_t)(t);t>>=8;\
	to[1] = (uint8_t)(t);t>>=8;\
	to[2] = (uint8_t)(t);t>>=8;\
	to[3] = (uint8_t)(t);      \
	}

ISP_BACKGND_T_type CoreTagFromDesc(uint32_t cache_bypass, uint32_t shadow, uint32_t skip, uint32_t param_offs_in_words, uint32_t tag_offset) {
    ISP_BACKGND_T_type rv;
    rv.full = 0;
    rv.tag_offset = tag_offset;
    rv.param_offs_in_words = param_offs_in_words;
    rv.skip = skip;
    rv.shadow = shadow;
    rv.cache_bypass = cache_bypass;

    return rv;
}

// render a triangle strip object list entry
void RenderTriangleStrip(RenderMode render_mode, ObjectListEntry obj, taRECT* rect)
{
    Vertex vtx[8];
    DrawParameters params;

    uint32_t param_base = PARAM_BASE & 0xF00000;

    uint32_t tag_address = param_base + obj.tstrip.param_offs_in_words * 4;

    bool two_volumes = obj.tstrip.shadow & ~FPU_SHAD_SCALE.intensity_shadow;
    decode_pvr_vertices(&params, tag_address, obj.tstrip.skip, two_volumes, vtx, 8, 0);

    for (int i = 0; i < 6; i++)
    {
        if (obj.tstrip.mask & (1 << (5-i)))
        {
            parameter_tag_t tag = CoreTagFromDesc(params.isp.CacheBypass, obj.tstrip.shadow, obj.tstrip.skip, obj.tstrip.param_offs_in_words, i).full;
            
            int not_even = i&1;
            int even = not_even ^ 1;
            RENDLOG("STRIP: %08X %f %f %f %f %f %f %f %f %f %d", tag,
                vtx[i+not_even].x, vtx[i+not_even].y, vtx[i+not_even].z,
                vtx[i+even].x, vtx[i+even].y, vtx[i+even].z,
                vtx[i+2].x, vtx[i+2].y, vtx[i+2].z,
                i
            );
            RenderTriangle(render_mode, &params, tag, vtx[i+not_even], vtx[i+even], vtx[i+2], nullptr, rect);
        }
    }
}


// render a triangle array object list entry
void RenderTriangleArray(RenderMode render_mode, ObjectListEntry obj, taRECT* rect)
{
    auto triangles = obj.tarray.prims + 1;
    uint32_t param_base = PARAM_BASE & 0xF00000;


    uint32_t param_ptr = param_base + obj.tarray.param_offs_in_words * 4;
    bool two_volumes = obj.tstrip.shadow & ~FPU_SHAD_SCALE.intensity_shadow;

    for (int i = 0; i<triangles; i++)
    {
        DrawParameters params;
        Vertex vtx[3];

        uint32_t tag_address = param_ptr;
        param_ptr = decode_pvr_vertices(&params, tag_address, obj.tarray.skip, two_volumes, vtx, 3, 0);
            
        parameter_tag_t tag  = CoreTagFromDesc(params.isp.CacheBypass, obj.tstrip.shadow, obj.tstrip.skip, (tag_address - param_base)/4, 0).full;

        RENDLOG("TARR: %08X %f %f %f %f %f %f %f %f %f %d", tag,
            vtx[0].x, vtx[0].y, vtx[0].z,
            vtx[1].x, vtx[1].y, vtx[1].z,
            vtx[2].x, vtx[2].y, vtx[2].z,
            i
        );

        RenderTriangle(render_mode, &params, tag, vtx[0], vtx[1], vtx[2], nullptr, rect);
    }
}

// render a quad array object list entry
void RenderQuadArray(RenderMode render_mode, ObjectListEntry obj, taRECT* rect)
{
    auto quads = obj.qarray.prims + 1;
    uint32_t param_base = PARAM_BASE & 0xF00000;


    uint32_t param_ptr = param_base + obj.qarray.param_offs_in_words * 4;
    bool two_volumes = obj.tstrip.shadow & ~FPU_SHAD_SCALE.intensity_shadow;

    for (int i = 0; i<quads; i++)
    {
        DrawParameters params;
        Vertex vtx[4];

        uint32_t tag_address = param_ptr;
        param_ptr = decode_pvr_vertices(&params, tag_address, obj.qarray.skip, two_volumes, vtx, 4, 0);
            
        parameter_tag_t tag = CoreTagFromDesc(params.isp.CacheBypass, obj.qarray.shadow, obj.qarray.skip, (tag_address - param_base)/4, 0).full;

        RENDLOG("QARR: %08X %f %f %f %f %f %f %f %f %f %f %f %f %d", tag,
            vtx[0].x, vtx[0].y, vtx[0].z,
            vtx[1].x, vtx[1].y, vtx[1].z,
            vtx[2].x, vtx[2].y, vtx[2].z,
            vtx[3].x, vtx[3].y, vtx[3].z,
            i
        );

        RenderTriangle(render_mode, &params, tag, vtx[0], vtx[1], vtx[2], &vtx[3], rect);
    }
}

// Render an object list
void RenderObjectList(RenderMode render_mode, pvr32addr_t base, taRECT* rect)
{
    ObjectListEntry obj;

    for (;;) {
        obj.full = vri(emu_vram, base);
        RENDLOG("OBJECT: %08X %08X", base, obj.full);
        base += 4;

        if (!obj.is_not_triangle_strip) {
            RenderTriangleStrip(render_mode, obj, rect);
        } else {
            switch(obj.type) {
                case 0b111: // link
                    if (obj.link.end_of_list)
                        return;

                    base = obj.link.next_block_ptr_in_words * 4;
                    break;

                case 0b100: // triangle array
                    RenderTriangleArray(render_mode, obj, rect);
                    break;
                    
                case 0b101: // quad array
                    RenderQuadArray(render_mode, obj, rect);
                    break;

                default:
                    printf("RenderObjectList: Not handled object type: %d\n", obj.type);
            }
        }
    }
}

// Render a frame
// Called on START_RENDER write
void RenderCORE() {
    {
        auto field = SCALER_CTL.fieldselect;
        auto interlace = SCALER_CTL.interlace;

        auto base = (interlace && field) ? FB_W_SOF2 : FB_W_SOF1;
        // printf("Rendering to %x\n", (interlace && field) ? FB_W_SOF2 : FB_W_SOF1);
    }
    uint32_t base = REGION_BASE;

    RegionArrayEntry entry;
    
    RENDLOG("REFSW2LOG: 0");
    RENDLOG("BGTAG: %08X", ISP_BACKGND_T.full);

    // Parse region array
    do {
        auto step = ReadRegionArrayEntry(base, &entry);
        
        RENDLOG("TILE: %08X %08X %08X %08X %08X %08X %08X", base, entry.control.full, entry.opaque.full, entry.opaque_mod.full, entry.trans.full, entry.trans_mod.full, entry.puncht.full);

        base += step;

        taRECT rect;
        rect.top = entry.control.tiley * 32;
        rect.left = entry.control.tilex * 32;

        rect.bottom = rect.top + 32;
        rect.right = rect.left + 32;

        parameter_tag_t bgTag;

        ClearFpuCache();
        // register BGPOLY to fpu
        {
            bgTag = ISP_BACKGND_T.full;
        }

        // Tile needs clear?
        if (!entry.control.z_keep)
        {
            RENDLOG("ZCLEAR");
            // Clear Param + Z + stencil buffers
            ClearBuffers(bgTag, ISP_BACKGND_D.f, 0);
        } else {
            RENDLOG("ZKEEP");
            ClearParamStatusBuffer();
        }

        // Render OPAQ to TAGS
        if (!entry.opaque.empty)
        {
            RENDLOG("OPAQ");
            RenderObjectList(RM_OPAQUE, entry.opaque.ptr_in_words * 4, &rect);
        
            if (!entry.opaque_mod.empty)
            {
                RENDLOG("OPAQ_MOD");
                RenderObjectList(RM_MODIFIER, entry.opaque_mod.ptr_in_words * 4, &rect);
            }
        }

        RENDLOG("OP_PARAMS");
        // Render TAGS to ACCUM
        RenderParamTags<RM_OPAQUE>(rect.left, rect.top);

        // render PT to TAGS
        if (!entry.puncht.empty)
        {
            RENDLOG("PT");

            PeelBuffersPTInitial(FLT_MAX);
            
            ClearMoreToDraw();

            // Render to TAGS
            RenderObjectList(RM_PUNCHTHROUGH_PASS0, entry.puncht.ptr_in_words * 4, &rect);

            // keep reference Z buffer
            PeelBuffersPT();

            RENDLOG("PT_PARAMS");
            // Render TAGS to ACCUM, making Z holes as-needed
            RenderParamTags<RM_PUNCHTHROUGH_PASS0>(rect.left, rect.top);

            while (GetMoreToDraw()) {
                RENDLOG("PT_N");
                ClearMoreToDraw();

                // Render to TAGS
                RenderObjectList(RM_PUNCHTHROUGH_PASSN, entry.puncht.ptr_in_words * 4, &rect);

                if (!GetMoreToDraw())
                    break;
                
                ClearMoreToDraw();
                // keep reference Z buffer
                PeelBuffersPT();

                RENDLOG("PT_N_PARAMS");
                // Render TAGS to ACCUM, making Z holes as-needed
                RenderParamTags<RM_PUNCHTHROUGH_PASS0>(rect.left, rect.top);
            }
            if (!entry.opaque_mod.empty)
            {
                RENDLOG("PT_MOD");
                RenderObjectList(RM_MODIFIER, entry.opaque_mod.ptr_in_words * 4, &rect);
                RENDLOG("PT_MOD_PARAMS");
                RenderParamTags<RM_PUNCHTHROUGH_MV>(rect.left, rect.top);
            }
        }

        // layer peeling rendering
        if (!entry.trans.empty)
        {
            if (entry.control.pre_sort) {
                RENDLOG("TR_PS");
                 // clear the param buffer
                 ClearParamStatusBuffer();

                 // render to TAGS
                 {
                     RenderObjectList(RM_TRANSLUCENT_PRESORT, entry.trans.ptr_in_words * 4, &rect);
                 }

                // what happens with modvols here?
                //  if (!entry.trans_mod.empty)
                //  {
                //      RenderObjectList(RM_MODIFIER, entry.trans_mod.ptr_in_words * 4, &rect);
                //  }
            } else {
                RENDLOG("TR_AS");
                SetTagToMax();
                do
                {
                    RENDLOG("TR_AS_N");
                    // prepare for a new pass
                    ClearMoreToDraw();

                    // copy depth test to depth reference buffer, clear depth test buffer, clear stencil
                    PeelBuffers(FLT_MAX, 0);

                    // render to TAGS
                    {
                        RenderObjectList(RM_TRANSLUCENT_AUTOSORT, entry.trans.ptr_in_words * 4, &rect);
                    }

                    if (!entry.trans_mod.empty)
                    {
                        RenderObjectList(RM_MODIFIER, entry.trans_mod.ptr_in_words * 4, &rect);
                    }

                    RENDLOG("TR_PARAMS");
                    // render TAGS to ACCUM
                    RenderParamTags<RM_TRANSLUCENT_AUTOSORT>(rect.left, rect.top);
                } while (GetMoreToDraw() != 0);
            }
        }

        {
            auto copy = (uint32_t*)GetColorOutputBuffer();
            RENDLOG("PIXELS");
            for (unsigned i = 0; i < MAX_RENDER_PIXELS; i++)
            {
                RENDLOG("%08X", copy[i]);
            }
        }
        
        // Copy to vram
        if (!entry.control.no_writeout)
        {
            // Precomputed “threshold biases” = bias4[bayer4[i][j]]
            static constexpr uint8_t bayerBias[4][4] = {
                {   8, 136,  40, 168 },  // 0→8, 8→136, 2→40, 10→168
                { 200,  72, 232, 104 },  //12→200,4→72, 14→232,6→104
                {  56, 184,  24, 152 },  // 3→56,11→184,1→24, 9→152
                { 248, 120, 216,  88 }   //15→248,7→120,13→216,5→88
            };

            auto copy = GetColorOutputBuffer();

            auto field = SCALER_CTL.fieldselect;
            auto interlace = SCALER_CTL.interlace;

            auto base = (interlace && field) ? FB_W_SOF2 : FB_W_SOF1;

            // very few configurations supported here
            assert(SCALER_CTL.hscale == 0);
            assert(SCALER_CTL.interlace == 0); // write both SOFs
            auto vscale = SCALER_CTL.vscalefactor;
            assert(vscale == 0x401 || vscale == 0x400 || vscale == 0x800);

            auto fb_packmode = FB_W_CTRL.fb_packmode;
            assert(fb_packmode == 0x1 || fb_packmode == 0x6); // 565 RGB16

            auto src = copy;
            auto bpp = fb_packmode == 0x1 ? 2 : 4;
            auto offset_bytes = entry.control.tilex * 32 * bpp + entry.control.tiley * 32 * FB_W_LINESTRIDE.stride * 8;

            for (int y = 0; y < 32; y++)
            {
                //auto base = (y&1) ? FB_W_SOF2 : FB_W_SOF1;
                auto dst = base + offset_bytes + (y)*FB_W_LINESTRIDE.stride * 8;

                for (int x = 0; x < 32; x++)
                {
                    if (fb_packmode == 0x1) {
                        int r8 = src[0];
                        int g8 = src[1];
                        int b8 = src[2];

                        int T = bayerBias[y & 3][x & 3];

                        // integer quantize exactly as before
                        int r5 = (r8 * 31 + T) / 255;
                        int g6 = (g8 * 63 + T) / 255;
                        int b5 = (b8 * 31 + T) / 255;

                        // clamp (just in case)
                        if(r5<0) r5=0; else if(r5>31) r5=31;
                        if(g6<0) g6=0; else if(g6>63) g6=63;
                        if(b5<0) b5=0; else if(b5>31) b5=31;
                        
                        auto pixel = (r5 << 0) | (g6 << 5) | (b5 << 11);
                        pvr_write_area1_16(emu_vram, dst, pixel);
                    }
                    else {
                        auto pixel = src[0] + src[1] * 256U + src[2] * 256U * 256U + src[3]  * 256U * 256U * 256U;
                        pvr_write_area1_32(emu_vram, dst, pixel);
                    }
                    

                    dst += bpp;
                    src += 4; // skip alpha
                }
            }
        }
    } while (!entry.control.last_region);
}