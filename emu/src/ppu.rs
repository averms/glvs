mod registers;
mod video;

use std::iter;

use crate::bus::PpuBus;
use crate::cartridge::CHR_ROM_SIZE;
use crate::ppu::registers::{Ctrl, Mask, Status};
use crate::ppu::video::{COLORS, Rgb8};

const PALETTE_RAM_START: u16 = 0x3F00;
const TILES_SIZE: usize = CHR_ROM_SIZE as usize / 16;

/// Canvas abstraction for the PPU to draw on.
pub trait Canvas {
    fn draw_point(&mut self, x: u32, y: u32, rgb: (u8, u8, u8));
}

pub struct Ppu {
    tiles: Box<[Tile; TILES_SIZE]>,
    regs: Registers,
    data_buffer: u8,
    first_write: bool,
}

#[derive(Debug, Default)]
struct Registers {
    status: Status,
    mask: Mask,
    ctrl: Ctrl,
    address: u16,
}

type Tile = [[PaletteIdx; 8]; 8];

/// A u2 index into a palette located in palette RAM.
#[derive(Debug, Default, Clone, Copy)]
struct PaletteIdx(u8);

impl PaletteIdx {
    pub fn new(value: u8) -> Self {
        assert!(value < 4, "value out of PaletteIdx's range");
        Self(value)
    }
}

impl Ppu {
    #[must_use]
    pub fn new(bus: &impl PpuBus) -> Self {
        let mut result = Self {
            tiles: vec![Tile::default(); TILES_SIZE]
                .try_into()
                .expect("boxed array idiom should work"),
            regs: Registers::default(),
            data_buffer: 0,
            first_write: true,
        };
        result.make_tiles(bus);
        result
    }

    fn make_tiles(&mut self, bus: &impl PpuBus) {
        for (tile_idx, tile) in iter::zip(0_u16.., self.tiles.iter_mut()) {
            let byte_idx = tile_idx * 16;
            for i in 0..8_u16 {
                let row_low_byte = bus.ppu_read(byte_idx + i);
                let row_high_byte = bus.ppu_read(byte_idx + i + 8);

                for j in 0..8_u16 {
                    let lsb = row_low_byte >> (7 - j) & 1;
                    let msb = row_high_byte >> (7 - j) & 1;
                    tile[usize::from(i)][usize::from(j)] = PaletteIdx::new(msb << 1 | lsb);
                }
            }
        }
    }

    pub fn draw_tiles(&self, canvas: &mut impl Canvas) {
        const TEST_PALETTE: [usize; 4] = [0x1, 0x20, 0x38, 0x7];

        for (tile_idx, tile) in iter::zip(0_u32.., self.tiles.iter()) {
            let x = tile_idx % 32 * 8;
            let y = tile_idx / 32 * 8;

            for i in 0..8_u16 {
                for j in 0..8_u16 {
                    let palette_idx = tile[usize::from(i)][usize::from(j)];
                    let color_idx = TEST_PALETTE[usize::from(palette_idx.0)];
                    let Rgb8(r, g, b) = COLORS[color_idx];
                    canvas.draw_point(x + u32::from(j), y + u32::from(i), (r, g, b));
                }
            }
        }
    }

    pub fn read_register(&mut self, bus: &impl PpuBus, addr: u16) -> u8 {
        match addr {
            0 => unreachable!(),
            1 => unreachable!(),
            2 => {
                // mutates self
                todo!("read status");
            }
            3 => todo!(),
            4 => todo!(),
            5 => unreachable!(),
            6 => unreachable!(),
            7 => {
                let buffered_result = self.data_buffer;
                self.data_buffer = bus.ppu_read(self.regs.address);
                self.regs.address += if self.regs.ctrl.vram_increment() {
                    32
                } else {
                    1
                };
                if self.regs.address >= PALETTE_RAM_START {
                    self.data_buffer
                } else {
                    buffered_result
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn write_register(&mut self, bus: &mut impl PpuBus, addr: u16, value: u8) {
        match addr {
            0 => self.regs.ctrl = Ctrl::new_with_raw_value(value),
            1 => self.regs.mask = Mask::new_with_raw_value(value),
            2 => {
                // can't write to the status register
            }
            3 => todo!(),
            4 => todo!(),
            5 => todo!(),
            6 => {
                if self.first_write {
                    self.regs.address = (self.regs.address & 0x00FF) | u16::from(value) << 8;
                } else {
                    self.regs.address = (self.regs.address & 0xFF00) | u16::from(value);
                }
                self.first_write ^= true;
            }
            7 => {
                bus.ppu_write(self.regs.address, value);
                self.regs.address += if self.regs.ctrl.vram_increment() {
                    32
                } else {
                    1
                };
            }
            _ => unreachable!(),
        }
    }
}
