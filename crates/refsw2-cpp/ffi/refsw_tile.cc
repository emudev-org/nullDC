/*
	This file is part of libswirl

   Implementes the Reference SoftWare renderer (RefRendInterface) backend for refrend_base.
   This includes buffer operations and rasterization

   Pixel operations are in refsw_pixel.cpp (PixelPipeline interface)
*/
// #include "license/bsd"

#include <cmath>
#include <memory>
#include <set>
#include <float.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "refsw_tile.h"
#include "TexUtils.h"
#include <cassert>

#define die(msg) assert(!msg)

// #define STB_IMAGE_WRITE_IMPLEMENTATION
// #include "deps/stb/stb_image_write.h"

extern uint8_t* emu_vram;

TagState       tagStatus[MAX_RENDER_PIXELS];
parameter_tag_t tagBuffer[2] [MAX_RENDER_PIXELS];
StencilType     stencilBuffer[MAX_RENDER_PIXELS];
uint32_t             colorBuffer1 [MAX_RENDER_PIXELS];
uint32_t             colorBuffer2 [MAX_RENDER_PIXELS];
ZType           depthBuffer[3] [MAX_RENDER_PIXELS];

constexpr const uint32_t tagBufferA = 0;
constexpr const uint32_t tagBufferB = 1;
constexpr const uint32_t depthBufferA = 0;
constexpr const uint32_t depthBufferB = 1;
constexpr const uint32_t depthBufferC = 2;

static float mmin(float a, float b, float c, float d)
{
    float rv = std::min(a, b);
    rv = std::min(c, rv);
    return std::max(d, rv);
}

static float mmax(float a, float b, float c, float d)
{
    float rv = std::max(a, b);
    rv = std::max(c, rv);
    return std::min(d, rv);
}


// Z buffer doesn't store sign, and has 19 bits of m
float  mask_w(float w) {
    // uint32_t wu = (uint32_t&)w;
    // wu = wu & 0x7FFFFFF8;
    // return (float&)wu;
    return w;
}

#if defined(__CLANG__) || defined(__GNUC__)
#define always_inline always_inline
#else
#define always_inline __forceinline
#endif

void ClearBuffers(uint32_t paramValue, float depthValue, uint32_t stencilValue)
{
    auto zb = depthBuffer[depthBufferA];
    auto stencil = stencilBuffer;
    auto pb = tagBuffer[tagBufferA];;

    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        zb[i] = mask_w(depthValue);
        stencil[i] = stencilValue;
        pb[i] = paramValue;
        tagStatus[i] = { true, false };
    }
}

void ClearParamStatusBuffer() {
    auto ts = tagStatus;

    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        ts[i] = { false, false };
    }
}

void PeelBuffersPTInitial(float depthValue) {
    memcpy(depthBuffer[depthBufferC], depthBuffer[depthBufferA], sizeof(ZType) * MAX_RENDER_PIXELS);
    auto ts = tagStatus;

    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        ts[i] = { false, false };
        stencilBuffer[i] = 0;
    }
}

void PeelBuffersPT() {
    memcpy(depthBuffer[depthBufferB], depthBuffer[depthBufferA], sizeof(ZType) * MAX_RENDER_PIXELS);
    memcpy(tagBuffer[tagBufferB], tagBuffer[tagBufferA], sizeof(parameter_tag_t) * MAX_RENDER_PIXELS);
}

void SetTagToMax()
{
    memset(tagBuffer[tagBufferA], 0xFF, sizeof(tagBuffer[tagBufferA]));
}
void PeelBuffers(float depthValue, uint32_t stencilValue)
{
    memcpy(depthBuffer[depthBufferB], depthBuffer[depthBufferA], sizeof(ZType) * MAX_RENDER_PIXELS);
    memcpy(tagBuffer[tagBufferB], tagBuffer[tagBufferA], sizeof(parameter_tag_t) * MAX_RENDER_PIXELS);


    auto zb = depthBuffer[depthBufferA];
    auto stencil = stencilBuffer;

    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        zb[i] = mask_w(depthValue);    // set the "closest" test to furthest value possible
        tagStatus[i] = { false, false };
        stencil[i] = stencilValue;
    }
}


void SummarizeStencilOr() {
    auto stencil = stencilBuffer;

    // post movdol merge INSIDE
    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        if (stencil[i] & 0b100) {
            stencil[i] |= (stencil[i] >>1);
            stencil[i] &=0b001; // keep only status bit
        }
    }
}

void SummarizeStencilAnd() {
    auto stencil = stencilBuffer;

    for (int i = 0; i < MAX_RENDER_PIXELS; i++) {
        // post movdol merge OUTSIDE
        if (stencil[i] & 0b100) {
            stencil[i] &= (stencil[i] >>1);
            stencil[i] &=0b001; // keep only status bit
        }
    }
}

bool MoreToDraw;
void ClearMoreToDraw()
{
    MoreToDraw = 0;
}

bool GetMoreToDraw()
{
    return MoreToDraw;
}

    // Render to ACCUM from TAG buffer
// TAG holds references to trianes, ACCUM is the tile framebuffer
template<RenderMode rm>
void RenderParamTags(int tileX, int tileY) {
    float halfpixel = HALF_OFFSET.tsp_pixel_half_offset ? 0.5f : 0;
    taRECT rect;
    rect.left = tileX;
    rect.top = tileY;
    rect.bottom = rect.top + 32;
    rect.right = rect.left + 32;

    for (int y = 0; y < 32; y++) {
        for (int x = 0; x < 32; x++) {
            auto index = y * 32 + x;
            auto tag =  tagBuffer[tagBufferA][index];
            ISP_BACKGND_T_type t { .full = tag };
            bool InVolume = (stencilBuffer[index] & 0b001) == 0b001 && t.shadow;
            bool TagValid = tagStatus[index].valid;
            
            if (rm == RM_PUNCHTHROUGH_MV) {
                if (!InVolume) {
                    continue;
                } else {
                    TagValid = tagStatus[index].rendered;
                }
            }

            if (rm == RM_PUNCHTHROUGH_PASS0 || rm == RM_PUNCHTHROUGH_PASSN) {
                InVolume = false;
            }

            if (TagValid) {
                const auto& Entry = GetFpuEntry(&rect, rm, t);
                auto invW = Entry.ips.invW.Ip(x + halfpixel, y + halfpixel);
                bool AlphaTestPassed = PixelFlush_tsp(rm == RM_PUNCHTHROUGH_PASS0 || rm == RM_PUNCHTHROUGH_PASSN, &Entry, x + halfpixel, y + halfpixel, index, invW, InVolume, t);

                if (rm == RM_PUNCHTHROUGH_PASS0 || rm == RM_PUNCHTHROUGH_PASSN) {
                    // can only happen when rm == RM_PUNCHTHROUGH
                    if (!AlphaTestPassed) {
                        MoreToDraw = true;
                        // Feedback Channel
                        depthBuffer[depthBufferA][index] = depthBuffer[depthBufferC][index];
                    } else {
                        tagStatus[index].rendered = true;
                        tagStatus[index].valid = false;
                    }
                }

                if (rm == RM_TRANSLUCENT_PRESORT) {
                    tagStatus[index].valid = false;
                }
            }
        }
    }
}

