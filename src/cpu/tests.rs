use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::bus::Bus;
use crate::cpu::{Cpu, Registers, Status};

#[derive(Debug, Deserialize)]
struct Case {
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
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/single_step");
    let tests_iter = tests_dir.read_dir().expect("test cases dir not found");
    for entry in tests_iter {
        let test_path = entry.unwrap().path();
        opcode_works(&test_path);
    }
}

fn opcode_works(test_file: &Path) {
    let text = fs::read_to_string(test_file).unwrap();
    let cases: Box<[Case]> = serde_json::from_str(&text).unwrap();

    let mut bus = TestBus::default();
    for case in cases {
        std::println!("executing {}.", case.name);
        let mut cpu = Cpu {
            registers: Registers {
                pc: case.initial.pc,
                sp: case.initial.s,
                a: case.initial.a,
                x: case.initial.x,
                y: case.initial.y,
                ps: Status(case.initial.p),
            },
            cycles_left: 0,
        };
        setup_bus(&mut bus, &case.initial.ram);

        cpu.cycle(&mut bus);
        let got_cycles = cpu.cycles_left + 1;
        for _ in 1..got_cycles {
            cpu.cycle(&mut bus);
        }

        assert_eq!(usize::from(got_cycles), case.cycles.len());
        assert_eq!(cpu.registers.pc, case.final_.pc);
        assert_eq!(cpu.registers.a, case.final_.a);
        assert_eq!(cpu.registers.x, case.final_.x);
        assert_eq!(cpu.registers.y, case.final_.y);
        assert_eq!(cpu.registers.sp, case.final_.s);
        assert_eq!(cpu.registers.ps.0, case.final_.p);
        assert_bus_passed(&bus, &case.final_.ram);
    }
}

fn setup_bus(bus: &mut TestBus, data: &[(u16, u8)]) {
    for &(addr, val) in data {
        bus.write(addr, val);
    }
}

fn assert_bus_passed(bus: &TestBus, data: &[(u16, u8)]) {
    for &(addr, val) in data {
        assert_eq!(bus.read(addr), val);
    }
}

const BUS_SIZE: usize = 64 * 1024;

struct TestBus {
    ram: Box<[u8; BUS_SIZE]>,
}

impl Default for TestBus {
    fn default() -> Self {
        Self {
            ram: vec![0; BUS_SIZE]
                .try_into()
                .expect("this is the idiom to create arrays on the heap."),
        }
    }
}

impl Bus for TestBus {
    fn read(&self, addr: u16) -> u8 {
        self.ram[usize::from(addr)]
    }

    fn write(&mut self, addr: u16, value: u8) {
        self.ram[usize::from(addr)] = value;
    }
}
