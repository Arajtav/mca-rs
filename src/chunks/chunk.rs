use std::{cmp::max, io::Read, ops::Range, rc::Rc};

use flate2::read::{GzDecoder, ZlibDecoder};
use nbt_rs::get_field as try_get_field;
use nbt_rs::{error::ParseError, parse_nbt};
use thiserror::Error;

use crate::chunks::{block::Block, section::Section};

const COMPRESSION_GZIP: u8 = 1;
const COMPRESSION_ZLIB: u8 = 2;
const COMPRESSION_RAW: u8 = 3;
// const COMPRESSION_LZ4: u8 = 4;
// const COMPRESSION_CUSTOM: u8 = 127;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Chunk {
    y_pos: i32,
    sections: Vec<Section>,
}

impl Chunk {
    pub fn get_y_range(&self) -> Range<i16> {
        let start = self.y_pos as i16 * 16;
        let end = start + self.sections.len() as i16 * 16;
        start..end
    }

    pub fn get(&self, x: u8, y: i16, z: u8) -> Option<&Block> {
        if x >= 16 || !self.get_y_range().contains(&y) || z >= 16 {
            return None;
        }

        let local_y = (y as i32 - self.y_pos * 16) as usize;
        let section = local_y >> 4;
        let block = (local_y as u8) & 0xF;

        self.sections[section].get_block(x, block, z)
    }

    pub fn get_section(&self, y: i32) -> Option<&Section> {
        self.sections.get((y - self.y_pos) as usize)
    }
}

#[derive(Error, Debug)]
pub enum ChunkParseError {
    #[error("input too short, expected at least {0} bytes but got {1}")]
    InputTooShort(usize, usize),

    #[error("the compression format the chunk uses is not supported")]
    UnsupportedCompression,

    #[error("failed to decompress the data: {0}")]
    DecompressionFailed(std::io::Error),

    #[error("failed to parse the chunk: {0}")]
    ParseFailed(ParseError),

    #[error("the filed {0} is missing or has an invalid type")]
    InvalidField(String),

    #[error("the block palette is invalid")]
    InvalidPalette,

    #[error("the section data is invalid")]
    InvalidSectionData,
}

macro_rules! get_field {
    ($input:ident, $field:literal $(, $($ty:ident).*)? ) => {{
        try_get_field!($input, $field $(, $($ty).*)?)
            .ok_or(ChunkParseError::InvalidField($field.to_owned()))?
    }};
}

pub fn parse_chunk(bytes: &[u8]) -> Result<Chunk, ChunkParseError> {
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

    let (_, decoded) = parse_nbt(&data).map_err(ChunkParseError::ParseFailed)?;
    let &y_pos = get_field!(decoded, "yPos", as_int);
    let original_sections = get_field!(decoded, "sections", as_list.as_compound);

    let mut sections: Vec<Section> = Vec::with_capacity(original_sections.len());
    for section in original_sections.iter().cloned() {
        let section = get_field!(section, "block_states", as_compound);
        let original_palette = get_field!(section, "palette", as_list.as_compound);
        let palette_len = original_palette.len();
        if palette_len == 0 && palette_len > 4096 {
            return Err(ChunkParseError::InvalidPalette);
        }

        let mut palette: Vec<Rc<Block>> = Vec::new();
        for block in original_palette.iter() {
            let name = get_field!(block, "Name", as_string).clone();
            let properties = try_get_field!(block, "Properties", as_compound).cloned();
            palette.push(Rc::new(Block { name, properties }));
        }

        let bits_per_index = max(
            4,
            (usize::BITS - (palette_len - 1).leading_zeros()) as usize,
        );

        if palette_len == 1 {
            sections.push(Section {
                blocks: vec![palette[0].clone(); 4096].try_into().unwrap(),
            });
            continue;
        }

        let data: Vec<i64> = get_field!(section, "data", as_long_array).to_vec();

        if data.len() < bits_per_index * 64 {
            return Err(ChunkParseError::InvalidSectionData);
        }

        let mut blocks: Vec<Rc<Block>> = Vec::with_capacity(4096);
        let mask: u64 = (1u64 << bits_per_index) - 1;
        let mut long_idx = 0;
        let mut bit_offset = 0;
        for _ in 0..4096 {
            if bit_offset + bits_per_index as usize > 64 {
                long_idx += 1;
                bit_offset = 0;
                let index = (data[long_idx] as u64 & mask) as usize;
                if index >= palette_len {
                    return Err(ChunkParseError::InvalidSectionData);
                }
                blocks.push(palette[index].clone());
                bit_offset += bits_per_index as usize;
                continue;
            }

            let long = data[long_idx] as u64;
            let index = ((long >> bit_offset) & mask) as usize;
            if index >= palette_len {
                return Err(ChunkParseError::InvalidSectionData);
            }
            blocks.push(palette[index].clone());
            bit_offset += bits_per_index as usize;

            if bit_offset == 64 {
                long_idx += 1;
                bit_offset = 0;
            }
        }

        sections.push(Section {
            blocks: blocks.try_into().unwrap(),
        });
    }

    Ok(Chunk { y_pos, sections })
}
