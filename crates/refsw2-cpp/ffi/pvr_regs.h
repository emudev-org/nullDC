/*
	This file is part of libswirl
*/
// #include "license/bsd"


#pragma once
#include <stdint.h>

#define pvr_RegSize (0x8000)
#define pvr_RegMask (pvr_RegSize-1)

#define PvrReg(x,t) (*(t*)&emu_regs[(x/4) & pvr_RegMask])

extern const uint32_t* emu_regs;

#define ID_addr                 0x00000000 // R   Device ID
#define REVISION_addr           0x00000004 // R   Revision number
#define SOFTRESET_addr          0x00000008 // RW  CORE & TA software reset
	
#define STARTRENDER_addr        0x00000014 // RW  Drawing start
#define TEST_SELECT_addr        0x00000018 // RW  Test (writing this register is prohibited)

#define PARAM_BASE_addr         0x00000020 // RW  Base address for ISP parameters

#define REGION_BASE_addr        0x0000002C // RW  Base address for Region Array
#define SPAN_SORT_CFG_addr      0x00000030 // RW  Span Sorter control

#define VO_BORDER_COL_addr      0x00000040 // RW  Border area color
#define FB_R_CTRL_addr          0x00000044 // RW  Frame buffer read control
#define FB_W_CTRL_addr          0x00000048 // RW  Frame buffer write control
#define FB_W_LINESTRIDE_addr    0x0000004C // RW  Frame buffer line stride
#define FB_R_SOF1_addr          0x00000050 // RW  Read start address for field - 1/strip - 1
#define FB_R_SOF2_addr          0x00000054 // RW  Read start address for field - 2/strip - 2

#define FB_R_SIZE_addr          0x0000005C // RW  Frame buffer XY size	
#define FB_W_SOF1_addr          0x00000060 // RW  Write start address for field - 1/strip - 1
#define FB_W_SOF2_addr          0x00000064 // RW  Write start address for field - 2/strip - 2
#define FB_X_CLIP_addr          0x00000068 // RW  Pixel clip X coordinate
#define FB_Y_CLIP_addr          0x0000006C // RW  Pixel clip Y coordinate


#define FPU_SHAD_SCALE_addr     0x00000074 // RW  Intensity Volume mode
#define FPU_CULL_VAL_addr       0x00000078 // RW  Comparison value for culling
#define FPU_PARAM_CFG_addr      0x0000007C // RW  Parameter read control
#define HALF_OFFSET_addr        0x00000080 // RW  Pixel sampling control
#define FPU_PERP_VAL_addr       0x00000084 // RW  Comparison value for perpendicular polygons
#define ISP_BACKGND_D_addr      0x00000088 // RW  Background surface depth
#define ISP_BACKGND_T_addr      0x0000008C // RW  Background surface tag

#define ISP_FEED_CFG_addr       0x00000098 // RW  Translucent polygon sort mode

#define SDRAM_REFRESH_addr      0x000000A0 // RW  Texture memory refresh counter
#define SDRAM_ARB_CFG_addr      0x000000A4 // RW  Texture memory arbiter control
#define SDRAM_CFG_addr          0x000000A8 // RW  Texture memory control

#define FOG_COL_RAM_addr        0x000000B0 // RW  Color for Look Up table Fog
#define FOG_COL_VERT_addr       0x000000B4 // RW  Color for vertex Fog
#define FOG_DENSITY_addr        0x000000B8 // RW  Fog scale value
#define FOG_CLAMP_MAX_addr      0x000000BC // RW  Color clamping maximum value
#define FOG_CLAMP_MIN_addr      0x000000C0 // RW  Color clamping minimum value
#define SPG_TRIGGER_POS_addr    0x000000C4 // RW  External trigger signal HV counter value
#define SPG_HBLANK_INT_addr     0x000000C8 // RW  H-blank interrupt control	
#define SPG_VBLANK_INT_addr     0x000000CC // RW  V-blank interrupt control	
#define SPG_CONTROL_addr        0x000000D0 // RW  Sync pulse generator control
#define SPG_HBLANK_addr         0x000000D4 // RW  H-blank control
#define SPG_LOAD_addr           0x000000D8 // RW  HV counter load value
#define SPG_VBLANK_addr         0x000000DC // RW  V-blank control
#define SPG_WIDTH_addr          0x000000E0 // RW  Sync width control
#define TEXT_CONTROL_addr       0x000000E4 // RW  Texturing control
#define VO_CONTROL_addr         0x000000E8 // RW  Video output control
#define VO_STARTX_addr          0x000000Ec // RW  Video output start X position
#define VO_STARTY_addr          0x000000F0 // RW  Video output start Y position
#define SCALER_CTL_addr         0x000000F4 // RW  X & Y scaler control
#define PAL_RAM_CTRL_addr       0x00000108 // RW  Palette RAM control
#define SPG_STATUS_addr         0x0000010C // R   Sync pulse generator status
#define FB_BURSTCTRL_addr       0x00000110 // RW  Frame buffer burst control
#define FB_C_SOF_addr           0x00000114 // R   Current frame buffer start address
#define Y_COEFF_addr            0x00000118 // RW  Y scaling coefficient

