// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/oncecell/spin.rs>
// Portions copied from upstream:
//   https://github.com/oliver-giersch/conquer-once (commit bc018e9)
//   Upstream original file: <src/spin.rs>
// Copyright (c) 2019 Oliver Giersch
// Modifications: Removed `std`-feature code
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

//! Synchronized one-time and lazy initialization primitives that use spin-locks
//! in case of concurrent accesses under contention.

use core::{hint, sync::atomic::Ordering};

use self::internal::Spin;
use crate::utils::oncecell::{
	POISON_PANIC_MSG,
	cell::{Block, Unblock},
	state::{AtomicOnceState, BlockedState, OnceState::WouldBlock}
};

/// A type for lazy initialization of e.g. global static variables, which
/// provides the same functionality as the `lazy_static!` macro.
///
/// This type uses spin-locks if the initialization is contended and is thus
/// `#[no_std]` compatible.
///
/// For the API of this type alias, see the API of the generic
/// [`Lazy`](crate::doc::Lazy) type.
pub type Lazy<T, F = fn() -> T> = crate::utils::oncecell::lazy::Lazy<T, Spin, F>;

/// An interior mutability cell type which allows synchronized one-time
/// initialization and read-only access exclusively after initialization.
///
/// This type uses spin-locks if the initialization is contended and is thus
/// `#[no_std]` compatible.
///
/// For the API of this type alias, see the generic
/// [`OnceCell`](crate::doc::OnceCell) type.
pub type OnceCell<T> = crate::utils::oncecell::cell::OnceCell<T, Spin>;

/// A synchronization primitive which can be used to run a one-time global
/// initialization.
///
/// This type uses spin-locks if the initialization is contended and is thus
/// `#[no_std]` compatible.
///
/// For the API of this type alias, see the generic
/// [`OnceCell`](crate::doc::OnceCell) type.
/// This is a specialization with `T = ()`.
pub type Once = OnceCell<()>;

mod internal {
	/// Blocking strategy for blocking threads using spin-locks.
	#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
	pub struct Spin;
}

impl Unblock for Spin {
	#[inline(always)]
	unsafe fn on_unblock(_: BlockedState) {}
}

unsafe impl Block for Spin {
	/// Spins until the [`OnceCell`] state is set to `READY`, or panics if it
	/// becomes poisoned.
	#[inline]
	fn block(state: &AtomicOnceState) {
		// (spin:1) this acquire load syncs-with the acq-rel swap (guard:2)
		while let WouldBlock(_) = state.load(Ordering::Acquire).expect(POISON_PANIC_MSG) {
			hint::spin_loop();
		}
	}
}
