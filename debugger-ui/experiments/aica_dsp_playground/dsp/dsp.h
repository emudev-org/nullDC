/*
	This file is part of libswirl
*/

#pragma once

#if defined(__EMSCRIPTEN__)
#include "emscripten.h"
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

#include <cstdint>

extern uint8_t aica_ram[];
extern uint32_t aram_mask;
extern uint8_t aica_reg[];

#define entry(name,sz) uint32_t name:sz;

#pragma pack(push, 1)
struct CommonData_struct
{
	//+0
	entry(MVOL,4);
	entry(VER,4);
	entry(DAC18B,1);
	entry(MEM8MB,1);
	entry(pad0_0,5);
	entry(Mono,1);
	
	uint32_t :16;
	//+4
	entry(RBP,12);
	entry(pad1_0,1);
	entry(RBL,2);
	entry(TESTB0,1);

	uint32_t :16;
	//+8
	entry(MIBUF,8);
	entry(MIEMP,1);
	entry(MIFUL ,1);
	entry(MIOVF ,1);
	entry(MOEMP ,1);
	entry(MOFUL ,1);
	entry(pad3_0,3);

	uint32_t :16;
	//+C
	entry(MOBUF,8);
	entry(MSLC,6);
	entry(AFSET,1);
	entry(padC_0,1);

	uint32_t :16;
	//+10
	entry(EG,13);
	entry(SGC,2);
	entry(LP,1);
	
	uint32_t :16;
	//+14
	entry(CA,16);

	uint32_t :16;
	
	//quite a bit padding here :)
	uint8_t pad_med_0[0x6C-4];

	//+80
	entry(MRWINH,4);
	entry($T,1);
	entry($TSCD,3);
	entry(pad80_0,1);
	entry(DMEA_hi,7);

	uint32_t :16;
	//+84
	entry(pad84_0,2);
	entry(DMEA_lo,14);

	uint32_t :16;
	//+88
	entry(pad88_0,2);
	entry(DRGA,13);
	entry(DGATE,1);

	uint32_t :16;
	//+8C
	entry(DEXE,1);
	entry(pad8C_0,1);
	entry(DLG,13);
	entry(DDIR,1);

	uint32_t :16;
	//+90
	entry(TIMA,8);
	entry(TACTL,3);
	entry(pad90_0,5);

	uint32_t :16;
	//+94
	entry(TIMB,8);
	entry(TBCTL,3);
	entry(pad94_0,5);

	uint32_t :16;
	//+98
	entry(TIMC,8);
	entry(TCCTL,3);
	entry(pad98_0,5);

	uint32_t :16;

	//+9C
	entry(SCIEB,11);
	entry(pad9C_0,5);

	uint32_t :16;

	//+A0
	entry(SCIPD,11);
	entry(padA0_0,5);

	uint32_t :16;

	//+A4
	entry(SCIRE,11);
	entry(padA4_0,5);

	uint32_t :16;

	//+A8
	entry(SCILV0,8);
	entry(padA8_0,8);

	uint32_t :16;

	//+AC
	entry(SCILV1,8);
	entry(padAC_0,8);

	uint32_t :16;

	//+B0
	entry(SCILV2,8);
	entry(padB0_0,8);

	uint32_t :16;

	//+B4
	entry(MCIEB,11);
	entry(padB4_0,5)

	uint32_t :16;

	//+B8
	entry(MCIPD,11);
	entry(padB8_0,5)

	uint32_t :16;

	//+BC
	entry(MCIRE,11);
	entry(padBC_0,5)

	uint32_t :16;
	
	//some other misc shit FAR away is here :p
	uint8_t pad_lot_0[0x344-4];

	//+400 , hopefully :p
	entry(AR,1);
	entry(pad400_0,7);
	entry(VREG,2);
	entry(pad400_1,6);

	uint32_t :16;

	//Even more
	uint8_t pad_lot_1[0x100-4];

	//+500 , hopefully :p
	entry(L0_r,1);
	entry(L1_r,1);
	entry(L2_r,1);
	entry(L3_r,1);
	entry(L4_r,1);
	entry(L5_r,1);
	entry(L6_r,1);
	entry(L7_r,1);
	
	entry(pad500_0,8);

	uint32_t :16;

	//+504

	entry(M0_r,1);
	entry(M1_r,1);
	entry(M2_r,1);
	entry(M3_r,1);
	entry(M4_r,1);
	entry(M5_r,1);
	entry(M6_r,1);
	entry(M7_r,1);
	entry(RP,1);
	
