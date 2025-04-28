//! The implementation of every 6502 instruction.
//!
//! Every pub function takes:
//! - `&mut Registers`
//!
//! They can optionally take:
//! - `&Bus`: load instructions.
//!   - `AddrMode`: load instructions with an addressing mode that is not implicit.
//! - `&mut Bus`: store or load-modify-store instructions.
//!   - `AddrMode`: store or load-modify-store instructions with an addressing mode that
//!     is not implicit.
//! - `u16`: JMP
//! - `&mut Bus` and `u16`: JSR
//!
//! Every function returns the number of additional cycles it is supposed to take. This
//! is always 0, 1, or 2.
//!
//! The rules are:
//! - Add one cycle if load addressing crossed a page boundary (calculated by
//!   `AddrMode`).
//! - Add one cycle on branch instructions if the branch was taken.
//! - Add another cycle on branch instructions if we jumped across a page boundary.

use crate::bus::Bus;
use crate::cpu::addressing::AddrMode;
use crate::cpu::{Registers, Status};

const STACK_BASE: u16 = 0x0100;
const IRQ_BASE: u16 = 0xFFFE;

// Access instructions.

pub fn lda(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.a = m.load(regs, bus);
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    m.extra_cycles_needed()
}

pub fn sta(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    m.store(regs, bus, regs.a);
    0
}

pub fn ldx(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.x = m.load(regs, bus);
    regs.ps.set_zero(regs.x == 0);
    regs.ps.set_if_negative(regs.x);
    m.extra_cycles_needed()
}

pub fn stx(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    m.store(regs, bus, regs.x);
    0
}

pub fn ldy(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.y = m.load(regs, bus);
    regs.ps.set_zero(regs.y == 0);
    regs.ps.set_if_negative(regs.y);
    m.extra_cycles_needed()
}

pub fn sty(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    m.store(regs, bus, regs.y);
    0
}

// Transfer instructions.

pub fn tax(regs: &mut Registers) -> u8 {
    regs.x = regs.a;
    regs.ps.set_zero(regs.x == 0);
    regs.ps.set_if_negative(regs.x);
    0
}

pub fn tsx(regs: &mut Registers) -> u8 {
    regs.x = regs.sp;
    regs.ps.set_zero(regs.x == 0);
    regs.ps.set_if_negative(regs.x);
    0
}

pub fn txa(regs: &mut Registers) -> u8 {
    regs.a = regs.x;
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    0
}

pub fn tya(regs: &mut Registers) -> u8 {
    regs.a = regs.y;
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    0
}

pub fn tay(regs: &mut Registers) -> u8 {
    regs.y = regs.a;
    regs.ps.set_zero(regs.y == 0);
    regs.ps.set_if_negative(regs.y);
    0
}

pub fn txs(regs: &mut Registers) -> u8 {
    regs.sp = regs.x;
    // stack pointer doesn't affect status flags.
    0
}

// Arithmetic instructions.

pub fn adc(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let carry_in = u8::from(regs.ps.carry());
    let operand = m.load(regs, bus);

    let (result1, carry1) = regs.a.overflowing_add(operand);
    let (result, carry2) = result1.overflowing_add(carry_in);
    let carry = carry1 || carry2;

    // The classic formula for the overflow flag is C6 ^ C7. C6 is calculated by doing the
    // addition again, this time with only the 7 LSBs of register A and the operand. If the
    // result is >= 0x80, there was a carry out of bit 6.
    let bit_6_carry = (regs.a & 0x7F) + (operand & 0x7F) + carry_in >= 1 << 7;

    regs.a = result;
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    regs.ps.set_carry(carry);
    regs.ps.set_overflow(bit_6_carry ^ carry);
    m.extra_cycles_needed()
}

pub fn sbc(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let borrow_in = u8::from(!regs.ps.carry());
    let operand = m.load(regs, bus);

    let (result1, borrowed1) = regs.a.overflowing_sub(operand);
    let (result, borrowed2) = result1.overflowing_sub(borrow_in);
    let borrowed = borrowed1 || borrowed2;

    // The classic formula for the overflow flag is still C6 ^ C7.
    let bit_6_borrowed = (regs.a & 0x7F) < (operand & 0x7F) + borrow_in;

    regs.a = result;
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    regs.ps.set_carry(!borrowed);
    regs.ps.set_overflow(bit_6_borrowed ^ borrowed);
    m.extra_cycles_needed()
}

