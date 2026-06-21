use crate::drivers::usb::uspi::bcm2835::{ARM_USB_CORE_BASE, ARM_USB_HOST_BASE};

#[allow(non_snake_case)]
pub mod frame_scheduler;
#[allow(non_snake_case)]
pub mod frameschedper;
#[allow(non_snake_case)]
pub mod frameschednper;
#[allow(non_snake_case)]
pub mod frameschednsplit;
#[allow(non_snake_case)]
pub mod register;

pub const DWHCI_MAX_CHANNELS: usize = 16;

pub const DWHCI_DATA_FIFO_SIZE: usize = 0x1000;

//
// Core Registers
//
pub const DWHCI_CORE_OTG_CTRL: usize = ARM_USB_CORE_BASE + 0x000;
pub const DWHCI_CORE_OTG_CTRL_HST_SET_HNP_EN: usize = 1 << 10;

pub const DWHCI_CORE_OTG_INT: usize = ARM_USB_CORE_BASE + 0x004;
pub const DWHCI_CORE_OTG_CFG: usize = ARM_USB_CORE_BASE + 0x008;
pub const DWHCI_CORE_AHB_CFG_GLOBALINT_MASK: usize = 1 << 0;
pub const DWHCI_CORE_AHB_CFG_MAX_AXI_BURST__SHIFT: usize = 1; // BCM2385 only
pub const DWHCI_CORE_AHB_CFG_MAX_AXI_BURST__MASK: usize = 3 << 1; // BCM2385 only
pub const DWHCI_CORE_AHB_CFG_WAIT_AXI_WRITES: usize = 1 << 4; // BCM2385 only
pub const DWHCI_CORE_AHB_CFG_DMAENABLE: usize = 1 << 5;
pub const DWHCI_CORE_AHB_CFG_AHB_SINGLE: usize = 1 << 23;

pub const DWHCI_CORE_USB_CFG: usize = ARM_USB_CORE_BASE + 0x00C;
pub const DWHCI_CORE_USB_CFG_PHYIF: usize = 1 << 3;
pub const DWHCI_CORE_USB_CFG_ULPI_UTMI_SEL: usize = 1 << 4;
pub const DWHCI_CORE_USB_CFG_SRP_CAPABLE: usize = 1 << 8;
pub const DWHCI_CORE_USB_CFG_HNP_CAPABLE: usize = 1 << 9;
pub const DWHCI_CORE_USB_CFG_ULPI_FSLS: usize = 1 << 17;
pub const DWHCI_CORE_USB_CFG_ULPI_CLK_SUS_M: usize = 1 << 19;
pub const DWHCI_CORE_USB_CFG_ULPI_EXT_VBUS_DRV: usize = 1 << 20;
pub const DWHCI_CORE_USB_CFG_TERM_SEL_DL_PULSE: usize = 1 << 22;

pub const DWHCI_CORE_RESET: usize = ARM_USB_CORE_BASE + 0x010;
pub const DWHCI_CORE_RESET_SOFT_RESET: usize = 1 << 0;
pub const DWHCI_CORE_RESET_RX_FIFO_FLUSH: usize = 1 << 4;
pub const DWHCI_CORE_RESET_TX_FIFO_FLUSH: usize = 1 << 5;
pub const DWHCI_CORE_RESET_TX_FIFO_NUM__SHIFT: usize = 6;
pub const DWHCI_CORE_RESET_TX_FIFO_NUM__MASK: usize = 0x1F << 6;
pub const DWHCI_CORE_RESET_AHB_IDLE: usize = 1 << 31;

pub const DWHCI_CORE_INT_STAT: usize = ARM_USB_CORE_BASE + 0x014;
pub const DWHCI_CORE_INT_STAT_SOF_INTR: usize = 1 << 3;
pub const DWHCI_CORE_INT_STAT_PORT_INTR: usize = 1 << 24;
pub const DWHCI_CORE_INT_STAT_HC_INTR: usize = 1 << 25;

