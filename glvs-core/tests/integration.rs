use std::fs;

use glvs_core::{Cpu, NesBus};

#[test]
fn load_donkey_kong() {
    let f = fs::read("../resources/dk.nes").unwrap();
    let mut bus = NesBus::new(f.into_boxed_slice()).unwrap();
    let _cpu = Cpu::new(&mut bus);
}
