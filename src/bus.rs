//! Owns the memory bus and handles memory mapping.

pub trait Bus {
    /// Returns the byte corresponding to the address, whether that be in RAM or
    /// data from another device on the bus.
    #[must_use]
    fn read(&self, addr: u16) -> u8;

    /// Write data on to the bus.
    fn write(&mut self, addr: u16, value: u8);
}

const CPU_RAM_SIZE: usize = 2048;

pub struct NesBus {
    cpu_ram: Box<[u8; CPU_RAM_SIZE]>,
}

impl Default for NesBus {
    fn default() -> Self {
        Self {
            cpu_ram: vec![0; CPU_RAM_SIZE]
                .try_into()
                .expect("this is the idiom to create arrays on the heap."),
        }
    }
}

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
            0x0000..0x2000 => self.cpu_ram[usize::from(mod_2048(addr))],

            _ => unimplemented!(),
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            // The CPU's internal memory.
            0x0000..0x2000 => self.cpu_ram[usize::from(mod_2048(addr))] = value,

            _ => unimplemented!(),
        }
    }
}

/// Calculates x mod 2048. Any number n mod 2^n is equal to n & 2^(n-1).
fn mod_2048(x: u16) -> u16 {
    x & 0x07FF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading1() {
        let bus = NesBus::default();
        assert_eq!(bus.read(0x0000), 0);
        assert_eq!(bus.read(0x1234), 0);
        assert_eq!(bus.read(0x1FFF), 0);
    }

    #[test]
    #[should_panic = "not implemented"]
    fn reading2() {
        let bus = NesBus::default();
        _ = bus.read(0x2000);
    }

    #[test]
    fn writing() {
        let cases = [(0x0000_u16, 127_u8), (0x0001, 150), (0x0173, 9)];

        let mut bus = NesBus::default();
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
    #[should_panic = "not implemented"]
    fn writing2() {
        let mut bus = NesBus::default();
        _ = bus.write(0x2000, 255);
    }
}
