use crate::{
    bus::Bus,
    cpu::{Cpu, Registers, Status},
};
use serde::Deserialize;
use serde::de::IgnoredAny;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Test {
    name: Box<str>,
    initial: State,
    #[serde(rename = "final")]
    final_: State,
    cycles: Box<[IgnoredAny]>,
}

#[derive(Debug, Deserialize)]
struct State {
    pc: u16,
    s: u8,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    ram: Box<[(u16, u8)]>,
}

#[test]
fn single_step() {
    let mut tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    tests_dir.push("resources/single_step");
    let tests_iter = tests_dir.read_dir().expect("test cases dir not found");
    for entry in tests_iter {
        let test_path = entry.unwrap().path();
        opcode_works(&test_path);
    }
}

fn opcode_works(test_file: &Path) {
    let text = fs::read_to_string(test_file).unwrap();
    let tests: Box<[Test]> = serde_json::from_str(&text).unwrap();

    let mut bus = Bus::default();
    for test in tests {
        println!("testing {}.", test.name);

        let mut cpu = Cpu {
            registers: Registers {
                pc: test.initial.pc,
                sp: test.initial.s,
                a: test.initial.a,
                x: test.initial.x,
                y: test.initial.y,
                ps: Status(test.initial.p),
            },
            cycles_left: 0,
        };
        set_bus(&mut bus, &test.initial.ram);

        cpu.cycle(&mut bus);
        let actual_cycles = cpu.cycles_left + 1;
        for _ in 1..actual_cycles {
            cpu.cycle(&mut bus);
        }

        assert_eq!(usize::from(actual_cycles), test.cycles.len());
        assert_eq!(cpu.registers.pc, test.final_.pc);
        assert_eq!(cpu.registers.a, test.final_.a);
        assert_eq!(cpu.registers.x, test.final_.x);
        assert_eq!(cpu.registers.y, test.final_.y);
        assert_eq!(cpu.registers.sp, test.final_.s);
        assert_eq!(cpu.registers.ps.0, test.final_.p);
        assert_bus_passed(&bus, &test.final_.ram);

        bus.reset();
    }
}

fn set_bus(bus: &mut Bus, data: &[(u16, u8)]) {
    for &(addr, val) in data {
        bus.write(addr, val);
    }
}

fn assert_bus_passed(bus: &Bus, data: &[(u16, u8)]) {
    for &(addr, val) in data {
        assert_eq!(bus.read(addr), val);
    }
}
