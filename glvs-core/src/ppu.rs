//! Emulates the NTSC 2C02.

mod registers;
mod video;

extern crate alloc;
use alloc::boxed::Box;
use core::ops::{Index, IndexMut};

use arbitrary_int::{u3, u5};

use crate::cartridge::{CHR_ROM_SIZE, Cartridge, Orientation};
use crate::ppu::registers::{CtrlBits, Loopy, MaskBits, StatusBits};
use crate::ppu::video::{COLORS, Rgb8};
use crate::util;

const PALETTE_RAM_START: u16 = 0x3F00;
const PRE_RENDER_SCANLINE: u32 = 261;

/// Canvas abstraction for the PPU to draw on.
pub trait Canvas {
    fn draw_point(&mut self, x: u32, y: u32, rgb: (u8, u8, u8));
}

#[derive(Debug)]
pub struct Ppu {
    nmtables: Box<[NmTable; 2]>,
    palettes: [u8; 32],
    pub oam: [Object; 64],
    line_oam: [Object; 8],
    regs: Registers,
    data_buffer: u8,
    second_write: bool,
    nmtable_mirroring: Orientation,
    scanline: u32,
    dot: u32,
    odd_frame: bool,
    sprite_count: u8,
    zero_hit_possible: bool,
    zero_being_rendered: bool,
    sprite_shifter_0: [u8; 8],
    sprite_shifter_1: [u8; 8],
    bg_pattern_shifter0: u16,
    bg_pattern_shifter1: u16,
    bg_attrib_shifter0: u16,
    bg_attrib_shifter1: u16,
    bg_next_tile_lsbits: u8,
    bg_next_tile_msbits: u8,
    bg_next_attrib: u8,
    bg_next_tile_idx: u8,
    pub frame_complete: bool,
    pub nmi: bool,
}

type NmTable = [u8; 32 * 32];

#[derive(Debug, Default)]
struct Registers {
    status: StatusBits,
    mask: MaskBits,
    ctrl: CtrlBits,
    oam_addr: u8,
    temp_addr: Loopy,
    addr: Loopy,
    fine_x: u3,
}

#[derive(Debug, Clone, Copy)]
pub enum RegisterKind {
    Ctrl,
    Mask,
    Status,
    OamAddr,
    OamData,
    Scroll,
    Addr,
    Data,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Object {
    x: u8,
    y: u8,
    idx: u8,
    attr: u8,
}

impl Index<u8> for Object {
    type Output = u8;

    fn index(&self, index: u8) -> &Self::Output {
        match index {
            0 => &self.y,
            1 => &self.idx,
            2 => &self.attr,
            3 => &self.x,
            _ => unreachable!(),
        }
    }
}

impl IndexMut<u8> for Object {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        match index {
            0 => &mut self.y,
            1 => &mut self.idx,
            2 => &mut self.attr,
            3 => &mut self.x,
            _ => unreachable!(),
        }
    }
}

impl Ppu {
    #[must_use]
    pub fn new(cart: &Cartridge) -> Self {
        Self {
            nmtables: {
                let mut result = Box::<[NmTable; 2]>::new_uninit();
                // SAFETY:
                // We're zeroing out the memory, which is a valid bit pattern for [NmTable; 2].
                unsafe {
                    result.as_mut_ptr().write_bytes(0, 1);
                    result.assume_init()
                }
            },
            palettes: [0; 32],
            oam: [Object::default(); 64],
            line_oam: [Object::default(); 8],
            regs: Registers::default(),
            data_buffer: 0,
            second_write: false,
            nmtable_mirroring: cart.nmtable_mirroring(),
            scanline: PRE_RENDER_SCANLINE,
            dot: 0,
            odd_frame: false,
            frame_complete: false,
            nmi: false,
            bg_pattern_shifter0: 0,
            bg_pattern_shifter1: 0,
            bg_attrib_shifter0: 0,
            bg_attrib_shifter1: 0,
            bg_next_attrib: 0,
            bg_next_tile_idx: 0,
            bg_next_tile_lsbits: 0,
            bg_next_tile_msbits: 0,
            sprite_shifter_0: [0; 8],
            sprite_shifter_1: [0; 8],
            sprite_count: 0,
            zero_hit_possible: false,
            zero_being_rendered: false,
        }
    }