pub const DWHCI_CORE_INT_MASK: usize = ARM_USB_CORE_BASE + 0x018;
pub const DWHCI_CORE_INT_MASK_MODE_MISMATCH: usize = 1 << 1;
pub const DWHCI_CORE_INT_MASK_SOF_INTR: usize = 1 << 3;
pub const DWHCI_CORE_INT_MASK_RX_STS_Q_LVL: usize = 1 << 4;
pub const DWHCI_CORE_INT_MASK_USB_SUSPEND: usize = 1 << 11;
pub const DWHCI_CORE_INT_MASK_PORT_INTR: usize = 1 << 24;
pub const DWHCI_CORE_INT_MASK_HC_INTR: usize = 1 << 25;
pub const DWHCI_CORE_INT_MASK_CON_ID_STS_CHNG: usize = 1 << 28;
pub const DWHCI_CORE_INT_MASK_DISCONNECT: usize = 1 << 29;
pub const DWHCI_CORE_INT_MASK_SESS_REQ_INTR: usize = 1 << 30;
pub const DWHCI_CORE_INT_MASK_WKUP_INTR: usize = 1 << 31;

pub const DWHCI_CORE_RX_STAT_RD: usize = ARM_USB_CORE_BASE + 0x01C; // RO, slave mode only
pub const DWHCI_CORE_RX_STAT_POP: usize = ARM_USB_CORE_BASE + 0x020; // RO, slave mode only
// for read and pop register in host mode
pub const DWHCI_CORE_RX_STAT_CHAN_NUMBER__MASK: usize = 0xF;
pub const DWHCI_CORE_RX_STAT_BYTE_COUNT__SHIFT: usize = 4;
pub const DWHCI_CORE_RX_STAT_BYTE_COUNT__MASK: usize = 0x7FF << 4;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS__SHIFT: usize = 17;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS__MASK: usize = 0xF << 17;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS_IN: usize = 2;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS_IN_XFER_COMP: usize = 3;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS_DATA_TOGGLE_ERR: usize = 5;
pub const DWHCI_CORE_RX_STAT_PACKET_STATUS_CHAN_HALTED: usize = 7;

pub const DWHCI_CORE_RX_FIFO_SIZ: usize = ARM_USB_CORE_BASE + 0x024;
pub const DWHCI_CORE_NPER_TX_FIFO_SIZ: usize = ARM_USB_CORE_BASE + 0x028;
pub const DWHCI_CORE_NPER_TX_STAT: usize = ARM_USB_CORE_BASE + 0x02C; // RO
pub fn dwhci_core_nper_tx_stat_queue_space_avl(reg: usize) -> u8 {
	((reg >> 16) & 0xFF) as u8
}
pub const DWHCI_CORE_I2C_CTRL: usize = ARM_USB_CORE_BASE + 0x030;
pub const DWHCI_CORE_PHY_VENDOR_CTRL: usize = ARM_USB_CORE_BASE + 0x034;
pub const DWHCI_CORE_GPIO: usize = ARM_USB_CORE_BASE + 0x038;
pub const DWHCI_CORE_USER_ID: usize = ARM_USB_CORE_BASE + 0x03C;
pub const DWHCI_CORE_VENDOR_ID: usize = ARM_USB_CORE_BASE + 0x040;
pub const DWHCI_CORE_HW_CFG1: usize = ARM_USB_CORE_BASE + 0x044; // RO

