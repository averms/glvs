extern crate alloc;
use alloc::boxed::Box;

use crate::cartridge::{Cartridge, PRG_ROM_SIZE};
use crate::ppu::{Canvas, Ppu, RegisterKind};
use crate::util::NesError;

const CPU_RAM_SIZE: u16 = 2 * 1024;

/// The main bus.
pub trait Bus {
    /// Returns the byte corresponding to the address, whether that be in RAM or
    /// data from another device on the bus.
    #[must_use]
    fn read(&mut self, addr: u16) -> u8;

    /// Write data on to the bus.
    fn write(&mut self, addr: u16, value: u8);
}

/// Owns the memory bus, PPU, and APU. Also handles memory mapping.
#[derive(Debug)]
pub struct NesBus {
    cpu_ram: Box<[u8; CPU_RAM_SIZE as usize]>,
    cart: Cartridge,
    ppu: Ppu,
    dma_page: u8,
    dma_idx: u8,
    pub in_dma_transfer: bool,
    controller_latches: [u8; 2],
    pub controllers: [u8; 2],
}

impl NesBus {
    /// Create a new `NesBus`. `rom_data` must include an iNES header.
    ///
    /// # Errors
    /// When `rom_data` fails to be parsed.
    pub fn new(rom_data: Box<[u8]>) -> Result<Self, NesError> {
        let cart = Cartridge::new(rom_data)?;
        let ppu = Ppu::new(&cart);

        Ok(Self {
            cpu_ram: alloc::vec![0; usize::from(CPU_RAM_SIZE)]
                .try_into()
                .expect("boxed array idiom should work"),
            cart,
            ppu,
            dma_page: 0,
            dma_idx: 0,
            in_dma_transfer: false,
            controller_latches: [0; 2],
            controllers: [0; 2],
        })
    }

    /// Perform one clock-cycle worth of emulation.
    pub fn tick(&mut self, canvas: &mut impl Canvas) {
        self.ppu.tick(&self.cart, canvas);
    }

    #[must_use]
    pub fn dma_read(&self) -> u8 {
        self.cpu_ram[usize::from(self.dma_page) << 8 | usize::from(self.dma_idx)]
    }

    pub fn dma_write(&mut self, value: u8) -> Option<()> {
        let object_idx = self.dma_idx / 4;
        let field_idx = self.dma_idx % 4;
        self.ppu.oam[usize::from(object_idx)][field_idx] = value;

        if let Some(new_idx) = self.dma_idx.checked_add(1) {
            self.dma_idx = new_idx;
            Some(())
        } else {
            self.in_dma_transfer = false;
            None
        }
    }

    #[must_use]
    pub fn frame_complete(&self) -> bool {
        self.ppu.frame_complete
    }

    pub fn set_frame_complete(&mut self, value: bool) {
        self.ppu.frame_complete = value;
    }

    #[must_use]
    pub fn ack_nmi(&mut self) -> bool {
        if self.ppu.nmi {
            self.ppu.nmi = false;
            true
        } else {
            false
        }
    }
}

/// This implementation supports only NROM-128 mappers.
impl Bus for NesBus {
    fn read(&mut self, addr: u16) -> u8 {
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

            0x2000..0x4000 => self.ppu.read_register(
                &self.cart,
                match (addr - 0x2000) % 8 {
                    0 => RegisterKind::Ctrl,
                    1 => RegisterKind::Mask,
                    2 => RegisterKind::Status,
                    3 => RegisterKind::OamAddr,
                    4 => RegisterKind::OamData,
                    5 => RegisterKind::Scroll,
                    6 => RegisterKind::Addr,
                    7 => RegisterKind::Data,
                    _ => unreachable!(),
                },
            ),

            0x4000..0x4016 => {
                // todo
                0
            }

            0x4016 | 0x4017 => {
                let i = usize::from((addr - 0x4016) % 2);
                let result = self.controller_latches[i] >> 7;
                self.controller_latches[i] <<= 1;
                result
            }

            0x8000..=0xFFFF => self.cart.prg()[usize::from((addr - 0x8000) % PRG_ROM_SIZE)],

            // _ => unimplemented!("read from open bus"),
            _ => 0,
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..0x2000 => self.cpu_ram[usize::from(addr % CPU_RAM_SIZE)] = value,

            0x2000..0x4000 => self.ppu.write_register(
                match (addr - 0x2000) % 8 {
                    0 => RegisterKind::Ctrl,
                    1 => RegisterKind::Mask,
                    2 => RegisterKind::Status,
                    3 => RegisterKind::OamAddr,
                    4 => RegisterKind::OamData,
                    5 => RegisterKind::Scroll,
                    6 => RegisterKind::Addr,
                    7 => RegisterKind::Data,
                    _ => unreachable!(),
                },
                value,
            ),

            0x4000..0x4014 | 0x4015 => {
                // todo
            }

            // OAM Direct memory access
            0x4014 => {
                self.dma_page = value;
                self.dma_idx = 0;
                self.in_dma_transfer = true;
            }

            0x4016 | 0x4017 => {
                let i = usize::from(addr % 2);
                self.controller_latches[i] = self.controllers[i];
            }

            // _ => unimplemented!("write to open bus"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading() {
        let mut bus = debug_bus();
        assert_eq!(bus.read(0x0000), 0);
        assert_eq!(bus.read(0x1234), 0);
        assert_eq!(bus.read(0x1FFF), 0);
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

    fn debug_bus() -> NesBus {
        let cart = Cartridge::debug_default();
        let ppu = Ppu::new(&cart);

        NesBus {
            cpu_ram: alloc::vec![0; usize::from(CPU_RAM_SIZE)]
                .try_into()
                .expect("boxed array idiom should work"),
            cart,
            ppu,
            dma_page: 0,
            dma_idx: 0,
            in_dma_transfer: false,
            controller_latches: [0; 2],
            controllers: [0; 2],
        }
    }
}
