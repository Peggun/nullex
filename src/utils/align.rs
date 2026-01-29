/// A wrapper for aligning a type of `T` to 16 bytes.
#[repr(align(16))]
pub struct Align16<T>(T);

impl<T> Align16<T> {
	pub const fn new(num: T) -> Self {
		Self(num)
	}

	pub fn value(self) -> T {
		self.0
	}
}

/// A wrapper for aligning a type of `T` to 8 bytes.
#[repr(align(8))]
pub struct Align8<T>(T);

impl<T> Align8<T> {
	pub const fn new(num: T) -> Self {
		Self(num)
	}

	pub fn value(self) -> T {
		self.0
	}
}

/// A wrapper for aligning a type of `T` to 4 bytes.
#[repr(align(4))]
pub struct Align4<T>(T);

impl<T> Align4<T> {
	pub const fn new(num: T) -> Self {
		Self(num)
	}

	pub fn value(self) -> T {
		self.0
	}
}

/// A wrapper for aligning a type of `T` to 2 bytes.
#[repr(align(2))]
pub struct Align2<T>(T);

impl<T> Align2<T> {
	pub const fn new(num: T) -> Self {
		Self(num)
	}

	pub fn value(self) -> T {
		self.0
	}
}
