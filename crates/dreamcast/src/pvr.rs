use once_cell::sync::Lazy;
use std::{ptr, sync::Mutex};

use crate::{
    asic, dreamcast_ptr, refsw2, spg,
    system_bus::{
        SystemBus, SB_C2DLEN_ADDR, SB_C2DSTAT_ADDR, SB_C2DST_ADDR, SB_LMMODE0_ADDR, SB_PDDIR_ADDR,
        SB_PDLEN_ADDR, SB_PDSTAP_ADDR, SB_PDSTAR_ADDR, SB_PDST_ADDR, SB_SDBAAW_ADDR, SB_SDDIV_ADDR,
        SB_SDLAS_ADDR, SB_SDSTAW_ADDR, SB_SDST_ADDR, SB_SDWLT_ADDR,
    },
    ta, Dreamcast,
};
use sh4_core::{
    dmac_get_chcr, dmac_get_dmaor, dmac_get_sar, dmac_set_chcr, dmac_set_dmatcr, dmac_set_sar,
    pvr::{
        FB_R_CTRL, FB_R_CTRL_ADDR, FB_R_SIZE, FB_R_SIZE_ADDR, FB_R_SOF1_ADDR, FB_R_SOF2_ADDR,
        ID_ADDR, PVR_REG_MASK, PVR_REG_SIZE, REVISION_ADDR, SOFTRESET_ADDR, SPG_CONTROL,
        SPG_CONTROL_ADDR, SPG_STATUS, SPG_STATUS_ADDR, STARTRENDER_ADDR, TA_ALLOC_CTRL_ADDR,
        TA_GLOB_TILE_CLIP_ADDR, TA_ISP_BASE_ADDR, TA_ISP_CURRENT_ADDR, TA_ISP_LIMIT_ADDR,
        TA_LIST_CONT_ADDR, TA_LIST_INIT_ADDR, TA_NEXT_OPB_ADDR, TA_NEXT_OPB_INIT_ADDR,
        TA_OL_BASE_ADDR, TA_OL_LIMIT_ADDR, TA_YUV_TEX_BASE_ADDR, TA_YUV_TEX_CNT_ADDR,
        TA_YUV_TEX_CTRL_ADDR,
    },
    sh4mem, Sh4Ctx,
};

const PVR_BASE_ADDR: u32 = 0x005F_8000;
const DMAOR_EXPECTED: u32 = 0x8201;
const DMAOR_MASK: u32 = 0xFFFF_8201;

const SYSRAM_SIZE: usize = 16 * 1024 * 1024;
const SYSRAM_MASK: u32 = (SYSRAM_SIZE as u32) - 1;

const VRAM_SIZE_BYTES: usize = 8 * 1024 * 1024;
const VRAM_MASK: u32 = (VRAM_SIZE_BYTES as u32) - 1;
const VRAM_BANK_BIT: u32 = 0x0040_0000;

const CH2_DMA_INTERRUPT_BIT: u8 = 19;
const PVR_DMA_INTERRUPT_BIT: u8 = 11;
const SORT_DMA_INTERRUPT_BIT: u8 = 20;

const RENDER_DONE_VD_INTERRUPT_BIT: u8 = 0;
const RENDER_DONE_ISP_INTERRUPT_BIT: u8 = 1;
const RENDER_DONE_INTERRUPT_BIT: u8 = 2;

fn pvr_map32(offset32: u32) -> u32 {
    let static_bits = (VRAM_MASK - (VRAM_BANK_BIT * 2 - 1)) | 3;
    let offset_bits = (VRAM_BANK_BIT - 1) & !3;

    let bank = (offset32 & VRAM_BANK_BIT) >> VRAM_BANK_BIT.trailing_zeros();

    let base = offset32 & static_bits;
    let interleaved = (offset32 & offset_bits) << 1;
    let bank_offset = bank << 2;

    base | interleaved | bank_offset
}

struct PvrState {
    regs: [u32; PVR_REG_SIZE / 4],
}

impl Default for PvrState {
    fn default() -> Self {
        Self {
            regs: [0; PVR_REG_SIZE / 4],
        }
    }
}

static PVR_STATE: Lazy<Mutex<PvrState>> = Lazy::new(|| Mutex::new(PvrState::default()));