template void RenderParamTags<RM_OPAQUE>(int tileX, int tileY);
template void RenderParamTags<RM_PUNCHTHROUGH_PASS0>(int tileX, int tileY);
template void RenderParamTags<RM_PUNCHTHROUGH_PASSN>(int tileX, int tileY);
template void RenderParamTags<RM_PUNCHTHROUGH_MV>(int tileX, int tileY);
template void RenderParamTags<RM_TRANSLUCENT_AUTOSORT>(int tileX, int tileY);
template void RenderParamTags<RM_TRANSLUCENT_PRESORT>(int tileX, int tileY);
template void RenderParamTags<RM_MODIFIER>(int tileX, int tileY);

#define vert_packed_color_(to,src) \
	{ \
	uint32_t t=src; \
	to[0] = (uint8_t)(t);t>>=8;\
	to[1] = (uint8_t)(t);t>>=8;\
	to[2] = (uint8_t)(t);t>>=8;\
	to[3] = (uint8_t)(t);      \
	}

//decode a vertex in the native pvr format
void decode_pvr_vertex(DrawParameters* params, pvr32addr_t ptr, Vertex* cv, uint32_t two_volumes)
{
    //XYZ
    //UV
    //Base Col
    //Offset Col

    //XYZ are _allways_ there :)
    cv->x=vrf(emu_vram, ptr);ptr+=4;
    cv->y=vrf(emu_vram, ptr);ptr+=4;
    cv->z=vrf(emu_vram, ptr);ptr+=4;

    if (params->isp.Texture)
    {	//Do texture , if any
        if (params->isp.UV_16b)
        {
            uint32_t uv=vri(emu_vram, ptr);
            cv->u = f16((uint16_t)(uv >>16));
            cv->v = f16((uint16_t)(uv >> 0));
            ptr+=4;
        }
        else
        {
            cv->u=vrf(emu_vram, ptr);ptr+=4;
            cv->v=vrf(emu_vram, ptr);ptr+=4;
        }
    }

    //Color
    uint32_t col=vri(emu_vram, ptr);ptr+=4;
    vert_packed_color_(cv->col,col);
    if (params->isp.Offset)
    {
        //Intensity color
        uint32_t col=vri(emu_vram, ptr);ptr+=4;
        vert_packed_color_(cv->spc,col);
    }

    if (two_volumes) {
        if (params->isp.Texture)
        {	//Do texture , if any
            if (params->isp.UV_16b)
            {
                uint32_t uv=vri(emu_vram, ptr);
                cv->u1 = f16((uint16_t)(uv >>16));
                cv->v1 = f16((uint16_t)(uv >> 0));
                ptr+=4;
            }
            else
            {
                cv->u1=vrf(emu_vram, ptr);ptr+=4;
                cv->v1=vrf(emu_vram, ptr);ptr+=4;
            }
        }

        //Color
        uint32_t col1=vri(emu_vram, ptr);ptr+=4;
        vert_packed_color_(cv->col1,col1);
        if (params->isp.Offset)
        {
            //Intensity color
            uint32_t col1=vri(emu_vram, ptr);ptr+=4;
            vert_packed_color_(cv->spc1,col1);
        }
    }
}

// decode an object (params + vertexes)
uint32_t decode_pvr_vertices(DrawParameters* params, pvr32addr_t base, uint32_t skip, uint32_t two_volumes, Vertex* vtx, int count, int offset)
{
    params->isp.full=vri(emu_vram, base);
    params->tsp[0].full=vri(emu_vram, base+4);
    params->tcw[0].full=vri(emu_vram, base+8);

    base += 12;
    if (two_volumes) {
        params->tsp[1].full=vri(emu_vram, base+0);
        params->tcw[1].full=vri(emu_vram, base+4);
        base += 8;
    }

    for (int i = 0; i < offset; i++) {
        base += (3 + skip * (two_volumes+1)) * 4;
    }

    for (int i = 0; i < count; i++) {
        decode_pvr_vertex(params,base, &vtx[i], two_volumes);
        base += (3 + skip * (two_volumes+1)) * 4;
    }

    return base;
}

struct {
    FpuEntry entry;
    uint32_t      tag;
} fpuCache[32];

const FpuEntry& GetFpuEntry(taRECT *rect, RenderMode render_mode, ISP_BACKGND_T_type core_tag)
{
    if (fpuCache[core_tag.param_offs_in_words & 31].tag == core_tag.full) {
        return fpuCache[core_tag.param_offs_in_words & 31].entry;
    }
    FpuEntry &entry = fpuCache[core_tag.param_offs_in_words & 31].entry;
    Vertex vtx[3];
    decode_pvr_vertices(&entry.params, PARAM_BASE + core_tag.param_offs_in_words * 4, core_tag.skip, core_tag.shadow & ~FPU_SHAD_SCALE.intensity_shadow, vtx, 3, core_tag.tag_offset);

    entry.ips.Setup(rect, &entry.params, vtx[0], vtx[1], vtx[2], core_tag.shadow & ~FPU_SHAD_SCALE.intensity_shadow);

    fpuCache[core_tag.param_offs_in_words & 31].tag = core_tag.full;

    return fpuCache[core_tag.param_offs_in_words & 31].entry ;
}

void ClearFpuCache() {
    memset(fpuCache, 0, sizeof(fpuCache));
}

// this is disabled for now, as it breaks game scenes
inline always_inline bool IsTopLeft(float x, float y) {
    bool IsTop = y == 0 && x > 0;
    bool IsLeft = y < 0;

    return IsTop || IsLeft;
}


