use crate::drivers::usb::uspi::usb::TUSBDescriptor;

#[repr(C)]
pub struct TUSBConfigurationParser {
    pub m_pBuffer: *const TUSBDescriptor,
    pub m_nBufLen: u32,
    pub m_bValid: bool,
    pub m_pEndPosition: *const TUSBDescriptor,
    pub m_pNextPosition: *const TUSBDescriptor,
    pub m_pCurrentDescriptor: *const TUSBDescriptor,
    pub m_pErrorPosition: *const TUSBDescriptor,
}