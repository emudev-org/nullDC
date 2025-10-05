#include "dsp.h"

uint8_t aica_reg[0x8000];

uint8_t aica_ram[ 2 * 1024 * 1024 ];
uint32_t aram_mask = 2 * 1024 * 1024 - 1;