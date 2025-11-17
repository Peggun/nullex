// Copyright 2016 lazy-static.rs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// https://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// https://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/lazy_static/core_lazy.rs>
// Portions copied from upstream:
//   https://github.com/rust-lang-nursery/lazy-static.rs (commit 6f864e4)
//   Upstream original file: <src/core_lazy.rs>
// Copyright (c) 2016 lazy-static.rs Developers
// Modifications: Removed `std`-feature code. Suited code for kernel paths.
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

use crate::utils::spin::once::Once;

pub struct Lazy<T: Sync>(Once<T>);

impl<T: Sync> Lazy<T> {
	pub const INIT: Self = Lazy(Once::INIT);

	#[inline(always)]
	pub fn get<F>(&'static self, builder: F) -> &'static T
	where
		F: FnOnce() -> T
	{
		self.0.call_once(builder)
	}
}

#[macro_export(local_inner_macros)]
#[doc(hidden)]
macro_rules! __lazy_static_create {
	($NAME:ident, $T:ty) => {
		static $NAME: $crate::utils::lazy_static::core_lazy::Lazy<$T> =
			$crate::utils::lazy_static::core_lazy::Lazy::INIT;
	};
}