// Depth processing for a pixel -- render_mode 0: OPAQ, 1: PT, 2: TRANS
template<RenderMode render_mode>
inline always_inline void PixelFlush_isp(uint32_t depth_mode, uint32_t ZWriteDis, float x, float y, float invW, uint32_t index, parameter_tag_t tag)
{
    auto pb = tagBuffer[tagBufferA] + index;
    auto ts = tagStatus + index;
    auto pb2 = tagBuffer[tagBufferB] + index;
    auto zb = depthBuffer[depthBufferA] + index;
    auto zb2 = depthBuffer[depthBufferB] + index;
    auto stencil = stencilBuffer + index;

    auto mode = depth_mode;
        
    if (render_mode == RM_PUNCHTHROUGH_PASS0 || render_mode == RM_PUNCHTHROUGH_PASSN)
        mode = 6;
    else if (render_mode == RM_TRANSLUCENT_AUTOSORT)
        mode = 3;
    else if (render_mode == RM_MODIFIER)
        mode = 6;

    RENDLOG("ISP: %d %d %d %f %f %f %08X", index, render_mode, mode, x, y, invW, tag);
    switch(mode) {
        // never
        case 0: RENDLOG("ZFAIL"); return; break;
        // less
        case 1: if (invW >= *zb) { RENDLOG("ZFAIL"); return; } break;
        // equal
        case 2: if (invW != *zb) { RENDLOG("ZFAIL"); return; } break;
        // less or equal
        case 3: if (invW > *zb) {
            if (render_mode == RM_TRANSLUCENT_AUTOSORT) {
                MoreToDraw = true;
            }
            RENDLOG("ZFAIL");
            return;
        }break;
        // greater
        case 4: if (invW <= *zb) { RENDLOG("ZFAIL"); return; } break;
        // not equal
        case 5: if (invW == *zb) { RENDLOG("ZFAIL"); return; } break;
        // greater or equal
        case 6: if (invW < *zb) { RENDLOG("ZFAIL"); return; } break;
        // always
        case 7: break;
    }

    switch (render_mode)
    {
        // OPAQ
        case RM_OPAQUE:
        {
            // Z pre-pass only
            if (!ZWriteDis) {
                *zb = mask_w(invW);
            }
            *pb = tag;
            ts->valid = 1;
            RENDLOG("RENDERED: %f", *zb);
        }
        break;

        case RM_MODIFIER:
        {
            // Flip on Z pass

            RENDLOG("STENCIL: %08X", *stencil);

            *stencil ^= 0b0010;

            // This pixel has valid stencil for summary
            *stencil |= 0b100;
        }
        break;

        case RM_PUNCHTHROUGH_PASS0:
        {
            *zb = mask_w(invW);
            *pb = tag;

            RENDLOG("RENDERED");
            ts->valid = 1;
        }
        break;
        // PT
        case RM_PUNCHTHROUGH_PASSN:
        {
            if (ts->rendered) {
                RENDLOG("ALREADY_DRAWN");
                return;
            }
            
            if (invW > *zb2) {
                RENDLOG("ZFAIL2");
                return;
            }

            if (invW == *zb2 || invW == *zb) {
                auto tagRendered = *pb2;

                if ((tag & PARAMETER_TAG_SORT_MASK) <= (tagRendered & PARAMETER_TAG_SORT_MASK)) {
                    RENDLOG("ZFAIL3");
                    return;
                }
            }
            
            MoreToDraw = true;

            *zb = mask_w(invW);
            *pb = tag;

            RENDLOG("RENDERED");
        }
        break;

        // Layer Peeling. zb2 holds the reference depth, zb is used to find closest to reference
        case RM_TRANSLUCENT_PRESORT:
        {
            if (!ZWriteDis) {
                *zb = mask_w(invW);
            }
            *pb = tag;
            ts->valid = true;
            RENDLOG("RENDERED: %f", *zb);
        }
        break;
        case RM_TRANSLUCENT_AUTOSORT:
        {
            if (invW < *zb2) {
                RENDLOG("ZFAIL4");
                return;
            }

            if (invW == *zb2) {
                auto tagRendered = *pb2;

                // if tag is earlier or same as last rendered, skip
                if ((tag & PARAMETER_TAG_SORT_MASK) <= (tagRendered & PARAMETER_TAG_SORT_MASK) && tagRendered != 0xFFFFFFFF) {
                    RENDLOG("ZFAIL7");
                    return;
                }
            }

            if (invW == *zb) {
                auto tagRendered = *pb2;

                // if tag is earlier or same as last rendered, skip
                if ((tag & PARAMETER_TAG_SORT_MASK) <= (tagRendered & PARAMETER_TAG_SORT_MASK) && tagRendered != 0xFFFFFFFF) {
                    RENDLOG("ZFAIL5");
                    return;
                }
                
                if (ts->valid) {
                    auto tagPending = *pb;
                    // if tag is later than the current pending, skip
                    if ((tag & PARAMETER_TAG_SORT_MASK) > (tagPending & PARAMETER_TAG_SORT_MASK)) {
                        MoreToDraw = true;
                        RENDLOG("ZFAIL6");
                        return;
                    }
                    assert(tag != tagPending);
                }
            }

            *zb = mask_w(invW);

            if (ts->valid) {
                MoreToDraw = true;
            }
            ts->valid = true;
            *pb = tag;
            RENDLOG("RENDERED");
        }
        break;

        case RM_PUNCHTHROUGH_MV: die("this is invalid here"); break;
    }
}

