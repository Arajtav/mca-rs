use std::io::Read;

use flate2::read::{GzDecoder, ZlibDecoder};
use thiserror::Error;

const COMPRESSION_GZIP: u8 = 1;
const COMPRESSION_ZLIB: u8 = 2;
const COMPRESSION_RAW: u8 = 3;
// const COMPRESSION_LZ4: u8 = 4;
// const COMPRESSION_CUSTOM: u8 = 127;

#[allow(unused)]
#[derive(Debug)]
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

    #[error("failed to decompress the data: {0}")]
    DecompressionFailed(std::io::Error),
}

impl Chunk {
    pub fn parse_bytes(timestamp: u32, bytes: &[u8]) -> Result<Self, ChunkParseError> {
        if bytes.len() < 5 {
            return Err(ChunkParseError::InputTooShort(5, bytes.len()));
        }
        let (header, body) = bytes.split_at(5);
        let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        if bytes.len() < len + 4 {
            return Err(ChunkParseError::InputTooShort(len + 4, bytes.len()));
        }

        let compression_format = header[4];
        let raw_data = &body[..len];

        let data = match compression_format {
            COMPRESSION_GZIP => {
                let mut decoder = GzDecoder::new(raw_data);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(ChunkParseError::DecompressionFailed)?;
                decompressed
            }
            COMPRESSION_ZLIB => {
                let mut decoder = ZlibDecoder::new(raw_data);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(ChunkParseError::DecompressionFailed)?;
                decompressed
            }
            COMPRESSION_RAW => raw_data.to_owned(),
            _ => return Err(ChunkParseError::UnsupportedCompression),
        };

        Ok(Self { timestamp, data })
    }
}
