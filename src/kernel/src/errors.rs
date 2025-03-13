// errors.rs
use core::fmt;

#[derive(Debug)]
pub enum KernelError {
	VgaError(i32),        // VGA-related errors (-3 to -20)
	MemoryError(i32),     // Memory allocation errors (-21 to -40)
	FileSystemError(i32), // File system errors (-41 to -60)
	SerialError(i32),     // Serial interface errors (-61 to -80)
	KeyboardError(i32),   // Keyboard errors (-81 to -100)
	VgaDriverError(i32),  // VGA driver errors (-101 to -120)
	CommandError(i32),    // Command errors (-121 to -140)
	ApicError(i32),       // APIC errors (-141 to -160)
	SystemError(i32)      // System errors (-161 to -180)
}

impl fmt::Display for KernelError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			KernelError::VgaError(code) => write!(f, "VGA Error: {}", code),
			KernelError::MemoryError(code) => write!(f, "Memory Error: {}", code),
			KernelError::FileSystemError(code) => write!(f, "File System Error: {}", code),
			KernelError::SerialError(code) => write!(f, "Serial Error: {}", code),
			KernelError::KeyboardError(code) => write!(f, "Keyboard Error: {}", code),
			KernelError::VgaDriverError(code) => write!(f, "VGA Driver Error: {}", code),
			KernelError::CommandError(code) => write!(f, "Command Error: {}", code),
			KernelError::ApicError(code) => write!(f, "APIC Error: {}", code),
			KernelError::SystemError(code) => write!(f, "System Error: {}", code)
		}
	}
}

pub const SUCCESS: i32 = 0;
pub const FAILURE: i32 = -1;

// ----- VGA Errors ----- -3 to 20 //
pub const VGA_BUFFER_UNINITIALIZED: i32 = -10; // VGA buffer not initialized
pub const VGA_BUFFER_OVERFLOW: i32 = -11; // VGA buffer overflow when trying to write outside the VGA Buffer bounds.
pub const VGA_BUFFER_UNDERFLOW: i32 = -12; // VGA buffer underflow when trying to read outside the VGA Buffer bounds.
pub const VGA_BUFFER_MEMORY_ERROR: i32 = -13; // VGA buffer memory error when trying to access the VGA Buffer memory.

// ----- Global Memory Allocator Errors ----- -21 to -40 //
pub const MEM_ALLOC_OUT_OF_MEMORY: i32 = -20; // Out of memory error when trying to allocate memory.
pub const MEM_ALLOC_INVALID_SIZE: i32 = -21; // Invalid size error when trying to allocate memory.
pub const MEM_ALLOC_INVALID_ADDRESS: i32 = -22; // Invalid address error when trying to allocate memory.
pub const MEM_ALLOC_CORRUPTION: i32 = -23; // Memory corruption error when trying to allocate memory.
pub const MEM_ALLOC_DOUBLE_FREE: i32 = -24; // Double free error when trying to allocate memory.

// ----- File System Errors ----- -41 to -60 //
pub const FS_FILE_NOT_FOUND: i32 = -41; // File not found error when trying to access a file.
pub const FS_FILE_EXISTS: i32 = -42; // File exists error when trying to create a file that already exists.
pub const FS_FILE_INVALID_PATH: i32 = -43; // Invalid path error when trying to access a file.
pub const FS_FILE_INVALID_PERMISSION: i32 = -44; // Invalid permission error when trying to access a file.
pub const FS_READ_ERROR: i32 = -45; // Read error when trying to read from a file.
pub const FS_WRITE_ERROR: i32 = -46; // Write error when trying to write to a file.
pub const FS_DELETE_ERROR: i32 = -47; // Delete error when trying to delete a file.
pub const FS_CLOSE_ERROR: i32 = -48; // Close error when trying to close a file.
pub const FS_OPEN_ERROR: i32 = -49; // Open error when trying to open a file.
pub const FS_INVALID_FILE_DESCRIPTOR: i32 = -50; // Invalid file descriptor error when trying to access a file.
pub const FS_MEMORY_ERROR: i32 = -51; // Memory error when trying to access a file.
pub const FS_DIR_LIST_FAILED_ERROR: i32 = -52; // Directory list failed error when trying to list the contents of a directory.
pub const FS_DIR_LIST_TYPES_FAILED_ERROR: i32 = -53; // Directory list types failed error when trying to list the contents of a directory.

