pub mod configparser;
pub mod device;
pub mod endpoint;
pub mod function;

// PID
#[repr(C)]
pub enum TUSDBPID {
    USBPIDSetup,
    USBPIDData0,
    USBPIDData1,
    //USBPIDData2,
    //USBPIDMData
}

// Device addresses
pub const USB_DEFAULT_ADDRESS: usize =          0;
pub const USB_FIRST_DEDICATED_ADDRESS: usize =  1;
pub const USB_MAX_ADDRESS: usize =              127;

// Speed
#[repr(C)]
pub enum TUSBSpeed {
    USBSpeedLow,
    USBSpeedFull,
    USBSpeedHigh,
    USBSpeedUnknown,
}

// Setup data
#[repr(C, packed)]
pub struct TSetupData {
    pub bmRequestType: u8,
    pub bRequest: u8,
    pub wValue: u16,
    pub wIndex: u16,
    pub wLength: u16,
    // data follows
}

// Request types
pub const REQUEST_OUT: usize =                  0;
pub const REQUEST_IN: usize =                   0x80;

pub const REQUEST_CLASS: usize =                0x20;
pub const REQUEST_VENDOR: usize =               0x40;

pub const REQUEST_TO_INTERFACE: usize =         1;
pub const REQUEST_TO_OTHER: usize =             3;

// Standard request codes
pub const GET_STATUS: usize =                   0;
pub const CLEAR_FEATURE: usize =                1;
pub const SET_FEATURE: usize =                  3;
pub const SET_ADDRESS: usize =                  5;
pub const GET_DESCRIPTOR: usize =               6;
pub const SET_CONFIGURATION: usize =            9;
pub const SET_INTERFACE: usize =                11;

// descriptor types
pub const DESCRIPTOR_DEVICE: usize =            1;
pub const DESCRIPTOR_CONFIGURATION: usize =     2;
pub const DESCRIPTOR_STRING: usize =            3;
pub const DESCRIPTOR_INTERFACE: usize =         4;
pub const DESCRIPTOR_ENDPOINT: usize =          5;
pub const DESCRIPTOR_CS_INTERFACE: usize =      36;
pub const DESCRIPTOR_CS_ENDPOINT: usize =       37;
pub const DESCRIPTOR_INDEX_DEFAULT: usize =     0;

pub const USB_DEFAULT_MAX_PACKET_SIZE: usize =  8;

#[repr(C, packed)]
pub struct TUSBDeviceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bcdUSB: u16,
    pub bDeviceClass: u8,
    pub bDeviceSubClass: u8,
    pub bDeviceProtocol: u8,
    pub bMaxPacketSize0: u8,

    pub idVendor: u16,
    pub idProduct: u16,
    pub bcdDevice: u16,
    pub iManufacturer: u8,
    pub iProduct: u8,
    pub iSerialNumber: u8,
    pub bNumConfigurations: u8,
}

#[repr(C, packed)]
pub struct TUSBConfigurationDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub wTotalLength: u16,
    pub bNumInterfaces: u8,
    pub bConfigurationValue: u8,
    pub iConfiguration: u8,
    pub bmAttributes: u8,
    pub bMaxPower: u8,
}

#[repr(C, packed)]
pub struct TUSBInterfaceDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bInterfaceNumber: u8,
    pub bAlternateSetting: u8,
    pub bNumEndpoints: u8,
    pub bInterfaceClass: u8,
    pub bInterfaceSubClass: u8,
    pub bInterfaceProtocol: u8,
    pub iInterface: u8,
}

#[repr(C, packed)]
pub struct TUSBEndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
}

// do we need these?
#[repr(C, packed)]
pub struct TUSBAudioEndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bEndpointAddress: u8,
    pub bmAttributes: u8,
    pub wMaxPacketSize: u16,
    pub bInterval: u8,
    pub bRefresh: u8,
    pub bSynchAddress: u8,
}

// do we need these?
#[repr(C, packed)]
pub struct TUSBMIDIStreamingEndpointDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bDescriptorSubType: u8,
    pub bNumEmbMIDIJack: u8,
    pub bAssocJackIDs: [u8; 0],
}

#[repr(C, packed)]
pub struct Header {
    pub bLength: u8,
    pub bDescriptorType: u8,
}

#[repr(C, packed)]
pub union TUSBDescriptor {
    pub Header: core::mem::ManuallyDrop<Header>,
    pub Configuration: core::mem::ManuallyDrop<TUSBConfigurationDescriptor>,
    pub Interface: core::mem::ManuallyDrop<TUSBInterfaceDescriptor>,
    pub Endpoint: core::mem::ManuallyDrop<TUSBEndpointDescriptor>,
    pub AudioEndpoint: core::mem::ManuallyDrop<TUSBAudioEndpointDescriptor>,
    pub MIDIStreamingEndpoint: core::mem::ManuallyDrop<TUSBMIDIStreamingEndpointDescriptor>,
}

#[repr(C, packed)]
pub struct TUSBStringDescriptor {
    pub bLength: u8,
    pub bDescriptorType: u8,
    pub bString: [u16; 0],
}