pub fn inc(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus).wrapping_add(1);
    m.store(regs, bus, temp);
    regs.ps.set_zero(temp == 0);
    regs.ps.set_if_negative(temp);
    0
}

pub fn dec(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus).wrapping_sub(1);
    m.store(regs, bus, temp);
    regs.ps.set_zero(temp == 0);
    regs.ps.set_if_negative(temp);
    0
}

pub fn inx(regs: &mut Registers) -> u8 {
    regs.x = regs.x.wrapping_add(1);
    regs.ps.set_zero(regs.x == 0);
    regs.ps.set_if_negative(regs.x);
    0
}

pub fn dex(regs: &mut Registers) -> u8 {
    regs.x = regs.x.wrapping_sub(1);
    regs.ps.set_zero(regs.x == 0);
    regs.ps.set_if_negative(regs.x);
    0
}

pub fn iny(regs: &mut Registers) -> u8 {
    regs.y = regs.y.wrapping_add(1);
    regs.ps.set_zero(regs.y == 0);
    regs.ps.set_if_negative(regs.y);
    0
}

pub fn dey(regs: &mut Registers) -> u8 {
    regs.y = regs.y.wrapping_sub(1);
    regs.ps.set_zero(regs.y == 0);
    regs.ps.set_if_negative(regs.y);
    0
}

// Shift instructions.

pub fn asl(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus);
    let result = temp << 1;
    regs.ps.set_carry(temp & (1 << 7) != 0);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    m.store(regs, bus, result);
    0
}

pub fn lsr(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus);
    let result = temp >> 1;
    regs.ps.set_carry(temp & (1 << 0) != 0);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    m.store(regs, bus, result);
    0
}

pub fn rol(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus);
    let result = temp << 1 | u8::from(regs.ps.carry());
    regs.ps.set_carry(temp & (1 << 7) != 0);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    m.store(regs, bus, result);
    0
}

pub fn ror(regs: &mut Registers, bus: &mut impl Bus, m: AddrMode) -> u8 {
    let temp = m.load(regs, bus);
    let result = u8::from(regs.ps.carry()) << 7 | temp >> 1;
    regs.ps.set_carry(temp & (1 << 0) != 0);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    m.store(regs, bus, result);
    0
}

// Bitwise instructions.

pub fn and(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.a &= m.load(regs, bus);
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    m.extra_cycles_needed()
}

pub fn ora(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.a |= m.load(regs, bus);
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    m.extra_cycles_needed()
}

pub fn eor(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    regs.a ^= m.load(regs, bus);
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    m.extra_cycles_needed()
}

pub fn bit(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let operand = m.load(regs, bus);
    let result = regs.a & operand;
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(operand);
    regs.ps.set_overflow(operand & (1 << 6) != 0);
    0
}

// Compare instructions.

pub fn cmp(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let operand = m.load(regs, bus);
    let result = regs.a.wrapping_sub(operand);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    regs.ps.set_carry(regs.a >= operand);
    m.extra_cycles_needed()
}

pub fn cpx(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let operand = m.load(regs, bus);
    let result = regs.x.wrapping_sub(operand);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    regs.ps.set_carry(regs.x >= operand);
    0
}

pub fn cpy(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    let operand = m.load(regs, bus);
    let result = regs.y.wrapping_sub(operand);
    regs.ps.set_zero(result == 0);
    regs.ps.set_if_negative(result);
    regs.ps.set_carry(regs.y >= operand);
    0
}

// Jump instructions.

pub fn jmp(regs: &mut Registers, operand: u16) -> u8 {
    regs.pc = operand;
    0
}

pub fn jsr(regs: &mut Registers, bus: &mut impl Bus, operand: u16) -> u8 {
    // Return address is one below the address of the instruction after the JSR.
    let return_addr = regs.pc.wrapping_sub(1);
    let [low, high] = return_addr.to_le_bytes();
    stack_push(regs, bus, high);
    stack_push(regs, bus, low);
    regs.pc = operand;
    0
}

pub fn rts(regs: &mut Registers, bus: &impl Bus) -> u8 {
    let low = stack_pop(regs, bus);
    let high = stack_pop(regs, bus);
    regs.pc = u16::from_le_bytes([low, high]).wrapping_add(1);
    0
}