// ----- SERIAL Errors ----- -61 to -80 //
pub const SERIAL_PORT_UNAVAILABLE: i32 = -60; // Serial port unavailable error when trying to access the serial port.
pub const SERIAL_BUFFER_OVERFLOW: i32 = -61; // Serial buffer overflow error when trying to write to the serial port.
pub const SERIAL_WRITE_ERROR: i32 = -62; // Serial write error when trying to write to the serial port.
pub const SERIAL_READ_ERROR: i32 = -63; // Serial read error when trying to read from the serial port.
pub const SERIAL_TIMEOUT: i32 = -64; // Serial timeout error when trying to access the serial port.

// ----- Keyboard Errors ----- -81 to -100 //
pub const KEYBOARD_DRIVER_NOT_INITIALIZED: i32 = -80; // Keyboard driver not initialized error when trying to access the keyboard.
pub const KEYBOARD_BUFFER_OVERFLOW: i32 = -81; // Keyboard buffer overflow error when trying to write to the keyboard buffer.
pub const KEYBOARD_BUFFER_UNDERFLOW: i32 = -82; // Keyboard buffer underflow error when trying to read from the keyboard buffer.
pub const KEYBOARD_INVALID_SCANCODE: i32 = -83; // Keyboard invalid scancode error when trying to access the keyboard.
pub const KEYBOARD_INTERRUPT_ERROR: i32 = -84; // Keyboard interrupt error when trying to access the keyboard.

// ----- VGA Driver Errors ----- -101 to -120 //
pub const VGA_DRIVER_NOT_INITIALIZED: i32 = -100; // VGA driver not initialized error when trying to access the VGA driver.
pub const VGA_DRIVER_INIT_FAILED: i32 = -101; // VGA driver initialization failed error when trying to initialize the VGA driver.
pub const VGA_DRIVER_INVALID_MODE: i32 = -102; // VGA driver invalid mode error when trying to access the VGA driver.
pub const VGA_DRIVER_BUFFER_ERROR: i32 = -103; // VGA driver buffer error when trying to access the VGA driver.

// ----- Command Errors ----- -121 to -140 //
pub const COMMAND_NOT_FOUND: i32 = -120; // Command not found error when trying to execute a command.
pub const COMMAND_NO_ARGUMENTS: i32 = -121; // No arguments error when trying to execute a command.
pub const COMMAND_INVALID_ARGUMENTS: i32 = -122; // Invalid arguments error when trying to execute a command.
pub const COMMAND_EXECUTION_FAILURE: i32 = -123; // Command execution failure error when trying to execute a command.
pub const COMMAND_PERMISSION_DENIED: i32 = -124; // Permission denied error when trying to execute a command
pub const COMMAND_MISSING_ARGUMENT: i32 = -125; // Missing argument error when trying to execute a command.

// ----- APIC Errors ----- -141 to -160 //
pub const APIC_TIMER_INIT_FAILED: i32 = -141; // APIC timer initialization failed error when trying to initialize the APIC timer.
pub const APIC_TIMER_CONFIGURATION_ERROR: i32 = -142; // APIC timer configuration error when trying to configure the APIC timer.
pub const APIC_TIMER_INVALID_FREQUENCY: i32 = -143; // APIC timer invalid frequency error when trying to set the APIC timer frequency.
pub const APIC_TIMER_INVALID_MODE: i32 = -144; // APIC timer invalid mode error when trying to set the APIC timer mode.
pub const APIC_TIMER_INTERRUPT_FAILURE: i32 = -145; // APIC timer interrupt failure error when trying to access the APIC timer interrupt.
pub const APIC_TIMER_TIMEOUT: i32 = -146; // APIC timer timeout error when trying to access the APIC timer.
pub const TSC_NOT_INVARIANT: i32 = -147;
pub const CPUID_LEAF_UNSUPPORTED: i32 = -148;
pub const APIC_CALIBRATION_FAILED: i32 = -149;

// ----- System Errors ----- -161 to -180 //
pub const SYSTEM_BUSY: i32 = -161; // System busy error when the system is busy.