pub fn handles_address(addr: u32) -> bool {
    (PVR_BASE_ADDR..=PVR_BASE_ADDR + PVR_REG_MASK as u32).contains(&addr)
}

pub fn read(addr: u32, size: usize) -> u32 {
    let offset = (addr - PVR_BASE_ADDR) as usize;
    let state = PVR_STATE.lock().expect("PVR_STATE lock poisoned");
    match size {
        1 => {
            let value = state.regs[(offset & !3) / 4];
            let shift = (offset & 3) * 8;
            ((value >> shift) & 0xFF) as u32
        }
        2 => {
            let value = state.regs[(offset & !3) / 4];
            let shift = (offset & 2) * 8;
            ((value >> shift) & 0xFFFF) as u32
        }
        _ => {
            let aligned = offset & !3;
            match aligned {
                x if x == ID_ADDR as usize => 0x17FD_11DB,
                x if x == REVISION_ADDR as usize => 0x0000_0011,
                x if is_ta_register(x) => ta::read_reg(x as u32),
                x => state.regs[x / 4],
            }
        }
    }
}

pub fn write(addr: u32, size: usize, value: u32) {
    let offset = (addr - PVR_BASE_ADDR) as usize;
    let mut state = PVR_STATE.lock().expect("PVR_STATE lock poisoned");
    match size {
        1 => {
            let idx = (offset & !3) / 4;
            let shift = (offset & 3) * 8;
            let mask = !(0xFFu32 << shift);
            let current = state.regs[idx];
            state.regs[idx] = (current & mask) | ((value as u32 & 0xFF) << shift);
        }
        2 => {
            let idx = (offset & !3) / 4;
            let shift = (offset & 2) * 8;
            let mask = !(0xFFFFu32 << shift);
            let current = state.regs[idx];
            state.regs[idx] = (current & mask) | ((value as u32 & 0xFFFF) << shift);
        }
        _ => match offset & !3 {
            x if x == ID_ADDR as usize || x == REVISION_ADDR as usize => {}
            x if x == TA_YUV_TEX_CNT_ADDR as usize => {}
            x if x == STARTRENDER_ADDR as usize => {
                println!("PVR: STARTRENDER write (value=0x{value:08X})");
                refsw2::refsw2_render(
                    dreamcast_mut()
                        .expect("Dreamcast instance not initialised")
                        .video_ram
                        .as_mut_ptr(),
                    state.regs.as_ptr(),
                );
                // TODO: Hook up renderer start when available.
                asic::raise_normal(RENDER_DONE_INTERRUPT_BIT);
                asic::raise_normal(RENDER_DONE_ISP_INTERRUPT_BIT);
                asic::raise_normal(RENDER_DONE_VD_INTERRUPT_BIT);
            }
            x if x == TA_LIST_INIT_ADDR as usize => {
                state.regs[x / 4] = value;
                if (value >> 31) != 0 {
                    if let Some(dc) = unsafe { dreamcast_ptr().as_mut() } {
                        ta::init(dc.video_ram.as_mut_ptr());
                    }
                    let isp_base = read_reg(&state, TA_ISP_BASE_ADDR as usize);
                    write_reg(&mut state, TA_ISP_CURRENT_ADDR as usize, isp_base);
                    ta::write_reg(TA_ISP_CURRENT_ADDR, isp_base);
                }
                ta::write_reg(x as u32, value);
                return;
            }
            x if x == SOFTRESET_ADDR as usize => {
                state.regs[x / 4] = value;
                if (value & 1) != 0 {
                    ta::reset();
                }
                return;
            }
            x if x == TA_LIST_CONT_ADDR as usize => {
                state.regs[x / 4] = value;
                ta::write_reg(x as u32, value);
                return;
            }
            x if x == TA_YUV_TEX_BASE_ADDR as usize || x == TA_YUV_TEX_CTRL_ADDR as usize => {
                state.regs[x / 4] = value;
                return;
            }
            x if is_ta_register(x) => {
                state.regs[x / 4] = value;
                ta::write_reg(x as u32, value);
                return;
            }
            x => {
                state.regs[x / 4] = value;
            }
        },
    }
}

fn read_reg(state: &PvrState, offset: usize) -> u32 {
    state.regs[(offset & !3) / 4]
}

