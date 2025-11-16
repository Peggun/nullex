// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/drivers/keyboard/layout.rs>
// Portions copied from upstream:
//   https://github.com/rust-embedded-community/pc-keyboard (commit 6d03cf7)
//   Upstream original file: <src/lib.rs>
// Copyright (c) 2020 Rust Embedded Community Developers
// Modifications: Renamed `Ps2Keyboard` -> `Keyboard`; `Error` ->
// `KeyboardError` See THIRD_PARTY_LICENSES.md for full license texts and
// upstream details.

use crate::{
	drivers::keyboard::scancode::KeyCode,
	io::keyboard::decode::{DecodedKey, HandleControl, Modifiers}
};

pub enum PhysicalKeyboard {
	Iso,
	Ansi,
	Jis
}

pub trait KeyboardLayout {
	fn map_keycode(
		&self,
		keycode: KeyCode,
		modifiers: &Modifiers,
		handle_ctrl: HandleControl
	) -> DecodedKey;
	fn get_physical(&self) -> PhysicalKeyboard;
}
