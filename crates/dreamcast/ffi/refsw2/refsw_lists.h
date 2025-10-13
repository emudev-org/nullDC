#pragma once
/*
	This file is part of libswirl
*/
// #include "license/bsd"

#include <functional>
#include "core_structs.h" // for ISP_TSP and co
#include "refsw_lists_regtypes.h"

struct RegionArrayEntry {
    RegionArrayEntryControl control;
    ListPointer opaque;
    ListPointer opaque_mod;
    ListPointer trans;
    ListPointer trans_mod;
    ListPointer puncht;
};

struct DrawParameters
{
    ISP_TSP isp;
    TSP tsp[2];
    TCW tcw[2];
};

enum RenderMode {
    RM_OPAQUE,
    RM_PUNCHTHROUGH_PASS0,
    RM_PUNCHTHROUGH_PASSN,
    RM_PUNCHTHROUGH_MV, // PT MODVOL 2nd pass
    RM_TRANSLUCENT_AUTOSORT,
    RM_TRANSLUCENT_PRESORT,
    RM_MODIFIER,
};

struct TagState {
    union {
        struct {
            bool valid: 1;
            bool rendered: 1;
        };
        uint8_t raw;
    };
};

#define PARAMETER_TAG_SORT_MASK 0x00FFFFFF

typedef uint32_t parameter_tag_t;

struct taRECT {
    int left, top, right, bottom;
};
