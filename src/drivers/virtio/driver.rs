use alloc::vec::Vec;

use crate::drivers::virtio::{Features, device::VirtIODeviceStatus, virtqueue::VirtQueue};

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