// Rasterize a single triangle to ISP (or ISP+TSP for PT)
template<RenderMode render_mode>
void RasterizeTriangle(DrawParameters* params, parameter_tag_t tag, const Vertex& v1, const Vertex& v2, const Vertex& v3, const Vertex* v4, taRECT* area)
{
    const int stride_bytes = STRIDE_PIXEL_OFFSET * 4;
    //Plane equation

#define FLUSH_NAN(a) std::isnan(a) ? 0 : a

    const float Y1 = FLUSH_NAN(v1.y);
    const float Y2 = FLUSH_NAN(v2.y);
    const float Y3 = FLUSH_NAN(v3.y);
    const float Y4 = v4 ? FLUSH_NAN(v4->y) : 0;

    const float X1 = FLUSH_NAN(v1.x);
    const float X2 = FLUSH_NAN(v2.x);
    const float X3 = FLUSH_NAN(v3.x);
    const float X4 = v4 ? FLUSH_NAN(v4->x) : 0;

    int sgn = 1;

    float tri_area = ((X1 - X3) * (Y2 - Y3) - (Y1 - Y3) * (X2 - X3));

    if (tri_area > 0)
        sgn = -1;

    // cull
    if (params->isp.CullMode != 0) {
        //area: (X1-X3)*(Y2-Y3)-(Y1-Y3)*(X2-X3)

        float abs_area = fabsf(tri_area);

        if (abs_area < FPU_CULL_VAL)
            return;

        if (params->isp.CullMode >= 2) {
            uint32_t mode = params->isp.CullMode & 1;

            if (
                (mode == 0 && tri_area < 0) ||
                (mode == 1 && tri_area > 0)) {
                RENDLOG("CULLED");
                return;
            }
        }
    }

    // Bounding rectangle
//  int minx = mmin(X1, X2, X3, area->left);
//  int miny = mmin(Y1, Y2, Y3, area->top);

//  int spanx = mmax(X1+1, X2+1, X3+1, area->right - 1) - minx + 1;
//  int spany = mmax(Y1+1, Y2+1, Y3+1, area->bottom - 1) - miny + 1;

    //Inside scissor area?
//  if (spanx < 0 || spany < 0)
//      return;

    // Half-edge constants
    const float DX12 = sgn * (X1 - X2);
    const float DX23 = sgn * (X2 - X3);
    const float DX31 = v4 ? sgn * (X3 - X4) : sgn * (X3 - X1);
    const float DX41 = v4 ? sgn * (X4 - X1) : 0;

    const float DY12 = sgn * (Y1 - Y2);
    const float DY23 = sgn * (Y2 - Y3);
    const float DY31 = v4 ? sgn * (Y3 - Y4) : sgn * (Y3 - Y1);
    const float DY41 = v4 ? sgn * (Y4 - Y1) : 0;

    float C1 = DY12 * (X1 - area->left) - DX12 * (Y1 - area->top);
    float C2 = DY23 * (X2 - area->left) - DX23 * (Y2 - area->top);
    float C3 = DY31 * (X3 - area->left) - DX31 * (Y3 - area->top);
    float C4 = v4 ? DY41 * (X4 - area->left) - DX41 * (Y4 - area->top) : 1;

    bool T1 = IsTopLeft(X2 - X1, Y2 - Y1);
    bool T2 = IsTopLeft(X3 - X2, Y3 - Y2);
    bool T3, T4;
    if (!v4) {
        T3 = IsTopLeft(X1 - X3, Y1 - Y3);
        T4 = true;
    } else {
        T3 = IsTopLeft(X4 - X3, Y4 - Y3);
        T4 = IsTopLeft(X1 - X4, Y1 - Y4);
    }
    PlaneStepper3 Z;
    Z.Setup(area, v1, v2, v3, v1.z, v2.z, v3.z);

    float halfpixel = HALF_OFFSET.fpu_pixel_half_offset ? 0.5f : 0;
    float y_ps    = halfpixel;
    float minx_ps = halfpixel;

    // Loop through ALL pixels in the tile (no smart clipping)
	for (int y = 0; y < 32; y++)
    {
        float x_ps = minx_ps;
            float kXhs12 = C1 + DX12 * y_ps - DY12 * 0;
            float kXhs23 = C2 + DX23 * y_ps - DY23 * 0;
            float kXhs31 = C3 + DX31 * y_ps - DY31 * 0;
            float kXhs41 = C4 + DX41 * y_ps - DY41 * 0;
            float zXhs12 = C1 + DX12 * y_ps - DY12 * 32.5f;
            float zXhs23 = C2 + DX23 * y_ps - DY23 * 32.5f;
            float zXhs31 = C3 + DX31 * y_ps - DY31 * 32.5f;
            float zXhs41 = C4 + DX41 * y_ps - DY41 * 32.5f;

	if ((kXhs12 < 0 && zXhs12 < 0) || (kXhs23 < 0 && zXhs23 < 0) || (kXhs31 < 0 && zXhs31 < 0) || (kXhs41 < 0 && kXhs41 < 0))
	{
		goto next_y;
	}

        for (int x = 0; x < 32; x++)
        {
            float Xhs12 = C1 + DX12 * y_ps - DY12 * x_ps;
            float Xhs23 = C2 + DX23 * y_ps - DY23 * x_ps;
            float Xhs31 = C3 + DX31 * y_ps - DY31 * x_ps;
            float Xhs41 = C4 + DX41 * y_ps - DY41 * x_ps;

            bool inTriangle = (Xhs12 > 0 || (T1 && Xhs12 == 0)) &&
                              (Xhs23 > 0 || (T2 && Xhs23 == 0)) &&
                              (Xhs31 > 0 || (T3 && Xhs31 == 0)) &&
                              (Xhs41 > 0 || (T4 && Xhs41 == 0));
			
            if (inTriangle) {
                uint32_t index = y * 32 + x;
                float invW = Z.Ip(x_ps, y_ps);
                PixelFlush_isp<render_mode>(params->isp.DepthMode, params->isp.ZWriteDis, x_ps, y_ps, invW, index, tag);
            }

            x_ps = x_ps + 1;
        }
    next_y:
        y_ps = y_ps + 1;
    }
}

void (*RasterizeTriangle_table[7])(DrawParameters* params, parameter_tag_t tag, const Vertex& v1, const Vertex& v2, const Vertex& v3, const Vertex* v4, taRECT* area) = {
    &RasterizeTriangle<RM_OPAQUE>,
    &RasterizeTriangle<RM_PUNCHTHROUGH_PASS0>,
    &RasterizeTriangle<RM_PUNCHTHROUGH_PASSN>,
    &RasterizeTriangle<RM_PUNCHTHROUGH_MV>,
    &RasterizeTriangle<RM_TRANSLUCENT_AUTOSORT>,
    &RasterizeTriangle<RM_TRANSLUCENT_PRESORT>,
    &RasterizeTriangle<RM_MODIFIER>
};

uint8_t* GetColorOutputBuffer() {
    return (uint8_t*)colorBuffer1;
}


// Clamp and flip a texture coordinate
template<bool pp_Clamp, bool pp_Flip>
inline always_inline int ClampFlip(int coord, int size) {
    if (pp_Clamp) { // clamp
        if (coord < 0) {
            coord = 0;
        } else if (coord >= size) {
            coord = size-1;
        }
    } else if (pp_Flip) { // flip
        coord &= size*2-1;
        if (coord & size) {
            coord ^= size*2-1;
        }
    } else { //wrap
        coord &= size-1;
    }

    return coord;
}


const uint32_t MipPoint[11] =
{
    0x00003,//1
    0x00001 * 4,//2
    0x00002 * 4,//4
	0x00006 * 4,//8
	0x00016 * 4,//16
	0x00056 * 4,//32
	0x00156 * 4,//64
	0x00556 * 4,//128
	0x01556 * 4,//256
	0x05556 * 4,//512
	0x15556 * 4//1024
};

#if 0
static Color TextureFetchOld(TSP tsp, TCW tcw, int u, int v) {
    auto textel_stride = 8 << tsp.TexU;

    uint32_t start_address = tcw.TexAddr << 3;
    uint32_t base_address = start_address;
    
    uint32_t mip_bpp;
    if (tcw.VQ_Comp) {
        mip_bpp = 2;
    } else if (tcw.PixelFmt == PixelPal8) {
        mip_bpp = 8;
    }
    else if (tcw.PixelFmt == PixelPal4) {
        mip_bpp = 4;
    }
    else {
        mip_bpp = 16;
    }

    if (tcw.MipMapped) {
        base_address += MipPoint[tsp.TexU] * mip_bpp / 2;
    }

    uint32_t offset;
    if (tcw.VQ_Comp) {
        offset = twop(u, v, tsp.TexU, tsp.TexV) / 4;
    } else if (!tcw.ScanOrder) {
        offset = twop(u, v, tsp.TexU, tsp.TexV);
    } else {
        offset = u + textel_stride * v;
    }

    uint16_t memtel;
    if (tcw.VQ_Comp) {
        uint8_t index = emu_vram[(base_address + offset + 256*4*2) & VRAM_MASK];
        uint16_t *vq_book = (uint16_t*)&emu_vram[start_address];
        memtel = vq_book[index * 4 + (u&1)*2 + (v&1) ];
    } else {
        memtel = (uint16_t&)emu_vram[(base_address + offset *2) & VRAM_MASK];
    }

    uint32_t textel;
    switch (tcw.PixelFmt)
    {
        case PixelReserved:
        case Pixel1555: textel = ARGB1555_32(memtel); break;
        case Pixel565: textel = ARGB565_32(memtel); break;
        case Pixel4444: textel = ARGB4444_32(memtel); break;
        case PixelYUV: textel = 0xDE000EEF; break;
        case PixelBumpMap: textel = 0xDEADBEEF; break;
        case PixelPal4: textel = 0xFF00FFF0; break;
        case PixelPal8: textel = 0xFF0FF0FF; break;
    }
    return { .raw =  textel };
}
#endif