pub const DWHCI_CORE_HW_CFG2: usize = ARM_USB_CORE_BASE + 0x048; // RO
pub fn dwhci_core_hw_cfg2_op_mode(reg: usize) -> usize {
	(reg >> 0) & 7
}
pub fn dwhci_core_hw_cfg2_architecture(reg: usize) -> usize {
	(reg >> 3) & 3
}
pub fn dwhci_core_hw_cfg2_hs_phy_type(reg: usize) -> usize {
	(reg >> 6) & 3
}
pub const DWHCI_CORE_HW_CFG2_HS_PHY_TYPE_NOT_SUPPORTED: usize = 0;
pub const DWHCI_CORE_HW_CFG2_HS_PHY_TYPE_UTMI: usize = 1;
pub const DWHCI_CORE_HW_CFG2_HS_PHY_TYPE_ULPI: usize = 2;
pub const DWHCI_CORE_HW_CFG2_HS_PHY_TYPE_UTMI_ULPI: usize = 3;
pub fn dwhci_core_hw_cfg2_fs_phy_type(reg: usize) -> usize {
	(reg >> 8) & 3
}
pub const DWHCI_CORE_HW_CFG2_FS_PHY_TYPE_DEDICATED: usize = 1;
pub fn dwhci_core_hw_cfg2_num_host_channels(reg: usize) -> usize {
	((reg >> 14) & 0xF) + 1
}

pub const DWHCI_CORE_HW_CFG3: usize = ARM_USB_CORE_BASE + 0x04C; // RO
pub fn dwhci_core_hw_cfg3_dfifo_depth(reg: usize) -> usize {
	(reg >> 16) & 0xFFFF
}

pub const DWHCI_CORE_HW_CFG4: usize = ARM_USB_CORE_BASE + 0x050; // RO
pub const DWHCI_CORE_HW_CFG4_DED_FIFO_EN: usize = 1 << 25;
pub fn dwhci_core_hw_cfg4_num_in_eps(reg: usize) -> usize {
	((reg) >> 26) & 0xF
}

pub const DWHCI_CORE_LPM_CFG: usize = ARM_USB_CORE_BASE + 0x054;
pub const DWHCI_CORE_POWER_DOWN: usize = ARM_USB_CORE_BASE + 0x058;
pub const DWHCI_CORE_DFIFO_CFG: usize = ARM_USB_CORE_BASE + 0x05C;
pub const DWHCI_CORE_DFIFO_CFG_EPINFO_BASE__SHIFT: usize = 16;
pub const DWHCI_CORE_DFIFO_CFG_EPINFO_BASE__MASK: usize = 0xFFFF << 16;

pub const DWHCI_CORE_ADP_CTRL: usize = ARM_USB_CORE_BASE + 0x060;
// gap
pub const DWHCI_VENDOR_MDIO_CTRL: usize = ARM_USB_CORE_BASE + 0x080; // BCM2835 only
pub const DWHCI_VENDOR_MDIO_DATA: usize = ARM_USB_CORE_BASE + 0x084; // BCM2835 only
pub const DWHCI_VENDOR_VBUS_DRV: usize = ARM_USB_CORE_BASE + 0x088; // BCM2835 only
// gap
pub const DWHCI_CORE_HOST_PER_TX_FIFO_SIZ: usize = ARM_USB_CORE_BASE + 0x100;
// fifo := 0..14 :
// dedicated FIFOs on
pub fn dwhci_core_dev_per_tx_fifo(fifo: usize) -> usize {
	ARM_USB_CORE_BASE + 0x104 + (fifo * 4)
}
// dedicated FIFOs off
pub fn dwhci_core_dev_tx_fifo(fifo: usize) -> usize {
	ARM_USB_CORE_BASE + 0x104 + (fifo * 4)
}

//
// Host registers
//
pub const DWHCI_HOST_CFG: usize = ARM_USB_HOST_BASE + 0x000;
pub const DWHCI_HOST_CFG_FSLS_PCLK_SEL__SHIFT: usize = 0;
pub const DWHCI_HOST_CFG_FSLS_PCLK_SEL__MASK: usize = 3 << 0;
pub const DWHCI_HOST_CFG_FSLS_PCLK_SEL_30_60_MHZ: usize = 0;
pub const DWHCI_HOST_CFG_FSLS_PCLK_SEL_48_MHZ: usize = 1;
pub const DWHCI_HOST_CFG_FSLS_PCLK_SEL_6_MHZ: usize = 2;

