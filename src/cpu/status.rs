/// 6502 status register.
///
/// 0. carry
/// 1. zero
/// 2. interrupt
/// 3. decimal (not actually supported but can be set)
/// 4. break (unused)
/// 5. (unused)
/// 6. overflow
/// 7. negative
#[derive(Default, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Status(pub u8);

impl Status {
    // Getters.
    pub fn carry(self) -> bool {
        self.0 & (1 << 0) != 0
    }
    pub fn zero(self) -> bool {
        self.0 & (1 << 1) != 0
    }
    #[expect(dead_code)]
    pub fn interrupt(self) -> bool {
        self.0 & (1 << 2) != 0
    }
    pub fn overflow(self) -> bool {
        self.0 & (1 << 6) != 0
    }
    pub fn negative(self) -> bool {
        self.0 & (1 << 7) != 0
    }

    // Setters.
    pub fn set_carry(&mut self, c: bool) {
        if c {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }
    pub fn set_zero(&mut self, z: bool) {
        if z {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
    }
    pub fn set_interrupt(&mut self, i: bool) {
        if i {
            self.0 |= 1 << 2;
        } else {
            self.0 &= !(1 << 2);
        }
    }
    pub fn set_decimal(&mut self, d: bool) {
        if d {
            self.0 |= 1 << 3;
        } else {
            self.0 &= !(1 << 3);
        }
    }
    pub fn set_overflow(&mut self, v: bool) {
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

    pub fn set_if_negative(&mut self, num: u8) {
        self.set_negative(i8::from_le_bytes([num]) < 0);
    }

    pub fn to_pushable(self) -> u8 {
        self.0 | 0b0011_0000
    }

    pub fn from_popped(value: u8) -> Status {
        Status(value & 0b1100_1111)
    }
}