template<bool ScanOrder /* TODO: Expansion Patterns */>
inline always_inline uint32_t ExpandToARGB8888(uint32_t color, uint32_t mode) {
	switch(mode)
	{
        case 0: return ARGB1555_32(color);
        case 1: return ARGB565_32(color);
        case 2: return ARGB4444_32(color);
        case 3: return ARGB8888_32(color);  // this one just shuffles
    }
    return 0xDEADBEEF;
}

template<bool VQ_Comp>
inline always_inline uint32_t TexAddressGen(TCW tcw) {
    uint32_t base_address = tcw.TexAddr << 3;

    if (VQ_Comp) {
        base_address += 256 * 4 * 2;
    }

    return base_address;
}

template<bool VQ_Comp, bool MipMapped, bool ScanOrder>
inline always_inline uint32_t TexOffsetGen(TSP tsp, int u, int v, uint32_t stride, uint32_t MipLevel) {
    uint32_t mip_offset;
    
    if (MipMapped) {
        mip_offset = MipPoint[3 + tsp.TexU - MipLevel];
    } else {
        mip_offset = 0;
    }

    if (VQ_Comp || !ScanOrder) {
        if (MipMapped) {
            return mip_offset + twop(u, v, (tsp.TexU - MipLevel), (tsp.TexU - MipLevel));
        } else {
            return mip_offset + twop(u, v, tsp.TexU, tsp.TexV);
        }
    } else {
        return mip_offset + u + stride * v;
    }
}

// 4.1 format
template<bool VQ_Comp, uint8_t PixelFmt>
constexpr uint32_t fBitsPerPixel() {
    uint32_t rv = 16;
    if (PixelFmt == PixelPal8) {
        rv = 8;
    }
    else if (PixelFmt == PixelPal4) {
        rv = 4;
    }
    else {
        rv = 16;
    }

    if (VQ_Comp) {
        return 8 * 2 / (64 / rv); // 8 bpp / (pixels per 64 bits)
    } else {
        return rv * 2;
    }
}

uint64_t VQLookup(uint32_t start_address, uint64_t memtel, uint32_t offset) {
    uint8_t* memtel8 = (uint8_t*)&memtel;

    uint64_t *vq_book = (uint64_t*)&emu_vram[start_address & (VRAM_MASK-7)];
    uint8_t index = memtel8[offset & 7];
    return vq_book[index];
}

template<uint32_t StrideSel, uint32_t ScanOrder>
inline always_inline uint32_t TexStride(uint32_t TexU, uint32_t MipLevel) {
    if (StrideSel && ScanOrder)
		return (TEXT_CONTROL&31)*32;
    else
        return (8U << TexU) >> MipLevel;
}

template<uint32_t PixelFmt>
inline always_inline uint32_t DecodeTextel(uint32_t PalSelect, uint64_t memtel, uint32_t offset) {
    auto memtel_32 = (uint32_t*)&memtel;
    auto memtel_16 = (uint16_t*)&memtel;
    auto memtel_8 = (uint8_t*)&memtel;

    switch (PixelFmt)
    {
        case PixelReserved:
        case Pixel1555:
        case Pixel565:
        case Pixel4444:
        case PixelBumpMap:
            return memtel_16[offset & 3]; break;

        case PixelYUV: {
            auto memtel_yuv = memtel_32[offset & 1];
            auto memtel_yuv8 = (uint8_t*)&memtel_yuv;
            return YUV422(memtel_yuv8[1 + (offset & 2)], memtel_yuv8[0], memtel_yuv8[2]);
            }

        case PixelPal4: {
            auto local_idx = (memtel >> (offset & 15)*4) & 15;
            auto idx = PalSelect * 16 | local_idx;
            return PALETTE_RAM[idx];
        }
        break;
        case PixelPal8: {
            auto local_idx = memtel_8[offset & 7];
            auto idx = (PalSelect / 16) * 256 | local_idx;
            return PALETTE_RAM[idx];
        }
        break;
    }
    return 0xDEADBEEF;
}

template<uint32_t PixelFmt>
inline always_inline uint32_t GetExpandFormat() {
    if (PixelFmt == PixelPal4 || PixelFmt == PixelPal8) {
        return PAL_RAM_CTRL&3;
    } else if (PixelFmt == PixelBumpMap || PixelFmt == PixelYUV) {
        return 3;
    } else {
        return PixelFmt & 3;
    }
}

Color MipDebugColor[11] = {
    {.raw = 0xFF000060},
    {.raw = 0xFF000090},
    {.raw = 0xFF0000A0},

    {.raw = 0xFF006000},
    {.raw = 0xFF009000},
    {.raw = 0xFF00A000},

    {.raw = 0xFF600000},
    {.raw = 0xFF900000},
    {.raw = 0xFFA00000},

    {.raw = 0xFFA0A0A0},
    {.raw = 0xFFF0F0F0},
};

template<bool VQ_Comp, bool MipMapped, bool ScanOrder_, bool StrideSel_, uint8_t PixelFmt>
static Color TextureFetch(TSP tsp, TCW tcw, int u, int v, uint32_t MipLevel) {

    if (MipLevel == (tsp.TexU + 3)) {
        if (PixelFmt == PixelYUV) {
            return TextureFetch<VQ_Comp, MipMapped, ScanOrder_, StrideSel_, Pixel565>(tsp, tcw, u, v, MipLevel);
        }
     }

    // These are fixed to zero for pal4/pal8
    constexpr uint32_t ScanOrder = ScanOrder_ & ~(PixelFmt == PixelPal4 || PixelFmt == PixelPal8);
    constexpr uint32_t StrideSel = StrideSel_ & ~(PixelFmt == PixelPal4 || PixelFmt == PixelPal8);
    
    uint32_t stride = TexStride<StrideSel, ScanOrder>(tsp.TexU, MipLevel);

    uint32_t start_address = tcw.TexAddr << 3;

    auto fbpp = fBitsPerPixel<VQ_Comp, PixelFmt>();
    
    auto base_address = TexAddressGen<VQ_Comp>(tcw);
    auto offset = TexOffsetGen<VQ_Comp, MipMapped, ScanOrder>(tsp, u, v, stride, MipLevel);

    uint64_t memtel = (uint64_t&)emu_vram[(base_address + offset * fbpp / 16) & (VRAM_MASK-7)];

    if (VQ_Comp) {
        memtel = VQLookup(start_address, memtel, offset * fbpp / 16);
    }

    uint32_t textel = DecodeTextel<PixelFmt>(tcw.PalSelect, memtel, offset);

    uint32_t expand_format = GetExpandFormat<PixelFmt>();

    

    textel = ExpandToARGB8888<ScanOrder>(textel, expand_format);

    // auto old = TextureFetch2(texture, u, v);
    // if (textel != old.raw) {
    //     textel = TextureFetch2(texture, u, v).raw;
    //     textel = TextureFetch(texture, u, v).raw;
    //     die("Missmatch");
    // }
    // auto old = raw_GetTexture(tsp, tcw)[u + v * stride];
    // if (textel != old) {
    //     die("Missmatch");
    // }
    // This uses the old path for debugging
    // return { .raw = raw_GetTexture(tsp, tcw)[u + v * textel_stride] };
    RENDLOG("T: %08X %d %d %d", textel, u, v, MipLevel);
    return { .raw =  textel };
    // return MipDebugColor[10-MipLevel];
}
uint32_t to_u8_256(uint8_t v) {
    return v + (v >> 7);
}
using TextureFetch_fp = decltype(&TextureFetch<false, false, false, false, 0>);