#define PT_ALPHA_REF_addr       0x0000011C // RW  Alpha value for Punch Through polygon comparison

union FB_R_CTRL_type
{
	struct
	{
		uint32_t fb_enable           : 1; //0
		uint32_t fb_line_double      : 1; //1
		uint32_t fb_depth            : 2; //3-2
		uint32_t fb_concat           : 3; //6-4
		uint32_t R                   : 1; //7
		uint32_t fb_chroma_threshold : 8; //15-8
		uint32_t fb_stripsize        : 6; //21-16
		uint32_t fb_strip_buf_en     : 1; //22
		uint32_t vclk_div            : 1; //23
		uint32_t Reserved            : 8; //31-24
	};
	uint32_t full;
};
enum fb_depth_enum
{
	fbde_0555 = 0, //0555, lower 3 bits on fb_concat
	fbde_565  = 1, //565, lower 3 bits on fb_concat, [1:0] for G
	fbde_888  = 2, //888, packed
	fbde_C888 = 3, //C888, first byte used for chroma
};
union FB_R_SIZE_type
{
	struct
	{
		uint32_t fb_x_size  : 10; //0
		uint32_t fb_y_size  : 10; //10
		uint32_t fb_modulus : 10; //20
		uint32_t fb_res     : 2; //30
	};
	uint32_t full;
};
union VO_BORDER_COL_type
{
	struct
	{
		uint32_t Blue   : 8; //0
		uint32_t Green  : 8; //8
		uint32_t Red    : 8; //16
		uint32_t Chroma : 1; //24
		uint32_t res    : 7; //25
	};
	uint32_t full;
};


union SPG_STATUS_type
{
	struct
	{
		uint32_t scanline : 10; //9-0
		uint32_t fieldnum : 1;  //10
		uint32_t blank    : 1;  //11
		uint32_t hsync    : 1;  //12
		uint32_t vsync    : 1;  //13
		uint32_t res      : 18; //31-14
	};
	uint32_t full;
};

union SPG_HBLANK_INT_type
{
	struct
	{
		uint32_t line_comp_val : 10;       //9-0
		uint32_t res1 : 2;                 //10-11
		uint32_t hblank_int_mode: 2;       //12-13
		uint32_t res2 : 2;                 //14-15
		uint32_t hblank_in_interrupt : 10; //25-16
		uint32_t res3 : 6;                 //31-26
	};
	uint32_t full;
};

union SPG_VBLANK_INT_type
{
	struct
	{
		uint32_t vblank_in_interrupt_line_number : 10;//9-0
		uint32_t res : 6 ; //15-10
		uint32_t vblank_out_interrupt_line_number : 10;//25-16
		uint32_t res1 : 6 ; //31-26
	};
	uint32_t full;
};
union SPG_CONTROL_type
{
	struct
	{
		uint32_t mhsync_pol     : 1;  //0
		uint32_t mvsync_pol     : 1;  //1
		uint32_t mcsync_pol     : 1;  //2
		uint32_t spg_lock       : 1;  //3
		uint32_t interlace      : 1;  //4
		uint32_t force_field2   : 1;  //5
		uint32_t NTSC           : 1;  //6
		uint32_t PAL            : 1;  //7
		uint32_t sync_direction : 1;  //8
		uint32_t csync_on_h     : 1;  //9
		uint32_t Reserved       : 22; //31-10
	};
	uint32_t full;
};
union SPG_HBLANK_type
{
	struct
	{
		uint32_t hstart : 10;//9-0
		uint32_t res    : 6; //15-10
		uint32_t hbend  : 10;//25-16
		uint32_t res1   : 6; //31-26
	};
	uint32_t full;
};

