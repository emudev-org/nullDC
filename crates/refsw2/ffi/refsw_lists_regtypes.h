#pragma once
/*
	This file is part of libswirl
*/
// #include "license/bsd"

#include <stdint.h>

#pragma pack(push, 1) 
union RegionArrayEntryControl {
    struct {
        uint32_t res0 : 2;
        uint32_t tilex : 6;
        uint32_t tiley : 6;
        uint32_t res1 : 14;
        uint32_t no_writeout : 1;
        uint32_t pre_sort : 1;
        uint32_t z_keep : 1;
        uint32_t last_region : 1;
    };
    uint32_t full;
};

typedef uint32_t pvr32addr_t;
typedef uint32_t pvr32words_t;
typedef uint32_t param_offset_words_t;

union ListPointer {
    struct
    {
        uint32_t pad0 : 2;
        pvr32words_t ptr_in_words : 22;
        uint32_t pad1 : 7;
        uint32_t empty : 1;
    };
    uint32_t full;
};

struct ObjectListTstrip {
    param_offset_words_t param_offs_in_words : 21;
    uint32_t skip : 3;
    uint32_t shadow : 1;
    uint32_t mask : 6;
    uint32_t is_not_triangle_strip : 1;
};

struct ObjectListTarray {
    param_offset_words_t param_offs_in_words : 21;
    uint32_t skip : 3;
    uint32_t shadow : 1;
    uint32_t prims : 4;
    uint32_t type : 3;
};

struct ObjectListQarray {
    param_offset_words_t param_offs_in_words : 21;
    uint32_t skip : 3;
    uint32_t shadow : 1;
    uint32_t prims : 4;
    uint32_t type : 3;
};

struct ObjectListLink {
    uint32_t pad3 : 2;
    pvr32words_t next_block_ptr_in_words : 22;
    uint32_t pad4 : 4;
    uint32_t end_of_list : 1;
    uint32_t type : 3;
};

union ObjectListEntry {
    struct {
        uint32_t pad0 : 31;
        uint32_t is_not_triangle_strip : 1;
    };

    struct {
        uint32_t pad1 : 29;
        uint32_t type : 3;
    };

    ObjectListTstrip tstrip;
    ObjectListTarray tarray;
    ObjectListQarray qarray;
    ObjectListLink link;

    uint32_t full;
};
    
#pragma pack(pop)

// extern FILE* rendlog;
#define RENDLOG(fmt, ...) do { } while(0) // do { if (rendlog) fprintf(rendlog, fmt "\n", ##__VA_ARGS__); } while (0)