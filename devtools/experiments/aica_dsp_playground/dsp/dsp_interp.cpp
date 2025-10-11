//
// Audio Overload SDK
//
// Copyright (c) 2007-2009 R. Belmont and Richard Bannister, and others.
// All rights reserved.
//

// TODO: FIXME mame is compatible, but double check

#include "dsp.h"
#include <cassert>
#include <cstring>
#include <cstdio>

CommonData_struct* CommonData = (CommonData_struct*)&aica_reg[0x2800];
DSPData_struct* DSPData = (DSPData_struct*)&aica_reg[0x3000];
uint32_t MDEC_CT = 1;

int32_t GetMEMS(unsigned idx) {
	return DSPData->MEMS[idx].l | (DSPData->MEMS[idx].h << 8);
}

void SetMEMS(unsigned idx, int32_t val) {
	DSPData->MEMS[idx].l = val & 0xFF;
	DSPData->MEMS[idx].h = (val >> 8) & 0xFFFF;
}

int32_t GetMIXS(unsigned idx) {
	return DSPData->MIXS[idx].l | (DSPData->MIXS[idx].h << 4);
}

int32_t GetTEMP(unsigned idx) {
	return DSPData->TEMP[idx].l | (DSPData->TEMP[idx].h << 8);
}

void SetTEMP(unsigned idx, int32_t val) {
	DSPData->TEMP[idx].l = val & 0xFF;
	DSPData->TEMP[idx].h = (val >> 8) & 0xFFFF;
}

uint32_t GetRBL() {
	switch(CommonData->RBL) {
		case 0: return 8 * 1024;
		case 1: return 16 * 1024;
		case 2: return 32 * 1024;
		case 3: return 64 * 1024;
		default: return 0;
	}
}

uint32_t GetRBP() {
	return CommonData->RBP * 2048; // pointer in 1K words
}

int32_t ACC = 0;		//26 bit
int32_t SHIFTED = 0;	//24 bit
int32_t X = 0;			//24 bit
int32_t Y = 0;			//13 bit
int32_t B = 0;			//26 bit
int32_t INPUTS = 0;		//24 bit
int32_t MEMVAL[4] = { 0 };
int32_t FRC_REG = 0;	//13 bit
int32_t Y_REG = 0;		//24 bit
uint32_t ADRS_REG = 0;	//13 bit