    /// One PPU cycle draws a single pixel. Implemented following
    /// <https://www.nesdev.org/wiki/File:Ppu.svg>.
    pub fn cycle(&mut self, cart: &Cartridge, canvas: &mut impl Canvas) {
        let is_rendering = self.regs.mask.bg_rendering() || self.regs.mask.sprite_rendering();

        match self.scanline {
            PRE_RENDER_SCANLINE | 0..240 => {
                self.handle_fetching_scanline(cart, is_rendering);

                // Foreground
                if self.dot == 257 {
                    self.line_oam.fill(Object::default());
                    self.sprite_count = 0;
                    self.sprite_shifter_0.fill(0);
                    self.sprite_shifter_1.fill(0);

                    self.zero_hit_possible = false;
                    for (i, obj) in self.oam.into_iter().enumerate() {
                        let diff = self.scanline as i16 - i16::from(obj.y);
                        if (0..if self.regs.ctrl.large_sprite() { 16 } else { 8 }).contains(&diff)
                            && self.sprite_count < 8
                        {
                            if i == 0 {
                                self.zero_hit_possible = true;
                            }
                            self.line_oam[usize::from(self.sprite_count)] = obj;
                            self.sprite_count += 1;
                        }
                    }

                    // set flags
                    self.regs.status.set_sprite_overflow(self.sprite_count > 8);
                }

                // now we're definitely no longer cycle-accurate
                if self.dot == 340 {
                    for i in 0..self.sprite_count {
                        let sprite = &self.line_oam[usize::from(i)];

                        let ydiff = self.scanline - u32::from(sprite.y);
                        let sprite_pattern0 = if !self.regs.ctrl.large_sprite() {
                            if util::bit(sprite.attr, 7) {
                                // flipped vertically
                                calculate_nmtable_addr(
                                    self.regs.ctrl.sprite_addr(),
                                    sprite.idx,
                                    7 - ydiff as u8,
                                )
                            } else {
                                calculate_nmtable_addr(
                                    self.regs.ctrl.sprite_addr(),
                                    sprite.idx,
                                    ydiff as u8,
                                )
                            }
                        } else {
                            0
                        };

                        let sprite_pattern1 = sprite_pattern0 + 8;
                        let mut sprite_pattern_lsbits = self.read(cart, sprite_pattern0);
                        let mut sprite_pattern_msbits = self.read(cart, sprite_pattern1);
                        if util::bit(sprite.attr, 6) {
                            sprite_pattern_lsbits = sprite_pattern_lsbits.reverse_bits();
                            sprite_pattern_msbits = sprite_pattern_msbits.reverse_bits();
                        }
                        self.sprite_shifter_0[usize::from(i)] = sprite_pattern_lsbits;
                        self.sprite_shifter_1[usize::from(i)] = sprite_pattern_msbits;
                    }
                }
            }
            240 => {
                // do nothing.
            }
            241..=260 => {
                if self.scanline == 241 && self.dot == 1 {
                    self.regs.status.set_vblank(true);
                    if self.regs.ctrl.vblank_nmi() {
                        self.nmi = true;
                    }
                }
            }
            _ => unreachable!("scanline {} is out-of-bounds", self.scanline),
        }

        let bg_pixel_idx = if is_rendering {
            let lsb = shift_out(self.bg_pattern_shifter0, self.regs.fine_x.value());
            let msb = shift_out(self.bg_pattern_shifter1, self.regs.fine_x.value());
            msb << 1 | lsb
        } else {
            0
        };
        let bg_palette_idx = if is_rendering {
            let lsb = shift_out(self.bg_attrib_shifter0, self.regs.fine_x.value());
            let msb = shift_out(self.bg_attrib_shifter1, self.regs.fine_x.value());
            msb << 1 | lsb
        } else {
            0
        };

        let mut fg_pixel_idx = 0;
        let mut fg_palette_idx = 0;
        let mut fg_priority = false;

        if self.regs.mask.sprite_rendering() {
            self.zero_being_rendered = false;

            for i in 0..usize::from(self.sprite_count) {
                let obj = &self.line_oam[i];
                if obj.x == 0 {
                    fg_pixel_idx = {
                        let lsb = self.sprite_shifter_0[i] >> 7 & 1;
                        let msb = self.sprite_shifter_1[i] >> 7 & 1;
                        msb << 1 | lsb
                    };
                    fg_palette_idx = (obj.attr & 0b11) + 4;
                    fg_priority = !util::bit(obj.attr, 5);

                    if fg_pixel_idx != 0 {
                        self.zero_being_rendered = i == 0;
                        break;
                    }
                }
            }
        }

        // Priority calculation
        let (pixel, palette) = match (bg_pixel_idx, fg_pixel_idx) {
            (0, 0) => (0, 0),
            (0, _) => (fg_pixel_idx, fg_palette_idx),
            (_, 0) => (bg_pixel_idx, bg_palette_idx),
            (_, _) => {
                if self.zero_being_rendered && self.zero_hit_possible && is_rendering {
                    if !self.regs.mask.show_right_sprites() && !self.regs.mask.show_left_sprites() {
                        if (9..258).contains(&self.dot) {
                            self.regs.status.set_sprite_zero_hit(true);
                        }
                    } else if (1..258).contains(&self.dot) {
                        self.regs.status.set_sprite_zero_hit(true);
                    }
                }
                if fg_priority {
                    (fg_pixel_idx, fg_palette_idx)
                } else {
                    (bg_pixel_idx, bg_palette_idx)
                }
            }
        };

        let Rgb8(r, g, b) = self.color_from_palette(palette, pixel);
        canvas.draw_point(self.dot, self.scanline, (r, g, b));

        self.advance_dot(is_rendering);
    }

