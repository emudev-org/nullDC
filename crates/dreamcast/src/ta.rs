//! PowerVR2 Tile Accelerator implementation ported from the lxdream TA core.
//!
//! The original implementation is licensed under the GPL and so is this file.

use once_cell::sync::Lazy;
use std::sync::Mutex;

use crate::asic;

const VRAM_SIZE_BYTES: usize = 8 * 1024 * 1024;
const VRAM_MASK: u32 = (VRAM_SIZE_BYTES as u32) - 1;
const VRAM_BANK_BIT: u32 = 0x0040_0000;

const SEGMENT_END: u32 = 0x8000_0000;
const SEGMENT_ZCLEAR: u32 = 0x4000_0000;
const SEGMENT_SORT_TRANS: u32 = 0x2000_0000;
const SEGMENT_START: u32 = 0x1000_0000;
const NO_POINTER: u32 = 0x8000_0000;

const TA_LIST_NONE: i32 = -1;
const TA_LIST_OPAQUE: i32 = 0;
const TA_LIST_OPAQUE_MOD: i32 = 1;
const TA_LIST_TRANS: i32 = 2;
const TA_LIST_TRANS_MOD: i32 = 3;
const TA_LIST_PUNCH_OUT: i32 = 4;

const TA_GROW_UP: i32 = 0;
const TA_GROW_DOWN: i32 = 1;

const TA_VERTEX_NONE: i32 = -1;
const TA_VERTEX_PACKED: i32 = 0x00;
const TA_VERTEX_TEX_PACKED: i32 = 0x08;
const TA_VERTEX_TEX_SPEC_PACKED: i32 = 0x0C;
const TA_VERTEX_TEX_UV16_PACKED: i32 = 0x09;
const TA_VERTEX_TEX_UV16_SPEC_PACKED: i32 = 0x0D;
const TA_VERTEX_FLOAT: i32 = 0x10;
const TA_VERTEX_TEX_FLOAT: i32 = 0x18;
const TA_VERTEX_TEX_SPEC_FLOAT: i32 = 0x1C;
const TA_VERTEX_TEX_UV16_FLOAT: i32 = 0x19;
const TA_VERTEX_TEX_UV16_SPEC_FLOAT: i32 = 0x1D;
const TA_VERTEX_INTENSITY: i32 = 0x20;
const TA_VERTEX_TEX_INTENSITY: i32 = 0x28;
const TA_VERTEX_TEX_SPEC_INTENSITY: i32 = 0x2C;
const TA_VERTEX_TEX_UV16_INTENSITY: i32 = 0x29;
const TA_VERTEX_TEX_UV16_SPEC_INTENSITY: i32 = 0x2D;
const TA_VERTEX_PACKED_MOD: i32 = 0x40;
const TA_VERTEX_TEX_PACKED_MOD: i32 = 0x48;
const TA_VERTEX_TEX_SPEC_PACKED_MOD: i32 = 0x4C;
const TA_VERTEX_TEX_UV16_PACKED_MOD: i32 = 0x49;
const TA_VERTEX_TEX_UV16_SPEC_PACKED_MOD: i32 = 0x4D;
const TA_VERTEX_INTENSITY_MOD: i32 = 0x60;
const TA_VERTEX_TEX_INTENSITY_MOD: i32 = 0x68;
const TA_VERTEX_TEX_SPEC_INTENSITY_MOD: i32 = 0x6C;
const TA_VERTEX_TEX_UV16_INTENSITY_MOD: i32 = 0x69;
const TA_VERTEX_TEX_UV16_SPEC_INTENSITY_MOD: i32 = 0x6D;
const TA_VERTEX_SPRITE: i32 = 0x80;
const TA_VERTEX_TEX_SPRITE: i32 = 0x88;
const TA_VERTEX_MOD_VOLUME: i32 = 0x81;
const TA_VERTEX_LISTLESS: i32 = 0xFF;

const TA_POLYCMD_COLOURFMT_ARGB32: u32 = 0x0000_0000;
const TA_POLYCMD_COLOURFMT_FLOAT: u32 = 0x0000_0010;
const TA_POLYCMD_COLOURFMT_INTENSITY: u32 = 0x0000_0020;
const TA_POLYCMD_COLOURFMT_LASTINT: u32 = 0x0000_0030;
const TA_POLYCMD_MODIFIED: u32 = 0x0000_0080;
const TA_POLYCMD_FULLMOD: u32 = 0x0000_0040;
const TA_POLYCMD_TEXTURED: u32 = 0x0000_0008;
const TA_POLYCMD_SPECULAR: u32 = 0x0000_0004;
const TA_POLYCMD_SHADED: u32 = 0x0000_0002;
const TA_POLYCMD_UV16: u32 = 0x0000_0001;

const HOLLY_OPAQUE_BIT: u8 = 7;
const HOLLY_OPAQUEMOD_BIT: u8 = 8;
const HOLLY_TRANS_BIT: u8 = 9;
const HOLLY_TRANSMOD_BIT: u8 = 10;
const HOLLY_PUNCHTHRU_BIT: u8 = 21;
const HOLLY_PRIM_NOMEM_BIT: u8 = 2;
const HOLLY_MATR_NOMEM_BIT: u8 = 3;
const HOLLY_ILLEGAL_PARAM_BIT: u8 = 4;

static TA_STATE: Lazy<Mutex<TaState>> = Lazy::new(|| Mutex::new(TaState::default()));

const STRIP_LENGTHS: [usize; 4] = [3, 4, 6, 8];
const TILEMATRIX_SIZES: [u32; 4] = [0, 8, 16, 32];
const PVR_BASE: u32 = 0x005F_8000;

fn normalise_reg_offset(addr: u32) -> u32 {
    if addr >= PVR_BASE {
        addr - PVR_BASE
    } else {
        addr
    }
}

#[inline(always)]
fn ta_cmd(word: u32) -> u32 {
    word >> 29
}

#[inline(always)]
fn ta_polycmd_listtype(word: u32) -> usize {
    ((word >> 24) & 0x0F) as usize
}

#[inline(always)]
fn ta_polycmd_uselength(word: u32) -> bool {
    (word & 0x0080_0000) != 0
}

#[inline(always)]
fn ta_polycmd_length(word: u32) -> usize {
    STRIP_LENGTHS[((word >> 18) & 0x03) as usize]
}

#[inline(always)]
fn ta_polycmd_clip(word: u32) -> u32 {
    (word >> 16) & 0x03
}

#[inline(always)]
fn ta_polycmd_colourfmt(word: u32) -> u32 {
    word & 0x0000_0030
}

#[inline(always)]
fn ta_polycmd_is_specular(word: u32) -> bool {
    (word & 0x0000_000C) == 0x0000_000C
}

