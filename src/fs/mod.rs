pub mod ata;
pub mod ramfs;

use crate::fs::ramfs::FileSystem;
use spin::Mutex;

pub static FS: Mutex<Option<FileSystem>> = Mutex::new(None);

pub fn init_fs(fs: FileSystem) {
    *FS.lock() = Some(fs);
}

pub fn with_fs<R>(f: impl FnOnce(&mut FileSystem) -> R) -> R {
    let mut fs_lock = FS.lock();
    let fs_ref = fs_lock.as_mut().expect("Filesystem must be initialized");

    // Release VGA lock before FS operations
    unsafe { crate::vga_buffer::WRITER.force_unlock() };
    let result = f(fs_ref);
    crate::vga_buffer::WRITER.lock();

    result
}