    fn handle_fetching_scanline(&mut self, cart: &Cartridge, is_rendering: bool) {
        if self.scanline == PRE_RENDER_SCANLINE && self.dot == 1 {
            self.regs.status.set_vblank(false);
            self.regs.status.set_sprite_overflow(false);
            self.regs.status.set_sprite_zero_hit(false);
            self.sprite_shifter_0.fill(0);
            self.sprite_shifter_1.fill(0);
        }

        if (1..257).contains(&self.dot) || (321..337).contains(&self.dot) {
            self.update_shifters();
            match (self.dot - 1) % 8 {
                0 => {
                    self.load_bg_shifters();
                    let nmtable_addr = 0x2000 | self.regs.addr.raw_value() & 0x0FFF;
                    self.bg_next_tile_idx = self.read(cart, nmtable_addr);
                }
                2 => {
                    let x = u16::from(self.regs.addr.coarse_x().value());
                    let y = u16::from(self.regs.addr.coarse_y().value());
                    let base = (0x2000
                        | u16::from(self.regs.addr.nmtable_y()) << 11
                        | u16::from(self.regs.addr.nmtable_x()) << 10)
                        + 0x3C0;
                    let result = self.read(cart, base + (y / 4) * 8 + (x / 4));
                    self.bg_next_attrib = match (x % 4, y % 4) {
                        (0..=1, 0..=1) => result & 0b11,
                        (2..=3, 0..=1) => (result >> 2) & 0b11,
                        (0..=1, 2..=3) => (result >> 4) & 0b11,
                        (2..=3, 2..=3) => (result >> 6) & 0b11,
                        _ => unreachable!(),
                    };
                }
                4 => {
                    self.bg_next_tile_lsbits = self.read(
                        cart,
                        calculate_nmtable_addr(
                            self.regs.ctrl.bg_addr(),
                            self.bg_next_tile_idx,
                            self.regs.addr.fine_y().value(),
                        ),
                    );
                }
                6 => {
                    self.bg_next_tile_msbits = self.read(
                        cart,
                        calculate_nmtable_addr(
                            self.regs.ctrl.bg_addr(),
                            self.bg_next_tile_idx,
                            self.regs.addr.fine_y().value() + 8,
                        ),
                    );
                }
                7 => {
                    if is_rendering {
                        self.increment_scroll_x();
                    }
                }
                _ => {}
            }
        }

        if is_rendering && self.dot == 256 {
            self.increment_scroll_y();
        } else if self.dot == 257 {
            self.load_bg_shifters();
            if is_rendering {
                self.transfer_address_x();
            }
        }

        if self.dot == 338 || self.dot == 340 {
            let nmtable_addr = 0x2000 | self.regs.addr.raw_value() & 0x0FFF;
            self.bg_next_tile_idx = self.read(cart, nmtable_addr);
        }

        if is_rendering && self.scanline == PRE_RENDER_SCANLINE && (280..305).contains(&self.dot) {
            self.transfer_address_y();
        }
    }

