// status field.
pub const VIRTIO_NET_S_LINK_UP: u16 =    1;
pub const VIRTIO_NET_S_ANNOUNCE: u16 =   2;

pub struct VirtIONetConfig {
    /// MAC address of the device.
    /// Always exists, but is only valid if `VIRTIO_NET_F_MAC` is set.
    pub mac: [u8; 6],

    /// Status of the device.
    /// Is `None` unless `VIRTIO_NET_F_STATUS` is set.
    pub status: Option<u16>, // documentation says le16, however, nullex is little endian, so u16 does fine.

    /// Maximum number of each of transit and receive `VirtQueues` that can be
    /// configured once at lease one of these features is negotiated.
    /// Is `None` unless `VIRTIO_NET_F_MQ` or `VIRTIO_NET_F_RSS` is set.
    max_virtqueue_pairs: Option<u16>,

    /// Maximum MTU for the driver to use
    /// Is `None` unless `VIRTIO_NET_F_MTU` is set.
    mtu: Option<u16>,

    /// The device speed, in units of 1MBit per second, 0 to 0x7fffffff, or 0xffffffff for unknown speed.
    /// Is `None` unless `VIRTIO_NET_F_SPEED_DUPLEX` is set.
    pub speed: Option<u32>,

    /// Has the values for duplex types: 
    /// - `0x01` - Full Duplex
    /// - `0x00` - Half Duplex
    /// - `0xFF` - Unknown duplex state.<br>
    /// Is `None` unless `VIRTIO_NET_F_SPEED_DUPLEX` is set.
    pub duplex: Option<u8>,

    /// The maximum supported length of RSS key in bytes.
    /// Is `None` unless `VIRTIO_NET_F_RSS` or `VIRTIO_NET_F_HASH_REPORT` is set.
    pub rss_max_key_size: Option<u8>,

    /// Maximum number of 16-bit entries in RSS indirection table.
    /// Is `None` unless `VIRTIO_NET_F_RSS` is set.
    pub rss_max_indirection_table_length: Option<u16>,

    /// The bitmask of supported hash types.
    /// Is `None` unless the device supports hash calculations,
    /// i.e `VIRTIO_NET_F_RSS` or `VIRTIO_NET_F_HASH_REPORT` is set.
    pub supported_hash_types: Option<u32>,

    /// The bitmask of encapsulation types supported by the device
    /// for inner header hash.
    /// Is `None` unless device supports inner head hash, i.e 
    /// if `VIRTIO_NET_F_HASH_TUNNEL` is set. 
    pub supported_tunnel_types: Option<u32>,
}

impl VirtIONetConfig {
    /// Gets the read-only `max_virtqueue_pairs` value of a `VirtIONetConfig` struct.
    pub fn get_max_virtqueue_pairs(&self) -> Option<u16> {
        self.max_virtqueue_pairs
    }

    /// Gets the read-only `mtu` value of a `VirtIONetConfig` struct.
    pub fn get_mtu(&self) -> Option<u16> {
        self.mtu
    }
}