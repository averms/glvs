use std::fs;

use nes::{Cpu, NesBus};

#[test]
fn load_donkey_kong() {
    let f = fs::read("./resources/Donkey Kong (World) (Rev 1).nes").unwrap();
    let bus = NesBus::new(&f).unwrap();
    let _cpu = Cpu::new(0x8000);
}
