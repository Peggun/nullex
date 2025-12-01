// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/bitflags/external.rs>
// Portions copied from upstream:
//   https://github.com/bitflags/bitflags (commit 7cc8595)
//   Upstream original file: <src/external.rs>
// Copyright (c) 2014 The Rust Project Developers
// Modifications: Removed `std`-feature & external features code. Suited code
// for kernel paths. See THIRD_PARTY_LICENSES.md for full license texts and
// upstream details.

//! Conditional trait implementations for external libraries.

/*
How do I support a new external library?

Let's say we want to add support for `my_library`.

First, we create a module under `external`, like `serde` with any specialized code.
Ideally, any utilities in here should just work off the `Flags` trait and maybe a
few other assumed bounds.

Next, re-export the library from the `__private` module here.

Next, define a macro like so:

```rust
#[macro_export]
#[doc(hidden)]
#[cfg(feature = "serde")]
macro_rules! __impl_external_bitflags_my_library {
	(
		$InternalBitFlags:ident: $T:ty, $PublicBitFlags:ident {
			$(
				$(#[$inner:ident $($args:tt)*])*
				const $Flag:tt;
			)*
		}
	) => {
		// Implementation goes here
	};
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "my_library"))]
macro_rules! __impl_external_bitflags_my_library {
	(
		$InternalBitFlags:ident: $T:ty, $PublicBitFlags:ident {
			$(
				$(#[$inner:ident $($args:tt)*])*
				const $Flag:tt;
			)*
		}
	) => {};
}
```

Note that the macro is actually defined twice; once for when the `my_library` feature
is available, and once for when it's not. This is because the `__impl_external_bitflags_my_library`
macro is called in an end-user's library, not in `bitflags`. In an end-user's library we don't
know whether or not a particular feature of `bitflags` is enabled, so we unconditionally call
the macro, where the body of that macro depends on the feature flag.

Now, we add our macro call to the `__impl_external_bitflags` macro body:

```rust
__impl_external_bitflags_my_library! {
	$InternalBitFlags: $T, $PublicBitFlags {
		$(
			$(#[$inner $($args)*])*
			const $Flag;
		)*
	}
}
```
*/

pub(crate) mod __private {}

/// Implements traits from external libraries for the internal bitflags type.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_external_bitflags {
	(
        $InternalBitFlags:ident: $T:ty, $PublicBitFlags:ident {
            $(
                $(#[$inner:ident $($args:tt)*])*
                const $Flag:tt;
            )*
        }
    ) => {};
}