fn write_reg(state: &mut PvrState, offset: usize, value: u32) {
    state.regs[(offset & !3) / 4] = value;
}

fn is_ta_register(offset: usize) -> bool {
    matches!(
        offset,
        x if x == TA_OL_BASE_ADDR as usize
            || x == TA_ISP_BASE_ADDR as usize
            || x == TA_OL_LIMIT_ADDR as usize
            || x == TA_ISP_LIMIT_ADDR as usize
            || x == TA_NEXT_OPB_ADDR as usize
            || x == TA_ISP_CURRENT_ADDR as usize
            || x == TA_GLOB_TILE_CLIP_ADDR as usize
            || x == TA_ALLOC_CTRL_ADDR as usize
            || x == TA_NEXT_OPB_INIT_ADDR as usize
    )
}

pub(crate) fn sb_c2dst_write(ctx: *mut u32, _addr: u32, data: u32) {
    let sb = unsafe { &mut *(ctx as *mut SystemBus) };
    sb.store_reg(SB_C2DST_ADDR, data & 1);
    if (data & 1) == 0 {
        return;
    }

    if let Err(err) = perform_channel_2_dma(sb) {
        println!("SB: Channel 2 DMA start failed: {err}");
        sb.store_reg(SB_C2DST_ADDR, 0);
    }
}

pub(crate) fn sb_sdst_write(ctx: *mut u32, _addr: u32, data: u32) {
    let sb = unsafe { &mut *(ctx as *mut SystemBus) };
    sb.store_reg(SB_SDST_ADDR, data & 1);
    if (data & 1) == 0 {
        return;
    }

    if let Err(err) = perform_sort_dma(sb) {
        println!("SB: Sort DMA start failed: {err}");
        sb.store_reg(SB_SDST_ADDR, 0);
    }
}

pub(crate) fn sb_pdst_write(ctx: *mut u32, _addr: u32, data: u32) {
    let sb = unsafe { &mut *(ctx as *mut SystemBus) };
    sb.store_reg(SB_PDST_ADDR, data & 1);
    if (data & 1) == 0 {
        return;
    }

    if let Err(err) = perform_pvr_dma(sb) {
        println!("SB: PVR DMA start failed: {err}");
        sb.store_reg(SB_PDST_ADDR, 0);
    }
}

fn perform_channel_2_dma(sb: &mut SystemBus) -> Result<(), String> {
    let dc = dreamcast_mut().ok_or_else(|| "Dreamcast instance not initialised".to_string())?;
    let sh4ctx: *mut Sh4Ctx = &mut dc.ctx;

    let dmaor = dmac_get_dmaor();
    if (dmaor & DMAOR_MASK) != DMAOR_EXPECTED {
        return Err(format!("DMAOR has invalid settings ({dmaor:08X})"));
    }

    let mut src = dmac_get_sar(2);
    let dst = sb.load_reg(SB_C2DSTAT_ADDR);
    let len = sb.load_reg(SB_C2DLEN_ADDR);

    if len == 0 {
        sb.store_reg(SB_C2DST_ADDR, 0);
        return Ok(());
    }

    if (len & 0x1F) != 0 {
        return Err(format!("SB_C2DLEN has invalid size ({len:08X})"));
    }

    const TA_CMD_BEGIN: u32 = 0x1000_0000;
    const TA_CMD_END: u32 = 0x10FF_FFFF;
    const TEX_BEGIN: u32 = 0x1100_0000;
    const TEX_END: u32 = 0x11FF_FFE0;
    const LNMODE1_BEGIN: u32 = 0x1300_0000;
    const LNMODE1_END: u32 = 0x13FF_FFE0;

    if (dst >= TA_CMD_BEGIN) && (dst <= TA_CMD_END) {
        ch2_dma_copy_to_ta(sh4ctx, src, len)?;
        src = src.wrapping_add(len);
    } else if (dst >= TEX_BEGIN) && (dst <= TEX_END) {
        let lmmode0 = sb.load_reg(SB_LMMODE0_ADDR);
        sb.store_reg(SB_C2DSTAT_ADDR, dst.wrapping_add(len));
        ch2_dma_copy_texture(dc, sh4ctx, src, dst, len, lmmode0)?;
        src = src.wrapping_add(len);
    } else if (dst >= LNMODE1_BEGIN) && (dst <= LNMODE1_END) {
        return Err(format!(
            "SB_C2DSTAT address {:08X} (LNMODE1) is not implemented",
            dst
        ));
    } else {
        return Err(format!("SB_C2DSTAT has invalid address ({dst:08X})"));
    }

    dmac_set_sar(2, src);
    let mut chcr = dmac_get_chcr(2);
    chcr &= !1;
    dmac_set_chcr(2, chcr);
    dmac_set_dmatcr(2, 0);

    sb.store_reg(SB_C2DST_ADDR, 0);
    sb.store_reg(SB_C2DLEN_ADDR, 0);
    asic::raise_normal(CH2_DMA_INTERRUPT_BIT);

    Ok(())
}