pub fn brk(regs: &mut Registers, bus: &mut impl Bus) -> u8 {
    // brk is a 1 byte instruction but skips the next byte.
    regs.pc = regs.pc.wrapping_add(1);
    let [low, high] = regs.pc.to_le_bytes();
    stack_push(regs, bus, high);
    stack_push(regs, bus, low);
    // TODO: implement 6502 bug.
    stack_push(regs, bus, regs.ps.to_pushable());
    regs.ps.set_interrupt(true);
    let irq_lo = bus.read(IRQ_BASE);
    let irq_hi = bus.read(IRQ_BASE + 1);
    regs.pc = u16::from_le_bytes([irq_lo, irq_hi]);
    0
}

pub fn rti(regs: &mut Registers, bus: &impl Bus) -> u8 {
    plp(regs, bus);
    let low = stack_pop(regs, bus);
    let high = stack_pop(regs, bus);
    regs.pc = u16::from_le_bytes([low, high]);
    0
}

// Stack manipluation instructions.

pub fn pha(regs: &mut Registers, bus: &mut impl Bus) -> u8 {
    stack_push(regs, bus, regs.a);
    0
}

pub fn pla(regs: &mut Registers, bus: &impl Bus) -> u8 {
    regs.a = stack_pop(regs, bus);
    regs.ps.set_zero(regs.a == 0);
    regs.ps.set_if_negative(regs.a);
    0
}

pub fn php(regs: &mut Registers, bus: &mut impl Bus) -> u8 {
    stack_push(regs, bus, regs.ps.to_pushable());
    0
}

pub fn plp(regs: &mut Registers, bus: &impl Bus) -> u8 {
    regs.ps = Status::from_popped(stack_pop(regs, bus));
    0
}

fn stack_push(regs: &mut Registers, bus: &mut impl Bus, value: u8) {
    bus.write(STACK_BASE.wrapping_add(regs.sp.into()), value);
    regs.sp = regs.sp.wrapping_sub(1);
}

fn stack_pop(regs: &mut Registers, bus: &impl Bus) -> u8 {
    regs.sp = regs.sp.wrapping_add(1);
    bus.read(STACK_BASE.wrapping_add(regs.sp.into()))
}

// Branch instructions. All have a relative address passed in as operand. This
// is exactly like immediate addressing.

pub fn bcc(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if !regs.ps.carry() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bcs(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if regs.ps.carry() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn beq(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if regs.ps.zero() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bne(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if !regs.ps.zero() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bpl(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if !regs.ps.negative() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bmi(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if regs.ps.negative() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bvc(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if !regs.ps.overflow() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

pub fn bvs(regs: &mut Registers, bus: &impl Bus, m: AddrMode) -> u8 {
    if regs.ps.overflow() {
        let operand = m.load(regs, bus);
        branch(regs, operand)
    } else {
        0
    }
}

fn branch(regs: &mut Registers, operand: u8) -> u8 {
    let offset = i8::from_le_bytes([operand]);
    let new_pc = regs.pc.wrapping_add_signed(offset.into());
    let extra_cycles_taken = if page_of(regs.pc) == page_of(new_pc) {
        1
    } else {
        2
    };
    regs.pc = new_pc;
    extra_cycles_taken
}

/// Return which of the 256 pages of memory addr resides in.
fn page_of(addr: u16) -> u16 {
    addr & 0xFF00
}

// Flag manipulation instructions.

pub fn clc(regs: &mut Registers) -> u8 {
    regs.ps.set_carry(false);
    0
}

pub fn sec(regs: &mut Registers) -> u8 {
    regs.ps.set_carry(true);
    0
}

pub fn cli(regs: &mut Registers) -> u8 {
    regs.ps.set_interrupt(false);
    0
}

pub fn sei(regs: &mut Registers) -> u8 {
    regs.ps.set_interrupt(true);
    0
}

pub fn cld(regs: &mut Registers) -> u8 {
    regs.ps.set_decimal(false);
    0
}

pub fn sed(regs: &mut Registers) -> u8 {
    regs.ps.set_decimal(true);
    0
}

pub fn clv(regs: &mut Registers) -> u8 {
    regs.ps.set_overflow(false);
    0
}

// No-op instructions.

pub fn nop() -> u8 {
    0
}
