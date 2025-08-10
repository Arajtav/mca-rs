use thiserror::Error;

use crate::chunk::Chunk;

#[derive(Error, Debug)]
pub enum RegionParseError {
    #[error("input too short, expected at least 8192 bytes but got {0}")]
    InputTooShort(usize),

    #[error("input size ({0}) is not multiple of 4096")]
    InputInvalidSize(usize),
}

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
                Chunk::parse_bytes(
                    timestamp,
                    &bytes[offset..offset + ((sector_count as usize) << 12)],
                )
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
        self.chunks.iter().filter(|chunk| chunk.is_some()).count() as u16
    }
}
