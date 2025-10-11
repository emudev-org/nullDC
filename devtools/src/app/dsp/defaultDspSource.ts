// Default high-level DSP source code
// Simple bypass effect that replicates input mixer channel 0 to output channels 0 and 1

export const DEFAULT_DSP_SOURCE = 
`// Copyright (C) 2025 Paul Cercueil

#define mLSAME 0
#define dLSAME 1
#define mRSAME 2
#define dRSAME 3
#define mLDIFF 4
#define dRDIFF 5
#define mRDIFF 6
#define dLDIFF 7
#define mLCOMB1 8
#define mLCOMB2 9
#define mLCOMB3 10
#define mLCOMB4 11
#define mRCOMB1 12
#define mRCOMB2 13
#define mRCOMB3 14
#define mRCOMB4 15
#define mLAPF1 16
#define mLAPF1_dAPF1 17 /* mLAPF1 - dAPF1 */
#define mRAPF1 18
#define mRAPF1_dAPF1 19 /* mRAPF1 - dAPF1 */
#define mLAPF2 20
#define mLAPF2_dAPF2 21 /* mLAPF2 - dAPF2 */
#define mRAPF2 22
#define mRAPF2_dAPF2 23 /* mRAPF2 - dAPF2 */

// TODO: All src/dst/disp/base registers are addresses in SPU memory, divided
// by 8.

// Room

// - Start from reg_addr
// - x8, to get the real offset in bytes
// - /2, since the DSP counts in 16-bit words
// - x2, since we want buffers twice as big to account for the 44100 Hz sample rate
//   instead of 22050 Hz
// - Negate the value and clamp to the ringbuffer size, since the DSP R/W
//   addresses go down while the PSX' reverb hardware goes up

#define PSX_TO_DSP_OFFT(offt) \
    ((((offt) * 8) ^ 0x3fff) + 1)

MADRS[mLSAME]		= 0x1950  // PSX_TO_DSP_OFFT(0x04d6)
MADRS[dLSAME]		= 0x2660  // PSX_TO_DSP_OFFT(0x0334)
MADRS[mRSAME]		= 0x2668  // PSX_TO_DSP_OFFT(0x0333)
MADRS[dRSAME]		= 0x3258  // PSX_TO_DSP_OFFT(0x01b5)
MADRS[mLDIFF]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[dRDIFF]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mRDIFF]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[dLDIFF]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mLCOMB1]		= 0x2080  // PSX_TO_DSP_OFFT(0x03f0)
MADRS[mLCOMB2]		= 0x2460  // PSX_TO_DSP_OFFT(0x0374)
MADRS[mLCOMB3]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mLCOMB4]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mRCOMB1]		= 0x2ec8  // PSX_TO_DSP_OFFT(0x0227)
MADRS[mRCOMB2]		= 0x3088  // PSX_TO_DSP_OFFT(0x01ef)
MADRS[mRCOMB3]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mRCOMB4]		= 0x4000  // PSX_TO_DSP_OFFT(0x0000)
MADRS[mLAPF1]		= 0x3260  // PSX_TO_DSP_OFFT(0x01b4)
MADRS[mLAPF1_dAPF1]	= 0x3648  // PSX_TO_DSP_OFFT(0x01b4 - 0x007d)
MADRS[mRAPF1]		= 0x3650  // PSX_TO_DSP_OFFT(0x0136)
MADRS[mRAPF1_dAPF1]	= 0x3a38  // PSX_TO_DSP_OFFT(0x0136 - 0x007d)
MADRS[mLAPF2]		= 0x3a40  // PSX_TO_DSP_OFFT(0x00b8)
MADRS[mLAPF2_dAPF2]	= 0x3d18  // PSX_TO_DSP_OFFT(0x00b8 - 0x005b)
MADRS[mRAPF2]		= 0x3d20  // PSX_TO_DSP_OFFT(0x005c)
MADRS[mRAPF2_dAPF2]	= 0x3ff8  // PSX_TO_DSP_OFFT(0x005c - 0x005b)


#define vIIR		3504 // 0x6d80 >> 3
#define vWALL		-2224 // 0xba80 >> 3
#define vCOMB1		2711 // 0x54b8 >> 3
#define vCOMB2		-2086 // 0xbed0 >> 3
#define vCOMB3		0
#define vCOMB4		0
#define vAPF1		2816 // 0x5800 >> 3
#define vAPF2		2656 // 0x5300 >> 3

// Same-side reflection, left-to-left:
// [mLSAME] = (Lin + [dLSAME]*vWALL - [mLSAME]) * vIIR + [mLSAME]

SMODE sat

LDF [madrs:mLSAME+],mems:mLSAME
LDF [madrs:dLSAME],mems:dLSAME

// ACC = [mLSAME]
INPUT mems:mLSAME
MAC input,#4095

// ACC = [dLSAME]*vWALL - [mLSAME]
INPUT mems:dLSAME
MAC input,#vWALL,-acc

// ACC = Lin + [dLSAME]*vWALL - [mLSAME]
INPUT mixer:0
MAC input,#2048,acc
ST [temp:0]

// ACC = (Lin + [dLSAME]*vWALL - [mLSAME]) * vIIR + [mLSAME]
INPUT mems:mLSAME
MAC input,#4095
MAC [temp:0],#vIIR,acc
STF [madrs:mLSAME]

// Same-side reflection, right-to-right:
// [mRSAME] = (Rin + [dRSAME]*vWALL - [mRSAME]) * vIIR + [mRSAME]

LDF [madrs:mRSAME+],mems:mRSAME
LDF [madrs:dRSAME],mems:dRSAME
INPUT mems:mRSAME
MAC input,#4095
INPUT mems:dRSAME
MAC input,#vWALL,-acc
INPUT mixer:1
MAC input,#2048,acc
ST [temp:0]
INPUT mems:mRSAME
MAC input,#4095
MAC [temp:0],#vIIR,acc
STF [madrs:mRSAME]

// Different side reflection, left-to-right:
// [mLDIFF] = (Lin + [dRDIFF]*vWALL - [mLDIFF]) * vIIR + [mLDIFF]

LDF [madrs:mLDIFF+],mems:mLDIFF
LDF [madrs:dRDIFF],mems:dRDIFF
INPUT mems:mLDIFF
MAC input,#4095
INPUT mems:dRDIFF
MAC input,#vWALL,-acc
INPUT mixer:0
MAC input,#2048,acc
ST [temp:0]
INPUT mems:mLDIFF
MAC input,#4095
MAC [temp:0],#vIIR,acc
STF [madrs:mLDIFF]

// Different side reflection, right-to-left:
// [mRDIFF] = (Rin + [dLDIFF]*vWALL - [mRDIFF]) * vIIR + [mRDIFF]

LDF [madrs:mRDIFF+],mems:mRDIFF
LDF [madrs:dLDIFF],mems:dLDIFF
INPUT mems:mRDIFF
MAC input,#4095
INPUT mems:dLDIFF
MAC input,#vWALL,-acc
INPUT mixer:1
MAC input,#2048,acc
ST [temp:0]
INPUT mems:mRDIFF
MAC input,#4095
MAC [temp:0],#vIIR,acc
STF [madrs:mRDIFF]

// Early echo (comb filter, with input from buffer)
// temp:0 = vCOMB1 * [mLCOMB1] + vCOMB2 * [mLCOMB2] + vCOMB3 * [mLCOMB3] + vCOMB4 * [mLCOMB4]
LDF [madrs:mLCOMB1],mems:mLCOMB1
LDF [madrs:mLCOMB2],mems:mLCOMB2
LDF [madrs:mLCOMB3],mems:mLCOMB3
LDF [madrs:mLCOMB4],mems:mLCOMB4

INPUT mems:mLCOMB1
MAC input,#vCOMB1
INPUT mems:mLCOMB2
MAC input,#vCOMB2,acc
INPUT mems:mLCOMB3
MAC input,#vCOMB3,acc
INPUT mems:mLCOMB4
MAC input,#vCOMB4,acc
ST [temp:0]

// temp:1 = vCOMB1 * [mRCOMB1] + vCOMB2 * [mRCOMB2] + vCOMB3 * [mRCOMB3] + vCOMB4 * [mRCOMB4]
LDF [madrs:mRCOMB1],mems:mRCOMB1
LDF [madrs:mRCOMB2],mems:mRCOMB2
LDF [madrs:mRCOMB3],mems:mRCOMB3
LDF [madrs:mRCOMB4],mems:mRCOMB4

INPUT mems:mRCOMB1
MAC input,#vCOMB1
INPUT mems:mRCOMB2
MAC input,#vCOMB2,acc
INPUT mems:mRCOMB3
MAC input,#vCOMB3,acc
INPUT mems:mRCOMB4
MAC input,#vCOMB4,acc
ST [temp:1]

// Late Reverb APF1 (All Pass Filter 1, with input from COMB)
// temp:0 = temp:0 - vAPF1 * [mLAPF1-dAPF1]
// [mLAPF1]=temp:0
// temp:0 = temp:0 * vAPF1 + [mLAPF1-dAPF1]

LDF [madrs:mLAPF1_dAPF1],mems:mLAPF1_dAPF1
INPUT mems:mLAPF1_dAPF1
MAC input,#-vAPF1,[temp:0]
ST [temp:0]
STF [madrs:mLAPF1]
MAC input,#4095
MAC [temp:0],#vAPF1,acc
ST [temp:0]

// temp:1 = temp:1 - vAPF1 * [mRAPF1-dAPF1]
// [mRAPF1]=temp:1
// temp:1 = temp:1 * vAPF1 + [mRAPF1-dAPF1]

LDF [madrs:mRAPF1_dAPF1],mems:mRAPF1_dAPF1
INPUT mems:mRAPF1_dAPF1
MAC input,#-vAPF1,[temp:1]
ST [temp:1]
STF [madrs:mRAPF1]
MAC input,#4095
MAC [temp:1],#vAPF1,acc
ST [temp:1]

// Late Reverb APF2 (All Pass Filter 2, with input from APF1)
// temp:0 = temp:0 - vAPF2 * [mLAPF2-dAPF2]
// [mLAPF2]=temp:0
// temp:0 = temp:0 * vAPF2 + [mLAPF2-dAPF2]

LDF [madrs:mLAPF2_dAPF2],mems:mLAPF2_dAPF2
INPUT mems:mLAPF2_dAPF2
MAC input,#-vAPF2,[temp:0]
ST [temp:0]
STF [madrs:mLAPF2]
MAC input,#4095
MAC [temp:0],#vAPF2,acc
SMODE sat2
OUTPUT mixer:0
SMODE sat

// temp:1 = temp:1 - vAPF2 * [mRAPF2-dAPF2]
// [mRAPF2]=temp:1
// temp:1 = temp:1 * vAPF2 + [mRAPF2-dAPF2]

LDF [madrs:mRAPF2_dAPF2],mems:mRAPF2_dAPF2
INPUT mems:mRAPF2_dAPF2
MAC input,#-vAPF2,[temp:1]
ST [temp:1]
STF [madrs:mRAPF2]
MAC input,#4095
MAC [temp:1],#vAPF2,acc
SMODE sat2
OUTPUT mixer:1
SMODE sat

`;