union SPG_LOAD_type
{
	struct
	{
		uint32_t hcount : 10; //9-0
		uint32_t res    : 6 ; //15-10	
		uint32_t vcount : 10; //25-16
		uint32_t res1   : 6 ; //31-26
	};
	uint32_t full;
};

union SPG_VBLANK_type
{
	struct
	{
		uint32_t vstart : 10; //9-0
		uint32_t res    : 6 ; //15-10	
		uint32_t vbend  : 10; //25-16
		uint32_t res1   : 6 ; //31-26
	};
	uint32_t full;
};

union SPG_WIDTH_type
{
	struct
	{
		uint32_t hswidth : 7;  //6-0
		uint32_t res     : 1;  //7-7
		uint32_t vswidth : 4;  //8-11
		uint32_t bpwidth : 10; //21-12
		uint32_t eqwidth : 10; //31-22
	};
	uint32_t full;
};

union SCALER_CTL_type
{
	struct
	{
		uint32_t vscalefactor : 16;//15-0
		uint32_t hscale       : 1; //16-16
		uint32_t interlace    : 1; //17-17
		uint32_t fieldselect  : 1; //18-18
	};
	uint32_t full;
};

union FB_X_CLIP_type
{
	struct
	{
		uint32_t min  : 11;
		uint32_t pad1 : 5;
		uint32_t max  : 11;
		uint32_t pad  : 5;
	};
	uint32_t full;
};

union FB_Y_CLIP_type
{
	struct
	{
		uint32_t min  : 10; //15-0
		uint32_t pad1 : 6 ; //16-16
		uint32_t max  : 10; //17-17
		uint32_t pad  : 6;  //18-18
	};
	uint32_t full;
};

union VO_CONTROL_type
{
	struct
	{
		uint32_t hsync_pol    : 1;  //0
		uint32_t vsync_pol    : 1;  //1
		uint32_t blank_pol    : 1;  //2
		uint32_t blank_video  : 1;  //3
		uint32_t field_mode   : 4;  //4
		uint32_t pixel_double : 1;  //8
		uint32_t res_1        : 7;  //9
		uint32_t pclk_delay   : 6;  //16
		uint32_t res_2        : 10; //22
	};
	uint32_t full;
};

union VO_STARTX_type
{
	struct
	{
		uint32_t HStart : 10; //0
		uint32_t res_1  : 22; //10
	};
	uint32_t full;
};
union VO_STARTY_type
{
	struct
	{
		uint32_t VStart_field1:10; //0
		uint32_t res_1:6;          //10
		uint32_t VStart_field2:10; //16
		uint32_t res_2:6;          //26
	};
	uint32_t full;
};

union ISP_BACKGND_D_type
{
	uint32_t i;
	float f;
};

union ISP_BACKGND_T_type
{
	struct
	{
		uint32_t tag_offset   : 3;
		uint32_t param_offs_in_words  : 21;
		uint32_t skip         : 3;
		uint32_t shadow       : 1;
		uint32_t cache_bypass : 1;
	};
	uint32_t full;
};

union ISP_FEED_CFG_type
{
	struct
	{
		uint32_t pre_sort : 1;
		uint32_t res : 2;
		uint32_t discard_mode : 1;
		uint32_t pt_chunk_size : 10;
		uint32_t tr_cache_size : 10;
		uint32_t res2 : 8;
	};
	uint32_t full;
};

union FB_W_CTRL_type
{
	struct
	{
		uint32_t fb_packmode        : 3;
		uint32_t fb_dither          : 1;
		uint32_t pad0               : 4;
		uint32_t fb_kval            : 8;
		uint32_t fb_alpha_threshold : 8;
		uint32_t pad1               : 8;
	};
	uint32_t full;
};

union FB_W_LINESTRIDE_type
{
	struct
	{
		uint32_t stride : 9;
		uint32_t pad0   : 23;
	};
	uint32_t full;
};

union FPU_SHAD_SCALE_type
{
	struct
	{
		uint32_t scale_factor    : 8;
		uint32_t intensity_shadow : 1;
	};
	uint32_t full;
};

union FPU_PARAM_CFG_type
{
	struct
	{
		uint32_t pointer_first_burst : 4;
		uint32_t pointer_burst : 4;
		uint32_t isp_param_burst_threshold : 6;
		uint32_t tsp_param_burst_threshold : 6;
		uint32_t res : 1;
		uint32_t region_header_type : 1;
		uint32_t res1 : 10;
	};
	uint32_t full;
};