// Fetch pixels from UVs, interpolate
template<bool pp_IgnoreTexA,  bool pp_ClampU, bool pp_ClampV, bool pp_FlipU, bool pp_FlipV, uint32_t pp_FilterMode>
static Color TextureFilter(TSP tsp, TCW tcw, float u, float v, uint32_t MipLevel, float dTrilinear, TextureFetch_fp fetch) {
        
    int halfpixel = HALF_OFFSET.texure_pixel_half_offset ? 0 : 127;

    if (MipLevel >= (tsp.TexU + 3)) {
        MipLevel = tsp.TexU+3;
    }
    int sizeU, sizeV;

    if (tcw.MipMapped) {
        sizeU = (8 << tsp.TexU) >> MipLevel;
        sizeV = (8 << tsp.TexU) >> MipLevel;
    } else {
        sizeU = 8 << tsp.TexU;
        sizeV = 8 << tsp.TexV;
    }

    int ui = u * sizeU * 256 + halfpixel;
    int vi = v * sizeV * 256 + halfpixel;

    auto offset00 = fetch(tsp, tcw, ClampFlip<pp_ClampU, pp_FlipU>((ui >> 8) + 1, sizeU), ClampFlip<pp_ClampV, pp_FlipV>((vi >> 8) + 1, sizeV), MipLevel);
    auto offset01 = fetch(tsp, tcw, ClampFlip<pp_ClampU, pp_FlipU>((ui >> 8) + 0, sizeU), ClampFlip<pp_ClampV, pp_FlipV>((vi >> 8) + 1, sizeV), MipLevel);
    auto offset10 = fetch(tsp, tcw, ClampFlip<pp_ClampU, pp_FlipU>((ui >> 8) + 1, sizeU), ClampFlip<pp_ClampV, pp_FlipV>((vi >> 8) + 0, sizeV), MipLevel);
    auto offset11 = fetch(tsp, tcw, ClampFlip<pp_ClampU, pp_FlipU>((ui >> 8) + 0, sizeU), ClampFlip<pp_ClampV, pp_FlipV>((vi >> 8) + 0, sizeV), MipLevel);

    Color textel = {0xAF674839};

    if (pp_FilterMode == 0) {
        // Point sampling
        for (int i = 0; i < 4; i++)
        {
            textel = offset11;
        }
    } else if (pp_FilterMode == 1) {
        // Bilinear filtering
        int ublend = to_u8_256(ui & 255);
        int vblend = to_u8_256(vi & 255);
        int nublend = 256 - ublend;
        int nvblend = 256 - vblend;

        for (int i = 0; i < 4; i++)
        {
            textel.bgra[i] = (
                (offset00.bgra[i] * ublend * vblend) +
                (offset01.bgra[i] * nublend * vblend) +
                (offset10.bgra[i] * ublend * nvblend) +
                (offset11.bgra[i] * nublend * nvblend)
            ) / 65536;
        };
    } else {
        // trilinear filtering A and B
        die("pp_FilterMode is trilinear");
    }
        

    if (pp_IgnoreTexA)
    {
        textel.a = 255;
    }

    RENDLOG("TF: %08X", textel.raw);

    return textel;
}

// Combine Base, Textel and Offset colors
template<bool pp_Texture, bool pp_Offset, uint32_t pp_ShadInstr>
static Color ColorCombiner(Color base, Color textel, Color offset) {

    Color rv = base;
    if (pp_Texture)
    {
        if (pp_ShadInstr == 0)
        {
            //color.rgb = texcol.rgb;
            //color.a = texcol.a;

            rv = textel;
        }
        else if (pp_ShadInstr == 1)
        {
            //color.rgb *= texcol.rgb;
            //color.a = texcol.a;
            for (int i = 0; i < 3; i++)
            {
                rv.bgra[i] = textel.bgra[i] * to_u8_256(base.bgra[i]) / 256;
            }

            rv.a = textel.a;
        }
        else if (pp_ShadInstr == 2)
        {
            //color.rgb=mix(color.rgb,texcol.rgb,texcol.a);
            uint32_t tb = to_u8_256(textel.a);
            uint32_t cb = 256 - tb;

            for (int i = 0; i < 3; i++)
            {
                rv.bgra[i] = (textel.bgra[i] * tb + base.bgra[i] * cb) / 256;
            }

            rv.a = base.a;
        }
        else if (pp_ShadInstr == 3)
        {
            //color*=texcol
            for (int i = 0; i < 4; i++)
            {
                rv.bgra[i] = textel.bgra[i] * to_u8_256(base.bgra[i]) / 256;
            }
        }

        if (pp_Offset) {
            // mix only color, saturate
            for (int i = 0; i < 3; i++)
            {
                rv.bgra[i] = std::min(rv.bgra[i] + offset.bgra[i], 255);
            }
        }
    }

    RENDLOG("CC: %08X", rv.raw);
    return rv;
}

static Color BumpMapper(Color textel, Color offset) {
    uint8_t K1 = offset.a;
    uint8_t K2 = offset.r;
    uint8_t K3 = offset.g;
    uint8_t Q = offset.b;

    uint8_t R = textel.b;
    uint8_t S = textel.g;
    
    int32_t I = (K1*127*127 + K2*BM_SIN90[S]*127 + K3*BM_COS90[S]*BM_COS360[(R - Q) & 255])/127/127;
    if (I < 0) {
        I = 0;
    } else if (I > 255) {
        I = 255;
    }

	Color res;
	res.b = 255;
	res.g = 255;
	res.r = 255;
	res.a = I;

    RENDLOG("BM: %08X", res.raw);
    return res;
}

// Interpolate the base color, also cheap shadows modifier
template<bool pp_UseAlpha, bool pp_CheapShadows>
inline always_inline Color InterpolateBase(const PlaneStepper3* Col, float x, float y, float W, bool InVolume) {
    Color rv;
    uint32_t mult = 256;

    if (pp_CheapShadows) {
        if (InVolume) {
            mult = to_u8_256(FPU_SHAD_SCALE.scale_factor);
        }
    }

    rv.bgra[0] = 0.5f + Col[0].IpU8(x, y, W) * mult / 256;
    rv.bgra[1] = 0.5f + Col[1].IpU8(x, y, W) * mult / 256;
    rv.bgra[2] = 0.5f + Col[2].IpU8(x, y, W) * mult / 256;
    rv.bgra[3] = 0.5f + Col[3].IpU8(x, y, W) * mult / 256;

    if (!pp_UseAlpha)
    {
        rv.a = 255;
    }

    RENDLOG("IB: %08X", rv.raw);
    return rv;
}

