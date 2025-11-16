// SPDX-License-Identifier: MIT OR Apache-2.0
// This file: <src/utils/volatile.rs>
// Portions copied from upstream:
//   https://github.com/rust-osdev/volatile (commit a5a6d78)
//   Upstream original file: <src/lib.rs>
// Copyright (c) 2020 Philipp Oppermann
// Modifications: None (using older version 0.2.6)
// See THIRD_PARTY_LICENSES.md for full license texts and upstream details.

use core::ptr;

#[derive(Debug)]
#[repr(transparent)]
pub struct Volatile<T: Copy>(T);

impl<T: Copy> Volatile<T> {
	pub const fn new(value: T) -> Volatile<T> {
		Volatile(value)
	}

	pub fn read(&self) -> T {
		unsafe { ptr::read_volatile(&self.0) }
	}

	pub fn write(&mut self, value: T) {
		unsafe {
			ptr::write_volatile(&mut self.0, value);
		}
	}

	pub fn update<F>(&mut self, f: F)
	where
		F: FnOnce(&mut T)
	{
		let mut value = self.read();
		f(&mut value);
		self.write(value)
	}
}

impl<T: Copy> Clone for Volatile<T> {
	fn clone(&self) -> Self {
		Volatile(self.read())
	}
}

#[derive(Debug, Clone)]
pub struct ReadOnly<T: Copy>(Volatile<T>);

impl<T: Copy> ReadOnly<T> {
	pub const fn new(value: T) -> ReadOnly<T> {
		ReadOnly(Volatile::new(value))
	}

	pub fn read(&self) -> T {
		self.0.read()
	}
}

#[derive(Debug, Clone)]
pub struct WriteOnly<T: Copy>(Volatile<T>);

impl<T: Copy> WriteOnly<T> {
	pub const fn new(value: T) -> WriteOnly<T> {
		WriteOnly(Volatile::new(value))
	}

	pub fn write(&mut self, value: T) {
		self.0.write(value)
	}
}

pub type ReadWrite<T> = Volatile<T>;
