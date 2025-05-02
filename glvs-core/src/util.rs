use std::{fmt, io};

#[derive(Debug)]
#[non_exhaustive]
pub enum NesError {
    Io(io::ErrorKind),
    RomParsing,
}

impl std::error::Error for NesError {}

impl fmt::Display for NesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Io(kind) => write!(f, "io error: {kind}"),
            Self::RomParsing => write!(f, "rom loading error"),
        }
    }
}

impl From<io::Error> for NesError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.kind())
    }
}

pub fn bit(byte: u8, bit: u8) -> bool {
    byte >> bit & 1 != 0
}