// Interpolate the offset color, also cheap shadows modifier
template<bool pp_CheapShadows>
inline always_inline Color InterpolateOffs(const PlaneStepper3* Ofs, float x, float y, float W, bool InVolume) {
    Color rv;
    uint32_t mult = 256;

    if (pp_CheapShadows) {
        if (InVolume) {
            mult = to_u8_256(FPU_SHAD_SCALE.scale_factor);
        }
    }

    rv.bgra[0] = 0.5f + Ofs[0].IpU8(x, y, W) * mult / 256;
    rv.bgra[1] = 0.5f + Ofs[1].IpU8(x, y, W) * mult / 256;
    rv.bgra[2] = 0.5f + Ofs[2].IpU8(x, y, W) * mult / 256;
    rv.bgra[3] = 0.5f + Ofs[3].IpU8(x, y, W);

    RENDLOG("IO: %08X", rv.raw);
    return rv;
}

// select/calculate blend coefficient for the blend unit
template<uint32_t pp_AlphaInst, bool srcOther>
inline always_inline Color BlendCoefs(Color src, Color dst) {
    Color rv;

    switch(pp_AlphaInst>>1) {
        // zero
        case 0: rv.raw = 0; break;
        // other color
        case 1: rv = srcOther? src : dst; break;
        // src alpha
        case 2: for (int i = 0; i < 4; i++) rv.bgra[i] = src.a; break;
        // dst alpha
        case 3: for (int i = 0; i < 4; i++) rv.bgra[i] = dst.a; break;
    }

    if (pp_AlphaInst & 1) {
        for (int i = 0; i < 4; i++)
            rv.bgra[i] = 255 - rv.bgra[i];
    }

    return rv;
}

// Blending Unit implementation. Alpha blend, accum buffers and such
template<uint32_t pp_SrcSel, uint32_t pp_DstSel, uint32_t pp_SrcInst, uint32_t pp_DstInst, bool pp_AlphaTest>
static bool BlendingUnit(uint32_t index, Color col)
{
    bool at = true;

    if (pp_AlphaTest) {
        if (col.a < PT_ALPHA_REF) {
            col.a = 0;
            at = false;
        } else {
            col.a = 255;
        }
    }

    Color rv;
    Color src = {.raw  = pp_SrcSel ? colorBuffer2[index] : col.raw };
    Color dst = {.raw = pp_DstSel ? colorBuffer2[index] : colorBuffer1[index] };
        
    Color src_blend = BlendCoefs<pp_SrcInst, false>(src, dst);
    Color dst_blend = BlendCoefs<pp_DstInst, true>(src, dst);

    for (int j = 0; j < 4; j++)
    {
        rv.bgra[j] = std::min((src.bgra[j] * to_u8_256(src_blend.bgra[j]) + dst.bgra[j] * to_u8_256(dst_blend.bgra[j])) >> 8, 255U);
    }
    
    
    (pp_DstSel ? colorBuffer2[index] : colorBuffer1[index]) = rv.raw;
    
    RENDLOG("BU: %08X %08X %08X %08X %08X %d", rv.raw, src_blend.raw, dst_blend.raw, src.raw, dst.raw, at);

    return at;
}

inline always_inline uint8_t LookupFogTable(float invW) {
    uint8_t* fog_density=(uint8_t*)&FOG_DENSITY;
    float fog_den_mant=fog_density[1]/128.0f;  //bit 7 -> x. bit, so [6:0] -> fraction -> /128
    int32_t fog_den_exp=(int8_t)fog_density[0];
        
    float fog_den = fog_den_mant*powf(2.0f,fog_den_exp);

    float fogW = fog_den * invW;

    fogW = std::max((float)fogW, 1.0f);
    fogW = std::min((float)fogW, 255.999985f);

    union f32_fields {
        float full;
        struct {
            uint32_t m: 23;
            uint32_t e: 8;
            uint32_t s: 1;
        };
    };

    f32_fields fog_fields = { fogW };

    uint32_t index = (((fog_fields.e +1) & 7) << 4) |  ((fog_fields.m>>19) & 15);

    uint8_t blendFactor = (fog_fields.m>>11) & 255;
    uint8_t blend_inv = 255^blendFactor;

    auto fog_entry = (uint8_t*)&FOG_TABLE[index];

    uint8_t fog_alpha = (fog_entry[0] * to_u8_256(blendFactor) + fog_entry[1] * to_u8_256(blend_inv)) >> 8;

    return fog_alpha;
}

// Color Clamp and Fog a pixel
template<bool pp_Offset, bool pp_ColorClamp, uint32_t pp_FogCtrl>
inline always_inline Color FogUnit(Color col, float invW, uint8_t offs_a) {
    if (pp_ColorClamp) {
        Color clamp_max = { FOG_CLAMP_MAX };
        Color clamp_min = { FOG_CLAMP_MIN };

        for (int i = 0; i < 4; i++)
        {
            col.bgra[i] = std::min(col.bgra[i], clamp_max.bgra[i]);
            col.bgra[i] = std::max(col.bgra[i], clamp_min.bgra[i]);
        }

        RENDLOG("FC: %08X", col.raw);
    }

    switch(pp_FogCtrl) {
        // Look up mode 1
        case 0b00:
        // look up mode 2
        case 0b11:
            {
                uint8_t fog_alpha = LookupFogTable(invW);
                    
                uint8_t fog_inv = 255^fog_alpha;

                Color col_ram = { FOG_COL_RAM };

                if (pp_FogCtrl == 0b00) {
                    for (int i = 0; i < 3; i++) {
                        col.bgra[i] = (col.bgra[i] * to_u8_256(fog_inv) + col_ram.bgra[i] * to_u8_256(fog_alpha))>>8;
                    }
                } else {
                    for (int i = 0; i < 3; i++) {
                        col.bgra[i] = col_ram.bgra[i];
                    }
                    col.a = fog_alpha;
                }
            }
            break;

        // Per Vertex
        case 0b01:
            if (pp_Offset) {
                Color col_vert = { FOG_COL_VERT };
                uint8_t alpha = offs_a;
                uint8_t inv = 255^alpha;
                  
                for (int i = 0; i < 3; i++)
                {
                    col.bgra[i] = (col.bgra[i] * to_u8_256(inv) + col_vert.bgra[i] * to_u8_256(alpha))>>8;
                }
            }
            break;

                
        // No Fog
        case 0b10:
            break;
    }

    RENDLOG("FU: %08X", col.raw);
    return col;
}

