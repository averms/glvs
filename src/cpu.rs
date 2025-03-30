//! CPU implementation.

mod addressing;
mod instructions;
#[cfg(test)]
mod tests;

use crate::bus::Bus;

#[derive(Debug)]
pub struct Cpu {
    registers: Registers,
    cycles_left: u8,
}

#[derive(Debug)]
pub struct Registers {
    pub pc: u16,
    pub sp: u8,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub ps: Status,
}

impl Registers {
    fn read_and_bump_pc(&mut self, bus: &impl Bus) -> u8 {
        let current_pc = self.pc;
        self.pc = self.pc.wrapping_add(1);
        bus.read(current_pc)
    }
}

/// 6502 status register.
///
/// 0. carry
/// 1. zero
/// 2. interrupt
/// 3. decimal (not actually supported but can be set)
/// 4. break (unused)
/// 5. (unused, always pushed as 1)
/// 6. overflow
/// 7. negative
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Status(u8);

impl Default for Status {
    fn default() -> Self {
        // according to blargg's cpu_reset test ROM.
        Self(0b0011_0100)
    }
}

impl Status {
    // Getters.
    fn carry(self) -> bool {
        self.0 & (1 << 0) != 0
    }
    fn zero(self) -> bool {
        self.0 & (1 << 1) != 0
    }
    #[expect(dead_code)]
    fn interrupt(self) -> bool {
        self.0 & (1 << 2) != 0
    }
    fn overflow(self) -> bool {
        self.0 & (1 << 6) != 0
    }
    fn negative(self) -> bool {
        self.0 & (1 << 7) != 0
    }

    // Setters.
    fn set_carry(&mut self, c: bool) {
        if c {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }
    fn set_zero(&mut self, z: bool) {
        if z {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
    }
    fn set_interrupt(&mut self, i: bool) {
        if i {
            self.0 |= 1 << 2;
        } else {
            self.0 &= !(1 << 2);
        }
    }
    fn set_decimal(&mut self, d: bool) {
        if d {
            self.0 |= 1 << 3;
        } else {
            self.0 &= !(1 << 3);
        }
    }
    fn set_overflow(&mut self, v: bool) {
        if v {
            self.0 |= 1 << 6;
        } else {
            self.0 &= !(1 << 6);
        }
    }
    fn set_negative(&mut self, n: bool) {
        if n {
            self.0 |= 1 << 7;
        } else {
            self.0 &= !(1 << 7);
        }
    }

    fn set_if_negative(&mut self, num: u8) {
        self.set_negative(i8::from_le_bytes([num]) < 0);
    }

    fn to_pushable(self) -> u8 {
        self.0 | 0b0011_0000
    }

    fn from_popped(value: u8) -> Self {
        Self(value & 0b1100_1111 | (1 << 5))
    }
}

impl Cpu {
    #[must_use]
    pub fn new(pc: u16) -> Self {
        Self {
            registers: Registers {
                pc,
                sp: 0xFD,
                a: 0,
                x: 0,
                y: 0,
                ps: Status::default(),
            },
            cycles_left: 0,
        }
    }

    /// Get a read-only view of the CPU's registers
    #[must_use]
    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    /// Execute one instruction. This calls [`Cpu::cycle`] one or more times.
    pub fn one_instruction(&mut self, bus: &mut impl Bus) {
        loop {
            self.cycle(bus);
            if self.cycles_left == 0 {
                break;
            }
        }
    }

    /// Perform one clock-cycle worth of emulation. This is not cycle-accurate
    /// at all. In fact, it does every operation in one cycle and then does
    /// nothing for the remaining cycles that instruction is supposed to
    /// take.
    pub fn cycle(&mut self, bus: &mut impl Bus) {
        if self.cycles_left > 0 {
            self.cycles_left -= 1;
            return;
        }

        let opcode = self.registers.read_and_bump_pc(bus);
        self.cycles_left = decode_and_execute(&mut self.registers, bus, opcode) - 1;
    }
}

/// Decode and execute one instruction, returning the number of cycles that
/// instruction was supposed to take in hardware.
fn decode_and_execute(regs: &mut Registers, bus: &mut impl Bus, opcode: u8) -> u8 {
    use crate::cpu::addressing::{AddrMode, jump_absolute, jump_indirect};
    #[expect(clippy::wildcard_imports)]
    use crate::cpu::instructions::*;

    let (base_cycles, extra_cycles) = match opcode {
        0x00 => (7, brk(regs, bus)),
        0x01 => {
            let a = AddrMode::indexed_indirect(regs, bus);
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
        0x08 => (3, php(regs, bus)),
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
        0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
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
        0x28 => (4, plp(regs, bus)),
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
        0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
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
        opcode => unimplemented!("0x{opcode:x}"),
    };
    base_cycles + extra_cycles
}
