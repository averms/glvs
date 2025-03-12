//! Owns the memory bus and handles memory mapping.

const BUS_SIZE: usize = 64 * 1024;

pub struct Bus {
    ram: Box<[u8; BUS_SIZE]>,
}

impl Default for Bus {
    fn default() -> Self {
        Bus {
            ram: vec![0; BUS_SIZE]
                .try_into()
                .expect("this is the idiom to create arrays on the heap."),
        }
    }
}

impl Bus {
    /// Returns the byte corresponding to the address, whether that be in RAM or
    /// data from another device on the bus.
    #[must_use]
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0xFFFF => self.ram[usize::from(addr)],
        }
    }

    /// Write data on to the bus.
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0xFFFF => self.ram[usize::from(addr)] = data,
        }
    }

    pub fn reset(&mut self) {
        self.ram.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_works() {
        let bus = Bus::default();
        assert_eq!(bus.read(0x0000), 0);
        assert_eq!(bus.read(0x1234), 0);
        assert_eq!(bus.read(0xFFFF), 0);
    }

    #[test]
    fn writing_works() {
        let cases = [(0x0000_u16, 127_u8), (0x4321, 150), (0xFFFF, 9)];

        let mut bus = Bus::default();
        for (addr, val) in cases {
            bus.write(addr, val);
        }
        for (addr, val) in cases {
            assert_eq!(bus.read(addr), val);
        }
    }
}
