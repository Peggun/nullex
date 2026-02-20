//!
//! mutex.rs
//! 
//! An implementation for a Mutually Exclusive thread-safe type.
//! 
//! 

use core::{
	cell::UnsafeCell,
	mem::MaybeUninit,
	sync::atomic::{AtomicBool, Ordering}
};

use x86_64::instructions::interrupts;

/// A Mutual Exclusion Object to prevent race conditions.
pub struct SpinMutex<T> {
	locked: AtomicBool,
	data: UnsafeCell<T>
}

unsafe impl<T: Send> Send for SpinMutex<T> {}
unsafe impl<T: Send> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
	/// Create a new `SpinMutex` with data `T` (any type)
	pub const fn new(data: T) -> Self {
		SpinMutex {
			locked: AtomicBool::new(false),
			data: UnsafeCell::new(data)
		}
	}

	/// Locks the current `SpinMutex`
	pub fn lock(&self) -> SpinMutexGuard<'_, T> {
		// fixed deadlock where ISR and other parts of code
		// tried to get data at the same time
		interrupts::disable();

		while self.locked.swap(true, Ordering::Acquire) {
			interrupts::enable();
			core::hint::spin_loop();
			interrupts::disable();
		}
		SpinMutexGuard {
			mutex: self
		}
	}

	/// Tries to lock the current `SpinMutex`
	pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
		if self
			.locked
			.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
			.is_ok()
		{
			Some(SpinMutexGuard {
				mutex: self
			})
		} else {
			None
		}
	}

	/// Forces the SpinMutex to unlock, regardless if another thread is trying to use it.
	pub unsafe fn force_unlock(&self) {
		self.locked.store(false, Ordering::Release);
	}
}

impl<T: Default> Default for SpinMutex<T> {
	fn default() -> Self {
		SpinMutex::new(T::default())
	}
}

#[allow(unused)]
// here because its probably good to have.
impl<T> SpinMutex<Option<T>> {
	const fn none() -> Self {
		SpinMutex {
			locked: AtomicBool::new(false),
			data: UnsafeCell::new(None)
		}
	}
}

#[allow(unused)]
// here because its probably good to have
impl<T> SpinMutex<MaybeUninit<T>> {
	const fn uninit() -> Self {
		SpinMutex {
			locked: AtomicBool::new(false),
			data: UnsafeCell::new(MaybeUninit::uninit())
		}
	}

	unsafe fn assume_init_ref(&self) -> &T {
		unsafe { &*((*self.data.get()).as_ptr()) }
	}

	unsafe fn assume_init_mut(&self) -> &mut T {
		unsafe { &mut *((*self.data.get()).as_mut_ptr()) }
	}
}

/// A guard to accessing the `SpinMutex` data with a specified (`'a`) lifetime
pub struct SpinMutexGuard<'a, T> {
	mutex: &'a SpinMutex<T>
}

impl<'a, T> core::ops::Deref for SpinMutexGuard<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		unsafe { &*self.mutex.data.get() }
	}
}

impl<'a, T> core::ops::DerefMut for SpinMutexGuard<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		unsafe { &mut *self.mutex.data.get() }
	}
}

impl<'a, T> Drop for SpinMutexGuard<'a, T> {
	fn drop(&mut self) {
		self.mutex.locked.store(false, Ordering::Release);
	}
}