union HALF_OFFSET_type
{
	struct
	{
		uint32_t fpu_pixel_half_offset : 1;
		uint32_t tsp_pixel_half_offset : 1;
		uint32_t texure_pixel_half_offset : 1;
	};
	uint32_t full;
};

union TA_GLOB_TILE_CLIP_type
{
	struct
	{
		uint32_t tile_x_num	: 6;
		uint32_t reserved 	: 10;
		uint32_t tile_y_num  : 4;
		uint32_t reserved2	: 12;
	};
	uint32_t full;
};
 
union TA_YUV_TEX_CTRL_type
{
	struct
	{
		uint32_t yuv_u_size	: 6;
		uint32_t reserved1	: 2;
		uint32_t yuv_v_size	: 6;
		uint32_t reserved2	: 2;
		uint32_t yuv_tex		: 1;
		uint32_t reserved3	: 7;
		uint32_t yuv_form	: 1;
		uint32_t reserved4	: 7;
	};
	uint32_t full;
};

// TA REGS
#define TA_OL_BASE_addr         0x00000124 // RW  Object list write start address
#define TA_ISP_BASE_addr        0x00000128 // RW  ISP/TSP Parameter write start address
#define TA_OL_LIMIT_addr        0x0000012C // RW  Start address of next Object Pointer Block
#define TA_ISP_LIMIT_addr       0x00000130 // RW  Current ISP/TSP Parameter write address
#define TA_NEXT_OPB_addr        0x00000134 // R   Global Tile clip control
#define TA_ISP_CURRENT_addr     0x00000138 // R   Current ISP/TSP Parameter write address
#define TA_GLOB_TILE_CLIP_addr  0x0000013C // RW  Global Tile clip control
#define TA_ALLOC_CTRL_addr      0x00000140 // RW  Object list control
#define TA_LIST_INIT_addr       0x00000144 // RW  TA initialization
#define TA_YUV_TEX_BASE_addr    0x00000148 // RW  YUV422 texture write start address
#define TA_YUV_TEX_CTRL_addr    0x0000014C // RW  YUV converter control
#define TA_YUV_TEX_CNT_addr     0x00000150 // R   YUV converter macro block counter value

#define TA_LIST_CONT_addr       0x00000160 // RW  TA continuation processing
#define TA_NEXT_OPB_INIT_addr   0x00000164 // RW  Additional OPB starting address



#define FOG_TABLE_START_addr        0x00000200 // RW  Look-up table Fog data
#define FOG_TABLE_END_addr          0x000003FC

#define TA_OL_POINTERS_START_addr   0x00000600 // R   TA object List Pointer data
#define TA_OL_POINTERS_END_addr     0x00000F5C

#define PALETTE_RAM_START_addr      0x00001000 // RW  Palette RAM
#define PALETTE_RAM_END_addr        0x00001FFC



// Regs -- Start

#define ID                PvrReg(ID_addr,uint32_t)	         // R   Device ID
#define REVISION          PvrReg(REVISION_addr,uint32_t)      // R   Revision number
#define SOFTRESET         PvrReg(SOFTRESET_addr,uint32_t)     // RW  CORE & TA software reset
	
#define STARTRENDER       PvrReg(STARTRENDER_addr,uint32_t)   // RW  Drawing start
#define TEST_SELECT       PvrReg(TEST_SELECT_addr,uint32_t)   // RW  Test (writing this register is prohibited)

#define PARAM_BASE        PvrReg(PARAM_BASE_addr,uint32_t)    // RW  Base address for ISP parameters

#define REGION_BASE       PvrReg(REGION_BASE_addr,uint32_t)   // RW  Base address for Region Array
#define SPAN_SORT_CFG     PvrReg(SPAN_SORT_CFG_addr,uint32_t) // RW  Span Sorter control

#define VO_BORDER_COL     PvrReg(VO_BORDER_COL_addr,VO_BORDER_COL_type)     // RW  Border area color
#define FB_R_CTRL         PvrReg(FB_R_CTRL_addr,FB_R_CTRL_type)             // RW  Frame buffer read control
#define FB_W_CTRL         PvrReg(FB_W_CTRL_addr,FB_W_CTRL_type)             // RW  Frame buffer write control
#define FB_W_LINESTRIDE   PvrReg(FB_W_LINESTRIDE_addr,FB_W_LINESTRIDE_type) // RW  Frame buffer line stride
#define FB_R_SOF1         PvrReg(FB_R_SOF1_addr,uint32_t)                      // RW  Read start address for field - 1/strip - 1
#define FB_R_SOF2         PvrReg(FB_R_SOF2_addr,uint32_t)                      // RW  Read start address for field - 2/strip - 2