extern "C" EMSCRIPTEN_KEEPALIVE void Step(int step) {
	uint32_t* IPtr = DSPData->MPRO + step * 4;

	uint32_t TRA = (IPtr[0] >> 9) & 0x7F;
	uint32_t TWT = (IPtr[0] >> 8) & 0x01;

	uint32_t XSEL = (IPtr[1] >> 15) & 0x01;
	uint32_t YSEL = (IPtr[1] >> 13) & 0x03;
	uint32_t IRA = (IPtr[1] >> 7) & 0x3F;
	uint32_t IWT = (IPtr[1] >> 6) & 0x01;

	uint32_t EWT = (IPtr[2] >> 12) & 0x01;
	uint32_t ADRL = (IPtr[2] >> 7) & 0x01;
	uint32_t FRCL = (IPtr[2] >> 6) & 0x01;
	uint32_t SHIFT = (IPtr[2] >> 4) & 0x03;
	uint32_t YRL = (IPtr[2] >> 3) & 0x01;
	uint32_t NEGB = (IPtr[2] >> 2) & 0x01;
	uint32_t ZERO = (IPtr[2] >> 1) & 0x01;
	uint32_t BSEL = (IPtr[2] >> 0) & 0x01;

	uint32_t COEF = step;

	// operations are done at 24 bit precision
#if 0
#define DUMP(v)	printf(" " #v ": %04X",v);

	printf("%d: ", step);
	DUMP(ACC);
	DUMP(SHIFTED);
	DUMP(X);
	DUMP(Y);
	DUMP(B);
	DUMP(INPUTS);
	// DUMP(MEMVAL);
	DUMP(FRC_REG);
	DUMP(Y_REG);
	// DUMP(ADDR);
	DUMP(ADRS_REG);
	printf("\n");
#endif

	// INPUTS RW
	assert(IRA < 0x38);
	if (IRA <= 0x1f)
		INPUTS = GetMEMS(IRA);
	else if (IRA <= 0x2F)
		INPUTS = GetMIXS(IRA - 0x20) << 4;		// MIXS is 20 bit
	else if (IRA <= 0x31)
		INPUTS = DSPData->EXTS[IRA - 0x30] << 8;	// EXTS is 16 bits
	else
		INPUTS = 0;

	INPUTS <<= 8;
	INPUTS >>= 8;

	if (IWT)
	{
		uint32_t IWA = (IPtr[1] >> 1) & 0x1F;
		SetMEMS(IWA, MEMVAL[step & 3]);	// MEMVAL was selected in previous MRD
		// "When read and write are specified simultaneously in the same step for INPUTS, TEMP, etc., write is executed after read."
		//if (IRA == IWA)
		//		INPUTS = MEMVAL[step & 3];
	}

	// Operand sel
	// B
	if (!ZERO)
	{
		if (BSEL)
			B = ACC;
		else
		{
			B = GetTEMP((TRA + MDEC_CT) & 0x7F) << 2; // expand to 26 bits
			B <<= 6;  //Sign extend
			B >>= 6;
		}
		if (NEGB)
			B = 0 - B;
	}
	else
		B = 0;

	// X
	if (XSEL)
		X = INPUTS;
	else
	{
		X = GetTEMP((TRA + MDEC_CT) & 0x7F);
		X <<= 8;
		X >>= 8;
	}

	// Y
	if (YSEL == 0)
		Y = FRC_REG;
	else if (YSEL == 1)
		Y = DSPData->COEF[COEF] >> 3;	//COEF is 16 bits
	else if (YSEL == 2)
		Y = (Y_REG >> 11) & 0x1FFF;
	else if (YSEL == 3)
		Y = (Y_REG >> 4) & 0x0FFF;

	if (YRL)
		Y_REG = INPUTS;

	// Shifter
	// There's a 1-step delay at the output of the X*Y + B adder. So we use the ACC value from the previous step.
	if (SHIFT == 0)
	{
		SHIFTED = ACC >> 2;				// 26 bits -> 24 bits
		if (SHIFTED > 0x0007FFFF)
			SHIFTED = 0x0007FFFF;
		if (SHIFTED < (-0x00080000))
			SHIFTED = -0x00080000;
	}
	else if (SHIFT == 1)
	{
		SHIFTED = ACC >> 1;				// 26 bits -> 24 bits and x2 scale
		if (SHIFTED > 0x0007FFFF)
			SHIFTED = 0x0007FFFF;
		if (SHIFTED < (-0x00080000))
			SHIFTED = -0x00080000;
	}
	else if (SHIFT == 2)
	{
		SHIFTED = ACC >> 1;
		SHIFTED <<= 8;
		SHIFTED >>= 8;
	}
	else if (SHIFT == 3)
	{
		SHIFTED = ACC >> 2;
		SHIFTED <<= 8;
		SHIFTED >>= 8;
	}

	// ACCUM
	Y <<= 19;
	Y >>= 19;

	int64_t v = ((int64_t)X * (int64_t)Y) >> 10;	// magic value from dynarec. 1 sign bit + 24-1 bits + 13-1 bits -> 26 bits?
	v <<= 6;	// 26 bits only
	v >>= 6;
	ACC = (int32_t)(v + B);
	ACC <<= 6;	// 26 bits only
	ACC >>= 6;

	if (TWT)
	{
		uint32_t TWA = (IPtr[0] >> 1) & 0x7F;
		SetTEMP((TWA + MDEC_CT) & 0x7F, SHIFTED);
	}

	if (FRCL)
	{
		if (SHIFT == 3)
			FRC_REG = SHIFTED & 0x0FFF;
		else
			FRC_REG = (SHIFTED >> 11) & 0x1FFF;
	}

	if (step & 1)
	{
		uint32_t MWT = (IPtr[2] >> 14) & 0x01;
		uint32_t MRD = (IPtr[2] >> 13) & 0x01;

		if (MRD || MWT)
		{
			uint32_t TABLE = (IPtr[2] >> 15) & 0x01;

			uint32_t NOFL = (IPtr[3] >> 15) & 1;		//????
			uint32_t MASA = (IPtr[3] >> 9) & 0x3f;	//???
			uint32_t ADREB = (IPtr[3] >> 8) & 0x1;
			uint32_t NXADR = (IPtr[3] >> 7) & 0x1;

			uint32_t ADDR = DSPData->MADRS[MASA];
			if (ADREB)
				ADDR += ADRS_REG & 0x0FFF;
			if (NXADR)
				ADDR++;
			if (!TABLE)
			{
				ADDR += MDEC_CT;
				ADDR &= GetRBL() - 1;		// RBL is ring buffer length
			}
			else
				ADDR &= 0xFFFF;

			ADDR <<= 1;					// Word -> byte address
			ADDR += GetRBP();			// RBP is already a byte address
			if (MRD)			// memory only allowed on odd. DoA inserts NOPs on even
			{
				if (NOFL)
					MEMVAL[(step + 2) & 3] = (*(int16_t *)&aica_ram[ADDR & aram_mask]) << 8;
				else
					MEMVAL[(step + 2) & 3] = UNPACK(*(uint16_t*)&aica_ram[ADDR & aram_mask]);
			}
			if (MWT)
			{
				// FIXME We should wait for the next step to copy stuff to SRAM (same as read)
				if (NOFL)
					*(int16_t *)&aica_ram[ADDR & aram_mask] = SHIFTED >> 8;
				else
					*(uint16_t*)&aica_ram[ADDR & aram_mask] = PACK(SHIFTED);
			}
		}
	}

	if (ADRL)
	{
		if (SHIFT == 3)
			ADRS_REG = (SHIFTED >> 12) & 0xFFF;
		else
			ADRS_REG = (INPUTS >> 16);
	}

	if (EWT)
	{
		uint32_t EWA = (IPtr[2] >> 8) & 0x0F;
		// 4 ????
		DSPData->EFREG[EWA] += SHIFTED >> 4;	// dynarec uses = instead of +=
	}
}

extern "C" EMSCRIPTEN_KEEPALIVE void Step128Start()
{
	memset(DSPData->EFREG, 0, sizeof(DSPData->EFREG));
}


extern "C" EMSCRIPTEN_KEEPALIVE void Step128SEnd()
{
	--MDEC_CT;
	if (MDEC_CT == 0)
		MDEC_CT = GetRBL();			// RBL is ring buffer length - 1
}

extern "C" EMSCRIPTEN_KEEPALIVE void Step128()
{
	Step128Start();
	for (int step = 0; step < 128; ++step)
	{
		Step(step);
	}
	Step128SEnd();
}
