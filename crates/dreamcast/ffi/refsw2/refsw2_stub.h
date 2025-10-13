#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void ffi_refsw2_render(uint8_t* vram, const uint32_t* regs);

#ifdef __cplusplus
}
#endif
