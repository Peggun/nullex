use core::ops::Add;
use crate::utils::bits::{sign_extend, BitsExt};

use x86_64::VirtAddr as OldAddr;

// bits 0-11 - page offset
// bits 12-47 - virtual page number
// bits 48-63 - sign extended
#[derive(Debug)]
pub struct VirtAddr(u64);

impl VirtAddr {
    const SIGN_EXT_MASK: u64 = 0xFFFF << 48;

    /// Creates a new `VirtAddr` which is canonical.
    pub fn new(addr: u64) -> Self {
        VirtAddr::try_new(addr).expect("Invalid virtual address.")
    }

    pub fn try_new(addr: u64) -> Option<Self> {
        if Self(addr).is_canonical() {
            Some(Self(addr))    
        } else {
            // is set to 1
            if addr.extract_bit(47) && (addr.extract_bits(48..63) & Self::SIGN_EXT_MASK) == 0 {
                // needs to be sign extended
                let a = sign_extend(addr, 48);// b is the length of the int. so u64 is 64.
                
                if Self(a).is_canonical() {
                    Some(Self(a))
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    /// Simple wrapper whilst I convert, eventually will get rid of this
    pub fn new_from_x86_64(addr: OldAddr) -> VirtAddr {
        VirtAddr(addr.as_u64())
    }

    /// Simple wrapper whilst I convert, eventually will get rid of this
    pub fn to_x86_64(&self) -> OldAddr {
        OldAddr::new(self.as_u64())
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn vpn(&self) -> u64 {
        self.0 >> 12
    }

    pub fn is_canonical(&self) -> bool {
        let bit47 = (self.as_u64() >> 47) & 1;
        let upper_bits = (self.as_u64() & Self::SIGN_EXT_MASK) >> 48;

        if bit47 == 0 {
            return upper_bits == 0
        } else {
            return upper_bits == Self::SIGN_EXT_MASK
        }
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.as_u64() as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }
}

impl Add<u64> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        VirtAddr(self.0 + rhs)
    }
}

pub struct PhysAddr(u64);