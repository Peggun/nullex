use alloc::{string::{String, ToString}, vec::Vec};
use core::{
    ffi::{c_char, c_void},
    ptr::NonNull,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Block,
    NonBlock,
}

pub struct DeviceEntry {
    name: String,
    device: NonNull<c_void>,
    kind: DeviceKind,
}

pub struct DeviceRegistry {
    devices: Vec<DeviceEntry>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self { devices: Vec::new() }
    }

    pub fn add(
        &mut self,
        name: impl Into<String>,
        device: NonNull<c_void>,
        kind: DeviceKind,
    ) {
        self.devices.push(DeviceEntry {
            name: name.into(),
            device,
            kind,
        });
    }

    pub fn get(&self, name: &str, kind: DeviceKind) -> Option<NonNull<c_void>> {
        self.devices
            .iter()
            .find(|entry| entry.name.to_string() == name && entry.kind == kind)
            .map(|entry| entry.device)
    }
}