#[inline(always)]
fn ta_polycmd_is_fullmod(word: u32) -> bool {
    (word & 0x0000_00C0) == 0x0000_00C0
}

#[inline(always)]
fn ta_is_end_vertex(word: u32) -> bool {
    (word & 0x1000_0000) != 0
}

#[inline(always)]
fn ta_is_modifier_list(list: i32) -> bool {
    list == TA_LIST_OPAQUE_MOD || list == TA_LIST_TRANS_MOD
}

#[inline(always)]
fn ta_is_normal_poly(current_vertex_type: i32) -> bool {
    current_vertex_type < TA_VERTEX_SPRITE
}

#[inline(always)]
fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

#[inline(always)]
fn min3(x1: i32, x2: i32, x3: i32) -> i32 {
    x1.min(x2).min(x3)
}

#[inline(always)]
fn max3(x1: i32, x2: i32, x3: i32) -> i32 {
    x1.max(x2).max(x3)
}

#[inline(always)]
fn ta_is_inf(value: f32) -> bool {
    value.is_infinite() && value.is_sign_positive()
}

#[inline(always)]
fn ta_is_ninf(value: f32) -> bool {
    value.is_infinite() && value.is_sign_negative()
}

fn parse_float_colour(a: f32, r: f32, g: f32, b: f32) -> u32 {
    let conv = |component: f32| -> u32 {
        if ta_is_inf(component) {
            255
        } else {
            let mut v = (256.0 * clamp(component, 0.0, 1.0)) - 1.0;
            if v.is_nan() {
                v = 0.0;
            }
            let i = v as i32;
            i.clamp(0, 255) as u32
        }
    };

    (conv(a) << 24) | (conv(r) << 16) | (conv(g) << 8) | conv(b)
}

fn parse_intensity_colour(base: u32, intensity: f32) -> u32 {
    let i = (256.0 * clamp(intensity, 0.0, 1.0)) as u32;
    (((((base & 0xFF) * i) & 0xFF00)
        | (((base & 0xFF00) * i) & 0xFF00_00)
        | (((base & 0xFF00_00) * i) & 0xFF00_0000))
        >> 8)
        | (base & 0xFF00_0000)
}