fn ch2_dma_copy_to_ta(ctx: *mut Sh4Ctx, mut src: u32, mut len: u32) -> Result<(), String> {
    let mut block = [0u8; 32];
    while len > 0 {
        read_block(ctx, src, &mut block);
        ta::write(&block);
        src = src.wrapping_add(32);
        len -= 32;
    }
    Ok(())
}

fn ch2_dma_copy_texture(
    dc: &mut Dreamcast,
    ctx: *mut Sh4Ctx,
    mut src: u32,
    dst: u32,
    mut len: u32,
    lmmode0: u32,
) -> Result<(), String> {
    if lmmode0 == 0 {
        let mut dst_offset = dst & 0x00FF_FFFF;
        let mut block = [0u8; 32];
        while len > 0 {
            read_block(ctx, src, &mut block);
            write_vram_linear(dc, dst_offset, &block);
            src = src.wrapping_add(32);
            dst_offset = dst_offset.wrapping_add(32);
            len -= 32;
        }
    } else {
        let mut vram_addr = (dst & 0x00FF_FFFF) | 0xA500_0000;
        while len > 0 {
            let value = read_u32(ctx, src);
            write_vram_area1_32(dc, vram_addr, value);
            src = src.wrapping_add(4);
            vram_addr = vram_addr.wrapping_add(4);
            len -= 4;
        }
    }
    Ok(())
}

fn perform_pvr_dma(sb: &mut SystemBus) -> Result<(), String> {
    let dc = dreamcast_mut().ok_or_else(|| "Dreamcast instance not initialised".to_string())?;
    let sh4ctx: *mut Sh4Ctx = &mut dc.ctx;

    let dmaor = dmac_get_dmaor();
    if (dmaor & DMAOR_MASK) != DMAOR_EXPECTED {
        return Err(format!("DMAOR has invalid settings ({dmaor:08X})"));
    }

    let src = sb.load_reg(SB_PDSTAR_ADDR);
    let dst = sb.load_reg(SB_PDSTAP_ADDR);
    let len = sb.load_reg(SB_PDLEN_ADDR);
    let dir = sb.load_reg(SB_PDDIR_ADDR) & 1;

    if (len & 0x1F) != 0 {
        return Err(format!("SB_PDLEN has invalid size ({len:08X})"));
    }

    if dir == 0 {
        // System -> PVR
        copy_system_to_vram(dc, sh4ctx, src, dst, len)?;
    } else {
        // PVR -> System
        copy_vram_to_system(dc, sh4ctx, dst, src, len)?;
    }

    dmac_set_sar(0, src.wrapping_add(len));
    let mut chcr = dmac_get_chcr(0);
    chcr &= !1;
    dmac_set_chcr(0, chcr);
    dmac_set_dmatcr(0, 0);

    sb.store_reg(SB_PDST_ADDR, 0);
    asic::raise_normal(PVR_DMA_INTERRUPT_BIT);

    Ok(())
}

fn copy_system_to_vram(
    dc: &mut Dreamcast,
    ctx: *mut Sh4Ctx,
    mut src: u32,
    mut dst: u32,
    mut len: u32,
) -> Result<(), String> {
    let mut block = [0u8; 32];
    while len > 0 {
        read_block(ctx, src, &mut block);
        write_vram_linear(dc, dst & 0x00FF_FFFF, &block);
        src = src.wrapping_add(32);
        dst = dst.wrapping_add(32);
        len -= 32;
    }
    Ok(())
}