void DumpTexture(TSP tsp, TCW tcw, TextureFetch_fp fetch) {
    char tex_dump[256];
    int max_mipmap = 0;
    int texU = tsp.TexU + 3;
    int texV = tsp.TexV + 3;
    if (tcw.MipMapped) {
        max_mipmap = texU;
        texV = texU;
    }
    for (int mip=0; mip<=max_mipmap; mip++) {
        int width = (1 << texU) >> mip;
        int height = (1 << texV) >> mip;
        snprintf(tex_dump, sizeof(tex_dump), "%s/texture-%08x-%08x-%d-%dx%d.png", dump_textures, tsp.full, tcw.full, mip, height, width);
        Color* tex = new Color[width * height];

        for (int t = 0; t < height; t++) {
            for (int s = 0; s < width; s++) {
                tex[t * width + s] = fetch(tsp, tcw, s, t, mip);
                std::swap(tex[t * width + s].r, tex[t * width + s].b);
            }   
        }

        // stbi_write_png(tex_dump, width, height, 4, tex, width * 4);
        delete[] tex;
    }
}
const char* dump_textures = nullptr;
std::set<uint64_t> texture_dumps;
using BlendingUnit_fp = decltype(&BlendingUnit<0,0,0,0,0>);
using ColorCombiner_fp = decltype(&ColorCombiner<0,0,0>);
using TextureFilter_fp = decltype(&TextureFilter<0,0,0,0,0,0>);
// Implement the full texture/shade pipeline for a pixel

static Color offs = { 0x20004080 }; // this one persists across invocations, as tested via bump maps. Default value was randomly chosen.
template<bool pp_UseAlpha, bool pp_Texture, bool pp_Offset, bool pp_ColorClamp, uint32_t pp_FogCtrl, bool pp_CheapShadows>
static bool PixelFlush_tsp(const FpuEntry *entry, float x, float y, float W, bool InVolume, uint32_t index, TextureFetch_fp fetch, TextureFilter_fp filter, ColorCombiner_fp combiner, BlendingUnit_fp blending)
{
    uint32_t two_voume_index = InVolume & !pp_CheapShadows;
    auto cb = (Color*)colorBuffer1 + index;
  
    Color base = { 0 }, textel = { 0 };

    base = InterpolateBase<pp_UseAlpha, pp_CheapShadows>(entry->ips.Col[two_voume_index], x, y, W, InVolume);

    float dTrilinear;
    uint32_t MipLevel;
    if (pp_Texture) {
        if (dump_textures) {

            uint64_t uid = entry->params.tsp[two_voume_index].full;
            uid = (uid << 32) | entry->params.tcw[two_voume_index].full;

            if (texture_dumps.count(uid) == 0) {
                texture_dumps.insert(uid);
                DumpTexture(entry->params.tsp[two_voume_index],  entry->params.tcw[two_voume_index], fetch);
            }
        }
        float u = entry->ips.U[two_voume_index].Ip(x, y, W);
        float v = entry->ips.V[two_voume_index].Ip(x, y, W);

        if (entry->params.tcw[two_voume_index].MipMapped) {
            int sizeU = 8 << entry->params.tsp[two_voume_index].TexU;
            // faux mip map cals
            // these really don't follow hw
            float ddx = (entry->ips.U[two_voume_index].ddx + entry->ips.V[two_voume_index].ddx);
            float ddy = (entry->ips.U[two_voume_index].ddy + entry->ips.V[two_voume_index].ddy);

            float dMip = fminf(fabsf(ddx), fabsf(ddy)) * W * sizeU * entry->params.tsp[two_voume_index].MipMapD / 4.0f;

            MipLevel = 0; // biggest
            while(dMip > 1.5 && MipLevel < 11) {
                MipLevel ++;
                dMip = dMip / 2;
            }
            dTrilinear = dMip;
        } else {
            dTrilinear = 0;
            MipLevel = 0;
        }
        
        textel = filter(entry->params.tsp[two_voume_index], entry->params.tcw[two_voume_index], u, v, MipLevel, dTrilinear, fetch);
        if (pp_Offset) {
            offs = InterpolateOffs<pp_CheapShadows>(entry->ips.Ofs[two_voume_index], x, y, W, InVolume);
        }
    }

    Color col;
    if (pp_Texture && entry->params.tcw[two_voume_index].PixelFmt == PixelBumpMap) {
        col = BumpMapper(textel, offs);
    } else {
        col = combiner(base, textel, offs);
    }
        
    col = FogUnit<pp_Offset, pp_ColorClamp, pp_FogCtrl>(col, 1/W, offs.a);

    // if (pp_Texture) {
    //     col = MipDebugColor[10-MipLevel];
    // } else {
    //     col = { .raw = 0 };
    // }

	return blending(index, col);
}
using PixelFlush_tsp_fp = decltype(&PixelFlush_tsp<0,0,0,0,0,0>);

#include "gentable.h"

// Lookup/create cached TSP parameters, and call PixelFlush_tsp
bool PixelFlush_tsp(bool pp_AlphaTest, const FpuEntry* entry, float x, float y, uint32_t index, float invW, bool InVolume, ISP_BACKGND_T_type core_tag)
{
    uint32_t two_voume_index = InVolume & !FPU_SHAD_SCALE.intensity_shadow;

    RENDLOG("TSP: %d %f %f %d %f %d %08X %08X %08X %08X", index, x, y, InVolume, invW, pp_AlphaTest, entry->params.isp.full, entry->params.tsp[two_voume_index].full, entry->params.tcw[two_voume_index].full, core_tag.full);

    auto fetch = TextureFetch_table
        [entry->params.tcw[two_voume_index].VQ_Comp]
        [entry->params.tcw[two_voume_index].MipMapped]
        [entry->params.tcw[two_voume_index].ScanOrder]
        [entry->params.tcw[two_voume_index].StrideSel]
        [entry->params.tcw[two_voume_index].PixelFmt];

    auto filter = TextureFilter_table
        [entry->params.tsp[two_voume_index].IgnoreTexA]
        [entry->params.tsp[two_voume_index].ClampU]
        [entry->params.tsp[two_voume_index].ClampV]
        [entry->params.tsp[two_voume_index].FlipU]
        [entry->params.tsp[two_voume_index].FlipV]
        [entry->params.tsp[two_voume_index].FilterMode];

    auto combiner = ColorCombiner_table
        [entry->params.isp.Texture]
        [entry->params.isp.Offset]
        [entry->params.tsp[two_voume_index].ShadInstr];

    auto blending = BlendingUnit_table
        [entry->params.tsp[two_voume_index].SrcSelect]
        [entry->params.tsp[two_voume_index].DstSelect]
        [entry->params.tsp[two_voume_index].SrcInstr]
        [entry->params.tsp[two_voume_index].DstInstr]
        [pp_AlphaTest];
    
    auto pixel = PixelFlush_tsp_table
        [entry->params.tsp[two_voume_index].UseAlpha]
        [entry->params.isp.Texture]
        [entry->params.isp.Offset]
        [entry->params.tsp[two_voume_index].ColorClamp]
        [entry->params.tsp[two_voume_index].FogCtrl]
        [FPU_SHAD_SCALE.intensity_shadow];

    return pixel(entry, x, y, 1/invW, InVolume, index, fetch, filter, combiner, blending);
}
