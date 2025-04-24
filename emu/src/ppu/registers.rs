//! PPU register types. This uses a macro to generate bitfields.

use arbitrary_int::{u3, u5};
use bitbybit::bitfield;

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct MaskBits {
    #[bit(0, rw)]
    grayscale: bool,
    #[bit(1, rw)]
    show_right_sprites: bool,
    #[bit(2, rw)]
    show_left_sprites: bool,
    #[bit(3, rw)]
    bg_rendering: bool,
    #[bit(4, rw)]
    sprite_rendering: bool,
    #[bit(5, r)]
    emph_red: bool,
    #[bit(6, r)]
    emph_green: bool,
    #[bit(7, r)]
    emph_blue: bool,
}

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct StatusBits {
    #[bit(5, rw)]
    sprite_overflow: bool,
    #[bit(6, rw)]
    sprite_zero_hit: bool,
    #[bit(7, rw)]
    vblank: bool,
}

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct CtrlBits {
    #[bit(0, rw)]
    nmtable_x: bool,
    #[bit(1, rw)]
    nmtable_y: bool,
    #[bit(2, rw)]
    vram_increment: bool,
    #[bit(3, rw)]
    sprite_addr: bool,
    #[bit(4, rw)]
    bg_addr: bool,
    #[bit(5, rw)]
    large_sprite: bool,
    #[bit(6)] // unused
    push_to_ext: bool,
    #[bit(7, rw)]
    vblank_nmi: bool,
}

/// This models the internal PPU registers v and t.
#[bitfield(u16)]
#[derive(Debug, Default)]
pub struct Loopy {
    #[bits(0..=4, rw)]
    coarse_x: u5,
    #[bits(5..=9, rw)]
    coarse_y: u5,
    #[bit(10, rw)]
    nmtable_x: bool,
    #[bit(11, rw)]
    nmtable_y: bool,
    #[bits(12..=14, rw)]
    fine_y: u3,
    // one unused bit.
}

impl Loopy {
    /// Return the 15-bit Loopy register as a 14-bit PPU address.
    pub fn as_address(self) -> u16 {
        self.raw_value & 0x3FFF
    }
}
