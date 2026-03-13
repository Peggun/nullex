//!
//! error.rs
//! 
//! Error handling module for the kernel.
//! 

use alloc::string::String;
use thiserror::Error;
use x86_64::{VirtAddr, structures::paging::{PhysFrame, Size4KiB, mapper::MapToError}};
use crate::alloc::string::ToString;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
/// An enum representing all Nullex Errors
pub enum NullexError {
    // --- Generic/Unknown error --- //
    /// An unspecified error occurred, carrying a static message.
    #[error("unknown error: {0}")]
    Unknown(&'static str),

    // --- Memory Errors --- //
    /// The system has exhausted its available physical or virtual memory.
    #[error("out of memory")]
    OutOfMemory,
    /// An attempt was made to access memory outside of valid ranges.
    #[error("memory access out of bounds")]
    MemoryOutOfBounds,
    /// A memory operation was attempted on an unsupported byte boundary.
    #[error("unaligned memory access")]
    MemoryUnaligned,
    /// The physical memory manager failed to provide a free frame.
    #[error("frame allocation failed")]
    FrameAllocationFailed,
    /// The provided virtual address does not follow the architecture's canonical form.
    #[error("(0x{0:x} is not a canonical virtual address")]
    NonCanonicalAddress(VirtAddr),
    /// A failure occurred while mapping a virtual page to a physical frame.
    #[error("frame allocation failed")]
    MapToFailed,
    /// A page table operation failed because a parent entry is already a huge page.
    #[error("parent entry huge page")]
    ParentEntryHugePage,
    /// An attempt was made to map a virtual page that is already assigned to a frame.
    #[error("page already mapped: frame={0:?}")]
    PageAlreadyMapped(PhysFrame),
    /// Failed to allocate a contiguous memory block for Direct Memory Access.
    #[error("dma allocation failed")]
    DmaAllocFailed,

    // --- Interrupt Errors --- //
    /// No free slots remain in the Interrupt Descriptor Table or vector list.
    #[error("vector table full")]
    VectorTableFull,

    // --- Driver / Device errors --- //
    /// The specified hardware device could not be located on the bus.
    #[error("device not found")]
    DeviceNotFound,
    /// An initialization routine was called on a device that is already active.
    #[error("device already initialized")]
    DeviceAlreadyInitialized,
    /// An operation was attempted on a device that has not been set up yet.
    #[error("device not initialized")]
    DeviceNotInitialized,
    /// The system failed to gracefully shut down or clean up a device.
    #[error("device finalization failed")]
    DeviceFinalizeFailed,
    /// The device hardware did not accept the configuration features requested by the driver.
    #[error("device rejected features")]
    DeviceRejectedFeatures, 
    /// The driver entered an error state or failed a status check.
    #[error("driver not ok")]
    DriverNotOk,

    // --- I/O / PCI Errors --- //
    /// A generic input/output error occurred with a specific description.
    #[error("io error: {0}")]
    Io(&'static str),
    /// A failure occurred while writing to the PCI configuration space.
    #[error("pci configuration write failed")]
    PciConfigWriteFailed,
    /// The system could not enable the PCI device or its memory/IO spaces.
    #[error("pci enable failed")]
    PciEnableFailed,
    /// The base address for an I/O operation does not meet alignment requirements.
    #[error("misaligned io base")]
    MisalignedIoBase,

    // --- Timer Errors --- //
    /// The timer frequency or offset calibration failed to complete accurately.
    #[error("calibration failed: {0}")]
    CalibrationFailed(&'static str),
    /// The starting value provided to the timer hardware is out of range.
    #[error("invalid initial count")]
    InvalidInitialCount,

    // -- Disk Errors -- //
    /// An ATA disk operation took longer than the maximum allowed time.
    #[error("ata timeout")]
    AtaTimeout,
    /// Data could not be read from the ATA disk.
    #[error("ata read failed")]
    AtaReadError,
    /// The ATA drive reported an internal hardware or controller fault.
    #[error("ata drive error")]
    AtaDriveError,

    // --- VirtIO / Network Errors --- //
    /// The handshake or setup process for a VirtIO device failed.
    #[error("virtio initialization failed")]
    VirtioInitFailed,
    /// The requested VirtIO queue does not exist or is not configured.
    #[error("VirtQueue unavailable")]
    VirtQueueUnavailable,
    /// No space is left in the VirtIO ring buffer for new descriptors.
    #[error("VirtQueue full")]
    VirtQueueFull,
    /// A failure occurred while pushing a packet to the VirtIO backend.
    #[error("virtio transmit error")]
    VirtioTransmitError,
    /// A required VirtIO device instance was not found in the system registry.
    #[error("missing virtio instance")]
    MissingVirtIOInstance,
    /// A failure occurred while sending a network packet.
    #[error("network send failed: {0}")]
    NetSend(&'static str),
    /// An ICMP echo request failed to receive a timely response.
    #[error("ping failed")]
    PingFailed,
    /// The system could not resolve a MAC address via the Address Resolution Protocol.
    #[error("arp request failed")]
    ArpFailed,
    /// A protocol-level error occurred during a UDP operation.
    #[error("udp error: {0}")]
    Udp(&'static str),
    /// The MAC address for the requested IP is not present in the ARP cache.
    #[error("mac not cached")]
    MacNotCached,
    /// A DNS query failed to receive a response within the timeout period.
    #[error("dns timed out")]
    DnsTimeout,
    /// The system was unable to resolve a hostname to an IP address.
    #[error("failed to resolve: {0}")]
    FailedToResolve(&'static str),
    /// A required MAC address was missing for a network operation.
    #[error("missing mac address")]
    MissingMacAddress,

    // --- Serial Output Errors --- //
    /// An unspecified error occurred during serial port communication.
    #[error("generic serial error")]
    GenericSerialError,

    // --- ACPI Errors --- //
    /// An ACPI table was rejected due to an incorrect or unexpected header signature.
    #[error("invalid signature: {0}")]
    InvalidAcpiSignature(&'static str),

    // --- Common Low-level Errors --- //
    /// A function received a parameter that is invalid or out of context.
    #[error("invalid argument")]
    InvalidArgument,
    /// The provided destination buffer cannot hold the required amount of data.
    #[error("buffer too small")]
    BufferTooSmall,
    /// Data integrity verification failed due to an incorrect checksum.
    #[error("checksum mismatch")]
    ChecksumMismatch,
    /// An operation did not complete within the allotted time.
    #[error("timeout")]
    Timeout,
    /// The current context lacks the required privileges for the operation.
    #[error("permission denied")]
    PermissionDenied,
    /// The requested operation is not implemented or supported by the hardware/driver.
    #[error("unsupported operation")]
    Unsupported,
    /// A numerical result or input was unexpectedly negative.
    #[error("value below 0")]
    ValueBelowZero,

    // --- Process/Executor Errors --- //
    /// A process with the given ID already exists in the executor.
    #[error("process already exists with id")]
    ProcessAlreadyExists,
    /// The requested process was not found in the executor.
    #[error("process not found")]
    ProcessNotFound,
    /// The process queue is full and cannot accept new processes.
    #[error("process queue full")]
    ProcessQueueFull,

    // non-panicking errors.
    /// A non-panicking failure occurred during a component's initialization phase.
    #[error("initialization failed: {0}")]
    InitFailed(&'static str),
}

impl From<&'static str> for NullexError {
	fn from(value: &'static str) -> Self {
		NullexError::Unknown(value)
	}
}

impl From<MapToError<Size4KiB>> for NullexError {
    fn from(value: MapToError<Size4KiB>) -> Self {
        match value {
            MapToError::FrameAllocationFailed => NullexError::FrameAllocationFailed,
            MapToError::ParentEntryHugePage => NullexError::ParentEntryHugePage,
            MapToError::PageAlreadyMapped(f) => NullexError::PageAlreadyMapped(f),
        }
    }
}

impl NullexError {
    // do we need this? im not sure if the #[error] does that already.
    /// Represents the Errors as `str`'s
	pub fn as_str(&self) -> String {
        self.to_string()
    }

}

/// Return early with an error.
///
/// This macro logs a warning and immediately returns `Err`, converting the
/// provided value into `NullexError` using `Into`.
///
/// # Behavior
/// - Logs the error using `println!` with a `[WARN] BAIL!` prefix.
/// - Converts the provided expression into `NullexError`.
/// - Returns early from the current function with `Err(...)`.
///
/// # Requirements
/// The provided expression must implement `Into<NullexError>`.
///
/// # Example
/// ```rust
/// fn check_user(id: u32) -> Result<(), NullexError> {
///     if id == 0 {
///         bail!("invalid user id");
///     }
///     Ok(())
/// }
/// ```
///
/// # Notes
/// This is similar in spirit to `anyhow::bail!`, but additionally emits a
/// warning log before returning.
#[macro_export]
macro_rules! bail {
    ($err:expr $(,)?) => {{
        $crate::println!("[WARN] BAIL! {:#?}", $err);
        return Err(($err).into());
    }};
}

/// Ensure a condition is true, otherwise return an error.
///
/// If the condition evaluates to `false`, this macro logs a warning and
/// returns early with the provided error converted into `NullexError`.
///
/// # Behavior
/// - Evaluates the provided condition.
/// - If `false`, logs the error using `println!` with `[WARN] ENSURE!`.
/// - Converts the provided error into `NullexError`.
/// - Returns early with `Err(...)`.
///
/// # Requirements
/// The error expression must implement `Into<NullexError>`.
///
/// # Example
/// ```rust
/// fn allocate(size: usize) -> Result<(), NullexError> {
///     ensure!(size > 0, "allocation size must be non-zero");
///     Ok(())
/// }
/// ```
///
/// # Notes
/// This macro is useful for guarding preconditions and invariants without
/// writing explicit `if` / `return Err(...)` boilerplate.
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {
        if !($cond) {
            $crate::println!("[WARN] ENSURE! {:#?}", $err);
            return Err(($err).into());
        }
    };
}

/// Unwrap a `Result`, converting the `Err` into `NullexError` and returning it.
///
/// - On `Ok(v)` returns `v`.
/// - On `Err(e)` logs a warning then returns `Err(e.into())`.
///
/// Use-cases:
/// - When calling an API that returns a different error type but you want to
///   propagate it as `NullexError` without writing `match` / `?` explicitly.
/// - Handy in `unsafe` blocks or contexts where `?` style is less convenient.
///
/// Example:
/// ```rust
/// let guard = map_err_bail!(vm.map(page, frame));
/// // guard is the successful value, or we've returned Err converted to NullexError
/// ```
#[macro_export]
macro_rules! map_err_bail {
    ($expr:expr $(,)?) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                $crate::println!("[WARN] BAIL! {:#?}", e);
                return Err(e.into())
            },
        }
    };
}

/// Trigger a kernel panic.
///
/// This macro prints a fatal log message and then invokes `panic!`.
///
/// # Behavior
/// - Logs `[FATAL] KERNEL PANIC!`.
/// - Forwards all arguments to `panic!`.
///
/// # Example
/// ```rust
/// kernel_panic!("unexpected page fault at {:#x}", addr);
/// ```
///
/// # Notes
/// Intended for unrecoverable kernel errors where execution cannot safely
/// continue.
#[macro_export]
macro_rules! kernel_panic {
    ($($arg:tt)*) => {{
        $crate::println!("[FATAL] KERNEL PANIC!");
        panic!($($arg)*);
    }};
}

/// Kernel assertion macro.
///
/// Checks that a condition is true. If the condition evaluates to `false`,
/// a fatal log message is emitted and a kernel panic is triggered.
///
/// # Behavior
/// - Evaluates the provided condition.
/// - If `false`:
///   - Logs `[FATAL] KERNEL ASSERT!`.
///   - Triggers `kernel_panic!` with the provided message.
///
/// # Example
/// ```rust
/// kassert!(ptr.is_aligned(), "pointer must be aligned");
/// ```
///
/// # Notes
/// - Unlike `debug_assert!`, this assertion is **always active**.
/// - Intended for validating critical kernel invariants.
/// - Failure results in an immediate kernel panic.
#[macro_export]
macro_rules! kassert {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            $crate::println!("[FATAL] KERNEL ASSERT!");
            $crate::kernel_panic!("{}", &format!($($arg)*));
        }
    };
}