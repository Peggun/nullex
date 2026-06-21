use core::{ptr::write_volatile, u32};

use crate::kassert;
// todo: change these functions into a impl block.

pub struct TDWHCIRegister {
    pub valid: bool,
    pub address: usize,
    pub buffer: u32,
}
impl TDWHCIRegister {
    pub fn new(nAddress: usize) -> Self  {
        Self {
            valid: false,
            address: nAddress,
            buffer: 0,
        }
    }

    pub fn with_value(nAddress: usize, nValue: u32) -> Self {
        Self {
            valid: true,
            address: nAddress,
            buffer: nValue,
        }
    }

    pub fn invalidate(&mut self) {
        self.valid = false
    }

    pub fn read(&mut self) -> u32 {
        kassert!(self.valid, "Tried to read from invalid TDWHCIRegister.");
        unsafe {
            let address_ptr = self.address as *const u32;
            self.buffer = address_ptr.read_volatile();
            self.buffer
        }
    }

    pub fn write(&self) {
        kassert!(self.valid, "Tried to write to invalid TDWHCIRegister.");
        unsafe {
            write_volatile(self.address as *mut u32, self.buffer);
        }
    }

    pub fn get(&mut self) -> u32 {
        kassert!(self.valid, "Tried to get an invalid TDWHCIRegister.");
        self.buffer
    }

    pub fn set(&mut self, value: u32) {
        self.buffer = value;
        self.valid = true;
    }

    pub fn is_set(&mut self, mask: u32) -> bool {
        kassert!(self.valid, "Tried to access an invalid TDWHCIRegister.");
        (self.buffer & mask) != 0
    }

    pub fn and(&mut self, mask: u32) {
        kassert!(self.valid, "Tried to access an invalid TDWHCIRegister.");
        self.buffer &= mask;
    }

    pub fn or(&mut self, mask: u32) {
        kassert!(self.valid, "Tried to access an invalid TDWHCIRegister.");
        self.buffer |= mask;
    }

    pub fn clear_bit(&mut self, nbit: usize) {
        kassert!(self.valid, "Tried to access an invalid TDWHCIRegister.");
        kassert!(nbit < (core::mem::size_of::<u32>() * 8), "nbit is out of bounds (> u32::BITS)");
        self.buffer &= !(1 << nbit);
    }

    pub fn set_bit(&mut self, nbit: usize) {
        kassert!(self.valid, "Tried to access an invalid TDWHCIRegister.");
        kassert!(nbit < (core::mem::size_of::<u32>() * 8), "nbit is out of bounds (> u32::BITS)");
        self.buffer |= 1 << nbit;
    }

    pub fn clear_all(&mut self) {
        self.buffer = 0;
        self.valid = true;
    }

    pub fn set_all(&mut self) {
        self.buffer = u32::MAX;
        self.valid = true;
    }
}