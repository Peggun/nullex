use zerocopy::{FromBytes, LittleEndian, U16, U32, U64, U128};

pub enum CompatibleFeatures {
    Ext2FeatureCompatDirPrealloc = 0x0001,
    Ext2FeatureCompatImagicInodes = 0x0002,
    Ext3FeatureCompatHasJournal = 0x0004,
    Ext2FeatureCompatExtAttr = 0x0008,
    Ext2FeatureCompatResizeIno = 0x0010,
    Ext2FeatureCompatDirIndex = 0x0020,
}

pub enum IncompatibleFeatures {
    Ext2FeatureIncompatCompression = 0x0001,
    Ext2FeatureIncompatFiletype = 0x0002,
    Ext3FeatureIncompatRecover = 0x0004,
    Ext3FeatureIncompatJournalDev = 0x0008,
    Ext2FeatureIncompatMetaBg = 0x0010,
}

pub enum CompatibleReadOnlyFeatures {
    Ext2FeatureRoCompatSparseSuper = 0x0001,
    Ext2FeatureRoCompatLargeFile = 0x0002,
    Ext2FeatureRoCompatBtreeDir = 0x0004,
}

pub enum AlgorithmBitmap {
    Ext2Lzv1Alg = 0x00000001,
    Ext2Lzrw3aAlg = 0x00000002,
    Ext2GzipAlg = 0x00000004,
    Ext2Bzip2Alg = 0x00000008,
    Ext2LzoAlg = 0x00000010,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes)]
pub struct Ext2Superblock {
    pub inode_count: U32<LittleEndian>,
    pub block_count: U32<LittleEndian>,
    pub reserved_block_count: U32<LittleEndian>,
    pub free_block_count: U32<LittleEndian>,
    pub free_inode_count: U32<LittleEndian>,
    pub first_data_block: U32<LittleEndian>,
    pub log_block_size: U32<LittleEndian>,
    pub log_frag_size: U32<LittleEndian>,
    pub blocks_per_group: U32<LittleEndian>,
    pub frags_per_group: U32<LittleEndian>,
    pub inodes_per_group: U32<LittleEndian>,
    pub last_mount_time: U32<LittleEndian>,
    pub last_write_time: U32<LittleEndian>,
    pub mounts_since_check: U16<LittleEndian>,
    pub max_mounts: U16<LittleEndian>,
    pub magic: U16<LittleEndian>,
    pub state: U16<LittleEndian>,
    pub errors: U16<LittleEndian>,
    pub minor_rev: U16<LittleEndian>,
    pub last_check: U32<LittleEndian>,
    pub check_interval: U32<LittleEndian>,
    pub creator_os: U32<LittleEndian>,
    pub rev_level: U32<LittleEndian>,
    pub default_resuid: U16<LittleEndian>,
    pub default_resgid: U16<LittleEndian>,
    pub first_inode: U32<LittleEndian>,
    pub inode_size: U16<LittleEndian>, // Corrected to U16
    pub block_group_nr: U16<LittleEndian>, // Corrected to U16
    pub feature_compatible: U32<LittleEndian>, // Typo corrected
    pub feature_incompatible: U32<LittleEndian>,
    pub feature_read_only_compatible: U32<LittleEndian>,
    pub uuid: U128<LittleEndian>,
    pub volume_name: [u8; 16], // Corrected to [u8; 16]
    pub last_mounted: [u8; 64], // Corrected to [u8; 64]
    pub algorithm_bitmap: U32<LittleEndian>,
    pub preallocated_blocks: u8,
    pub preallocated_directory_blocks: u8,
    _alignment: u16, // Added alignment field
    pub journal_uuid: [u8; 16], // Corrected to [u8; 16]
    pub journal_inum: U32<LittleEndian>,
    pub journal_dev: U32<LittleEndian>,
    pub last_orphan: U32<LittleEndian>,
    pub hash_seed: [U32<LittleEndian>; 4],
    pub def_hash_version: u8,
    _padding: [u8; 3], // Added padding field
    pub default_mount_options: U32<LittleEndian>,
    pub first_meta_bg: U32<LittleEndian>, // Renamed to first_meta_bg and corrected name
    _reserved: [u8; 760], // Added reserved space
}

impl Ext2Superblock {
    pub const EXT2_SUPER_MAGIC: u16 = 0xEF53;
    pub const EXT2_GOOD_FS: u16 = 1;
    pub const EXT2_ERROR_FS: u16 = 2;

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic.get() != Self::EXT2_SUPER_MAGIC {
            return Err("Invalid EXT2 magic");
        }
        if self.rev_level.get() > 1 {
            return Err("Unsupported EXT2 revision");
        }
        Ok(())
    }

    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size.get()
    }
}