pub const DWHCI_HOST_FRM_INTERVAL: usize = ARM_USB_HOST_BASE + 0x004;
pub const DWHCI_HOST_FRM_NUM: usize = ARM_USB_HOST_BASE + 0x008;
pub fn dwhci_host_frm_num_number(reg: usize) -> usize {
	reg & 0xFFFF
}
pub const DWHCI_MAX_FRAME_NUMBER: usize = 0xFFFF;
pub fn dwhci_host_frm_num_remaining(reg: usize) -> usize {
	(reg >> 16) & 0xFFFF
}
// gap
pub const DWHCI_HOST_PER_TX_FIFO_STAT: usize = ARM_USB_HOST_BASE + 0x010;
pub const DWHCI_HOST_ALLCHAN_INT: usize = ARM_USB_HOST_BASE + 0x014;
pub const DWHCI_HOST_ALLCHAN_INT_MASK: usize = ARM_USB_HOST_BASE + 0x018;
pub const DWHCI_HOST_FRMLST_BASE: usize = ARM_USB_HOST_BASE + 0x01C;
// gap
pub const DWHCI_HOST_PORT: usize = ARM_USB_HOST_BASE + 0x040;
pub const DWHCI_HOST_PORT_CONNECT: usize = 1 << 0;
pub const DWHCI_HOST_PORT_CONNECT_CHANGED: usize = 1 << 1;
pub const DWHCI_HOST_PORT_ENABLE: usize = 1 << 2;
pub const DWHCI_HOST_PORT_ENABLE_CHANGED: usize = 1 << 3;
pub const DWHCI_HOST_PORT_OVERCURRENT: usize = 1 << 4;
pub const DWHCI_HOST_PORT_OVERCURRENT_CHANGED: usize = 1 << 5;
pub const DWHCI_HOST_PORT_RESET: usize = 1 << 8;
pub const DWHCI_HOST_PORT_POWER: usize = 1 << 12;
pub fn dwhci_host_port_speed(reg: usize) -> usize {
	(reg >> 17) & 3
}
pub const DWHCI_HOST_PORT_SPEED_HIGH: usize = 0;
pub const DWHCI_HOST_PORT_SPEED_FULL: usize = 1;
pub const DWHCI_HOST_PORT_SPEED_LOW: usize = 2;
pub const DWHCI_HOST_PORT_DEFAULT_MASK: usize = DWHCI_HOST_PORT_CONNECT_CHANGED
	| DWHCI_HOST_PORT_ENABLE
	| DWHCI_HOST_PORT_ENABLE_CHANGED
	| DWHCI_HOST_PORT_OVERCURRENT_CHANGED;

