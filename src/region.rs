use thiserror::Error;

use crate::chunk::{Chunk, parse_chunk};

#[derive(Error, Debug)]
pub enum RegionParseError {
    #[error("input too short, expected at least 8192 bytes but got {0}")]
    InputTooShort(usize),

    #[error("input size ({0}) is not multiple of 4096")]
    InputInvalidSize(usize),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Region {
    chunks: [Option<Chunk>; 1024],
}

impl Region {
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, RegionParseError> {
        let len = bytes.len();
        if len < 8192 {
            return Err(RegionParseError::InputTooShort(len));
        }
        if !len.is_multiple_of(4096) {
            return Err(RegionParseError::InputInvalidSize(len));
        }

        let locations = &bytes[0..4096];
        let timestamps = &bytes[4096..8192];

        // the alignment is the same, only the structure changes
        let locations = unsafe { &*(locations.as_ptr() as *const [[u8; 4]; 1024]) };
        let timestamps = unsafe { &*(timestamps.as_ptr() as *const [[u8; 4]; 1024]) };

        let chunks: Vec<Option<Chunk>> = locations
            .iter()
            .zip(timestamps.iter())
            .map(|(&location, &timestamp)| {
                let timestamp = u32::from_be_bytes(timestamp);
                let sector_count: u8 = location[3];
                let offset = ((location[0] as u32) << 16)
                    | ((location[1] as u32) << 8)
                    | (location[2] as u32);

                if offset == 0 && sector_count == 0 && timestamp == 0 {
                    return None;
                }

                let offset = (offset as usize) << 12;
                parse_chunk(&bytes[offset..offset + ((sector_count as usize) << 12)])
                    // TODO: proper error handling
                    .ok()
            })
            .collect();

        Ok(Self {
            // chunks is always 1024 long, since both of the iters are 1024
            chunks: unsafe { chunks.try_into().unwrap_unchecked() },
        })
    }

    pub fn count_chunks(&self) -> u16 {
        1024 - self.chunks.iter().filter(|&chunk| chunk.is_none()).count() as u16
    }

    pub fn get_chunk(&self, x: usize, z: usize) -> Option<&Chunk> {
        if x >= 32 || z >= 32 {
            return None;
        }

        let index = x + z * 32;
        self.chunks[index].as_ref()
    }
}