fn copy_vram_to_system(
    dc: &mut Dreamcast,
    ctx: *mut Sh4Ctx,
    mut src: u32,
    mut dst: u32,
    mut len: u32,
) -> Result<(), String> {
    let mut block = [0u8; 32];
    while len > 0 {
        read_vram_linear(dc, src & 0x00FF_FFFF, &mut block);
        write_block(ctx, dst, &block);
        src = src.wrapping_add(32);
        dst = dst.wrapping_add(32);
        len -= 32;
    }
    Ok(())
}

fn perform_sort_dma(sb: &mut SystemBus) -> Result<(), String> {
    let dc = dreamcast_mut().ok_or_else(|| "Dreamcast instance not initialised".to_string())?;

    sb.store_reg(SB_SDDIV_ADDR, 0);
    let mut link_addr = calculate_start_link_addr(sb, &dc.sys_ram[..]);
    let link_base = sb.load_reg(SB_SDBAAW_ADDR);
    let sdl_as = sb.load_reg(SB_SDLAS_ADDR) & 1;

    const END_LINK: u32 = 1;
    const RESTART_LINK: u32 = 2;

    while link_addr != END_LINK {
        let mut current_link = link_addr;
        if sdl_as == 1 {
            current_link = current_link.wrapping_mul(32);
        }

        let ea = link_base.wrapping_add(current_link) & SYSRAM_MASK;
        let block_count = read_sysram_u32(&dc.sys_ram[..], ea.wrapping_add(0x18));
        let next_link = read_sysram_u32(&dc.sys_ram[..], ea.wrapping_add(0x1C));
        transfer_sort_blocks(&dc.sys_ram[..], ea, block_count);

        link_addr = next_link;
        if link_addr == RESTART_LINK {
            link_addr = calculate_start_link_addr(sb, &dc.sys_ram[..]);
        }
    }

    sb.store_reg(SB_SDST_ADDR, 0);
    asic::raise_normal(SORT_DMA_INTERRUPT_BIT);
    Ok(())
}

fn calculate_start_link_addr(sb: &mut SystemBus, sys_ram: &[u8]) -> u32 {
    let table_addr = sb.load_reg(SB_SDSTAW_ADDR);
    let idx = sb.load_reg(SB_SDDIV_ADDR);
    let width = sb.load_reg(SB_SDWLT_ADDR) & 1;
    let entry_addr = if width == 0 {
        table_addr.wrapping_add(idx.wrapping_mul(2))
    } else {
        table_addr.wrapping_add(idx.wrapping_mul(4))
    };

    let value = if width == 0 {
        read_sysram_u16(sys_ram, entry_addr) as u32
    } else {
        read_sysram_u32(sys_ram, entry_addr)
    };

    sb.store_reg(SB_SDDIV_ADDR, idx.wrapping_add(1));
    value
}

fn transfer_sort_blocks(sys_ram: &[u8], base_addr: u32, count: u32) {
    if count == 0 {
        return;
    }

    let mut addr = base_addr;
    for _ in 0..count {
        let mut block = [0u8; 32];
        read_sysram_block(sys_ram, addr, &mut block);
        ta::write(&block);
        addr = addr.wrapping_add(32);
    }
}

