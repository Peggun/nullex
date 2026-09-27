// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/oncecell/mod.rs>
// Portions copied from upstream:
//   https://github.com/oliver-giersch/conquer-once (commit bc018e9)
//   Upstream original file: <src/lib.rs>
// Copyright (c) 2019 Oliver Giersch
// Modifications: Removed `std`-feature code
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

pub mod cell;
pub mod lazy;
pub mod noblock;
pub mod spin;
pub mod state;

pub const POISON_PANIC_MSG: &str = "OnceCell instance has been poisoned.";