#define FB_R_SIZE         PvrReg(FB_R_SIZE_addr,FB_R_SIZE_type)           // RW  Frame buffer XY size
#define FB_W_SOF1         PvrReg(FB_W_SOF1_addr,uint32_t)                      // RW  Write start address for field - 1/strip - 1
#define FB_W_SOF2         PvrReg(FB_W_SOF2_addr,uint32_t)                      // RW  Write start address for field - 2/strip - 2
#define FB_X_CLIP         PvrReg(FB_X_CLIP_addr,FB_X_CLIP_type)           // RW  Pixel clip X coordinate
#define FB_Y_CLIP         PvrReg(FB_Y_CLIP_addr,FB_Y_CLIP_type)           // RW  Pixel clip Y coordinate


#define FPU_SHAD_SCALE    PvrReg(FPU_SHAD_SCALE_addr,FPU_SHAD_SCALE_type) // RW  Intensity Volume mode
#define FPU_CULL_VAL      PvrReg(FPU_CULL_VAL_addr,float)                   // RW  Comparison value for culling
#define FPU_PARAM_CFG     PvrReg(FPU_PARAM_CFG_addr,FPU_PARAM_CFG_type)                  // RW  Parameter read control
#define HALF_OFFSET       PvrReg(HALF_OFFSET_addr,HALF_OFFSET_type)                    // RW  Pixel sampling control
#define FPU_PERP_VAL      PvrReg(FPU_PERP_VAL_addr,uint32_t)                   // RW  Comparison value for perpendicular polygons
#define ISP_BACKGND_D     PvrReg(ISP_BACKGND_D_addr,ISP_BACKGND_D_type)   // RW  Background surface depth
#define ISP_BACKGND_T     PvrReg(ISP_BACKGND_T_addr,ISP_BACKGND_T_type)   // RW  Background surface tag

#define ISP_FEED_CFG      PvrReg(ISP_FEED_CFG_addr,ISP_FEED_CFG_type)                   // RW  Translucent polygon sort mode

#define SDRAM_REFRESH     PvrReg(SDRAM_REFRESH_addr,uint32_t)                  // RW  Texture memory refresh counter
#define SDRAM_ARB_CFG     PvrReg(SDRAM_ARB_CFG_addr,uint32_t)                  // RW  Texture memory arbiter control
#define SDRAM_CFG         PvrReg(SDRAM_CFG_addr,uint32_t)                      // RW  Texture memory control

#define FOG_COL_RAM       PvrReg(FOG_COL_RAM_addr,uint32_t)                    // RW  Color for Look Up table Fog
#define FOG_COL_VERT      PvrReg(FOG_COL_VERT_addr,uint32_t)                   // RW  Color for vertex Fog
#define FOG_DENSITY       PvrReg(FOG_DENSITY_addr,uint32_t)                    // RW  Fog scale value
#define FOG_CLAMP_MAX     PvrReg(FOG_CLAMP_MAX_addr,uint32_t)                  // RW  Color clamping maximum value
#define FOG_CLAMP_MIN     PvrReg(FOG_CLAMP_MIN_addr,uint32_t)                  // RW  Color clamping minimum value
#define SPG_TRIGGER_POS   PvrReg(SPG_TRIGGER_POS_addr,uint32_t)                // RW  External trigger signal HV counter value
#define SPG_HBLANK_INT    PvrReg(SPG_HBLANK_INT_addr,SPG_HBLANK_INT_type) // RW  H-blank interrupt control
#define SPG_VBLANK_INT    PvrReg(SPG_VBLANK_INT_addr,SPG_VBLANK_INT_type) // RW  V-blank interrupt control
#define SPG_CONTROL       PvrReg(SPG_CONTROL_addr,SPG_CONTROL_type)       // RW  pulse generator control
#define SPG_HBLANK        PvrReg(SPG_HBLANK_addr,SPG_HBLANK_type)         // RW  H-blank control
#define SPG_LOAD          PvrReg(SPG_LOAD_addr,SPG_LOAD_type)             // RW  HV counter load value
#define SPG_VBLANK        PvrReg(SPG_VBLANK_addr,SPG_VBLANK_type)         // RW  V-blank control
#define SPG_WIDTH         PvrReg(SPG_WIDTH_addr,SPG_WIDTH_type)           // RW  Sync width control
#define TEXT_CONTROL      PvrReg(TEXT_CONTROL_addr,uint32_t)                   // RW  Texturing control
#define VO_CONTROL        PvrReg(VO_CONTROL_addr,VO_CONTROL_type)         // RW  Video output control
#define VO_STARTX         PvrReg(VO_STARTX_addr,VO_STARTX_type)           // RW  Video output start X position
#define VO_STARTY         PvrReg(VO_STARTY_addr,VO_STARTY_type)           // RW  Video output start Y position
#define SCALER_CTL        PvrReg(SCALER_CTL_addr,SCALER_CTL_type)         // RW  X & Y scaler control
#define PAL_RAM_CTRL      PvrReg(PAL_RAM_CTRL_addr,uint32_t)                   // RW  Palette RAM control
#define SPG_STATUS        PvrReg(SPG_STATUS_addr,SPG_STATUS_type)         // R   Sync pulse generator status
#define FB_BURSTCTRL      PvrReg(FB_BURSTCTRL_addr,uint32_t)                   // RW  Frame buffer burst control
#define FB_C_SOF          PvrReg(FB_C_SOF_addr,uint32_t)                       // R   Current frame buffer start address
#define Y_COEFF           PvrReg(Y_COEFF_addr,uint32_t)                        // RW  Y scaling coefficient