pub fn present_for_texture() -> Option<(Vec<u8>, usize, usize)> {
    let (fb_r_ctrl_val, fb_r_size_val, fb_r_sof1, fb_r_sof2) = {
        let state = PVR_STATE.lock().ok()?;
        (
            read_reg(&state, FB_R_CTRL_ADDR as usize),
            read_reg(&state, FB_R_SIZE_ADDR as usize),
            read_reg(&state, FB_R_SOF1_ADDR as usize),
            read_reg(&state, FB_R_SOF2_ADDR as usize),
        )
    };

    let fb_ctrl = FB_R_CTRL(fb_r_ctrl_val);
    if !fb_ctrl.fb_enable() {
        return None;
    }

    let fb_size = FB_R_SIZE(fb_r_size_val);
    let mut width = ((fb_size.fb_x_size() as i32) + 1) << 1;
    let field_height = (fb_size.fb_y_size() as i32) + 1;
    if width <= 0 || field_height <= 0 {
        return None;
    }

    let mut modulus = ((fb_size.fb_modulus() as i32) - 1) << 1;
    let depth = fb_ctrl.fb_depth();
    let mut bytes_per_pixel = match depth {
        0 | 1 => 2,
        2 => 3,
        3 => 4,
        _ => return None,
    };

    if depth == 2 {
        width = (width * 2) / 3;
        modulus = (modulus * 2) / 3;
    } else if depth == 3 {
        width /= 2;
        modulus /= 2;
    }

    if width <= 0 {
        return None;
    }

    let width_usize = width as usize;
    let field_height_usize = field_height as usize;
    if width_usize == 0 || field_height_usize == 0 {
        return None;
    }

    let spg_control_val = spg::read(PVR_BASE_ADDR + SPG_CONTROL_ADDR, 4);
    let spg_status_val = spg::read(PVR_BASE_ADDR + SPG_STATUS_ADDR, 4);
    let spg_control = SPG_CONTROL(spg_control_val);
    let spg_status = SPG_STATUS(spg_status_val);
    let interlace = spg_control.interlace();
    let fieldnum = spg_status.fieldnum();

    let output_height = if interlace {
        field_height_usize.saturating_mul(2)
    } else {
        field_height_usize
    };
    if output_height == 0 {
        return None;
    }

    let mut pixels = vec![0u8; width_usize * output_height * 4];
    let line_stride = width_usize * 4;
    let mut dst_row_offset = if interlace && fieldnum {
        line_stride
    } else {
        0
    };
    let dst_row_step = if interlace {
        line_stride * 2
    } else {
        line_stride
    };

    let mut addr = if interlace && fieldnum {
        fb_r_sof2
    } else {
        fb_r_sof1
    };

    let fb_concat = fb_ctrl.fb_concat() as u8;
    let fb_concat_green = fb_concat >> 1;
    let row_increment = ((modulus as i64) * (bytes_per_pixel as i64)) as i32;

    let dc = dreamcast_mut()?;
    let vram = dc.video_ram.as_mut_ptr();

    match depth {
        0 => {
            let bpp = bytes_per_pixel as u32;
            for _ in 0..field_height_usize {
                if dst_row_offset + line_stride > pixels.len() {
                    break;
                }
                let row = &mut pixels[dst_row_offset..dst_row_offset + line_stride];
                let mut pixel_addr = addr;
                for x in 0..width_usize {
                    let base = x * 4;
                    let src = pvr_read_area1_16(vram, pixel_addr);
                    let b = (((src >> 0) & 0x1F) << 3) as u8;
                    let g = (((src >> 5) & 0x1F) << 3) as u8;
                    let r = (((src >> 10) & 0x1F) << 3) as u8;
                    row[base] = r.wrapping_add(fb_concat);
                    row[base + 1] = g.wrapping_add(fb_concat);
                    row[base + 2] = b.wrapping_add(fb_concat);
                    row[base + 3] = 0xFF;
                    pixel_addr = pixel_addr.wrapping_add(bpp);
                }
                addr = pixel_addr.wrapping_add(row_increment as u32);
                dst_row_offset = dst_row_offset.saturating_add(dst_row_step);
            }
        }
        1 => {
            let bpp = bytes_per_pixel as u32;
            for _ in 0..field_height_usize {
                if dst_row_offset + line_stride > pixels.len() {
                    break;
                }
                let row = &mut pixels[dst_row_offset..dst_row_offset + line_stride];
                let mut pixel_addr = addr;
                for x in 0..width_usize {
                    let base = x * 4;
                    let src = pvr_read_area1_16(vram, pixel_addr);
                    let b = (((src >> 0) & 0x1F) << 3) as u8;
                    let g = (((src >> 5) & 0x3F) << 2) as u8;
                    let r = (((src >> 11) & 0x1F) << 3) as u8;
                    row[base] = r.wrapping_add(fb_concat);
                    row[base + 1] = g.wrapping_add(fb_concat_green);
                    row[base + 2] = b.wrapping_add(fb_concat);
                    row[base + 3] = 0xFF;
                    pixel_addr = pixel_addr.wrapping_add(bpp);
                }
                addr = pixel_addr.wrapping_add(row_increment as u32);
                dst_row_offset = dst_row_offset.saturating_add(dst_row_step);
            }
        }
        2 => {
            bytes_per_pixel = 3;
            let bpp = bytes_per_pixel as u32;
            for _ in 0..field_height_usize {
                if dst_row_offset + line_stride > pixels.len() {
                    break;
                }
                let row = &mut pixels[dst_row_offset..dst_row_offset + line_stride];
                let mut pixel_addr = addr;
                for x in 0..width_usize {
                    let base = x * 4;
                    let sample_addr = pixel_addr;
                    let src = if (sample_addr & 1) != 0 {
                        pvr_read_area1_32(vram, sample_addr.wrapping_sub(1))
                    } else {
                        pvr_read_area1_32(vram, sample_addr)
                    };
                    if (sample_addr & 1) != 0 {
                        row[base + 2] = ((src >> 0) & 0xFF) as u8;
                        row[base + 1] = ((src >> 8) & 0xFF) as u8;
                        row[base + 0] = ((src >> 16) & 0xFF) as u8;
                    } else {
                        row[base + 2] = ((src >> 8) & 0xFF) as u8;
                        row[base + 1] = ((src >> 16) & 0xFF) as u8;
                        row[base + 0] = ((src >> 24) & 0xFF) as u8;
                    }
                    row[base + 3] = 0xFF;
                    pixel_addr = pixel_addr.wrapping_add(bpp);
                }
                addr = pixel_addr.wrapping_add(row_increment as u32);
                dst_row_offset = dst_row_offset.saturating_add(dst_row_step);
            }
        }
        3 => {
            bytes_per_pixel = 4;
            let bpp = bytes_per_pixel as u32;
            for _ in 0..field_height_usize {
                if dst_row_offset + line_stride > pixels.len() {
                    break;
                }
                let row = &mut pixels[dst_row_offset..dst_row_offset + line_stride];
                let mut pixel_addr = addr;
                for x in 0..width_usize {
                    let base = x * 4;
                    let src = pvr_read_area1_32(vram, pixel_addr);
                    row[base + 2] = ((src >> 0) & 0xFF) as u8;
                    row[base + 1] = ((src >> 8) & 0xFF) as u8;
                    row[base + 0] = ((src >> 16) & 0xFF) as u8;
                    row[base + 3] = 0xFF;
                    pixel_addr = pixel_addr.wrapping_add(bpp);
                }
                addr = pixel_addr.wrapping_add(row_increment as u32);
                dst_row_offset = dst_row_offset.saturating_add(dst_row_step);
            }
        }
        _ => return None,
    }

    Some((pixels, width_usize, output_height))
}

