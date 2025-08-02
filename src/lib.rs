use thiserror::Error;

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

#[repr(u8)]
enum ChunkPayloadCompression {
    Uncompressed = 3,
}

#[allow(unused)]
pub struct Chunk {
    timestamp: u32,
    data: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum ChunkParseError {
    #[error("input too short, expected at least {0} bytes but got {1}")]
    InputTooShort(usize, usize),

    #[error("the compression format the chunk uses is not supported")]
    UnsupportedCompression,
}

impl Chunk {
    fn parse_bytes(timestamp: u32, bytes: &[u8]) -> Result<Self, ChunkParseError> {
        if bytes.len() < 5 {
            return Err(ChunkParseError::InputTooShort(5, bytes.len()));
        }
        let (header, data) = bytes.split_at(5);
        let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        if bytes.len() < len + 4 {
            return Err(ChunkParseError::InputTooShort(len + 4, bytes.len()));
        }

        let compression_format = header[4];
        let data = &data[..len];

        if compression_format != ChunkPayloadCompression::Uncompressed as u8 {
            return Err(ChunkParseError::UnsupportedCompression);
        }

        Ok(Self {
            timestamp,
            data: data.to_owned(),
        })
    }
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
