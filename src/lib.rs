use std::{fmt, io};

mod bus;
mod cartridge;
mod cpu;

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

pub use crate::bus::{Bus, NesBus};
pub use crate::cartridge::Cartridge;
pub use crate::cpu::{Cpu, Registers, Status};
