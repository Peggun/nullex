//!
//! rng.rs
//!
//! RNG system handling for the kernel
//! 

use core::num::NonZeroU32;

use rand_core::{CryptoRng, RngCore};
use x86_64::instructions::random::RdRand;

const RDRAND_RETRIES: usize = 10;
const RAND_CORE_ERROR_CODE: u32 = rand_core::Error::CUSTOM_START;

/// Structure representing the RNG system of the kernel.
pub struct KernelRng {}

impl KernelRng {
	/// Try and create a new `KernelRng`
	pub fn try_new() -> Result<Self, getrandom::Error> {
		let _ = rdrand_u64()?;
		Ok(Self {})
	}

	/// Reseed the current `KernelRng`
	pub fn reseed(&mut self) -> Result<(), getrandom::Error> {
		let _ = rdrand_u64()?;
		Ok(())
	}
}

impl RngCore for KernelRng {
	fn fill_bytes(&mut self, dest: &mut [u8]) {
		kernel_entropy_fill(dest).expect("RDRAND failed while filling random bytes");
	}

	fn next_u32(&mut self) -> u32 {
		self.next_u64() as u32
	}

	fn next_u64(&mut self) -> u64 {
		rdrand_u64().expect("RDRAND failed while generating random u64")
	}

	fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
		kernel_entropy_fill(dest).map_err(|_| rand_core_error())
	}
}

impl CryptoRng for KernelRng {}

/// Fill the kernel's entropy for (P)RNG
pub fn kernel_entropy_fill(out: &mut [u8]) -> Result<(), getrandom::Error> {
	let mut filled = 0;
	while filled < out.len() {
		let value = rdrand_u64()?;
		let bytes = value.to_le_bytes();
		let to_copy = (out.len() - filled).min(bytes.len());
		out[filled..filled + to_copy].copy_from_slice(&bytes[..to_copy]);
		filled += to_copy;
	}
	Ok(())
}

fn rdrand_u64() -> Result<u64, getrandom::Error> {
	let rdrand = RdRand::new().ok_or(getrandom::Error::UNSUPPORTED)?;
	for _ in 0..RDRAND_RETRIES {
		if let Some(value) = rdrand.get_u64() {
			return Ok(value);
		}
		core::hint::spin_loop();
	}

	Err(getrandom::Error::UNEXPECTED)
}

fn rand_core_error() -> rand_core::Error {
	let code = NonZeroU32::new(RAND_CORE_ERROR_CODE)
		.expect("rand_core custom error code must be non-zero");
	rand_core::Error::from(code)
}
