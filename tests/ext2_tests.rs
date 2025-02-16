#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use nullex::fs::ext2::{block_group_descriptor::Ext2BlockGroupDescriptor, inode::Ext2Inode, superblock::{CompatibleFeatures, Ext2Superblock}};
use zerocopy::FromBytes;

#[unsafe(no_mangle)] // don't mangle the name of this function
pub extern "C" fn _start() -> ! {
    test_main();

    loop {}
}

#[test_case]
fn superblock_size() {
    use core::mem::size_of;
    assert_eq!(size_of::<Ext2Superblock>(), 1024);
}

#[test_case]
fn superblock_magic() {
    let mut bytes = [0u8; 1024];
    // Magic is at offset 56 (0x38)
    bytes[56..58].copy_from_slice(&0xEF53u16.to_le_bytes());
    let superblock = Ext2Superblock::read_from_bytes(&bytes).unwrap();
    assert_eq!(superblock.magic.get(), Ext2Superblock::EXT2_SUPER_MAGIC);
}

#[test_case]
fn superblock_features() {
    let mut bytes = [0u8; 1024];
    // Enable EXT3_FEATURE_COMPAT_HAS_JOURNAL (0x0004)
    bytes[96..100].copy_from_slice(&0x0004u32.to_le_bytes());
    let superblock = Ext2Superblock::read_from_bytes(&bytes).unwrap();
    assert!(superblock.feature_compatible.get() & CompatibleFeatures::Ext3FeatureCompatHasJournal as u32 != 0);
}

#[test_case]
fn inode_size() {
    assert_eq!(size_of::<Ext2Inode>(), 128);
}

#[test_case]
fn inode_mode() {
    let mut bytes = [0u8; 128];
    bytes[0..2].copy_from_slice(&0o755u16.to_le_bytes());
    let inode = Ext2Inode::read_from_bytes(&bytes).unwrap();
    assert_eq!(inode.mode.get(), 0o755);
}

#[test_case]
fn bgd_size() {
    assert_eq!(size_of::<Ext2BlockGroupDescriptor>(), 32);
}

#[test_case]
fn bgd_block_bitmap() {
    let mut bytes = [0u8; 32];
    bytes[0..4].copy_from_slice(&123u32.to_le_bytes());
    let bgd = Ext2BlockGroupDescriptor::read_from_bytes(&bytes).unwrap();
    assert_eq!(bgd.block_bitmap.get(), 123);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    loop {}
}