#[inline(always)]
fn pvr_map32(offset32: u32) -> u32 {
    let static_bits = (VRAM_MASK - (VRAM_BANK_BIT * 2 - 1)) | 3;
    let offset_bits = (VRAM_BANK_BIT - 1) & !3;
    let bank = (offset32 & VRAM_BANK_BIT) / VRAM_BANK_BIT;

    let mut rv = offset32 & static_bits;
    rv |= (offset32 & offset_bits) * 2;
    rv |= bank * 4;
    rv
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessingState {
    Idle,
    InList,
    InPolygon,
    ExpectPolyBlock2,
    ExpectVertexBlock2,
    ExpectEndVertexBlock2,
    Error,
}

impl Default for ProcessingState {
    fn default() -> Self {
        ProcessingState::Error
    }
}

#[derive(Clone, Copy)]
struct TileBounds {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl Default for TileBounds {
    fn default() -> Self {
        Self {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct TaVertex {
    x: f32,
    y: f32,
    z: f32,
    detail: [u32; 8],
}

impl Default for TaVertex {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            detail: [0; 8],
        }
    }
}

#[derive(Default)]
struct TaRegisters {
    glob_tile_clip: u32,
    alloc_ctrl: u32,
    ol_base: u32,
    ol_limit: u32,
    isp_base: u32,
    isp_limit: u32,
    isp_current: u32,
    next_opb: u32,
    next_opb_init: u32,
}

#[derive(Default)]
struct TaState {
    vram: Option<*mut u8>,
    regs: TaRegisters,

    state: ProcessingState,
    width: i32,
    height: i32,
    tilelist_dir: i32,
    tilelist_start: u32,
    polybuf_start: u32,
    current_vertex_type: i32,
    accept_vertexes: bool,
    vertex_count: usize,
    max_vertex: usize,
    current_list_type: i32,
    current_tile_matrix: u32,
    current_tile_size: u32,
    intensity1: u32,
    intensity2: u32,
    clip: TileBounds,
    clip_mode: i32,
    poly_context_size: usize,
    poly_vertex_size: usize,
    poly_parity: i32,
    poly_context: [u32; 5],
    poly_pointer: u32,
    last_triangle_bounds: TileBounds,
    poly_vertex: [TaVertex; 8],
    debug_output: u32,
    modifier_last_volume: bool,
    modifier_bounds: TileBounds,
}

unsafe impl Send for TaState {}
unsafe impl Sync for TaState {}

#[derive(Clone, Copy)]
struct TaBlock {
    words: [u32; 8],
}

impl TaBlock {
    fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() == 32);
        let mut words = [0u32; 8];
        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            words[i] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Self { words }
    }

    #[inline(always)]
    fn word(&self, idx: usize) -> u32 {
        self.words[idx]
    }

    #[inline(always)]
    fn float(&self, idx: usize) -> f32 {
        f32::from_bits(self.words[idx])
    }

    #[inline(always)]
    fn words(&self) -> &[u32; 8] {
        &self.words
    }
}

impl TaState {
    fn read_vram_u32(&self, addr: u32) -> Option<u32> {
        if addr >= VRAM_SIZE_BYTES as u32 {
            return None;
        }
        let vram = self.vram?;
        let mapped = pvr_map32(addr & VRAM_MASK);
        unsafe {
            let ptr = vram.add(mapped as usize) as *const u32;
            Some(ptr.read_unaligned())
        }
    }

    fn write_vram_u32(&mut self, addr: u32, value: u32) {
        if addr >= VRAM_SIZE_BYTES as u32 {
            return;
        }
        if let Some(vram) = self.vram {
            let mapped = pvr_map32(addr & VRAM_MASK);
            unsafe {
                let ptr = vram.add(mapped as usize) as *mut u32;
                ptr.write_unaligned(value);
            }
        }
    }

    fn reset(&mut self) {
        self.state = ProcessingState::Error;
        self.debug_output = 0;
    }

    fn init(&mut self, vram: *mut u8) {
        if vram.is_null() {
            self.vram = None;
        } else {
            self.vram = Some(vram);
        }
        self.state = ProcessingState::Idle;
        self.current_list_type = TA_LIST_NONE;
        self.current_vertex_type = TA_VERTEX_LISTLESS;
        self.poly_parity = 0;
        self.vertex_count = 0;
        self.max_vertex = 3;
        self.poly_vertex_size = 0;
        self.poly_context = [0; 5];
        self.poly_pointer = 0;
        self.accept_vertexes = true;
        self.last_triangle_bounds = TileBounds {
            x1: -1,
            y1: 0,
            x2: 0,
            y2: 0,
        };
        self.modifier_last_volume = false;
        self.modifier_bounds = TileBounds {
            x1: i32::MAX,
            y1: i32::MAX,
            x2: i32::MIN,
            y2: i32::MIN,
        };

        let size = self.regs.glob_tile_clip;
        self.width = ((size & 0xFFFF) + 1) as i32;
        self.height = (((size >> 16) & 0xFFFF) + 1) as i32;

        self.clip = TileBounds {
            x1: 0,
            y1: 0,
            x2: self.width - 1,
            y2: self.height - 1,
        };
        self.clip_mode = 0;

        self.tilelist_dir = ((self.regs.alloc_ctrl >> 20) & 0x01) as i32;
        self.regs.isp_current = self.regs.isp_base;
        self.regs.next_opb = self.regs.next_opb_init >> 2;
        self.tilelist_start = self.regs.next_opb;
        self.polybuf_start = self.regs.isp_base & 0x00F0_0000;
    }

    fn init_list(&mut self, listtype: usize) {
        let mut config = self.regs.alloc_ctrl;
        let tile_matrix = self.regs.ol_base;
        let list_end = self.regs.ol_limit;

        self.current_tile_matrix = tile_matrix;

        let listtype_i32 = listtype as i32;
        if ((self.tilelist_dir == TA_GROW_DOWN && list_end <= tile_matrix)
            || (self.tilelist_dir == TA_GROW_UP && list_end >= tile_matrix))
            && listtype_i32 <= TA_LIST_PUNCH_OUT
        {
            for _ in 0..listtype {
                let idx = (config & 0x03) as usize;
                let size = TILEMATRIX_SIZES[idx] << 2;
                self.current_tile_matrix = self
                    .current_tile_matrix
                    .wrapping_add((self.width * self.height) as u32 * size);
                config >>= 4;
            }
            let idx = (config & 0x03) as usize;
            self.current_tile_size = TILEMATRIX_SIZES[idx];

            if self.current_tile_size != 0 {
                let mut p = self.current_tile_matrix;
                let total_tiles = (self.width * self.height) as u32;
                for _ in 0..total_tiles {
                    self.write_vram_u32(p, 0xF000_0000);
                    p = p.wrapping_add(self.current_tile_size * 4);
                }
            }
        } else {
            self.current_tile_size = 0;
        }

        if tile_matrix == list_end {
            self.current_tile_size = 0;
        }

        self.state = ProcessingState::InList;
        self.current_list_type = listtype_i32;
        self.last_triangle_bounds.x1 = -1;
    }

    fn raise_list_interrupt(listtype: i32) {
        match listtype {
            TA_LIST_OPAQUE => asic::raise_normal(HOLLY_OPAQUE_BIT),
            TA_LIST_OPAQUE_MOD => asic::raise_normal(HOLLY_OPAQUEMOD_BIT),
            TA_LIST_TRANS => asic::raise_normal(HOLLY_TRANS_BIT),
            TA_LIST_TRANS_MOD => asic::raise_normal(HOLLY_TRANSMOD_BIT),
            TA_LIST_PUNCH_OUT => asic::raise_normal(HOLLY_PUNCHTHRU_BIT),
            _ => {}
        }
    }

    fn end_list(&mut self) {
        if self.current_list_type != TA_LIST_NONE {
            Self::raise_list_interrupt(self.current_list_type);
        }
        self.current_list_type = TA_LIST_NONE;
        self.current_vertex_type = TA_VERTEX_LISTLESS;
        self.poly_vertex_size = 0;
        self.poly_context[1] = 0;
        self.state = ProcessingState::Idle;
    }

    fn bad_input_error(&mut self) {
        asic::raise_error(HOLLY_ILLEGAL_PARAM_BIT);
        println!("TA error: holly_ILLEGAL_PARAM. Interrupt raised");
    }

    fn write_polygon_buffer(&mut self, data: &[u32]) -> usize {
        let mut posn = self.regs.isp_current;
        let end = self.regs.isp_limit;
        let mut written = 0;
        for word in data {
            if posn == end {
                asic::raise_error(HOLLY_PRIM_NOMEM_BIT);
                println!("TA error: holly_PRIM_NOMEM. Interrupt raised");
                break;
            }
            if posn < VRAM_SIZE_BYTES as u32 {
                self.write_vram_u32(posn, *word);
            }
            posn = posn.wrapping_add(4);
            written += 1;
        }
        self.regs.isp_current = posn;
        written
    }

    fn alloc_tilelist(&mut self, reference: u32) -> Option<u32> {
        if self.current_tile_size == 0 {
            return None;
        }

        let mut posn = self.regs.next_opb;
        let limit = self.regs.ol_limit >> 2;

        if self.tilelist_dir == TA_GROW_DOWN {
            posn = posn.wrapping_sub(self.current_tile_size);
            let newposn = posn;

            if posn == limit {
                self.write_vram_u32(posn << 2, 0xF000_0000);
                self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
                return None;
            } else if posn < limit {
                self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
                return None;
            } else if newposn <= limit {
                // nothing
            } else if newposn <= limit + self.current_tile_size {
                asic::raise_error(HOLLY_MATR_NOMEM_BIT);
                println!("TA error: holly_MATR_NOMEM. Interrupt raised");
                self.regs.next_opb = newposn;
            } else {
                self.regs.next_opb = newposn;
            }

            self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
            Some(posn << 2)
        } else {
            let newposn = posn.wrapping_add(self.current_tile_size);
            if posn == limit {
                self.write_vram_u32(posn << 2, 0xF000_0000);
                self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
                return None;
            } else if posn > limit {
                self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
                return None;
            } else if newposn >= limit {
                // nothing
            } else if newposn >= limit.wrapping_sub(self.current_tile_size) {
                asic::raise_error(HOLLY_MATR_NOMEM_BIT);
                println!("TA error: holly_MATR_NOMEM. Interrupt raised");
                self.regs.next_opb = newposn;
            } else {
                self.regs.next_opb = newposn;
            }

            self.write_vram_u32(reference, 0xE000_0000 | (posn << 2));
            Some(posn << 2)
        }
    }

    fn write_tile_entry(&mut self, x: i32, y: i32, tile_entry: u32) {
        if self.clip_mode == 3
            && x >= self.clip.x1
            && x <= self.clip.x2
            && y >= self.clip.y1
            && y <= self.clip.y2
        {
            return;
        }

        let tile_offset = (y * self.width + x) as u32;
        let mut tile = self
            .current_tile_matrix
            .wrapping_add(((self.current_tile_size * tile_offset) << 2) as u32);
        let mut tilestart = tile;
        let mut lasttri = 0u32;

        if (tile_entry & 0x8000_0000) != 0
            && self.last_triangle_bounds.x1 != -1
            && self.last_triangle_bounds.x1 <= x
            && self.last_triangle_bounds.x2 >= x
            && self.last_triangle_bounds.y1 <= y
            && self.last_triangle_bounds.y2 >= y
        {
            lasttri = tile_entry & 0xE1E0_0000;
        }

        if self.read_vram_u32(tile) == Some(0xF000_0000) {
            self.write_vram_u32(tile, tile_entry);
            self.write_vram_u32(tile.wrapping_add(4), 0xF000_0000);
            return;
        }

        loop {
            let mut value = self.read_vram_u32(tile).unwrap_or(0);
            for i in 1..self.current_tile_size {
                tile = tile.wrapping_add(4);
                let nextval = self.read_vram_u32(tile).unwrap_or(0);
                if nextval == 0xF000_0000 {
                    if lasttri != 0 && lasttri == (value & 0xE1E0_0000) {
                        let count = (value & 0x1E00_0000).wrapping_add(0x0200_0000);
                        if count < 0x2000_0000 {
                            self.write_vram_u32(
                                tile.wrapping_sub(4),
                                (value & 0xE1FF_FFFF) | count,
                            );
                            return;
                        }
                    }
                    if i < self.current_tile_size - 1 {
                        self.write_vram_u32(tile, tile_entry);
                        self.write_vram_u32(tile.wrapping_add(4), 0xF000_0000);
                        return;
                    }
                }
                value = nextval;
            }

            if value == 0xF000_0000 {
                if let Some(newtile) = self.alloc_tilelist(tile) {
                    self.write_vram_u32(newtile, tile_entry);
                    self.write_vram_u32(newtile.wrapping_add(4), 0xF000_0000);
                }
                return;
            } else if (value & 0xFF00_0000) == 0xE000_0000 {
                let next = value & 0x00FF_FFFF;
                if next == tilestart {
                    return;
                }
                tilestart = next;
                tile = next;
            } else {
                return;
            }
        }
    }

    fn commit_polygon(&mut self) {
        let vertex_count = self.vertex_count;
        if vertex_count < 3 {
            return;
        }

        let mut tx = vec![0i32; vertex_count];
        let mut ty = vec![0i32; vertex_count];

        for (i, vertex) in self.poly_vertex.iter().take(vertex_count).enumerate() {
            tx[i] = if vertex.x < 0.0 || ta_is_ninf(vertex.x) {
                -1
            } else if vertex.x > (i32::MAX as f32) || ta_is_inf(vertex.x) {
                i32::MAX / 32
            } else {
                (vertex.x / 32.0) as i32
            };

            ty[i] = if vertex.y < 0.0 || ta_is_ninf(vertex.y) {
                -1
            } else if vertex.y > (i32::MAX as f32) || ta_is_inf(vertex.y) {
                i32::MAX / 32
            } else {
                (vertex.y / 32.0) as i32
            };
        }

        let mut triangle_bound = vec![TileBounds::default(); vertex_count - 2];

        triangle_bound[0].x1 = min3(tx[0], tx[1], tx[2]);
        triangle_bound[0].x2 = max3(tx[0], tx[1], tx[2]);
        triangle_bound[0].y1 = min3(ty[0], ty[1], ty[2]);
        triangle_bound[0].y2 = max3(ty[0], ty[1], ty[2]);

        let mut polygon_bound = TileBounds {
            x1: triangle_bound[0].x1,
            y1: triangle_bound[0].y1,
            x2: triangle_bound[0].x2,
            y2: triangle_bound[0].y2,
        };

        for i in 1..(vertex_count - 2) {
            triangle_bound[i].x1 = min3(tx[i], tx[i + 1], tx[i + 2]);
            triangle_bound[i].x2 = max3(tx[i], tx[i + 1], tx[i + 2]);
            triangle_bound[i].y1 = min3(ty[i], ty[i + 1], ty[i + 2]);
            triangle_bound[i].y2 = max3(ty[i], ty[i + 1], ty[i + 2]);

            polygon_bound.x1 = polygon_bound.x1.min(triangle_bound[i].x1);
            polygon_bound.x2 = polygon_bound.x2.max(triangle_bound[i].x2);
            polygon_bound.y1 = polygon_bound.y1.min(triangle_bound[i].y1);
            polygon_bound.y2 = polygon_bound.y2.max(triangle_bound[i].y2);
        }

        polygon_bound.x1 = polygon_bound.x1.clamp(0, self.width - 1);
        polygon_bound.x2 = polygon_bound.x2.clamp(0, self.width - 1);
        polygon_bound.y1 = polygon_bound.y1.clamp(0, self.height - 1);
        polygon_bound.y2 = polygon_bound.y2.clamp(0, self.height - 1);

        if self.current_vertex_type == TA_VERTEX_MOD_VOLUME {
            self.modifier_bounds.x1 = self.modifier_bounds.x1.min(polygon_bound.x1);
            self.modifier_bounds.x2 = self.modifier_bounds.x2.max(polygon_bound.x2);
            self.modifier_bounds.y1 = self.modifier_bounds.y1.min(polygon_bound.y1);
            self.modifier_bounds.y2 = self.modifier_bounds.y2.max(polygon_bound.y2);

            if self.modifier_last_volume {
                polygon_bound = self.modifier_bounds;
            }
        }

        if polygon_bound.x1 == polygon_bound.x2 && polygon_bound.y1 == polygon_bound.y2 {
            self.poly_context[0] |= 0x0020_0000;
        }

        match self.clip_mode {
            0 => {
                if polygon_bound.x2 < 0
                    || polygon_bound.x1 >= self.width
                    || polygon_bound.y2 < 0
                    || polygon_bound.y1 >= self.height
                {
                    return;
                }
            }
            2 => {
                if polygon_bound.x2 < self.clip.x1
                    || polygon_bound.x1 > self.clip.x2
                    || polygon_bound.y2 < self.clip.y1
                    || polygon_bound.y1 > self.clip.y2
                {
                    return;
                }
                polygon_bound.x1 = polygon_bound.x1.max(self.clip.x1);
                polygon_bound.x2 = polygon_bound.x2.min(self.clip.x2);
                polygon_bound.y1 = polygon_bound.y1.max(self.clip.y1);
                polygon_bound.y2 = polygon_bound.y2.min(self.clip.y2);
            }
            3 => {
                if polygon_bound.x1 >= self.clip.x1
                    && polygon_bound.x2 <= self.clip.x2
                    && polygon_bound.y1 >= self.clip.y1
                    && polygon_bound.y2 <= self.clip.y2
                {
                    return;
                }
            }
            _ => {}
        }

        let isp_current = self.regs.isp_current;
        let mut tile_entry =
            ((isp_current.wrapping_sub(self.polybuf_start)) >> 2) | self.poly_pointer;

        let context_words: Vec<u32> = self.poly_context[..self.poly_context_size].to_vec();
        if self.write_polygon_buffer(&context_words) < context_words.len() {
            return;
        }

        for i in 0..vertex_count {
            let vertex = self.poly_vertex[i];
            let mut words = Vec::with_capacity(3 + self.poly_vertex_size);
            words.push(vertex.x.to_bits());
            words.push(vertex.y.to_bits());
            words.push(vertex.z.to_bits());
            for detail in vertex.detail[..self.poly_vertex_size].iter() {
                words.push(*detail);
            }

            if self.write_polygon_buffer(&words) < words.len() {
                return;
            }
        }

        if self.current_tile_size == 0 {
            return;
        }

        if vertex_count == 3 {
            tile_entry |= 0x8000_0000;
            for y in polygon_bound.y1..=polygon_bound.y2 {
                for x in polygon_bound.x1..=polygon_bound.x2 {
                    self.write_tile_entry(x, y, tile_entry);
                }
            }
            self.last_triangle_bounds = polygon_bound;
        } else if self.current_vertex_type == TA_VERTEX_SPRITE
            || self.current_vertex_type == TA_VERTEX_TEX_SPRITE
        {
            tile_entry |= 0xA000_0000;
            for y in polygon_bound.y1..=polygon_bound.y2 {
                for x in polygon_bound.x1..=polygon_bound.x2 {
                    self.write_tile_entry(x, y, tile_entry);
                }
            }
            self.last_triangle_bounds = polygon_bound;
        } else {
            for y in polygon_bound.y1..=polygon_bound.y2 {
                for x in polygon_bound.x1..=polygon_bound.x2 {
                    let mut entry = tile_entry;
                    for (i, bound) in triangle_bound.iter().enumerate() {
                        if bound.x1 <= x && bound.x2 >= x && bound.y1 <= y && bound.y2 >= y {
                            entry |= 0x4000_0000 >> i;
                        }
                    }
                    self.write_tile_entry(x, y, entry);
                }
            }
            self.last_triangle_bounds.x1 = -1;
        }
    }

    fn split_polygon(&mut self) {
        self.commit_polygon();
        if ta_is_normal_poly(self.current_vertex_type) {
            if self.vertex_count == 3 {
                if self.poly_parity == 0 {
                    self.poly_vertex[0] = self.poly_vertex[2];
                    self.poly_parity = 1;
                } else {
                    self.poly_vertex[1] = self.poly_vertex[2];
                    self.poly_parity = 0;
                }
            } else if self.vertex_count >= 2 {
                let last = self.vertex_count;
                self.poly_vertex[0] = self.poly_vertex[last - 2];
                self.poly_vertex[1] = self.poly_vertex[last - 1];
                self.poly_parity = 0;
            }
            self.vertex_count = 2;
        } else {
            self.vertex_count = 0;
        }
    }

    fn parse_polygon_context(&mut self, data: &TaBlock) {
        let word0 = data.word(0);
        let mut colourfmt = ta_polycmd_colourfmt(word0);

        if ta_polycmd_uselength(word0) {
            self.max_vertex = ta_polycmd_length(word0);
        }

        self.clip_mode = ta_polycmd_clip(word0) as i32;
        if self.clip_mode == 1 {
            self.clip_mode = 2;
        }
        self.vertex_count = 0;
        self.poly_context[0] = (data.word(1) & 0xFC1F_FFFF) | ((word0 & 0x0B) << 22);
        self.poly_context[1] = data.word(2);
        self.poly_context[3] = data.word(4);
        self.poly_parity = 0;

        if (word0 & TA_POLYCMD_TEXTURED) != 0 {
            self.current_vertex_type = (word0 & 0x0D) as i32;
            self.poly_context[2] = data.word(3);
            self.poly_context[4] = data.word(5);
            if (word0 & TA_POLYCMD_SPECULAR) != 0 {
                self.poly_context[0] |= 0x0100_0000;
                self.poly_vertex_size = 4;
            } else {
                self.poly_vertex_size = 3;
            }
            if (word0 & TA_POLYCMD_UV16) != 0 {
                self.poly_vertex_size = self.poly_vertex_size.saturating_sub(1);
            }
        } else {
            self.current_vertex_type = 0;
            self.poly_vertex_size = 1;
            self.poly_context[2] = 0;
            self.poly_context[4] = 0;
        }

        self.poly_pointer = (self.poly_vertex_size as u32) << 21;
        self.poly_context_size = 3;

        if (word0 & TA_POLYCMD_MODIFIED) != 0 {
            self.poly_pointer |= 0x0100_0000;
            if (word0 & TA_POLYCMD_FULLMOD) != 0 {
                self.poly_context_size = 5;
                self.poly_vertex_size <<= 1;
                self.current_vertex_type |= 0x40;
                if colourfmt == TA_POLYCMD_COLOURFMT_FLOAT {
                    colourfmt = TA_POLYCMD_COLOURFMT_LASTINT;
                }
            }
        }

        if colourfmt == TA_POLYCMD_COLOURFMT_INTENSITY {
            if ta_polycmd_is_fullmod(word0) || ta_polycmd_is_specular(word0) {
                self.state = ProcessingState::ExpectPolyBlock2;
            } else {
                self.intensity1 =
                    parse_float_colour(data.float(4), data.float(5), data.float(6), data.float(7));
            }
        } else if colourfmt == TA_POLYCMD_COLOURFMT_LASTINT {
            colourfmt = TA_POLYCMD_COLOURFMT_INTENSITY;
        }

        self.current_vertex_type |= colourfmt as i32;
    }

    fn parse_modifier_context(&mut self, data: &TaBlock) {
        let word0 = data.word(0);
        self.current_vertex_type = TA_VERTEX_MOD_VOLUME;
        self.poly_vertex_size = 0;
        self.clip_mode = ta_polycmd_clip(word0) as i32;
        if self.clip_mode == 1 {
            self.clip_mode = 2;
        }
        self.poly_context_size = 3;
        self.poly_context[0] = (data.word(1) & 0xFC1F_FFFF) | ((word0 & 0x0B) << 22);
        if ta_polycmd_is_specular(word0) {
            self.poly_context[0] |= 0x0100_0000;
        }
        self.poly_context[1] = 0;
        self.poly_context[2] = 0;
        self.vertex_count = 0;
        self.max_vertex = 3;
        self.poly_pointer = 0;

        if self.modifier_last_volume {
            self.modifier_bounds = TileBounds {
                x1: i32::MAX / 32,
                y1: i32::MAX / 32,
                x2: -1,
                y2: -1,
            };
        }
        self.modifier_last_volume = (word0 & TA_POLYCMD_FULLMOD) != 0;
    }

    fn parse_sprite_context(&mut self, data: &TaBlock) {
        let word0 = data.word(0);
        self.poly_context_size = 3;
        self.poly_context[0] = (data.word(1) & 0xFC1F_FFFF) | ((word0 & 0x0B) << 22) | 0x0040_0000;
        self.clip_mode = ta_polycmd_clip(word0) as i32;
        if self.clip_mode == 1 {
            self.clip_mode = 2;
        }
        if ta_polycmd_is_specular(word0) {
            self.poly_context[0] |= 0x0100_0000;
        }
        self.poly_context[1] = data.word(2);
        self.poly_context[2] = data.word(3);
        if (word0 & TA_POLYCMD_TEXTURED) != 0 {
            self.poly_vertex_size = 2;
            self.poly_vertex[2].detail[1] = data.word(4);
            self.current_vertex_type = TA_VERTEX_TEX_SPRITE;
        } else {
            self.poly_vertex_size = 1;
            self.poly_vertex[2].detail[0] = data.word(4);
            self.current_vertex_type = TA_VERTEX_SPRITE;
        }
        self.vertex_count = 0;
        self.max_vertex = 4;
        self.poly_pointer = (self.poly_vertex_size as u32) << 21;
    }

    fn fill_vertexes(&mut self) {
        if self.vertex_count == 0 {
            return;
        }
        let last_idx = self.vertex_count - 1;
        let last = self.poly_vertex[last_idx];
        for i in self.vertex_count..self.max_vertex {
            self.poly_vertex[i] = last;
        }
    }

    fn parse_vertex(&mut self, data: &TaBlock) {
        if self.vertex_count >= self.poly_vertex.len() {
            return;
        }

        let idx = self.vertex_count;
        self.poly_vertex[idx].x = data.float(1);
        self.poly_vertex[idx].y = data.float(2);
        self.poly_vertex[idx].z = data.float(3);

        match self.current_vertex_type {
            TA_VERTEX_PACKED => {
                self.poly_vertex[idx].detail[0] = data.word(6);
            }
            TA_VERTEX_FLOAT => {
                self.poly_vertex[idx].detail[0] =
                    parse_float_colour(data.float(4), data.float(5), data.float(6), data.float(7));
            }
            TA_VERTEX_INTENSITY => {
                self.poly_vertex[idx].detail[0] =
                    parse_intensity_colour(self.intensity1, data.float(6));
            }

            TA_VERTEX_TEX_SPEC_PACKED => {
                self.poly_vertex[idx].detail[3] = data.word(7);
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] = data.word(6);
            }
            TA_VERTEX_TEX_PACKED => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] = data.word(6);
            }
            TA_VERTEX_TEX_UV16_SPEC_PACKED => {
                self.poly_vertex[idx].detail[2] = data.word(7);
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(6);
            }
            TA_VERTEX_TEX_UV16_PACKED => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(6);
            }

            TA_VERTEX_TEX_FLOAT | TA_VERTEX_TEX_SPEC_FLOAT => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_UV16_FLOAT | TA_VERTEX_TEX_UV16_SPEC_FLOAT => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.state = ProcessingState::ExpectVertexBlock2;
            }

            TA_VERTEX_TEX_SPEC_INTENSITY => {
                self.poly_vertex[idx].detail[3] =
                    parse_intensity_colour(self.intensity2, data.float(7));
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity1, data.float(6));
            }
            TA_VERTEX_TEX_INTENSITY => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity1, data.float(6));
            }
            TA_VERTEX_TEX_UV16_SPEC_INTENSITY => {
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity2, data.float(7));
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] =
                    parse_intensity_colour(self.intensity1, data.float(6));
            }
            TA_VERTEX_TEX_UV16_INTENSITY => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] =
                    parse_intensity_colour(self.intensity1, data.float(6));
            }

            TA_VERTEX_PACKED_MOD => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
            }
            TA_VERTEX_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[0] =
                    parse_intensity_colour(self.intensity1, data.float(4));
                self.poly_vertex[idx].detail[1] =
                    parse_intensity_colour(self.intensity2, data.float(5));
            }

            TA_VERTEX_TEX_SPEC_PACKED_MOD => {
                self.poly_vertex[idx].detail[3] = data.word(7);
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] = data.word(6);
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_PACKED_MOD => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] = data.word(6);
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_UV16_SPEC_PACKED_MOD => {
                self.poly_vertex[idx].detail[2] = data.word(7);
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(6);
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_UV16_PACKED_MOD => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(6);
                self.state = ProcessingState::ExpectVertexBlock2;
            }

            TA_VERTEX_TEX_SPEC_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[3] =
                    parse_intensity_colour(self.intensity1, data.float(7));
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity1, data.float(6));
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] = data.word(5);
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity1, data.float(6));
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_UV16_SPEC_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[2] =
                    parse_intensity_colour(self.intensity1, data.float(7));
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] =
                    parse_intensity_colour(self.intensity1, data.float(6));
                self.state = ProcessingState::ExpectVertexBlock2;
            }
            TA_VERTEX_TEX_UV16_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[0] = data.word(4);
                self.poly_vertex[idx].detail[1] =
                    parse_intensity_colour(self.intensity1, data.float(6));
                self.state = ProcessingState::ExpectVertexBlock2;
            }

            TA_VERTEX_SPRITE | TA_VERTEX_TEX_SPRITE | TA_VERTEX_MOD_VOLUME | TA_VERTEX_LISTLESS => {
                if idx + 2 < self.poly_vertex.len() {
                    self.poly_vertex[idx + 1].x = data.float(4);
                    self.poly_vertex[idx + 1].y = data.float(5);
                    self.poly_vertex[idx + 1].z = data.float(6);
                    self.poly_vertex[idx + 2].x = data.float(7);
                    self.vertex_count += 2;
                    if self.current_vertex_type == TA_VERTEX_SPRITE
                        || self.current_vertex_type == TA_VERTEX_TEX_SPRITE
                    {
                        self.state = ProcessingState::ExpectEndVertexBlock2;
                    } else {
                        self.state = ProcessingState::ExpectVertexBlock2;
                    }
                }
            }

            _ => {}
        }

        self.vertex_count += 1;
    }

    fn parse_vertex_block2(&mut self, data: &TaBlock) {
        if self.vertex_count == 0 {
            return;
        }

        let idx = self.vertex_count - 1;

        match self.current_vertex_type {
            TA_VERTEX_TEX_SPEC_FLOAT => {
                self.poly_vertex[idx].detail[3] =
                    parse_float_colour(data.float(4), data.float(5), data.float(6), data.float(7));
                self.poly_vertex[idx].detail[2] =
                    parse_float_colour(data.float(0), data.float(1), data.float(2), data.float(3));
            }
            TA_VERTEX_TEX_FLOAT => {
                self.poly_vertex[idx].detail[2] =
                    parse_float_colour(data.float(0), data.float(1), data.float(2), data.float(3));
            }
            TA_VERTEX_TEX_UV16_SPEC_FLOAT => {
                self.poly_vertex[idx].detail[2] =
                    parse_float_colour(data.float(4), data.float(5), data.float(6), data.float(7));
                self.poly_vertex[idx].detail[1] =
                    parse_float_colour(data.float(0), data.float(1), data.float(2), data.float(3));
            }
            TA_VERTEX_TEX_UV16_FLOAT => {
                self.poly_vertex[idx].detail[1] =
                    parse_float_colour(data.float(0), data.float(1), data.float(2), data.float(3));
            }

            TA_VERTEX_TEX_PACKED_MOD => {
                self.poly_vertex[idx].detail[3] = data.word(0);
                self.poly_vertex[idx].detail[4] = data.word(1);
                self.poly_vertex[idx].detail[5] = data.word(2);
            }
            TA_VERTEX_TEX_SPEC_PACKED_MOD => {
                self.poly_vertex[idx].detail[4] = data.word(0);
                self.poly_vertex[idx].detail[5] = data.word(1);
                self.poly_vertex[idx].detail[6] = data.word(2);
                self.poly_vertex[idx].detail[7] = data.word(3);
            }
            TA_VERTEX_TEX_UV16_PACKED_MOD => {
                self.poly_vertex[idx].detail[2] = data.word(0);
                self.poly_vertex[idx].detail[3] = data.word(2);
            }
            TA_VERTEX_TEX_UV16_SPEC_PACKED_MOD => {
                self.poly_vertex[idx].detail[3] = data.word(0);
                self.poly_vertex[idx].detail[4] = data.word(2);
                self.poly_vertex[idx].detail[5] = data.word(3);
            }

            TA_VERTEX_TEX_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[3] = data.word(0);
                self.poly_vertex[idx].detail[4] = data.word(1);
                self.poly_vertex[idx].detail[5] =
                    parse_intensity_colour(self.intensity2, data.float(2));
            }
            TA_VERTEX_TEX_SPEC_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[4] = data.word(0);
                self.poly_vertex[idx].detail[5] = data.word(1);
                self.poly_vertex[idx].detail[6] =
                    parse_intensity_colour(self.intensity2, data.float(2));
                self.poly_vertex[idx].detail[7] =
                    parse_intensity_colour(self.intensity2, data.float(3));
            }
            TA_VERTEX_TEX_UV16_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[2] = data.word(0);
                self.poly_vertex[idx].detail[3] =
                    parse_intensity_colour(self.intensity2, data.float(2));
            }
            TA_VERTEX_TEX_UV16_SPEC_INTENSITY_MOD => {
                self.poly_vertex[idx].detail[3] = data.word(0);
                self.poly_vertex[idx].detail[4] =
                    parse_intensity_colour(self.intensity2, data.float(2));
                self.poly_vertex[idx].detail[5] =
                    parse_intensity_colour(self.intensity2, data.float(3));
            }

            TA_VERTEX_SPRITE => {
                self.poly_vertex[idx].y = data.float(0);
                self.poly_vertex[idx].z = data.float(1);
                if self.vertex_count < self.poly_vertex.len() {
                    let next_idx = self.vertex_count;
                    self.poly_vertex[next_idx].x = data.float(2);
                    self.poly_vertex[next_idx].y = data.float(3);
                    self.poly_vertex[next_idx].z = 0.0;
                    self.poly_vertex[next_idx].detail[0] = 0;
                    self.poly_vertex[0].detail[0] = 0;
                    if self.max_vertex > 1 {
                        self.poly_vertex[1].detail[0] = 0;
                    }
                    self.vertex_count += 1;
                }
            }
            TA_VERTEX_TEX_SPRITE => {
                self.poly_vertex[idx].y = data.float(0);
                self.poly_vertex[idx].z = data.float(1);
                if self.vertex_count < self.poly_vertex.len() {
                    let next_idx = self.vertex_count;
                    self.poly_vertex[next_idx].x = data.float(2);
                    self.poly_vertex[next_idx].y = data.float(3);
                    self.poly_vertex[next_idx].z = 0.0;
                    self.poly_vertex[next_idx].detail[0] = 0;
                    self.poly_vertex[next_idx].detail[1] = 0;
                    self.poly_vertex[0].detail[0] = data.word(5);
                    self.poly_vertex[0].detail[1] = 0;
                    if self.max_vertex > 1 {
                        self.poly_vertex[1].detail[0] = data.word(6);
                        self.poly_vertex[1].detail[1] = 0;
                    }
                    self.poly_vertex[2].detail[0] = data.word(7);
                    self.vertex_count += 1;
                }
            }
            TA_VERTEX_MOD_VOLUME | TA_VERTEX_LISTLESS => {
                self.poly_vertex[idx].y = data.float(0);
                self.poly_vertex[idx].z = data.float(1);
            }

            _ => {}
        }

        self.state = ProcessingState::InPolygon;
    }

    fn process_block(&mut self, block: &TaBlock) {
        match self.state {
            ProcessingState::Error => return,
            ProcessingState::ExpectPolyBlock2 => {
                self.intensity1 = parse_float_colour(
                    block.float(0),
                    block.float(1),
                    block.float(2),
                    block.float(3),
                );
                self.intensity2 = parse_float_colour(
                    block.float(4),
                    block.float(5),
                    block.float(6),
                    block.float(7),
                );
                self.state = ProcessingState::InList;
            }
            ProcessingState::ExpectVertexBlock2 => {
                self.parse_vertex_block2(block);
                if self.vertex_count == self.max_vertex {
                    self.split_polygon();
                }
            }
            ProcessingState::ExpectEndVertexBlock2 => {
                self.parse_vertex_block2(block);
                if self.vertex_count < 3 {
                    self.bad_input_error();
                } else {
                    self.commit_polygon();
                }
                self.vertex_count = 0;
                self.poly_parity = 0;
                self.state = ProcessingState::InList;
            }
            ProcessingState::InList | ProcessingState::InPolygon | ProcessingState::Idle => {
                match ta_cmd(block.word(0)) {
                    0 => {
                        if self.state == ProcessingState::InPolygon {
                            self.bad_input_error();
                            self.end_list();
                            self.state = ProcessingState::Error;
                        } else {
                            self.end_list();
                        }
                    }
                    1 => {
                        if self.state == ProcessingState::InPolygon {
                            self.bad_input_error();
                            self.accept_vertexes = false;
                        }
                        self.clip.x1 = (block.word(4) & 0x3F) as i32;
                        self.clip.y1 = (block.word(5) & 0x0F) as i32;
                        self.clip.x2 = (block.word(6) & 0x3F) as i32;
                        self.clip.y2 = (block.word(7) & 0x0F) as i32;
                        if self.clip.x2 >= self.width {
                            self.clip.x2 = self.width - 1;
                        }
                        if self.clip.y2 >= self.height {
                            self.clip.y2 = self.height - 1;
                        }
                    }
                    4 => {
                        if self.state == ProcessingState::Idle {
                            let list = ta_polycmd_listtype(block.word(0));
                            self.init_list(list);
                        }

                        if self.current_list_type == TA_LIST_NONE {
                            println!(
                                "TA error: polygon context in listless mode, state {} list {:?}",
                                self.state as u32,
                                ta_polycmd_listtype(block.word(0))
                            );
                        }

                        if self.vertex_count != 0 {
                            self.bad_input_error();
                            self.accept_vertexes = false;
                        } else if ta_is_modifier_list(self.current_list_type) {
                            self.parse_modifier_context(block);
                        } else {
                            self.parse_polygon_context(block);
                        }
                    }
                    5 => {
                        if self.state == ProcessingState::Idle {
                            let list = ta_polycmd_listtype(block.word(0));
                            self.init_list(list);
                        }

                        if self.current_list_type == TA_LIST_NONE {
                            println!("TA error: sprite context in listless mode");
                        }

                        if self.vertex_count != 0 {
                            self.fill_vertexes();
                            self.commit_polygon();
                        }

                        self.parse_sprite_context(block);
                    }
                    7 => {
                        if self.current_list_type == TA_LIST_NONE {
                            println!(
                                "TA error: vertex in listless mode, state {}",
                                self.state as u32
                            );
                            self.bad_input_error();
                            return;
                        }
                        self.state = ProcessingState::InPolygon;
                        self.parse_vertex(block);

                        match self.state {
                            ProcessingState::ExpectEndVertexBlock2 => {}
                            ProcessingState::ExpectVertexBlock2 => {
                                if ta_is_end_vertex(block.word(0)) {
                                    self.state = ProcessingState::ExpectEndVertexBlock2;
                                }
                            }
                            _ => {
                                if ta_is_end_vertex(block.word(0)) {
                                    if self.vertex_count < 3 {
                                        self.bad_input_error();
                                    } else {
                                        self.commit_polygon();
                                    }
                                    self.vertex_count = 0;
                                    self.poly_parity = 0;
                                    self.state = ProcessingState::InList;
                                } else if self.vertex_count == self.max_vertex {
                                    self.split_polygon();
                                }
                            }
                        }
                    }
                    _ => {
                        if self.state == ProcessingState::InPolygon {
                            self.bad_input_error();
                        }
                    }
                }
            }
        }
    }
}