// gap
// chan := 0..15 :
pub fn dwhci_host_chan_character(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x100 + (chan * 0x20)
}
pub const DWHCI_HOST_CHAN_CHARACTER_MAX_PKT_SIZ__MASK: usize = 0x7FF;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_NUMBER__SHIFT: usize = 11;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_NUMBER__MASK: usize = 0xF << 11;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_DIRECTION_IN: usize = 1 << 15;
pub const DWHCI_HOST_CHAN_CHARACTER_LOW_SPEED_DEVICE: usize = 1 << 17;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE__SHIFT: usize = 18;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE__MASK: usize = 3 << 18;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE_CONTROL: usize = 0;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE_ISO: usize = 1;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE_BULK: usize = 2;
pub const DWHCI_HOST_CHAN_CHARACTER_EP_TYPE_INTERRUPT: usize = 3;
pub const DWHCI_HOST_CHAN_CHARACTER_MULTI_CNT__SHIFT: usize = 20;
pub const DWHCI_HOST_CHAN_CHARACTER_MULTI_CNT__MASK: usize = 3 << 20;
pub const DWHCI_HOST_CHAN_CHARACTER_DEVICE_ADDRESS__SHIFT: usize = 22;
pub const DWHCI_HOST_CHAN_CHARACTER_DEVICE_ADDRESS__MASK: usize = 0x7F << 22;
pub const DWHCI_HOST_CHAN_CHARACTER_PER_ODD_FRAME: usize = 1 << 29;
pub const DWHCI_HOST_CHAN_CHARACTER_DISABLE: usize = 1 << 30;
pub const DWHCI_HOST_CHAN_CHARACTER_ENABLE: usize = 1 << 31;
pub fn dwhci_host_chan_split_ctrl(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x104 + (chan * 0x20)
}
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_PORT_ADDRESS__MASK: usize = 0x7F;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_HUB_ADDRESS__SHIFT: usize = 7;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_HUB_ADDRESS__MASK: usize = 0x7F << 7;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_XACT_POS__SHIFT: usize = 14;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_XACT_POS__MASK: usize = 3 << 14;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_ALL: usize = 3;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_COMPLETE_SPLIT: usize = 1 << 16;
pub const DWHCI_HOST_CHAN_SPLIT_CTRL_SPLIT_ENABLE: usize = 1 << 31;
pub fn dwhci_host_chan_int(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x108 + (chan * 0x20)
}
pub const DWHCI_HOST_CHAN_INT_XFER_COMPLETE: usize = 1 << 0;
pub const DWHCI_HOST_CHAN_INT_HALTED: usize = 1 << 1;
pub const DWHCI_HOST_CHAN_INT_AHB_ERROR: usize = 1 << 2;
pub const DWHCI_HOST_CHAN_INT_STALL: usize = 1 << 3;
pub const DWHCI_HOST_CHAN_INT_NAK: usize = 1 << 4;
pub const DWHCI_HOST_CHAN_INT_ACK: usize = 1 << 5;
pub const DWHCI_HOST_CHAN_INT_NYET: usize = 1 << 6;
pub const DWHCI_HOST_CHAN_INT_XACT_ERROR: usize = 1 << 7;
pub const DWHCI_HOST_CHAN_INT_BABBLE_ERROR: usize = 1 << 8;
pub const DWHCI_HOST_CHAN_INT_FRAME_OVERRUN: usize = 1 << 9;
pub const DWHCI_HOST_CHAN_INT_DATA_TOGGLE_ERROR: usize = 1 << 10;
pub const DWHCI_HOST_CHAN_INT_ERROR_MASK: usize = DWHCI_HOST_CHAN_INT_AHB_ERROR
	| DWHCI_HOST_CHAN_INT_STALL
	| DWHCI_HOST_CHAN_INT_XACT_ERROR
	| DWHCI_HOST_CHAN_INT_BABBLE_ERROR
	| DWHCI_HOST_CHAN_INT_FRAME_OVERRUN
	| DWHCI_HOST_CHAN_INT_DATA_TOGGLE_ERROR;
pub fn dwhci_host_chan_int_mask(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x10C + (chan * 0x20)
}
pub fn dwhci_host_chan_xfer_siz(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x110 + (chan * 0x20)
}
pub const DWHCI_HOST_CHAN_XFER_SIZ_BYTES__MASK: usize = 0x7FFFF;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PACKETS__SHIFT: usize = 19;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PACKETS__MASK: usize = 0x3FF << 19;
pub fn dwhci_host_chan_xfer_siz_packets(reg: usize) -> usize {
	(reg >> 19) & 0x3FF
}
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID__SHIFT: usize = 29;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID__MASK: usize = 3 << 29;
pub fn dwhci_host_chan_xfer_siz_pid(reg: usize) -> usize {
	(reg >> 29) & 3
}
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID_DATA0: usize = 0;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID_DATA1: usize = 2;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID_DATA2: usize = 1;
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID_MDATA: usize = 3; // non-control transfer
pub const DWHCI_HOST_CHAN_XFER_SIZ_PID_SETUP: usize = 3;
pub fn dwhci_host_chan_dma_addr(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x114 + (chan * 0x20)
}
// gap
// DDMA only
pub fn dwhci_host_chan_dma_buf(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x11C + (chan * 0x20)
}

//
// Data FIFOs (non-DMA mode only)
//
pub fn dwhci_data_fifo(chan: usize) -> usize {
	ARM_USB_HOST_BASE + 0x1000 + (chan * DWHCI_DATA_FIFO_SIZE)
}