    fn advance_dot(&mut self, is_rendering: bool) {
        self.dot += 1;
        if self.dot == 341 {
            self.dot = 0;
            self.scanline += 1;

            if self.scanline == PRE_RENDER_SCANLINE {
                self.frame_complete = true;
            }
            if self.scanline == PRE_RENDER_SCANLINE + 1 {
                self.scanline = 0;
                if self.odd_frame && is_rendering {
                    // skip the first cycle on the next frame (an even frame)
                    self.dot = 1;
                }
                self.odd_frame = !self.odd_frame;
            }
        }
    }

    fn increment_scroll_x(&mut self) {
        if self.regs.addr.coarse_x() == u5::new(31) {
            self.regs.addr.set_coarse_x(u5::new(0));
            self.regs.addr.set_nmtable_x(!self.regs.addr.nmtable_x());
        } else {
            self.regs
                .addr
                .set_coarse_x(self.regs.addr.coarse_x() + u5::new(1));
        }
    }

    fn increment_scroll_y(&mut self) {
        if self.regs.addr.fine_y() < u3::new(7) {
            self.regs
                .addr
                .set_fine_y(self.regs.addr.fine_y() + u3::new(1));
            return;
        }

        self.regs.addr.set_fine_y(u3::new(0));
        if self.regs.addr.coarse_y() == u5::new(29) {
            self.regs.addr.set_coarse_y(u5::new(0));
            self.regs.addr.set_nmtable_y(!self.regs.addr.nmtable_y());
        } else if self.regs.addr.coarse_y() == u5::new(31) {
            self.regs.addr.set_coarse_y(u5::new(0));
        } else {
            self.regs
                .addr
                .set_coarse_y(self.regs.addr.coarse_y() + u5::new(1));
        }
    }

    fn transfer_address_x(&mut self) {
        self.regs
            .addr
            .set_nmtable_x(self.regs.temp_addr.nmtable_x());
        self.regs.addr.set_coarse_x(self.regs.temp_addr.coarse_x());
    }

    fn transfer_address_y(&mut self) {
        self.regs.addr.set_coarse_y(self.regs.temp_addr.coarse_y());
        self.regs.addr.set_fine_y(self.regs.temp_addr.fine_y());
        self.regs
            .addr
            .set_nmtable_y(self.regs.temp_addr.nmtable_y());
    }

    fn update_shifters(&mut self) {
        if self.regs.mask.bg_rendering() {
            self.bg_pattern_shifter0 <<= 1;
            self.bg_pattern_shifter1 <<= 1;
            self.bg_attrib_shifter0 <<= 1;
            self.bg_attrib_shifter1 <<= 1;
        }
        if self.regs.mask.sprite_rendering() && (1..258).contains(&self.dot) {
            for i in 0..usize::from(self.sprite_count) {
                let obj = &mut self.line_oam[i];
                if obj.x > 0 {
                    obj.x -= 1;
                } else {
                    self.sprite_shifter_0[i] <<= 1;
                    self.sprite_shifter_1[i] <<= 1;
                }
            }
        }
    }

