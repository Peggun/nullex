// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/drivers/keyboard/error.rs>
// Portions copied from upstream:
//   https://github.com/rust-embedded-community/pc-keyboard (commit 6d03cf7)
//   Upstream original file: <src/lib.rs>
// Copyright (c) 2020 Rust Embedded Community Developers
// Modifications: Renamed `Ps2Keyboard` -> `Keyboard`; `Error` ->
// `KeyboardError` See THIRD_PARTY_LICENSES.md for full license texts and
// upstream details.

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum KeyboardError {
	BadStartBit,
	BadStopBit,
	ParityError,
	UnknownKeyCode,
	#[doc(hidden)]
	InvalidState
}
