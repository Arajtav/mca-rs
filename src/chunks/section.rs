use std::rc::Rc;

use crate::chunks::block::Block;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Section {
    pub(crate) blocks: [Rc<Block>; 4096],
}

impl Section {
    #[inline(always)]
    fn get_block_pos(x: u8, y: u8, z: u8) -> usize {
        let (x, y, z) = (x as u32, y as u32, z as u32);
        return ((((y << 4) | z) << 4) | x) as usize;
    }

    pub fn get_block(&self, x: u8, y: u8, z: u8) -> Option<&Block> {
        if x >= 16 || y >= 16 || z >= 16 {
            return None;
        }

        return Some(&self.blocks[Section::get_block_pos(x, y, z)]);
    }

    pub fn set_block(&mut self, x: u8, y: u8, z: u8, block: Block) {
        if x >= 16 || y >= 16 || z >= 16 {
            return;
        }

        self.blocks[Section::get_block_pos(x, y, z)] = Rc::new(block);
    }
}
