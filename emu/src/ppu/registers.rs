use arbitrary_int::u2;
use bitbybit::bitfield;

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct Mask {
    #[bit(7, rw)]
    emph_blue: bool,
    #[bit(6, rw)]
    emph_green: bool,
    #[bit(5, rw)]
    emph_red: bool,
    #[bit(4, rw)]
    sprite_rendering: bool,
    #[bit(3, rw)]
    bg_rendering: bool,
    #[bit(2, rw)]
    show_left_sprites: bool,
    #[bit(1, rw)]
    show_right_sprites: bool,
    #[bit(0, rw)]
    grayscale: bool,
}

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct Status {
    #[bit(7, rw)]
    vblank: bool,
    #[bit(6, rw)]
    sprite_zero_hit: bool,
    #[bit(5, rw)]
    sprite_overflow: bool,
}

#[bitfield(u8)]
#[derive(Debug, Default)]
pub struct Ctrl {
    #[bit(7, rw)]
    vblank_nmi: bool,
    #[bit(6)] // unused
    push_to_ext: bool,
    #[bit(5, rw)]
    large_sprite: bool,
    #[bit(4, rw)]
    bg_addr: bool,
    #[bit(3, rw)]
    sprite_addr: bool,
    #[bit(2, rw)]
    vram_increment: bool,
    #[bits(0..=1, rw)]
    base_nmtable_addr: u2,
}
