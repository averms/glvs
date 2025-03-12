//! Emulator for the Ricoh 2A03.

mod instructions;
mod status;

#[expect(clippy::wildcard_imports)]
use crate::cpu::instructions::*;
use crate::{bus::Bus, cpu::status::Status};

#[derive(Debug)]
pub struct Cpu {
    registers: Registers,
    cycles_left: u8,
}

#[derive(Debug)]
struct Registers {
    pc: u16,
    sp: u8,
    a: u8,
    x: u8,
    y: u8,
    ps: Status,
}

impl Registers {
    fn read_and_bump_pc(&mut self, bus: &Bus) -> u8 {
        let current_pc = self.pc;
        self.pc = self.pc.wrapping_add(1);
        bus.read(current_pc)
    }
}

/// According to <https://www.nesdev.org/wiki/CPU_addressing_modes>, there are 13 addressing modes on
/// the MOS 6502.
///
/// Here we have immediate, accumulator, and all the memory addressing modes
/// except for relative and the addressing modes for JMP and JSR. We consider
/// the relative addressing mode identical in implementation to the immediate
/// addressing mode. We choose not to model the addressing modes for JMP and
/// JSR, which are special instructions because their version of the absolute
/// addressing mode is more like a 2-byte immediate.
#[derive(Debug, Clone, Copy)]
enum AddrMode {
    Immediate(u8),
    Accumulator,
    Memory(u16, bool),
}

impl AddrMode {
    fn load(self, regs: &mut Registers, bus: &mut Bus) -> u8 {
        match self {
            AddrMode::Immediate(value) => value,
            AddrMode::Accumulator => regs.a,
            AddrMode::Memory(addr, _) => bus.read(addr),
        }
    }

    fn store(self, regs: &mut Registers, bus: &mut Bus, value: u8) {
        match self {
            AddrMode::Immediate(_) => unreachable!("can't store to immediate"),
            AddrMode::Accumulator => regs.a = value,
            AddrMode::Memory(addr, _) => bus.write(addr, value),
        }
    }

    /// Return the number of extra cycles needed for getting data from memory.
    /// Always a 0 or 1.
    fn extra_cycles_needed(self) -> u8 {
        match self {
            AddrMode::Memory(_, needs_extra_cycle) => needs_extra_cycle.into(),
            AddrMode::Accumulator | AddrMode::Immediate(_) => 0,
        }
    }

    // Constructors.

    fn relative(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        AddrMode::immediate(regs, bus)
    }

    fn immediate(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        AddrMode::Immediate(regs.read_and_bump_pc(bus))
    }

    fn zero_page(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        AddrMode::Memory(regs.read_and_bump_pc(bus).into(), false)
    }

    fn zero_page_y(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let base = regs.read_and_bump_pc(bus);
        let zero_page_addr = base.wrapping_add(regs.y);
        AddrMode::Memory(zero_page_addr.into(), false)
    }

    fn zero_page_x(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let base = regs.read_and_bump_pc(bus);
        let zero_page_addr = base.wrapping_add(regs.x);
        AddrMode::Memory(zero_page_addr.into(), false)
    }

    fn absolute(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let addr = u16::from_le_bytes([low, high]);
        AddrMode::Memory(addr, false)
    }

    fn absolute_x(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.x.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        AddrMode::Memory(addr, page_crossed)
    }

    fn absolute_y(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.y.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        AddrMode::Memory(addr, page_crossed)
    }

    fn indirect_indexed(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let zero_page_ptr = regs.read_and_bump_pc(bus);
        let low = bus.read(zero_page_ptr.into());
        let high = bus.read(zero_page_ptr.wrapping_add(1).into());
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.y.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        AddrMode::Memory(addr, page_crossed)
    }

    fn indexed_indirect(regs: &mut Registers, bus: &mut Bus) -> AddrMode {
        let ptr_base = regs.read_and_bump_pc(bus);
        let ptr = ptr_base.wrapping_add(regs.x);
        let low = bus.read(ptr.into());
        let high = bus.read(ptr.wrapping_add(1).into());
        let addr = u16::from_le_bytes([low, high]);
        AddrMode::Memory(addr, false)
    }
}

fn jump_indirect(regs: &mut Registers, bus: &mut Bus) -> u16 {
    let ptr_low = regs.read_and_bump_pc(bus);
    let ptr_high = regs.read_and_bump_pc(bus);
    // replicate 6502 bug where the high byte of the address can be fetched from the
    // wrong page.
    let low = bus.read(u16::from_le_bytes([ptr_low, ptr_high]));
    let high = bus.read(u16::from_le_bytes([ptr_low.wrapping_add(1), ptr_high]));
    u16::from_le_bytes([low, high])
}

