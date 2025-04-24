use std::fs;

use emu::{Cpu, NesBus};

#[test]
fn load_donkey_kong() {
    let f = fs::read("./resources/Donkey Kong (World) (Rev 1).nes").unwrap();
    let mut bus = NesBus::new(&f).unwrap();
    let _cpu = Cpu::new(&mut bus);
}
