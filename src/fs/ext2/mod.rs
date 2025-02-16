use block_group_descriptor::Ext2BlockGroupDescriptor;
use inode::Ext2Inode;
use superblock::Ext2Superblock;
use zerocopy::FromBytes;

use crate::align_buffer;

use super::ata::AtaDisk;

pub mod superblock;
pub mod block_group_descriptor;
pub mod inode;