    fn load_bg_shifters(&mut self) {
        self.bg_pattern_shifter0 =
            self.bg_pattern_shifter0 & 0xFF00 | u16::from(self.bg_next_tile_lsbits);
        self.bg_pattern_shifter1 =
            self.bg_pattern_shifter1 & 0xFF00 | u16::from(self.bg_next_tile_msbits);

        self.bg_attrib_shifter0 = self.bg_attrib_shifter0 & 0xFF00
            | if util::bit(self.bg_next_attrib, 0) {
                0xFF
            } else {
                0x00
            };
        self.bg_attrib_shifter1 = self.bg_attrib_shifter1 & 0xFF00
            | if util::bit(self.bg_next_attrib, 1) {
                0xFF
            } else {
                0x00
            };
    }

    fn color_from_palette(&self, palette_idx: u8, pixel_idx: u8) -> Rgb8 {
        let color_idx = self.palettes[usize::from(palette_idx) * 4 + usize::from(pixel_idx)];
        COLORS[usize::from(color_idx)]
    }

    pub fn read_register(&mut self, cart: &Cartridge, r: RegisterKind) -> u8 {
        match r {
            RegisterKind::Ctrl
            | RegisterKind::Mask
            | RegisterKind::Scroll
            | RegisterKind::Addr
            | RegisterKind::OamAddr => 0,

            RegisterKind::Status => {
                let result = self.regs.status.raw_value() & 0xE0;
                let open_bus = self.data_buffer & !0xE0;
                self.regs.status.set_vblank(false);
                self.second_write = false;
                result | open_bus
            }
            RegisterKind::Data => {
                let mut result = self.data_buffer;
                self.data_buffer = self.read(cart, self.regs.addr.as_address());
                if self.regs.addr.as_address() >= PALETTE_RAM_START {
                    result = self.data_buffer;
                }
                self.increment_vram_addr();
                result
            }
            RegisterKind::OamData => {
                let object_idx = usize::from(self.regs.oam_addr / 4);
                let field_idx = self.regs.oam_addr % 4;
                self.oam[object_idx][field_idx]
            }
        }
    }

    pub fn write_register(&mut self, r: RegisterKind, value: u8) {
        match r {
            RegisterKind::Ctrl => {
                self.regs.ctrl = CtrlBits::new_with_raw_value(value);
                self.regs
                    .temp_addr
                    .set_nmtable_x(self.regs.ctrl.nmtable_x());
                self.regs
                    .temp_addr
                    .set_nmtable_y(self.regs.ctrl.nmtable_y());
            }
            RegisterKind::Mask => self.regs.mask = MaskBits::new_with_raw_value(value),
            RegisterKind::Status => {
                // can't write to the status register
            }
            RegisterKind::Scroll => {
                if !self.second_write {
                    self.regs.temp_addr.set_coarse_x(u5::new(value >> 3));
                    self.regs.fine_x = u3::new(value & 0x07);
                } else {
                    self.regs.temp_addr.set_coarse_y(u5::new(value >> 3));
                    self.regs.temp_addr.set_fine_y(u3::new(value & 0x07));
                }
                self.second_write = !self.second_write;
            }
            RegisterKind::Addr => {
                if !self.second_write {
                    self.regs.temp_addr = Loopy::new_with_raw_value(
                        // tram must be 15 bits so we mask
                        self.regs.temp_addr.raw_value() & 0x00FF | u16::from(value) << 8 & 0x7FFF,
                    );
                } else {
                    self.regs.temp_addr = Loopy::new_with_raw_value(
                        self.regs.temp_addr.raw_value() & 0xFF00 | u16::from(value),
                    );
                    self.regs.addr = self.regs.temp_addr;
                }
                self.second_write = !self.second_write;
            }
            RegisterKind::Data => {
                self.write(self.regs.addr.as_address(), value);
                self.increment_vram_addr();
            }
            RegisterKind::OamAddr => {
                self.regs.oam_addr = value;
            }
            RegisterKind::OamData => {
                let object_idx = usize::from(self.regs.oam_addr / 4);
                let field_idx = self.regs.oam_addr % 4;
                self.oam[object_idx][field_idx] = value;
            }
        }
    }

