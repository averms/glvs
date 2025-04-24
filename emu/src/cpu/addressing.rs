use crate::bus::Bus;
use crate::cpu::Registers;

/// According to <https://www.nesdev.org/wiki/CPU_addressing_modes>, there are 13 addressing modes
/// on the MOS 6502.
///
/// Here we have immediate, accumulator, and all the memory addressing modes
/// except for relative and the addressing modes for JMP and JSR. We consider
/// the relative addressing mode identical in implementation to the immediate
/// addressing mode. We choose not to model the addressing modes for JMP and
/// JSR, which are special instructions because their version of the absolute
/// addressing mode is more like a 2-byte immediate.
#[derive(Debug, Clone, Copy)]
pub enum AddrMode {
    Immediate(u8),
    Accumulator,
    Memory(u16, bool),
}

// TODO: maybe implement this as a trait and the instructions as generic to allow for
// monomorphization.
impl AddrMode {
    pub fn load(self, regs: &Registers, bus: &impl Bus) -> u8 {
        match self {
            Self::Immediate(value) => value,
            Self::Accumulator => regs.a,
            Self::Memory(addr, _) => bus.read(addr),
        }
    }

    pub fn store(self, regs: &mut Registers, bus: &mut impl Bus, value: u8) {
        match self {
            Self::Immediate(_) => unreachable!("can't store to immediate"),
            Self::Accumulator => regs.a = value,
            Self::Memory(addr, _) => bus.write(addr, value),
        }
    }

    /// Return the number of extra cycles needed for getting data from memory.
    /// Always a 0 or 1.
    pub fn extra_cycles_needed(self) -> u8 {
        match self {
            Self::Memory(_, needs_extra_cycle) => needs_extra_cycle.into(),
            Self::Accumulator | Self::Immediate(_) => 0,
        }
    }

    // Constructors.

    pub fn relative(regs: &mut Registers, bus: &impl Bus) -> Self {
        Self::immediate(regs, bus)
    }

    pub fn immediate(regs: &mut Registers, bus: &impl Bus) -> Self {
        Self::Immediate(regs.read_and_bump_pc(bus))
    }

    pub fn zero_page(regs: &mut Registers, bus: &impl Bus) -> Self {
        Self::Memory(regs.read_and_bump_pc(bus).into(), false)
    }

    pub fn zero_page_y(regs: &mut Registers, bus: &impl Bus) -> Self {
        let base = regs.read_and_bump_pc(bus);
        let zero_page_addr = base.wrapping_add(regs.y);
        Self::Memory(zero_page_addr.into(), false)
    }

    pub fn zero_page_x(regs: &mut Registers, bus: &impl Bus) -> Self {
        let base = regs.read_and_bump_pc(bus);
        let zero_page_addr = base.wrapping_add(regs.x);
        Self::Memory(zero_page_addr.into(), false)
    }

    pub fn absolute(regs: &mut Registers, bus: &impl Bus) -> Self {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let addr = u16::from_le_bytes([low, high]);
        Self::Memory(addr, false)
    }

    pub fn absolute_x(regs: &mut Registers, bus: &impl Bus) -> Self {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.x.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        Self::Memory(addr, page_crossed)
    }

    pub fn absolute_y(regs: &mut Registers, bus: &impl Bus) -> Self {
        let low = regs.read_and_bump_pc(bus);
        let high = regs.read_and_bump_pc(bus);
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.y.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        Self::Memory(addr, page_crossed)
    }

    pub fn indirect_indexed(regs: &mut Registers, bus: &impl Bus) -> Self {
        let zero_page_ptr = regs.read_and_bump_pc(bus);
        let low = bus.read(zero_page_ptr.into());
        let high = bus.read(zero_page_ptr.wrapping_add(1).into());
        let base = u16::from_le_bytes([low, high]);
        let addr = base.wrapping_add(regs.y.into());
        let page_crossed = (base & 0xFF00) != (addr & 0xFF00);
        Self::Memory(addr, page_crossed)
    }

    pub fn indexed_indirect(regs: &mut Registers, bus: &impl Bus) -> Self {
        let ptr_base = regs.read_and_bump_pc(bus);
        let ptr = ptr_base.wrapping_add(regs.x);
        let low = bus.read(ptr.into());
        let high = bus.read(ptr.wrapping_add(1).into());
        let addr = u16::from_le_bytes([low, high]);
        Self::Memory(addr, false)
    }
}

pub fn jump_indirect(regs: &mut Registers, bus: &impl Bus) -> u16 {
    let ptr_low = regs.read_and_bump_pc(bus);
    let ptr_high = regs.read_and_bump_pc(bus);
    // replicate 6502 bug where the high byte of the address can be fetched from the
    // wrong page.
    let low = bus.read(u16::from_le_bytes([ptr_low, ptr_high]));
    let high = bus.read(u16::from_le_bytes([ptr_low.wrapping_add(1), ptr_high]));
    u16::from_le_bytes([low, high])
}

pub fn jump_absolute(regs: &mut Registers, bus: &impl Bus) -> u16 {
    let low = regs.read_and_bump_pc(bus);
    let high = regs.read_and_bump_pc(bus);
    u16::from_le_bytes([low, high])
}
