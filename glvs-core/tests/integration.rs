use std::fs;

use glvs_core::{Cpu, NesBus};

#[test]
fn load_rom() {
    let f = fs::read("./tests/nestest.nes").unwrap();
    let mut bus = NesBus::new(f.into_boxed_slice()).unwrap();
    let _cpu = Cpu::new(&mut bus);
}
