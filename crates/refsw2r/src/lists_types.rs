/*
    This file is part of libswirl
*/
// #include "license/bsd"

// List register types from refsw_lists_regtypes.h

use bitfield::bitfield;

pub type Pvr32Addr = u32;
pub type Pvr32Words = u32;
pub type ParamOffsetWords = u32;

bitfield! {
    #[derive(Copy, Clone)]
    pub struct RegionArrayEntryControl(u32);
    impl Debug;

    pub tilex, set_tilex: 7, 2;
    pub tiley, set_tiley: 13, 8;
    pub no_writeout, set_no_writeout: 28;
    pub pre_sort, set_pre_sort: 29;
    pub z_keep, set_z_keep: 30;
    pub last_region, set_last_region: 31;
}

impl RegionArrayEntryControl {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct ListPointer(u32);
    impl Debug;

    pub ptr_in_words, set_ptr_in_words: 23, 2;
    pub empty, set_empty: 31;
}

impl ListPointer {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct ObjectListTstrip(u32);
    impl Debug;

    pub param_offs_in_words, set_param_offs_in_words: 20, 0;
    pub skip, set_skip: 23, 21;
    pub shadow, set_shadow: 24;
    pub mask, set_mask: 30, 25;
    pub is_not_triangle_strip, set_is_not_triangle_strip: 31;
}

impl ObjectListTstrip {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct ObjectListTarray(u32);
    impl Debug;

    pub param_offs_in_words, set_param_offs_in_words: 20, 0;
    pub skip, set_skip: 23, 21;
    pub shadow, set_shadow: 24;
    pub prims, set_prims: 28, 25;
    pub obj_type, set_obj_type: 31, 29;
}

impl ObjectListTarray {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct ObjectListQarray(u32);
    impl Debug;

    pub param_offs_in_words, set_param_offs_in_words: 20, 0;
    pub skip, set_skip: 23, 21;
    pub shadow, set_shadow: 24;
    pub prims, set_prims: 28, 25;
    pub obj_type, set_obj_type: 31, 29;
}

impl ObjectListQarray {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct ObjectListLink(u32);
    impl Debug;

    pub next_block_ptr_in_words, set_next_block_ptr_in_words: 23, 2;
    pub end_of_list, set_end_of_list: 28;
    pub obj_type, set_obj_type: 31, 29;
}

impl ObjectListLink {
    pub fn full(&self) -> u32 { self.0 }
    pub fn set_full(&mut self, val: u32) { self.0 = val; }
}

// Union-like structure for ObjectListEntry
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ObjectListEntry {
    data: u32,
}

impl ObjectListEntry {
    pub fn new(val: u32) -> Self {
        Self { data: val }
    }

    pub fn full(&self) -> u32 {
        self.data
    }

    pub fn is_not_triangle_strip(&self) -> bool {
        (self.data >> 31) & 1 != 0
    }

    pub fn obj_type(&self) -> u32 {
        (self.data >> 29) & 0b111
    }

    pub fn as_tstrip(&self) -> ObjectListTstrip {
        ObjectListTstrip(self.data)
    }

    pub fn as_tarray(&self) -> ObjectListTarray {
        ObjectListTarray(self.data)
    }

    pub fn as_qarray(&self) -> ObjectListQarray {
        ObjectListQarray(self.data)
    }

    pub fn as_link(&self) -> ObjectListLink {
        ObjectListLink(self.data)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RegionArrayEntry {
    pub control: RegionArrayEntryControl,
    pub opaque: ListPointer,
    pub opaque_mod: ListPointer,
    pub trans: ListPointer,
    pub trans_mod: ListPointer,
    pub puncht: ListPointer,
}

impl Default for RegionArrayEntry {
    fn default() -> Self {
        Self {
            control: RegionArrayEntryControl(0),
            opaque: ListPointer(0),
            opaque_mod: ListPointer(0),
            trans: ListPointer(0),
            trans_mod: ListPointer(0),
            puncht: ListPointer(0),
        }
    }
}

// RENDLOG macro - disabled by default
#[macro_export]
macro_rules! rendlog {
    ($($arg:tt)*) => {
        // do { } while(0) equivalent - no-op
        {}
    };
}

pub const PARAMETER_TAG_SORT_MASK: u32 = 0x00FFFFFF;

pub type ParameterTag = u32;
