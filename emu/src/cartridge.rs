use std::io::Read;

use crate::util::NesError;

pub const CHR_ROM_SIZE: u16 = 8 * 1024;
pub const PRG_ROM_SIZE: u16 = 16 * 1024;

#[derive(Debug)]
pub struct Cartridge {
    prg: Box<[u8]>,
    chr: Box<[u8]>,
    nmtable_mirroring: Orientation,
}

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
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
    pub fn new(mut r: impl Read) -> Result<Self, NesError> {
        let mut header = [0_u8; 16];
        r.read_exact(&mut header)?;

        // magic bytes
        if &header[..4] != b"NES\x1A" {
            return Err(NesError::RomParsing);
        }

        // prg rom size in 16 KiB chunks
        if header[4] != 1 {
            return Err(NesError::RomParsing);
        }

        // chr rom size in 8 KiB chunks
        if header[5] != 1 {
            return Err(NesError::RomParsing);
        }

        // no trainer support
        if header[6] & (1 << 2) != 0 {
            return Err(NesError::RomParsing);
        }

        let nmtable_mirroring = if header[6] & (1 << 0) != 0 {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        };

        let mut prg = vec![0; usize::from(PRG_ROM_SIZE)].into_boxed_slice();
        r.read_exact(&mut prg)?;

        let mut chr = vec![0; usize::from(CHR_ROM_SIZE)].into_boxed_slice();
        r.read_exact(&mut chr)?;

        Ok(Self {
            prg,
            chr,
            nmtable_mirroring,
        })
    }

    #[must_use]
    pub fn chr(&self) -> &[u8] {
        &self.chr
    }

    #[must_use]
    pub fn prg(&self) -> &[u8] {
        &self.prg
    }

    pub fn nmtable_mirroring(&self) -> Orientation {
        self.nmtable_mirroring
    }

    #[cfg(test)]
    pub fn debug_default() -> Self {
        Self {
            prg: vec![0; usize::from(PRG_ROM_SIZE)].into_boxed_slice(),
            chr: vec![0; usize::from(CHR_ROM_SIZE)].into_boxed_slice(),
            nmtable_mirroring: Orientation::Horizontal,
        }
    }
}
