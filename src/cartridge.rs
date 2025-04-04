use std::fs::File;
use std::io::{BufReader, Read as _};

use crate::NesError;

#[derive(Debug)]
pub struct Cartridge {
    prg: Box<[u8]>,
    chr: Box<[u8]>,
}

impl Cartridge {
    /// Read an NES ROM file. Currently this supports a subset of the iNES format.
    ///
    /// # Errors
    ///
    /// Returns an [`NesError`].
    pub fn new(f: &File) -> Result<Self, NesError> {
        let mut reader = BufReader::with_capacity(64 * 1024, f);

        let mut header = [0_u8; 16];
        reader.read_exact(&mut header)?;

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

        let mut prg = vec![0; 16 * 1024].into_boxed_slice();
        reader.read_exact(&mut prg)?;

        let mut chr = vec![0; 8 * 1024].into_boxed_slice();
        reader.read_exact(&mut chr)?;

        Ok(Self { prg, chr })
    }

    #[must_use]
    pub fn prg_chunks_count(&self) -> usize {
        self.prg.len() / (16 * 1024)
    }

    #[must_use]
    pub fn chr_chunks_count(&self) -> usize {
        self.chr.len() / (8 * 1024)
    }
}
