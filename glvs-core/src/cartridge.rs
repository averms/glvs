extern crate alloc;
use alloc::boxed::Box;

use crate::util;
use crate::util::NesError;

const HEADER_SIZE: usize = 16;
pub const CHR_ROM_SIZE: u16 = 8 * 1024;
pub const PRG_ROM_SIZE: u16 = 16 * 1024;

#[derive(Debug)]
pub struct Cartridge {
    data: Box<[u8]>,
    chr_idx: usize,
    nmtable_mirroring: MirrorMode,
}

#[derive(Debug, Clone, Copy)]
pub enum MirrorMode {
    Horizontal,
    Vertical,
}

impl Cartridge {
    /// Read an NES ROM file. Currently, this supports a subset of the iNES format.
    ///
    /// # Errors
    ///
    /// Returns an [`NesError::RomParsing`] if the ROM file couldn't be parsed and an
    /// [`NesError::Io`] if there was an I/O error.
    pub fn new(data: Box<[u8]>) -> Result<Self, NesError> {
        if data.len() < (16 + usize::from(PRG_ROM_SIZE) + usize::from(CHR_ROM_SIZE)) {
            return Err(NesError::RomParsing);
        }

        // magic bytes
        if &data[..4] != b"NES\x1A" {
            return Err(NesError::RomParsing);
        }

        // prg rom size in 16 KiB chunks
        if data[4] != 1 {
            return Err(NesError::RomParsing);
        }

        // chr rom size in 8 KiB chunks
        if data[5] != 1 {
            return Err(NesError::RomParsing);
        }

        // no support for trainers, PRG RAM, VS Unisystem, or PlayChoice-10.
        if util::bit(data[6], 1)
            || util::bit(data[6], 2)
            || util::bit(data[7], 0)
            || util::bit(data[7], 1)
        {
            return Err(NesError::RomParsing);
        }

        let nmtable_mirroring = if util::bit(data[6], 0) {
            MirrorMode::Vertical
        } else {
            MirrorMode::Horizontal
        };

        Ok(Self {
            data,
            chr_idx: 16 + usize::from(PRG_ROM_SIZE),
            nmtable_mirroring,
        })
    }

    #[must_use]
    pub fn chr(&self) -> &[u8] {
        &self.data[self.chr_idx..]
    }

    #[must_use]
    pub fn prg(&self) -> &[u8] {
        &self.data[HEADER_SIZE..self.chr_idx]
    }

    pub fn nmtable_mirroring(&self) -> MirrorMode {
        self.nmtable_mirroring
    }

    #[cfg(test)]
    pub fn debug_default() -> Self {
        Self {
            data:
                alloc::vec![0; HEADER_SIZE + usize::from(PRG_ROM_SIZE) + usize::from(CHR_ROM_SIZE)]
                    .into_boxed_slice(),
            chr_idx: 0,
            nmtable_mirroring: MirrorMode::Horizontal,
        }
    }
}
