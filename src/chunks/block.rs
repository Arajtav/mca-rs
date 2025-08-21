use nbt_rs::types::{NbtCompound, NbtString};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Block {
    pub(crate) name: NbtString,
    pub(crate) properties: Option<NbtCompound>,
}

impl Block {
    pub fn get_name(&self) -> &NbtString {
        &self.name
    }

    pub fn get_properties(&self) -> &Option<NbtCompound> {
        &self.properties
    }
}
