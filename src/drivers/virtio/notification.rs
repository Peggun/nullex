/// The different notification types from either device to driver, or vice versa.
pub enum NotificationType {
    /// A notification which indicates that the device configuration space has changed.
    /// Can only be sent by the device.
    ConfigurationChangeNotification,

    /// A notification which indicates a buffer may have been made available
    /// on the virtqueue designated by the notification
    /// Can only be sent by the driver.
    AvailableBufferNotification,
    
    /// A notification which indicates that a buffer may have been made used on the virtqueue
    /// designated by the notification.
    /// Can only be sent by the driver.
    UsedBufferNotification,
}

pub struct Notification(pub NotificationType);

impl Notification {
    pub const fn new(notif_type: NotificationType) -> Notification {
        Self(notif_type)
    }
    
    // TODO: improve error handling instead of &'static str.
    pub fn send() -> Result<(), &'static str> {
        todo!();
    }
}