    fn increment_vram_addr(&mut self) {
        self.regs.addr = Loopy::new_with_raw_value(
            self.regs.addr.raw_value()
                + if self.regs.ctrl.vram_increment() {
                    32
                } else {
                    1
                },
        );
    }

    fn read(&self, cart: &Cartridge, addr: u16) -> u8 {
        match addr {
            // pattern tables from the cartridge.
            0x0000..CHR_ROM_SIZE => cart.chr()[usize::from(addr)],

            // name tables.
            0x2000..0x3000 => {
                let (table_idx, cell_idx) = nmtable_indices(self.nmtable_mirroring, addr);
                self.nmtables[table_idx][cell_idx]
            }

            // a mirror of the region from 0x2000..0x3F00.
            0x3000..0x3F00 => {
                // could recurse here instead of copy-pasting but...
                let (table_idx, cell_idx) = nmtable_indices(self.nmtable_mirroring, addr - 0x1000);
                self.nmtables[table_idx][cell_idx]
            }

            // a 256-byte repeating view of the first 32 bytes, which are the palette RAM.
            0x3F00..0x4000 => {
                let i = match addr % 32 {
                    0x10 => 0x00,
                    0x14 => 0x04,
                    0x18 => 0x08,
                    0x1C => 0x0C,
                    addr => addr.into(),
                };
                self.palettes[i]
            }

            _ => unreachable!("addr should be 14-bits"),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..CHR_ROM_SIZE => {
                // read-only
            }

            0x2000..0x3000 => {
                let (table_idx, cell_idx) = nmtable_indices(self.nmtable_mirroring, addr);
                self.nmtables[table_idx][cell_idx] = value;
            }

            0x3000..0x3F00 => {
                let (table_idx, cell_idx) = nmtable_indices(self.nmtable_mirroring, addr - 0x1000);
                self.nmtables[table_idx][cell_idx] = value;
            }

            0x3F00..0x4000 => {
                let i = match addr % 32 {
                    0x10 => 0x00,
                    0x14 => 0x04,
                    0x18 => 0x08,
                    0x1C => 0x0C,
                    addr => addr.into(),
                };
                self.palettes[i] = value;
            }

            _ => unreachable!("addr should be 14-bits"),
        }
    }
}

/// Returns (name table 0 or 1, address of cell inside the name table).
fn nmtable_indices(mirroring: Orientation, addr: u16) -> (usize, usize) {
    const TABLE_1_START: u16 = 0x2000;
    const TABLE_2_START: u16 = 0x2400;
    const TABLE_3_START: u16 = 0x2800;
    const TABLE_4_START: u16 = 0x2C00;

    // name table "mirroring": https://www.nesdev.org/wiki/PPU_nametables.
    match addr {
        TABLE_1_START..TABLE_2_START => (0, usize::from(addr - TABLE_1_START)),
        TABLE_2_START..TABLE_3_START => (
            match mirroring {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1,
            },
            usize::from(addr - TABLE_2_START),
        ),
        TABLE_3_START..TABLE_4_START => (
            match mirroring {
                Orientation::Horizontal => 1,
                Orientation::Vertical => 0,
            },
            usize::from(addr - TABLE_3_START),
        ),
        TABLE_4_START..0x3000 => (1, usize::from(addr - TABLE_4_START)),
        _ => unreachable!(),
    }
}

fn shift_out(reg: u16, bit: u8) -> u8 {
    (reg >> (15 - bit) & 1) as u8
}

fn calculate_nmtable_addr(nmtable_idx: bool, tile_idx: u8, row_idx: u8) -> u16 {
    (u16::from(nmtable_idx) << 12) + u16::from(tile_idx) * 16 + u16::from(row_idx)
}
