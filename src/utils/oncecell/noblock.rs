// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/oncecell/noblock.rs>
// Portions copied from upstream:
//   https://github.com/oliver-giersch/conquer-once (commit bc018e9)
//   Upstream original file: <src/noblock.rs>
// Copyright (c) 2019 Oliver Giersch
// Modifications: Removed `std`-feature code
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

//! Synchronized one-time and lazy initialization primitives that permit only
//! non-blocking synchronized initialization operations.

use self::internal::NoBlock;
use crate::utils::oncecell::{cell::Unblock, state::BlockedState};

/// A type for lazy initialization of e.g. global static variables, which
/// provides the same functionality as the `lazy_static!` macro.
///
/// This type does not permit any (potentially) blocking operations, only their
/// respective non-blocking counterparts and is thus `#[no_std]` compatible.
///
/// For the API of this type alias, see the API of the generic
/// [`Lazy`](crate::lazy::Lazy) type.
pub type Lazy<T, F = fn() -> T> = crate::utils::oncecell::lazy::Lazy<T, NoBlock, F>;

/// An interior mutability cell type which allows synchronized one-time
/// initialization and read-only access exclusively after initialization.
///
/// This type does not permit any (potentially) blocking operations, only their
/// respective non-blocking counterparts and is thus `#[no_std]` compatible.
///
/// For the API of this type alias, see the generic
/// [`OnceCell`](crate::doc::OnceCell) type.
pub type OnceCell<T> = crate::utils::oncecell::cell::OnceCell<T, NoBlock>;

/// A synchronization primitive which can be used to run a one-time global
/// initialization.
///
/// This type does not permit any (potentially) blocking operations, only their
/// respective non-blocking counterparts and is thus `#[no_std]` compatible.
///
/// For the API of this type alias, see the generic
/// [`OnceCell`](crate::doc::OnceCell) type.
/// This is a specialization with `T = ()`.
pub type Once = crate::utils::oncecell::cell::OnceCell<(), NoBlock>;

mod internal {
	/// "Blocking" strategy which does not actually allow blocking.
	#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
	pub struct NoBlock;
}

impl Unblock for NoBlock {
	#[inline(always)]
	unsafe fn on_unblock(_: BlockedState) {}
}
