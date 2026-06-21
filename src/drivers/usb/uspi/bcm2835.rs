
#[cfg(feature = "rpi1")]
pub const ARM_IO_BASE: usize = 0x2000000;

// ignore error. this is due to vscode rust-analyzer settings for now.
#[cfg(any(feature = "rpi2", feature = "rpi3"))]
pub const ARM_IO_BASE: usize = 0x3F000000;

pub const ARM_USB_BASE: usize = ARM_IO_BASE + 0x980000;
pub const ARM_USB_CORE_BASE: usize = ARM_USB_BASE;
pub const ARM_USB_HOST_BASE: usize = ARM_USB_BASE + 0x400;
pub const ARM_USB_POWER: usize =     ARM_USB_BASE + 0xE00;