fn dreamcast_mut() -> Option<&'static mut Dreamcast> {
    let ptr = dreamcast_ptr();
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { &mut *ptr })
    }
}

fn read_u32(ctx: *mut Sh4Ctx, addr: u32) -> u32 {
    let mut value = 0u32;
    sh4mem::read_mem(ctx, addr, &mut value);
    value
}

fn read_block(ctx: *mut Sh4Ctx, mut addr: u32, buf: &mut [u8]) {
    for chunk in buf.chunks_exact_mut(4) {
        let value = read_u32(ctx, addr);
        chunk.copy_from_slice(&value.to_le_bytes());
        addr = addr.wrapping_add(4);
    }
}

fn write_block(ctx: *mut Sh4Ctx, mut addr: u32, buf: &[u8]) {
    for chunk in buf.chunks_exact(4) {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        let _ = sh4mem::write_mem::<u32>(ctx, addr, value);
        addr = addr.wrapping_add(4);
    }
}

fn write_vram_linear(dc: &mut Dreamcast, addr: u32, data: &[u8]) {
    let base = (addr & VRAM_MASK) as usize;
    let vram_len = dc.video_ram.len();
    let first_len = vram_len.saturating_sub(base).min(data.len());
    if first_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                dc.video_ram.as_mut_ptr().add(base),
                first_len,
            );
        }
    }
    if first_len < data.len() {
        let remaining = data.len() - first_len;
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr().add(first_len),
                dc.video_ram.as_mut_ptr(),
                remaining,
            );
        }
    }
}

