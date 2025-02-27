/// APIC timer and register definitions.
pub mod apic {
    use core::ptr::{read_volatile, write_volatile};

    /// The base address of the Local APIC (xAPIC mode).
    pub const APIC_BASE: usize = 0xFEE00000;

    // Register offsets (relative to the APIC base)
    pub const ID: usize              = 0x020;
    pub const EOI: usize             = 0x0B0;
    pub const SVR: usize             = 0x0F0;
    pub const LVT_TIMER: usize       = 0x320;
    pub const TIMER_INIT_COUNT: usize = 0x380;
    pub const TIMER_CURRENT_COUNT: usize = 0x390;
    pub const TIMER_DIVIDE: usize    = 0x3E0;

    // Timer mode and configuration bits.
    /// Bit flag for periodic mode in the LVT Timer Register.
    pub const TIMER_PERIODIC: u32 = 0x20000;
    /// The interrupt vector you choose for timer interrupts (commonly 0x20).
    pub const TIMER_INTERRUPT_VECTOR: u32 = 0x20;
    /// Divide configuration value: here, 0x3 typically means divide by 16.
    pub const DIVIDE_BY_16: u32 = 0x3;

    /// Write a 32-bit value to a Local APIC register.
    pub unsafe fn write_register(offset: usize, value: u32) {
        let reg = (APIC_BASE + offset) as *mut u32;
        write_volatile(reg, value);
    }

    /// Read a 32-bit value from a Local APIC register.
    pub unsafe fn read_register(offset: usize) -> u32 {
        let reg = (APIC_BASE + offset) as *const u32;
        read_volatile(reg)
    }

    /// Initialize the APIC timer in periodic mode.
    ///
    /// `initial_count` is the value from which the timer will count down.
    /// You may need to calibrate this value based on your desired tick rate.
    pub unsafe fn init_timer(initial_count: u32) {
        // Set the timer divide configuration to divide by 16.
        write_register(TIMER_DIVIDE, DIVIDE_BY_16);
        // Configure the LVT timer: set periodic mode and the chosen interrupt vector.
        write_register(LVT_TIMER, TIMER_PERIODIC | TIMER_INTERRUPT_VECTOR);
        // Set the initial count so that the timer starts counting down.
        write_register(TIMER_INIT_COUNT, initial_count);
    }

    /// Signal End-of-Interrupt (EOI) to the Local APIC.
    pub unsafe fn send_eoi() {
        write_register(EOI, 0);
    }

    use x86_64::registers::model_specific::Msr;

    pub unsafe fn enable_apic() {
        let mut msr = Msr::new(0x1B);
        let value = msr.read();
        msr.write(value | 0x800); // Set the "Enable APIC" bit (bit 11)
    }
}