fn jump_absolute(regs: &mut Registers, bus: &mut Bus) -> u16 {
    let low = regs.read_and_bump_pc(bus);
    let high = regs.read_and_bump_pc(bus);
    u16::from_le_bytes([low, high])
}

impl Cpu {
    #[must_use]
    pub fn new(pc: u16) -> Self {
        Cpu {
            registers: Registers {
                pc,
                // this might need to be 0xFD?
                sp: 0xFF,
                a: 0,
                x: 0,
                y: 0,
                ps: Status::default(),
            },
            cycles_left: 0,
        }
    }

    pub fn clock_one_instruction(&mut self, bus: &mut Bus) {
        loop {
            self.clock(bus);
            if self.cycles_left == 0 {
                break;
            }
        }
    }

    /// Perform one clock-cycle worth of emulation. This is not cycle-accurate
    /// at all. In fact, it does every operation in one cycle and then does
    /// nothing for the remaining cycles that instruction is supposed to
    /// take.
    pub fn clock(&mut self, bus: &mut Bus) {
        if self.cycles_left > 0 {
            self.cycles_left -= 1;
            return;
        }

        let opcode = self.registers.read_and_bump_pc(bus);
        self.cycles_left = decode_and_execute(&mut self.registers, bus, opcode) - 1;
    }
}

/// Return which of the 256 pages of memory addr resides in.
fn page_of(addr: u16) -> u16 {
    addr & 0xFF00
}

