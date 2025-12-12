use crate::utils::align::*;

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