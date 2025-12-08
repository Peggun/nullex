// https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-2340001

pub mod config;

pub const VIRTIO_DEVICE_ID: u8 = 1;

// other virtqueues like (2n+1) arent implemented

// Feature bits
/// Device handles packets with partial checksum.
pub const VIRTIO_NET_F_CSUM: u64 =               1 << 0;
/// Driver handles packets with partial checksum.
pub const VIRTIO_NET_F_GUEST_CSUM: u64 =         1 << 1;
/// Control channel offloads reconfiguration support.
pub const VIRTIO_NET_F_GUEST_OFFLOADS: u64 =     1 << 2;
/// Device maximum MTU (Maximum Transmission Unit) reporting is supported.
pub const VIRTIO_NET_F_MTU: u64 =                1 << 3;
// 4 not in use
/// Device has given MAC address.
pub const VIRTIO_NET_F_MAC: u64 =                1 << 5;
// 6 not in use
/// Driver can receive TSOv4.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`
pub const VIRTIO_NET_F_GUEST_TSO4: u64 =         1 << 7;
/// Driver can receive TSOv6.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`
pub const VIRTIO_NET_F_GUEST_TSO6: u64 =         1 << 8;
/// Driver can receive TSO with ECN.
/// Requires `VIRTIO_NET_F_GUEST_TSO4` or `VIRTIO_NET_F_GUEST_TSO6`.
pub const VIRTIO_NET_F_GUEST_ECN: u64 =          1 << 9;
/// Driver can receive UFO.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`.
pub const VIRTIO_NET_F_GUEST_UFO: u64 =          1 << 10;
/// Device can receive TSOv4.
/// Requires `VIRTIO_NET_F_CSUM`.
pub const VIRTIO_NET_F_HOST_TSO4: u64 =          1 << 11;
/// Device can receive TSOv6.
/// Requires `VIRTIO_NET_F_CSUM`.
pub const VIRTIO_NET_F_HOST_TSO6: u64 =          1 << 12;
/// Device can receive TSO with ECN.
/// Requires `VIRTIO_NET_F_HOST_TSO4` or `VIRTIO_NET_F_HOST_TSO6`.
pub const VIRTIO_NET_F_HOST_ECN: u64 =           1 << 13;
/// Device can receive UFO.
/// Requires `VIRTIO_NET_F_CSUM`.
pub const VIRTIO_NET_F_HOST_UFO: u64 =           1 << 14;
/// Driver can merge receive buffers.
pub const VIRTIO_NET_F_MRG_RXBUF: u64 =          1 << 15;
/// Configuration status field is available.
pub const VIRTIO_NET_F_STATUS: u64 =             1 << 16;
/// Control channel is available.
pub const VIRTIO_NET_F_CTRL_VQ: u64 =            1 << 17;
/// Control channel RX mode support.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_CTRL_RX: u64 =            1 << 18;
/// Control channel VLAN filtering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_CTRL_VLAN: u64 =          1 << 19;
/// Control channel RX extra mode support.
pub const VIRTIO_NET_F_CTRL_RX_EXTRA: u64 =      1 << 20;
/// Driver can send gratuitous packets.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_GUEST_ANNOUNCE: u64 =     1 << 21;
/// Driver supports multiqueue with automatic receive steering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_MQ: u64 =                 1 << 22;
/// Set MAC address through control channel.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_CTRL_MAC_ADDR: u64 =      1 << 23;
// 24-50 not in use
/// Device supports inner header hash for encapsulated packets.
/// Requires `VIRTIO_NET_F_CTRL_VQ` along with 
/// `VIRTIO_NET_F_RSS` or `VIRTIO_NET_F_HASH_REPORT`.
pub const VIRTIO_NET_F_HASH_TUNNEL: u64 =        1 << 51;
/// Device supports `VirtQueues` notification coalescing.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_VQ_NOTF_COAL: u64 =       1 << 52;
/// Device supports notification coalescing.
pub const VIRTIO_NET_F_NOTF_COAL: u64 =          1 << 53;
/// Driver can receive USOv4 packets.
pub const VIRTIO_NET_F_GUEST_USO4: u64 =         1 << 54;
/// Driver can receive USOv6 packets.   
pub const VIRTIO_NET_F_GUEST_USO6: u64 =         1 << 55;
/// Device can receive USO packets.
/// Requires `VIRTIO_NET_F_CSUM`.
pub const VIRTIO_NET_F_HOST_USO: u64 =           1 << 56;
/// Device can report per-packet hash value and a type of calculated hash.
pub const VIRTIO_NET_F_HASH_REPORT: u64 =        1 << 57;
// 58 not in use
/// Driver can provide the exact `hdr_len` value. Device benefits from knowing the exact header length.
pub const VIRTIO_NET_F_GUEST_HDRLEN: u64 =       1 << 59;
/// Device supports RSS (receive-side scaling) with Toeplitz hash calculation
/// and configurable hash parameters for receive steering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
pub const VIRTIO_NET_F_RSS: u64 =                1 << 60;
/// Device can process duplicated ACKs and 
/// report number of coalesced segments and duplicated ACKS.
/// Requires `VIRTIO_NET_F_HOST_TSO4` or `VIRTIO_NET_F_HOST_TSO6`.
pub const VIRTIO_NET_F_RSC_EXT: u64 =            1 << 61;
/// Device may act as a standby for a primary device with the same MAC address.
pub const VIRTIO_NET_F_STANDBY: u64 =            1 << 62;
/// Device reports speed and duplex.
pub const VIRTIO_NET_F_SPEED_DUPLEX: u64 =       1 << 63;