#define PT_ALPHA_REF      PvrReg(PT_ALPHA_REF_addr,uint32_t)                   // RW  Alpha value for Punch Through polygon comparison



//	TA REGS
#define TA_OL_BASE        PvrReg(TA_OL_BASE_addr,uint32_t)        // RW Object list write start address
#define TA_ISP_BASE       PvrReg(TA_ISP_BASE_addr,uint32_t)       // RW ISP/TSP Parameter write start address
#define TA_OL_LIMIT       PvrReg(TA_OL_LIMIT_addr,uint32_t)       // RW Start address of next Object Pointer Block
#define TA_ISP_LIMIT      PvrReg(TA_ISP_LIMIT_addr,uint32_t)      // RW Current ISP/TSP Parameter write address
#define TA_NEXT_OPB       PvrReg(TA_NEXT_OPB_addr,uint32_t)       // R  Global Tile clip control
#define TA_ISP_CURRENT    PvrReg(TA_ISP_CURRENT_addr,uint32_t)    // R  Current ISP/TSP Parameter write address
#define TA_GLOB_TILE_CLIP PvrReg(TA_GLOB_TILE_CLIP_addr, TA_GLOB_TILE_CLIP_type) // RW Global Tile clip control
#define TA_ALLOC_CTRL     PvrReg(TA_ALLOC_CTRL_addr,uint32_t)     // RW Object list control
#define TA_LIST_INIT      PvrReg(TA_LIST_INIT_addr,uint32_t)      // RW TA initialization
#define TA_YUV_TEX_BASE   PvrReg(TA_YUV_TEX_BASE_addr,uint32_t)   // RW YUV422 texture write start address
#define TA_YUV_TEX_CTRL   PvrReg(TA_YUV_TEX_CTRL_addr, TA_YUV_TEX_CTRL_type)   // RW YUV converter control
#define TA_YUV_TEX_CNT    PvrReg(TA_YUV_TEX_CNT_addr,uint32_t)    // R  YUV converter macro block counter value

#define TA_LIST_CONT      PvrReg(TA_LIST_CONT_addr,uint32_t)      // RW TA continuation processing
#define TA_NEXT_OPB_INIT  PvrReg(TA_NEXT_OPB_INIT_addr,uint32_t)  // RW Additional OPB starting address


#define FOG_TABLE        (&PvrReg(FOG_TABLE_START_addr,uint32_t))      // RW Look-up table Fog data
#define TA_OL_POINTERS   (&PvrReg(TA_OL_POINTERS_START_addr),uint32_t) // R  TA object List Pointer data
#define PALETTE_RAM      (&PvrReg(PALETTE_RAM_START_addr,uint32_t))    // RW Palette RAM


#define TA_CURRENT_CTX (TA_ISP_BASE & 0xF00000)
#define CORE_CURRENT_CTX (PARAM_BASE & 0xF00000)
