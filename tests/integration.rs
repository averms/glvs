use std::fs::File;

use nes::{Bus as _, Cartridge, Cpu, NesBus};

const PROGRAM_START: u16 = 0x0600;

#[test]
fn easy() {
    #[rustfmt::skip]
    let easy: [u8; 15] = [
        0xA9, 0x01,
        0x8D, 0x00, 0x01,
        0xA9, 0x05,
        0x8D, 0x01, 0x01,
        0xA9, 0x08,
        0x8D, 0x02, 0x01,
    ];

    let mut cpu = Cpu::new(PROGRAM_START);
    let mut bus = setup(&easy);
    let program_end = usize::from(PROGRAM_START) + easy.len();

    while usize::from(cpu.registers().pc) < program_end {
        cpu.one_instruction(&mut bus);
    }

    assert_eq!(0x01, bus.read(0x0100));
    assert_eq!(0x05, bus.read(0x0101));
    assert_eq!(0x08, bus.read(0x0102));
}

fn setup(program: &[u8]) -> NesBus {
    let mut bus = NesBus::default();
    for (i, &byte) in program.iter().enumerate() {
        bus.write(PROGRAM_START + u16::try_from(i).unwrap(), byte);
    }
    bus
}

#[test]
fn load_donkey_kong() {
    let f = File::open("./resources/Donkey Kong (World) (Rev 1).nes").unwrap();
    _ = Cartridge::new(&f).unwrap();
}