	entry(pad504_0,7);

	uint32_t :16;
};


//should be 0x15C8 in size
struct DSPData_struct
{
	//+0x000
	uint32_t COEF[128];		//15:3

	//+0x200
	uint32_t MADRS[64];		//15:0
	
	//+0x300
	uint8_t PAD0[0x100];

	//+0x400
	uint32_t MPRO[128*4];	//15:0
	
	//+0xC00
	uint8_t PAD1[0x400];

	//+0x1000
	struct 
	{ 
		uint32_t l;			//7:0
		uint32_t h;			//15:0 (23:8)
	} 
	TEMP[128];

	//+0x1400
	struct 
	{ 
		uint32_t l;			//7:0
		uint32_t h;			//15:0 (23:8)
	} 
	MEMS[32];
	
	//+0x1500
	struct 
	{ 
		uint32_t l;			//3:0
		uint32_t h;			//15:0 (19:4)
	} 
	MIXS[16];

	//+0x1580
	uint32_t EFREG[16];		//15:0
	
	//+0x15C0
	uint32_t EXTS[2];		//15:0
};

static_assert(sizeof(DSPData_struct) == 0x15C8);
#pragma pack(pop)
#if 0
struct dsp_context_t
{
	//buffered DSP state
	//24 bit wide
	int32_t TEMP[128];
	//24 bit wide
	int32_t MEMS[32];
	//20 bit wide
	int32_t MIXS[16];

	//RBL/RBP (decoded)
	uint32_t RBP;
	uint32_t RBL;

	struct
	{
		bool MAD_OUT;
		bool MEM_ADDR;
		bool MEM_RD_DATA;
		bool MEM_WT_DATA;
		bool FRC_REG;
		bool ADRS_REG;
		bool Y_REG;

		bool MDEC_CT;
		bool MWT_1;
		bool MRD_1;
		//bool MADRS;
		bool MEMS;
		bool NOFL_1;
		bool NOFL_2;

		bool TEMPS;
		bool EFREG;
	}regs_init;

	//int32_t -> stored as signed extended to 32 bits
	struct
	{
		int32_t MAD_OUT;
		int32_t MEM_ADDR;
		int32_t MEM_RD_DATA;
		int32_t MEM_WT_DATA;
		int32_t FRC_REG;
		int32_t ADRS_REG;
		int32_t Y_REG;

		uint32_t MDEC_CT;
		uint32_t MWT_1;
		uint32_t MRD_1;
		uint32_t MADRS;
		uint32_t NOFL_1;
		uint32_t NOFL_2;
	}regs;
	//DEC counter :)
	//uint32_t DEC;

	//various dsp regs
	signed int ACC;        //26 bit
	signed int SHIFTED;    //24 bit
	signed int B;          //26 bit
	signed int MEMVAL[4];
	signed int FRC_REG;    //13 bit
	signed int Y_REG;      //24 bit
	unsigned int ADDR;
	unsigned int ADRS_REG; //13 bit

	//Direct Mapped data :
	//COEF  *128
	//MADRS *64
	//MPRO(dsp code) *4 *128
	//EFREG *16
	//EXTS  *2

	// Interpreter flags
	bool Stopped;

	//Dynarec flags
	bool dyndirty;
};
#endif

struct _INST
{
	unsigned int TRA;
	unsigned int TWT;
	unsigned int TWA;

	unsigned int XSEL;
	unsigned int YSEL;
	unsigned int IRA;
	unsigned int IWT;
	unsigned int IWA;

	unsigned int EWT;
	unsigned int EWA;
	unsigned int ADRL;
	unsigned int FRCL;
	unsigned int SHIFT;
	unsigned int YRL;
	unsigned int NEGB;
	unsigned int ZERO;
	unsigned int BSEL;

	unsigned int NOFL;  //MRQ set
	unsigned int TABLE; //MRQ set
	unsigned int MWT;   //MRQ set
	unsigned int MRD;   //MRQ set
	unsigned int MASA;  //MRQ set
	unsigned int ADREB; //MRQ set
	unsigned int NXADR; //MRQ set
};


uint16_t PACK(int32_t val);
int32_t UNPACK(uint16_t val);
void DecodeInst(uint32_t* IPtr, _INST* i);
void EncodeInst(uint32_t* IPtr, _INST* i);

extern "C" void Step(int step);
extern "C" void Step128();
extern "C" uint32_t ReadReg(uint32_t addr);
extern "C" void WriteReg(uint32_t addr, uint32_t data);