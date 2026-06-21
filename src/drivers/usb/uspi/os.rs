use core::{ffi::c_void, hint::spin_loop};

use crate::{drivers::keyboard::scancode::KeyCode::W, rtc::{self, rtc_instant}};

/*
 *  Raspberry Pi Configuration
 *  
 *  You will need to change this before compiling the kernel
 *  if need be.
 *
 */

// todo: figure a why to create this dynamically through something like menuconfig

pub const GPU_L2_CACHE_ENABLED: bool = true;

pub const USPI_DEFAULT_KEYMAP_DE: bool = false;
pub const USPI_DEFAULT_KEYMAP_ES: bool = false;
pub const USPI_DEFAULT_KEYMAP_FR: bool = false;
pub const USPI_DEFAULT_KEYMAP_IT: bool = false;
pub const USPI_DEFAULT_KEYMAP_UK: bool = true;
pub const USPI_DEFAULT_KEYMAP_US: bool = false;

#[cfg(target_arch = "arm")]
pub type TKernelTimerHandle: u32;

#[cfg(target_arch = "aarch64")]
pub type TKernelTimerHandle: u64;

pub type TKernelTimerHandler = fn(
    hTimer: TKernelTimerHandle,
    pParam: *mut c_void,
    pContext: *mut c_void,
);

pub type TInterruptHandler = fn(pParam: *mut c_void);

pub fn ms_delay(nMilliseconds: u32) {
    let end = rtc_instant().total_millis() + nMilliseconds as i64;  
    while rtc_instant().total_millis() < end {
        spin_loop();
    }
}

pub fn us_delay(nMicroseconds: u32) {
    let end = rtc_instant().total_micros() + nMicroseconds as i64;
    while rtc_instant().total_micros() < end {
        spin_loop();
    }
}
