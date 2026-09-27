pub unsafe fn c_strlen(ptr: *const u8) -> usize {
	let mut len = 0;
	while unsafe { *ptr.add(len) } != 0 {
		len += 1;
	}
	len
}
