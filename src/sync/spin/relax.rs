// SPDX-License-Identifier: MIT
// This file: <src/utils/spin/once.rs>
// Portions copied from upstream:
//   https://github.com/zesterer/once-rs (commit 71a1d8e)
//   Upstream original file: <src/once.rs>
// Copyright (c) 2014 Mathijs van de Nes
// Modifications: Removed code parts that are not needed currently for the
// kernel. See THIRD_PARTY_LICENSES.md for full license texts and upstream
// details.

//! Strategies that determine the behaviour of locks when encountering
//! contention.

/// A trait implemented by spinning relax strategies.
pub trait RelaxStrategy {
	/// Perform the relaxing operation during a period of contention.
	fn relax();
}

/// A strategy that rapidly spins while informing the CPU that it should power
/// down non-essential components via [`core::hint::spin_loop`].
///
/// Note that spinning is a 'dumb' strategy and most schedulers cannot correctly
/// differentiate it from useful work, thereby misallocating even more CPU time
/// to the spinning process. This is known as ['priority inversion'](https://matklad.github.io/2020/01/02/spinlocks-considered-harmful.html).
///
/// If you see signs that priority inversion is occurring, consider switching to
/// [`Yield`] or, even better, not using a spinlock at all and opting for a
/// proper scheduler-aware lock. Remember also that different targets, operating
/// systems, schedulers, and even the same scheduler with different workloads
/// will exhibit different behaviour. Just because priority inversion isn't
/// occurring in your tests does not mean that it will not occur. Use a
/// scheduler- aware lock if at all possible.
pub struct Spin;

impl RelaxStrategy for Spin {
	#[inline(always)]
	fn relax() {
		// Use the deprecated spin_loop_hint() to ensure that we don't get
		// a higher MSRV than we need to.
		#[allow(deprecated)]
		core::sync::atomic::spin_loop_hint();
	}
}

/// A strategy that rapidly spins, without telling the CPU to do any powering
/// down.
///
/// You almost certainly do not want to use this. Use [`Spin`] instead. It
/// exists for completeness and for targets that, for some reason, miscompile or
/// do not support spin hint intrinsics despite attempting to generate code for
/// them (i.e: this is a workaround for possible compiler bugs).
pub struct Loop;

impl RelaxStrategy for Loop {
	#[inline(always)]
	fn relax() {}
}
