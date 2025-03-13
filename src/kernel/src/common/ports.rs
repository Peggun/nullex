// ports.rs

/*
Ports IO module for the kernel.
*/

use core::arch::asm;

pub fn port_byte_in(port: u16) -> u8 {
	let mut result;
	unsafe {
		asm!(
			"in al, dx", // Assembly instruction: in al, dx (read byte from port dx into al)
			in("dx") port,  // Input operand:  dx register gets the value of 'port'
			out("al") result, // Output operand: al register's value is written to 'result'
		);
	}
	return result;
}

pub fn port_byte_out(port: u16, data: u8) {
	unsafe {
		asm!(
			"out dx, al", // Assembly instruction: out dx, al (write byte from al to port dx)
			in("al") data,  // Input operand:  al register gets the value of 'data'
			in("dx") port,  // Input operand:  dx register gets the value of 'port'
		);
	}
}

pub fn port_word_out(port: u16, data: u16) {
	unsafe {
		asm!(
			"out dx, ax", // Assembly instruction: out dx, al (write byte from al to port dx)
			in("ax") data,  // Input operand:  al register gets the value of 'data'
			in("dx") port,  // Input operand:  dx register gets the value of 'port'
		);
	}
}

pub fn port_word_in(port: u16) -> u16 {
	let mut result: u16;
	unsafe {
		asm!(
			"in ax, dx", // Assembly instruction: in ax, dx (read word from port dx into ax)
			in("dx") port,  // Input operand: dx register gets the value of 'port' (u16)
			out("ax") result, // Output operand: ax register's value is written to 'result' (u16)
		);
	}
	return result;
}
