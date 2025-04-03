use nes::{Bus as _, Cpu, NesBus};

#[test]
fn easy() {
    const COUNT: usize = 6;
    let easy: [&[u8]; COUNT] = [
        &[0xA9, 0x01],
        &[0x8D, 0x00, 0x02],
        &[0xA9, 0x05],
        &[0x8D, 0x01, 0x02],
        &[0xA9, 0x08],
        &[0x8D, 0x02, 0x02],
    ];
    let mut bus = NesBus::default();
    let mut cpu = Cpu::new(0x0600);

    setup(&mut bus, &easy);
    for _ in 0..COUNT {
        cpu.one_instruction(&mut bus);
    }

    assert_eq!(0x01, bus.read(0x0200));
    assert_eq!(0x05, bus.read(0x0201));
    assert_eq!(0x08, bus.read(0x0202));
}

fn setup(bus: &mut NesBus, program: &[&[u8]]) {
    let mut i = 0;
    for &inst in program {
        for &byte in inst {
            bus.write(0x0600 + i, byte);
            i += 1;
        }
    }
}
