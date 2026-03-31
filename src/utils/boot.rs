//!
//! boot.rs
//! 
//! Boot-time module for the kernel.
//! 

use ::x86_64::registers::model_specific::{Efer, EferFlags};

/// Initialises the EFER register to allow for x86_64 NO_EXECUTE page table flags.
pub fn init_efer() {
    unsafe {
        Efer::update(|flags| {
            *flags |= EferFlags::NO_EXECUTE_ENABLE;
        })
    }
}