fn read_vram_linear(dc: &Dreamcast, addr: u32, buf: &mut [u8]) {
    let base = (addr & VRAM_MASK) as usize;
    let vram_len = dc.video_ram.len();
    let first_len = vram_len.saturating_sub(base).min(buf.len());
    if first_len > 0 {
        buf[..first_len].copy_from_slice(&dc.video_ram[base..base + first_len]);
    }
    if first_len < buf.len() {
        let remaining = buf.len() - first_len;
        buf[first_len..].copy_from_slice(&dc.video_ram[..remaining]);
    }
}

fn write_vram_area1_32(dc: &mut Dreamcast, addr: u32, value: u32) {
    let mapped = pvr_map32(addr & VRAM_MASK) as usize;
    if mapped + 4 > dc.video_ram.len() {
        return;
    }
    unsafe {
        let ptr = dc.video_ram.as_mut_ptr().add(mapped) as *mut u32;
        ptr.write_unaligned(value);
    }
}

fn read_sysram_u16(sys_ram: &[u8], addr: u32) -> u16 {
    let offset = (addr & SYSRAM_MASK) as usize;
    let hi = sys_ram[(offset + 1) % sys_ram.len()];
    u16::from_le_bytes([sys_ram[offset], hi])
}

fn read_sysram_u32(sys_ram: &[u8], addr: u32) -> u32 {
    let offset = (addr & SYSRAM_MASK) as usize;
    let mut bytes = [0u8; 4];
    for i in 0..4 {
        bytes[i] = sys_ram[(offset + i) % sys_ram.len()];
    }
    u32::from_le_bytes(bytes)
}

fn read_sysram_block(sys_ram: &[u8], addr: u32, buf: &mut [u8]) {
    let offset = (addr & SYSRAM_MASK) as usize;
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = sys_ram[(offset + i) % sys_ram.len()];
    }
}

fn pvr_read_area1_8(_ctx: *mut u8, _addr: u32) -> u8 {
    println!("8-bit VRAM reads are not possible");
    0
}

fn pvr_read_area1_16(ctx: *mut u8, addr: u32) -> u16 {
    unsafe {
        let vram = ctx;
        let offset = pvr_map32(addr);
        ptr::read_unaligned(vram.add(offset as usize) as *const u16)
    }
}

fn pvr_read_area1_32(ctx: *mut u8, addr: u32) -> u32 {
    unsafe {
        let vram = ctx;
        let offset = pvr_map32(addr);
        ptr::read_unaligned(vram.add(offset as usize) as *const u32)
    }
}

fn pvr_read_area1_64(ctx: *mut u8, addr: u32) -> u64 {
    return (pvr_read_area1_32(ctx, addr) as u64)
        | ((pvr_read_area1_32(ctx, addr.wrapping_add(4)) as u64) << 32);
}

fn pvr_write_area1_8(_ctx: *mut u8, _addr: u32, _data: u8) {
    println!("8-bit VRAM writes are not possible");
}

fn pvr_write_area1_16(ctx: *mut u8, addr: u32, data: u16) {
    unsafe {
        let vram = ctx;
        let vaddr = addr & VRAM_MASK;
        let offset = pvr_map32(vaddr);
        ptr::write_unaligned(vram.add(offset as usize) as *mut u16, data);
    }
}

fn pvr_write_area1_32(ctx: *mut u8, addr: u32, data: u32) {
    unsafe {
        let vram = ctx;
        let vaddr = addr & VRAM_MASK;
        let offset = pvr_map32(vaddr);
        ptr::write_unaligned(vram.add(offset as usize) as *mut u32, data);
    }
}

fn pvr_write_area1_64(ctx: *mut u8, addr: u32, data: u64) {
    pvr_write_area1_32(ctx, addr, data as u32);
    pvr_write_area1_32(ctx, addr.wrapping_add(4), (data >> 32) as u32);
}

pub const PVR_32BIT_HANDLERS: sh4_core::MemHandlers = sh4_core::MemHandlers {
    read8: pvr_read_area1_8,
    read16: pvr_read_area1_16,
    read32: pvr_read_area1_32,
    read64: pvr_read_area1_64,

    write8: pvr_write_area1_8,
    write16: pvr_write_area1_16,
    write32: pvr_write_area1_32,
    write64: pvr_write_area1_64,
    write256: sh4_core::dummy_write256,
};
