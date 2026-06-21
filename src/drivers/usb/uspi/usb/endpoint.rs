pub enum TEndpointType {
    EndpointTypeControl,
    EndpointTypeBulk,
    EndpointTypeInterrupt,
    EndpointTypeIsochronous,
}
    
pub struct TUSBEndpoint {
    pub m_pDevice: *TUSBDevice
}