/*
	This file is part of libswirl
*/
// #include "license/bsd"


#pragma once
#include <stdint.h>

#define VRAM_SIZE (8*1024*1024)
#define VRAM_MASK (VRAM_SIZE - 1)


#define VRAM_BANK_BIT 0x400000

inline uint32_t pvr_map32(uint32_t offset32)
{
	//64b wide bus is achieved by interleaving the banks every 32 bits
	//const uint32_t bank_bit = VRAM_BANK_BIT;
	const uint32_t static_bits = (VRAM_MASK - (VRAM_BANK_BIT * 2 - 1)) | 3;
	const uint32_t offset_bits = (VRAM_BANK_BIT - 1) & ~3;

	uint32_t bank = (offset32 & VRAM_BANK_BIT) / VRAM_BANK_BIT;

	uint32_t rv = offset32 & static_bits;

	rv |= (offset32 & offset_bits) * 2;

	rv |= bank * 4;
	
	return rv;
}
inline float vrf(uint8_t* vram, uint32_t addr)
{
	return *(float*)&vram[pvr_map32(addr)];
}
inline uint32_t vri(uint8_t* vram, uint32_t addr)
{
	return *(uint32_t*)&vram[pvr_map32(addr)];
}
//write
inline void pvr_write_area1_16(void* ctx, uint32_t addr,uint16_t data)
{
	auto vram = reinterpret_cast<uint8_t*>(ctx);

    uint32_t vaddr = addr & VRAM_MASK;
	*(uint16_t*)&vram[pvr_map32(addr)]=data;
}
inline void pvr_write_area1_32(void* ctx, uint32_t addr,uint32_t data)
{
	auto vram = reinterpret_cast<uint8_t*>(ctx);

    uint32_t vaddr = addr & VRAM_MASK;
	*(uint32_t*)&vram[pvr_map32(addr)] = data;
}