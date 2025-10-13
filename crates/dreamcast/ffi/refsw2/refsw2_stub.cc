#include "refsw2_stub.h"
#include "refsw_tile.h"
#include "TexUtils.h"

uint8_t* emu_vram;
const uint32_t* emu_regs;

void ffi_refsw2_render(uint8_t* vram, const uint32_t* regs) {
    emu_vram = vram;
    emu_regs = regs;

    RenderCORE();
}

void ffi_refsw2_init(void) {
    InitTexUtils();
}