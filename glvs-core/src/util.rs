use core::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum NesError {
    RomParsing,
}

impl core::error::Error for NesError {}

impl fmt::Display for NesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::RomParsing => write!(f, "rom loading error"),
        }
    }
}

pub fn bit(byte: u8, bit: u8) -> bool {
    byte >> bit & 1 != 0
}
