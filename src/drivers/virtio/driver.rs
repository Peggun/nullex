use alloc::vec::Vec;

use crate::drivers::virtio::device::{VirtIODeviceStatus, VirtQueue};

/// Type wrapper for u64, accepting the 0-63 currently active features
/// ## General Information<br>
/// - According to v1.3 of [VirtIO Specification](https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html),
/// features may be used up to 128 in the future, thus needing to change the `u64` to a `u128`
/// 
/// ## Driver Requirements<br>
/// The driver MUST NOT accept a feature which the device did not offer, and MUST NOT accept a feature
/// which requires another feature which was not accepted.<br>
/// 
/// The driver MUST validate the feature bits offered by the device. The driver MUST ignore and MUST NOT accept
/// any feature bit the is:
/// - not described in the specification
/// - marked as reserved
/// - not supported for the specific transport
/// - not defined for the device type<br>
/// 
/// The driver SHOULD go into backwards compatability mode if the device does not offer a feature it understands,
/// otherwise MUST set the FAILED device status bit and cease initialization.
/// <br>
/// 
/// ## Device Requirements
/// The device MUST NOT offer a feature which requries another feature which was not offered. 
/// The device SHOULD accept any valid subset of features the driver accepts, otherwise it MUST fail
/// to set the FEATURES_OK device status bit when the driver writes it.<br>
/// 
/// The device MUST NOT offer feature bits corresponding to features it would not support if accepted by the driver
/// (even if the driver is prohibited from accepting the feature bits by the specification); for the sake of clarity,
/// this refers to feature bits not described in the specification, reserved feature bits and feature bits reserved or
/// not supported for the specific transport or the specific device type, but this does no preclude devices written to
/// a future version of the specification from offering such feature bits should such a specification have a provision 
/// for devices to support the corresponding features.<br>
/// 
/// If a device has successfully negotiated a set of features at least once (by accepting the FEATURES_OK device status
/// bit during device initialization), the it SHOULD NOT fail re-negotiation of the same set of features after a device
/// or system reset. Failure to do so would interfere with the resuming from suspend and error recovery.
pub type Features = u64;

pub struct VirtIODriver {
    pub device_status: VirtIODeviceStatus,
    pub virtqueues: Vec<Option<VirtQueue>>,

    pub negotiated_features: Features,
}

impl VirtIODriver {
    #[inline]
    pub fn has_feature(&self, bit: Features) -> bool {
        (self.negotiated_features & bit) != 0
    }
}

#[cfg(feature = "test")]
pub mod tests {
    use crate::{drivers::virtio::*, utils::ktest::TestError};

    pub fn test_virtqueue_buffer_overflow() -> Result<(), TestError> {
        let vq = VirtQueue {
            metadata: VirtQueueMetadata { index: 1, kind: VirtQueueKind::ReceiveQueue, flags: 0 },
            backend: VirtQueueBackend::Split(SplitVirtQueue::new(u16::MAX))
        };

        match vq.backend {
            VirtQueueBackend::Split(Err(_)) => Ok(()),
            _ => Err(TestError::Error),
        }
    }

    crate::create_test!(test_virtqueue_buffer_overflow);

    pub fn test_virtqueue_buffer_power_of_two() -> Result<(), TestError> {
        let vq = VirtQueue {
            metadata: VirtQueueMetadata { index: 1, kind: VirtQueueKind::ReceiveQueue, flags: 0 },
            backend: VirtQueueBackend::Split(SplitVirtQueue::new(12345))
        };

        match vq.backend {
            VirtQueueBackend::Split(Err(_)) => Ok(()),
            _ => Err(TestError::Error),
        }

    }

    crate::create_test!(test_virtqueue_buffer_power_of_two);
}