pub fn reset() {
    if let Ok(mut state) = TA_STATE.lock() {
        state.reset();
    }
}

pub fn init(vram: *mut u8) {
    if let Ok(mut state) = TA_STATE.lock() {
        state.init(vram);
    }
}

pub fn write(buf: &[u8]) {
    if buf.len() < 32 {
        return;
    }
    if let Ok(mut state) = TA_STATE.lock() {
        for chunk in buf.chunks_exact(32) {
            let block = TaBlock::from_bytes(chunk);
            state.process_block(&block);
        }
    }
}

pub fn write_burst(_addr: u32, data: &[u8]) {
    if data.len() < 32 {
        return;
    }
    if let Ok(mut state) = TA_STATE.lock() {
        let block = TaBlock::from_bytes(&data[..32]);
        state.process_block(&block);
    }
}

pub fn find_polygon_context(buf: &[u32]) -> Option<usize> {
    if buf.len() < 8 {
        return None;
    }
    for (index, chunk) in buf.chunks_exact(8).enumerate() {
        let cmd = ta_cmd(chunk[0]);
        if cmd == 4 || cmd == 5 {
            return Some(index * 8);
        }
    }
    None
}

pub fn write_reg(offset: u32, value: u32) {
    let offset = normalise_reg_offset(offset);
    if let Ok(mut state) = TA_STATE.lock() {
        match offset {
            0x124 => state.regs.ol_base = value,
            0x128 => state.regs.isp_base = value,
            0x12C => state.regs.ol_limit = value,
            0x130 => state.regs.isp_limit = value,
            0x134 => state.regs.next_opb = value,
            0x138 => state.regs.isp_current = value,
            0x13C => state.regs.glob_tile_clip = value,
            0x140 => state.regs.alloc_ctrl = value,
            0x164 => state.regs.next_opb_init = value,
            _ => {}
        }
    }
}

