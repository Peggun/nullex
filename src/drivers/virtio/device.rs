use alloc::vec::Vec;

use crate::{bitflags, drivers::virtio::{Features, virtqueue::VirtQueue}};

bitflags! {
    /// A simple low-level indication of the completed steps in the device 
    /// initialisation.<br>
    /// ## Driver Requirements<br>
    /// The driver MUST update device status, setting bits to indicate the completed steps
    /// of the driver initialization sequence.<br>
    /// 
    /// The driver MUST NOT clear a device status bit. If the driver sets the `FAILED` bit 
    /// the driver MUST later reset the device before attempting to re-initialize<br>
    /// 
    /// The driver SHOULD NOT rely on completion of operations of a device if DEVICE_NEEDS_RESET 
    /// is set.
    /// 
    /// ## Device Requirements<br>
    /// The device MUST NOT consume buffers or send any used buffer notifications to the driver
    /// before `DRIVER_OK`.<br>
    /// 
    /// The device SHOULD set `DEVICE_NEEDS_RESET` when it enters an error state that a reset is needed.
    /// If `DRIVER_OK` is set, after it sets `DEVICE_NEEDS_RESET`, the device MUST send a device configuration
    /// change notification to the driver.
    pub struct VirtIODeviceStatus: u8 {
        const ZERO = 0;

        /// Indicates that the guest OS has found the device and recognised it as a
        /// valid virtio device.
        const ACKNOWLEDGE = 1 << 0;

        /// Indicates that the guest OS knows how to drive the device.
        const DRIVER = 1 << 1;

        /// Indicates that the driver is set up and ready to drive the device.
        const DRIVER_OK = 1 << 2;

        /// Indicates that the driver has acknowledged all the features it
        /// understand, and feature negotiation is complete.
        const FEATURES_OK = 1 << 3;

        /// Indicates that the device has experienced an error from which it
        /// can't recover.
        const DEVICE_NEEDS_RESET = 1 << 6;

        /// Indicates that something went wrong in the guest, and it has given
        /// up on the device.
        const FAILED = 1 << 7;

        /// incase for C АBI
        const _ = !0;
    }
}

pub trait VirtIODeviceTrait {
    fn init(&mut self);
    fn reset(&mut self);

    fn has_feature(&mut self, bit: Features) -> bool;

    fn get_config_generation(&mut self) -> u32;
    fn read_config_field(&mut self, bit: u32) -> Option<u32>;
}

pub struct VirtIODevice {
    pub offered_features: Features,
    pub device_status: VirtIODeviceStatus,
    pub configuration_space: u32,

    pub virtqueues: Vec<Option<VirtQueue>>,

    pub can_send_notifs: bool,
}

impl VirtIODeviceTrait for VirtIODevice {
    fn init(&mut self) {}

    /// Reset the device.
    fn reset(&mut self) {
        self.device_status = VirtIODeviceStatus::ZERO;
        self.can_send_notifs = false;


    }

    fn has_feature(&mut self, bit: Features) -> bool {
        (self.offered_features & bit) != 0
    }

    fn get_config_generation(&mut self) -> u32 {
        self.configuration_space
    }

    fn read_config_field(&mut self, bit: u32) -> Option<u32>{
        if !self.has_feature(VirtIODeviceStatus::FEATURES_OK.bits() as u64) {
            Some(self.configuration_space & bit)
        } else { None }
    }
}