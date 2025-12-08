use crate::{bitflags, utils::align::*};


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
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VirtQueueKind {
    ReceiveQueue = 0,
    TransmitQueue = 1,
}

pub enum VirtQueueBackend {
    Split(Result<SplitVirtQueue, &'static str>),
    Packed(PackedVirtQueue),
}

pub struct VirtQueue {
    pub metadata: VirtQueueMetadata,
    pub backend: VirtQueueBackend,
}

pub struct VirtQueueMetadata {
    pub index: u16,
    pub kind: VirtQueueKind,
    pub flags: u64,
}

pub struct SplitVirtQueue {
    pub descriptor_table: Align16<u32>,
    pub available_ring: Align2<u32>,
    pub used_ring: Align4<u32>,
}

impl SplitVirtQueue {
    pub fn new(queue_size: u16) -> Result<Self, &'static str> {
        if queue_size > 32768 {
            return Err("queue_size must be less than or equal to 32768.")
        }

        // if number isnt a power of 2
        if queue_size > 0 && (queue_size & (queue_size - 1)) != 0 {
            return Err("queue_size must be a power of 2.")
        }

        Ok(
            Self {
                descriptor_table: Align16::new((16 * queue_size) as u32),
                available_ring: Align2::new((6 + 2 * queue_size) as u32),
                used_ring: Align4::new((6 + 8 * queue_size) as u32),
            }
        )
    }
}

pub struct PackedVirtQueue { /* */ }