pub fn read_reg(offset: u32) -> u32 {
    let offset = normalise_reg_offset(offset);
    if let Ok(state) = TA_STATE.lock() {
        match offset {
            0x124 => state.regs.ol_base,
            0x128 => state.regs.isp_base,
            0x12C => state.regs.ol_limit,
            0x130 => state.regs.isp_limit,
            0x134 => state.regs.next_opb,
            0x138 => state.regs.isp_current,
            0x13C => state.regs.glob_tile_clip,
            0x140 => state.regs.alloc_ctrl,
            0x164 => state.regs.next_opb_init,
            _ => 0,
        }
    } else {
        0
    }
}


fn ta_read<T: Copy + std::fmt::LowerHex>(_ctx: *mut u8, offset: u32) -> T {
    panic!(
        "ta_read: Attempted read::<u{}> {:x}",
        std::mem::size_of::<T>() * 4,
        offset
    );
}

fn ta_write<T: Copy + std::fmt::LowerHex>(_ctx: *mut u8, addr: u32, value: T) {
    panic!(
        "ta_write: Attempted write::<u{}> {:x} data = {:x}",
        std::mem::size_of::<T>() * 4,
        addr,
        value
    );
}

fn ta_write256(_ctx: *mut u8, _addr: u32, value: *const u32) {
    let block: &[u8] = unsafe {
        std::slice::from_raw_parts(
            value as *const u8,
            8 * std::mem::size_of::<u32>(),
        )
    };
    write(&block);
}

pub const TA_HANDLERS: sh4_core::MemHandlers = sh4_core::MemHandlers {
    read8: ta_read::<u8>,
    read16: ta_read::<u16>,
    read32: ta_read::<u32>,
    read64: ta_read::<u64>,

    write8: ta_write::<u8>,
    write16: ta_write::<u16>,
    write32: ta_write::<u32>,
    write64: ta_write::<u64>,
    write256: ta_write256,
};