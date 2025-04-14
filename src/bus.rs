//! Owns the memory bus and handles memory mapping.

use crate::cartridge::{CHR_ROM_SIZE, Cartridge, PRG_ROM_SIZE};
use crate::util::NesError;

const CPU_RAM_SIZE: u16 = 2 * 1024;

/// The main bus.
pub trait Bus {
    /// Returns the byte corresponding to the address, whether that be in RAM or
    /// data from another device on the bus.
    #[must_use]
    fn read(&self, addr: u16) -> u8;

    /// Write data on to the bus.
    fn write(&mut self, addr: u16, value: u8);
}

/// The PPU bus.
pub trait PpuBus {
    /// Returns the byte corresponding to the address, whether that be in RAM or
    /// data from another device on the bus.
    #[must_use]
    fn ppu_read(&self, addr: u16) -> u8;

    /// Write data on to the bus.
    fn ppu_write(&mut self, addr: u16, value: u8);
}

#[derive(Debug)]
pub struct NesBus {
    cpu_ram: Box<[u8; CPU_RAM_SIZE as usize]>,
    rom: Cartridge,
}

impl NesBus {
    /// Create a new `NesBus`. This is a pretty central struct that owns a lot of data.
    ///
    /// # Errors
    /// When `rom_data` fails to be parsed.
    pub fn new(rom_data: &[u8]) -> Result<Self, NesError> {
        Ok(Self {
            cpu_ram: vec![0; usize::from(CPU_RAM_SIZE)]
                .try_into()
                .expect("boxed array idiom should work"),
            rom: Cartridge::new(rom_data)?,
        })
    }
}

/// This implementation supports only NROM-128 mappers.
impl Bus for NesBus {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            // The CPU's internal memory. The first 8 KiB, meaning address ranges
            //
            // - 0x0800-0x0FFF
            // - 0x1000-0x17FF
            // - 0x1800-0x1FFF
            //
            // are views of 0x0000-0x7FFF. The NES dev community calls this mirroring,
            // but there isn't any reflection going on.
            0x0000..0x2000 => self.cpu_ram[usize::from(addr % CPU_RAM_SIZE)],

            0x2000..0x4000 => todo!("ppu regs"),

            0x4000..0x4018 => todo!("apu and i/o"),

            0x8000..=0xFFFF => self.rom.prg()[usize::from((addr - 0x8000) % PRG_ROM_SIZE)],

            _ => unimplemented!(),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // The CPU's internal memory.
            0x0000..0x2000 => self.cpu_ram[usize::from(addr % CPU_RAM_SIZE)] = value,

            0x2000..0x4000 => todo!("ppu regs"),

            0x4000..0x4018 => todo!("apu and i/o"),

            _ => unimplemented!(),
        }
    }
}

impl PpuBus for NesBus {
    fn ppu_read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..CHR_ROM_SIZE => self.rom.chr()[usize::from(addr)],
            _ => unimplemented!(),
        }
    }

    fn ppu_write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..CHR_ROM_SIZE => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading1() {
        let bus = debug_bus();
        assert_eq!(bus.read(0x0000), 0);
        assert_eq!(bus.read(0x1234), 0);
        assert_eq!(bus.read(0x1FFF), 0);
    }

    #[test]
    #[should_panic = "not yet implemented"]
    fn reading2() {
        let bus = debug_bus();
        _ = bus.read(0x2000);
    }

    #[test]
    fn writing() {
        let cases = [(0x0000_u16, 127_u8), (0x0001, 150), (0x0173, 9)];

        let mut bus = debug_bus();
        for (addr, val) in cases {
            bus.write(addr, val);
        }
        for (addr, val) in cases {
            assert_eq!(bus.read(addr), val);
        }

        assert_eq!(bus.read(0x0800), cases[0].1);
        assert_eq!(bus.read(0x1000), cases[0].1);
        assert_eq!(bus.read(0x1800), cases[0].1);

        assert_eq!(bus.read(0x0801), cases[1].1);
        assert_eq!(bus.read(0x1001), cases[1].1);
        assert_eq!(bus.read(0x1801), cases[1].1);

        assert_eq!(bus.read(0x0973), cases[2].1);
        assert_eq!(bus.read(0x1173), cases[2].1);
        assert_eq!(bus.read(0x1973), cases[2].1);
    }

    #[test]
    #[should_panic = "not yet implemented"]
    fn writing2() {
        let mut bus = debug_bus();
        bus.write(0x2000, 255);
    }

    fn debug_bus() -> NesBus {
        NesBus {
            cpu_ram: vec![0; usize::from(CPU_RAM_SIZE)]
                .try_into()
                .expect("boxed array idiom should work"),
            rom: Cartridge::debug_default(),
        }
    }
}
