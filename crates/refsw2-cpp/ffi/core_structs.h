/*
	This file is part of libswirl
*/
// #include "license/bsd"


//structs were getting tooo many , so i moved em here !

#pragma once
#include <stdint.h>

//bits that affect drawing (for caching params)
#define PCW_DRAW_MASK (0x000000CE)

#pragma pack(push, 1)   // n = 1
//	Global Param/misc structs
//4B
union PCW
{
	struct
	{
		//Obj Control        //affects drawing ?
		uint32_t UV_16bit    : 1; //0
		uint32_t Gouraud     : 1; //1
		uint32_t Offset      : 1; //1
		uint32_t Texture     : 1; //1
		uint32_t Col_Type    : 2; //00
		uint32_t Volume      : 1; //1
		uint32_t Shadow      : 1; //1

		uint32_t Reserved    : 8; //0000 0000

		// Group Control
		uint32_t User_Clip   : 2;
		uint32_t Strip_Len   : 2;
		uint32_t Res_2       : 3;
		uint32_t Group_En    : 1;

		// Para Control
		uint32_t ListType    : 3;
		uint32_t Res_1       : 1;
		uint32_t EndOfStrip  : 1;
		uint32_t ParaType    : 3;
	};
	uint8_t obj_ctrl;
	struct
	{
		uint32_t padin  : 8;
		uint32_t S6X    : 1;    //set by TA preprocessing if sz64
		uint32_t padin2 : 19;
		uint32_t PTEOS  : 4;
	};
	uint32_t full;
};


//// ISP/TSP Instruction Word

union ISP_TSP
{
	struct
	{
		uint32_t Reserved    : 20;
		uint32_t DCalcCtrl   : 1;
		uint32_t CacheBypass : 1;
		uint32_t UV_16b      : 1; //In TA they are replaced
		uint32_t Gouraud     : 1; //by the ones on PCW
		uint32_t Offset      : 1; //
		uint32_t Texture     : 1; // -- up to here --
		uint32_t ZWriteDis   : 1;
		uint32_t CullMode    : 2;
		uint32_t DepthMode   : 3;
	};
	struct
	{
		uint32_t res        : 27;
		uint32_t CullMode   : 2;
		uint32_t VolumeMode : 3;	// 0 normal polygon, 1 inside last, 2 outside last
	} modvol;
	uint32_t full;
};

union ISP_Modvol
{
	struct
	{
		uint32_t id         : 26;
		uint32_t VolumeLast : 1;
		uint32_t CullMode   : 2;
		uint32_t DepthMode  : 3;
	};
	uint32_t full;
};


//// END ISP/TSP Instruction Word


//// TSP Instruction Word

union TSP
{
	struct 
	{
		uint32_t TexV        : 3;
		uint32_t TexU        : 3;
		uint32_t ShadInstr   : 2;
		uint32_t MipMapD     : 4;
		uint32_t SupSample   : 1;
		uint32_t FilterMode  : 2;
		uint32_t ClampV      : 1;
		uint32_t ClampU      : 1;
		uint32_t FlipV       : 1;
		uint32_t FlipU       : 1;
		uint32_t IgnoreTexA  : 1;
		uint32_t UseAlpha    : 1;
		uint32_t ColorClamp  : 1;
		uint32_t FogCtrl     : 2;
		uint32_t DstSelect   : 1; // Secondary Accum
		uint32_t SrcSelect   : 1; // Primary Accum
		uint32_t DstInstr    : 3;
		uint32_t SrcInstr    : 3;
	};
	uint32_t full;
} ;


//// END TSP Instruction Word


/// Texture Control Word
union TCW
{
	struct
	{
		uint32_t TexAddr   :21;
		uint32_t Reserved  : 4;
		uint32_t StrideSel : 1;
		uint32_t ScanOrder : 1;
		uint32_t PixelFmt  : 3;
		uint32_t VQ_Comp   : 1;
		uint32_t MipMapped : 1;
	} ;
	struct
	{
		uint32_t pading_0  :21;
		uint32_t PalSelect : 6;
	} ;
	uint32_t full;
};

/// END Texture Control Word
#pragma pack(pop) 

//generic vertex storage type
struct Vertex
{
	float x,y,z;

	uint8_t col[4];
	uint8_t spc[4];

	float u,v;

	// Two volumes format
	uint8_t col1[4];
	uint8_t spc1[4];

	float u1,v1;
};

enum PixelFormat
{
	Pixel1555 = 0,
	Pixel565 = 1,
	Pixel4444 = 2,
	PixelYUV = 3,
	PixelBumpMap = 4,
	PixelPal4 = 5,
	PixelPal8 = 6,
	PixelReserved = 7
};
