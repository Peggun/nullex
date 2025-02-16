use zerocopy::{FromBytes, LittleEndian, U16, U32};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes)]
pub struct Ext2BlockGroupDescriptor {
    pub block_bitmap: U32<LittleEndian>,
    pub inode_bitmap: U32<LittleEndian>,
    pub inode_table: U32<LittleEndian>,
    pub free_blocks_count: U16<LittleEndian>,
    pub free_inodes_count: U16<LittleEndian>,
    pub used_dirs_count: U16<LittleEndian>,
    pub pad: U16<LittleEndian>,
    pub reserved: [u8; 12], // Corrected: Use [u8; 12] for 12 bytes
}