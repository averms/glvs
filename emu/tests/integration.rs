use std::fs;

use emu::{Cpu, NesBus, Ppu};

#[test]
fn load_donkey_kong() {
    let f = fs::read("./resources/Donkey Kong (World) (Rev 1).nes").unwrap();
    let bus = NesBus::new(&f).unwrap();
    let _cpu = Cpu::new(0x8000);
    let _ppu = Ppu::new(&bus);
}
