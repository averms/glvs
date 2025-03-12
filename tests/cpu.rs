use nes::{Bus, Cpu};

#[test]
fn basic() {
    let easy: [&[u8]; 6] = [
        &[0xA9, 0x01],
        &[0x8D, 0x00, 0x02],
        &[0xA9, 0x05],
        &[0x8D, 0x01, 0x02],
        &[0xA9, 0x08],
        &[0x8D, 0x02, 0x02],
    ];
    let mut bus = Bus::default();
    let mut cpu = Cpu::new(0x0600);
    setup(&mut bus, &easy);
    cpu.clock_until_brk(&mut bus);
    assert_eq!(0x01, bus.read(0x0200));
    assert_eq!(0x05, bus.read(0x0201));
    assert_eq!(0x08, bus.read(0x0202));
}

fn setup(bus: &mut Bus, program: &[&[u8]]) {
    let mut i = 0;
    for &inst in program {
        for &byte in inst {
            bus.write(0x0600 + i, byte);
            i += 1;
        }
    }
}