/// Decode and execute one instruction, returning the number of cycles that
/// instruction was supposed to take in hardware.
#[expect(clippy::too_many_lines)]
fn decode_and_execute(regs: &mut Registers, bus: &mut Bus, opcode: u8) -> u8 {
    let (base_cycles, extra_cycles) = match opcode {
        0x00 => (7, brk(regs, bus)),
        0x01 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (6, ora(regs, bus, a))
        }
        0x04 | 0x44 | 0x64 => {
            AddrMode::zero_page(regs, bus);
            (3, nop())
        }
        0x05 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, ora(regs, bus, a))
        }
        0x06 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, asl(regs, bus, a))
        }
        0x08 => (2, php(regs, bus)),
        0x09 => {
            let a = AddrMode::immediate(regs, bus);
            (2, ora(regs, bus, a))
        }
        0x0A => {
            let a = AddrMode::Accumulator;
            (2, asl(regs, bus, a))
        }
        0x0C => {
            AddrMode::absolute(regs, bus);
            (4, nop())
        }
        0x0D => {
            let a = AddrMode::absolute(regs, bus);
            (4, ora(regs, bus, a))
        }
        0x0E => {
            let a = AddrMode::absolute(regs, bus);
            (6, asl(regs, bus, a))
        }
        0x10 => {
            let a = AddrMode::relative(regs, bus);
            (2, bpl(regs, bus, a))
        }
        0x11 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, ora(regs, bus, a))
        }
        0x14 | 0x34 | 0x54 | 0x74 => {
            AddrMode::zero_page_x(regs, bus);
            (4, nop())
        }
        0x15 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, ora(regs, bus, a))
        }
        0x16 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, asl(regs, bus, a))
        }
        0x18 => (2, clc(regs)),
        0x19 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, ora(regs, bus, a))
        }
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA => (2, nop()),
        0x1D => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, ora(regs, bus, a))
        }
        0x1E => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, asl(regs, bus, a))
        }
        0x20 => {
            let a = jump_absolute(regs, bus);
            (6, jsr(regs, bus, a))
        }
        0x21 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, and(regs, bus, a))
        }
        0x24 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, bit(regs, bus, a))
        }
        0x25 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, and(regs, bus, a))
        }
        0x26 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, rol(regs, bus, a))
        }
        0x28 => (2, plp(regs, bus)),
        0x29 => {
            let a = AddrMode::immediate(regs, bus);
            (2, and(regs, bus, a))
        }
        0x2A => {
            let a = AddrMode::Accumulator;
            (2, rol(regs, bus, a))
        }
        0x2C => {
            let a = AddrMode::absolute(regs, bus);
            (4, bit(regs, bus, a))
        }
        0x2D => {
            let a = AddrMode::absolute(regs, bus);
            (4, and(regs, bus, a))
        }
        0x2E => {
            let a = AddrMode::absolute(regs, bus);
            (6, rol(regs, bus, a))
        }
        0x30 => {
            let a = AddrMode::relative(regs, bus);
            (2, bmi(regs, bus, a))
        }
        0x31 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, and(regs, bus, a))
        }
        0x35 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, and(regs, bus, a))
        }
        0x36 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, rol(regs, bus, a))
        }
        0x38 => (2, sec(regs)),
        0x39 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, and(regs, bus, a))
        }
        0x3D => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, and(regs, bus, a))
        }
        0x3E => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, rol(regs, bus, a))
        }
        0x40 => (6, rti(regs, bus)),
        0x41 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, eor(regs, bus, a))
        }
        0x45 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, eor(regs, bus, a))
        }
        0x46 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, lsr(regs, bus, a))
        }
        0x48 => (3, pha(regs, bus)),
        0x49 => {
            let a = AddrMode::immediate(regs, bus);
            (2, eor(regs, bus, a))
        }
        0x4A => {
            let a = AddrMode::Accumulator;
            (2, lsr(regs, bus, a))
        }
        0x4C => {
            let a = jump_absolute(regs, bus);
            (3, jmp(regs, a))
        }
        0x4D => {
            let a = AddrMode::absolute(regs, bus);
            (4, eor(regs, bus, a))
        }
        0x4E => {
            let a = AddrMode::absolute(regs, bus);
            (6, lsr(regs, bus, a))
        }
        0x50 => {
            let a = AddrMode::relative(regs, bus);
            (2, bvc(regs, bus, a))
        }
        0x51 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, eor(regs, bus, a))
        }
        0x55 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, eor(regs, bus, a))
        }
        0x56 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, lsr(regs, bus, a))
        }
        0x58 => (2, cli(regs)),
        0x59 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, eor(regs, bus, a))
        }
        0x5D => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, eor(regs, bus, a))
        }
        0x5E => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, lsr(regs, bus, a))
        }
        0x60 => (6, rts(regs, bus)),
        0x61 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, adc(regs, bus, a))
        }
        0x65 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, adc(regs, bus, a))
        }
        0x66 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, ror(regs, bus, a))
        }
        0x68 => (4, pla(regs, bus)),
        0x69 => {
            let a = AddrMode::immediate(regs, bus);
            (2, adc(regs, bus, a))
        }
        0x6A => {
            let a = AddrMode::Accumulator;
            (2, ror(regs, bus, a))
        }
        0x6C => {
            let a = jump_indirect(regs, bus);
            (5, jmp(regs, a))
        }
        0x6D => {
            let a = AddrMode::absolute(regs, bus);
            (4, adc(regs, bus, a))
        }
        0x6E => {
            let a = AddrMode::absolute(regs, bus);
            (6, ror(regs, bus, a))
        }
        0x70 => {
            let a = AddrMode::relative(regs, bus);
            (2, bvs(regs, bus, a))
        }
        0x71 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, adc(regs, bus, a))
        }
        0x75 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, adc(regs, bus, a))
        }
        0x76 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, ror(regs, bus, a))
        }
        0x78 => (2, sei(regs)),
        0x79 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, adc(regs, bus, a))
        }
        0x7D => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, adc(regs, bus, a))
        }
        0x7E => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, ror(regs, bus, a))
        }
        0x80 => {
            AddrMode::immediate(regs, bus);
            (2, nop())
        }
        0x81 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, sta(regs, bus, a))
        }
        0x84 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, sty(regs, bus, a))
        }
        0x85 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, sta(regs, bus, a))
        }
        0x86 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, stx(regs, bus, a))
        }
        0x88 => (2, dey(regs)),
        0x8A => (2, txa(regs)),
        0x8C => {
            let a = AddrMode::absolute(regs, bus);
            (4, sty(regs, bus, a))
        }
        0x8D => {
            let a = AddrMode::absolute(regs, bus);
            (4, sta(regs, bus, a))
        }
        0x8E => {
            let a = AddrMode::absolute(regs, bus);
            (4, stx(regs, bus, a))
        }
        0x90 => {
            let a = AddrMode::relative(regs, bus);
            (2, bcc(regs, bus, a))
        }
        0x91 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (6, sta(regs, bus, a))
        }
        0x94 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, sty(regs, bus, a))
        }
        0x95 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, sta(regs, bus, a))
        }
        0x96 => {
            let a = AddrMode::zero_page_y(regs, bus);
            (4, stx(regs, bus, a))
        }
        0x98 => (2, tya(regs)),
        0x99 => {
            let a = AddrMode::absolute_y(regs, bus);
            (5, sta(regs, bus, a))
        }
        0x9A => (2, txs(regs)),
        0x9D => {
            let a = AddrMode::absolute_x(regs, bus);
            (5, sta(regs, bus, a))
        }
        0xA0 => {
            let a = AddrMode::immediate(regs, bus);
            (2, ldy(regs, bus, a))
        }
        0xA1 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, lda(regs, bus, a))
        }
        0xA2 => {
            let a = AddrMode::immediate(regs, bus);
            (2, ldx(regs, bus, a))
        }
        0xA4 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, ldy(regs, bus, a))
        }
        0xA5 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, lda(regs, bus, a))
        }
        0xA6 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, ldx(regs, bus, a))
        }
        0xA8 => (2, tay(regs)),
        0xA9 => {
            let a = AddrMode::immediate(regs, bus);
            (2, lda(regs, bus, a))
        }
        0xAA => (2, tax(regs)),
        0xAC => {
            let a = AddrMode::absolute(regs, bus);
            (4, ldy(regs, bus, a))
        }
        0xAD => {
            let a = AddrMode::absolute(regs, bus);
            (4, lda(regs, bus, a))
        }
        0xAE => {
            let a = AddrMode::absolute(regs, bus);
            (4, ldx(regs, bus, a))
        }
        0xB0 => {
            let a = AddrMode::relative(regs, bus);
            (2, bcs(regs, bus, a))
        }
        0xB1 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, lda(regs, bus, a))
        }
        0xB4 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, ldy(regs, bus, a))
        }
        0xB5 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, lda(regs, bus, a))
        }
        0xB6 => {
            let a = AddrMode::zero_page_y(regs, bus);
            (4, ldx(regs, bus, a))
        }
        0xB8 => (2, clv(regs)),
        0xB9 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, lda(regs, bus, a))
        }
        0xBA => (2, tsx(regs)),
        0xBC => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, ldy(regs, bus, a))
        }
        0xBD => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, lda(regs, bus, a))
        }
        0xBE => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, ldx(regs, bus, a))
        }
        0xC0 => {
            let a = AddrMode::immediate(regs, bus);
            (2, cpy(regs, bus, a))
        }
        0xC1 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, cmp(regs, bus, a))
        }
        0xC4 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, cpy(regs, bus, a))
        }
        0xC5 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, cmp(regs, bus, a))
        }
        0xC6 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, dec(regs, bus, a))
        }
        0xC8 => (2, iny(regs)),
        0xC9 => {
            let a = AddrMode::immediate(regs, bus);
            (2, cmp(regs, bus, a))
        }
        0xCA => (2, dex(regs)),
        0xCC => {
            let a = AddrMode::absolute(regs, bus);
            (4, cpy(regs, bus, a))
        }
        0xCD => {
            let a = AddrMode::absolute(regs, bus);
            (4, cmp(regs, bus, a))
        }
        0xCE => {
            let a = AddrMode::absolute(regs, bus);
            (6, dec(regs, bus, a))
        }
        0xD0 => {
            let a = AddrMode::relative(regs, bus);
            (2, bne(regs, bus, a))
        }
        0xD1 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, cmp(regs, bus, a))
        }
        0xD5 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, cmp(regs, bus, a))
        }
        0xD6 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, dec(regs, bus, a))
        }
        0xD8 => (2, cld(regs)),
        0xD9 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, cmp(regs, bus, a))
        }
        0xDD => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, cmp(regs, bus, a))
        }
        0xDE => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, dec(regs, bus, a))
        }
        0xE0 => {
            let a = AddrMode::immediate(regs, bus);
            (2, cpx(regs, bus, a))
        }
        0xE1 => {
            let a = AddrMode::indexed_indirect(regs, bus);
            (6, sbc(regs, bus, a))
        }
        0xE4 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, cpx(regs, bus, a))
        }
        0xE5 => {
            let a = AddrMode::zero_page(regs, bus);
            (3, sbc(regs, bus, a))
        }
        0xE6 => {
            let a = AddrMode::zero_page(regs, bus);
            (5, inc(regs, bus, a))
        }
        0xE8 => (2, inx(regs)),
        0xE9 => {
            let a = AddrMode::immediate(regs, bus);
            (2, sbc(regs, bus, a))
        }
        0xEC => {
            let a = AddrMode::absolute(regs, bus);
            (4, cpx(regs, bus, a))
        }
        0xED => {
            let a = AddrMode::absolute(regs, bus);
            (4, sbc(regs, bus, a))
        }
        0xEE => {
            let a = AddrMode::absolute(regs, bus);
            (6, inc(regs, bus, a))
        }
        0xF0 => {
            let a = AddrMode::relative(regs, bus);
            (2, beq(regs, bus, a))
        }
        0xF1 => {
            let a = AddrMode::indirect_indexed(regs, bus);
            (5, sbc(regs, bus, a))
        }
        0xF5 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (4, sbc(regs, bus, a))
        }
        0xF6 => {
            let a = AddrMode::zero_page_x(regs, bus);
            (6, inc(regs, bus, a))
        }
        0xF8 => (2, sed(regs)),
        0xF9 => {
            let a = AddrMode::absolute_y(regs, bus);
            (4, sbc(regs, bus, a))
        }
        0xFD => {
            let a = AddrMode::absolute_x(regs, bus);
            (4, sbc(regs, bus, a))
        }
        0xFE => {
            let a = AddrMode::absolute_x(regs, bus);
            (7, inc(regs, bus, a))
        }
        _ => unimplemented!(),
    };
    base_cycles + extra_cycles
}

#[cfg(test)]
mod tests;
