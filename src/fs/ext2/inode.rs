use zerocopy::{FromBytes, LittleEndian, U16, U32};

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, FromBytes)]
pub struct Ext2Inode {
    pub mode: U16<LittleEndian>,             // File mode and type
    pub uid: U16<LittleEndian>,              // User ID
    pub size: U32<LittleEndian>,             // File size in bytes (lower 32 bits for large files in rev 1+)
    pub atime: U32<LittleEndian>,            // Last access time (POSIX timestamp)
    pub ctime: U32<LittleEndian>,            // Creation time (POSIX timestamp)
    pub mtime: U32<LittleEndian>,            // Last modification time (POSIX timestamp)
    pub dtime: U32<LittleEndian>,            // Deletion time (POSIX timestamp)
    pub gid: U16<LittleEndian>,              // Group ID
    pub links_count: U16<LittleEndian>,      // Link count
    pub blocks: U32<LittleEndian>,           // Blocks count (in 512-byte sectors)
    pub flags: U32<LittleEndian>,            // Inode flags
    pub osd1: U32<LittleEndian>,             // OS dependent field 1
    pub block: [U32<LittleEndian>; 15],      // Block pointers (direct, indirect, doubly, triply)
    pub generation: U32<LittleEndian>,       // File version/generation number (NFS)
    pub file_acl: U32<LittleEndian>,         // File ACL (Extended Attribute) block number (rev 0: always 0)
    pub dir_acl: U32<LittleEndian>,          // Directory ACL block number or high 32 bits of file size (rev 1+)
    pub faddr: U32<LittleEndian>,            // Fragment address (obsolete)
    pub osd2: [u8; 12],                      // OS dependent field 2 (12 bytes)
}