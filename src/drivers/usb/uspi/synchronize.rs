use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::{self, Relaxed, SeqCst}, compiler_fence};

//
// -------------------- ARM32 (non-AArch64) --------------------
//

//
// RPi 1
//
#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn invalidate_instruction_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c5, 0",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn flush_prefetch_buffer() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c5, 4",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn flush_branch_target_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c5, 6",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn invalidate_data_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c6, 0",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn clean_data_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c10, 0",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn data_sync_barrier() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c10, 4",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn data_mem_barrier() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c10, 5",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn instruction_sync_barrier() {
    flush_prefetch_buffer();
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn instruction_mem_barrier() {
    flush_prefetch_buffer();
}

//
// RPi 2/3/4 running 32-bit ARM
//
#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn invalidate_instruction_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c5, 0",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn flush_prefetch_buffer() {
    unsafe {
        core::arch::asm!("isb", options(nostack));
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn flush_branch_target_cache() {
    unsafe {
        core::arch::asm!(
            "mcr p15, 0, {0}, c7, c5, 6",
            in(reg) 0,
            options(nostack)
        );
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn data_sync_barrier() {
    unsafe {
        core::arch::asm!("dsb", options(nostack));
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn data_mem_barrier() {
    unsafe {
        core::arch::asm!("dmb", options(nostack));
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn instruction_sync_barrier() {
    unsafe {
        core::arch::asm!("isb", options(nostack));
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn instruction_mem_barrier() {
    unsafe {
        core::arch::asm!("isb", options(nostack));
    }
}

//
// -------------------- AArch64 --------------------
//

#[cfg(target_arch = "aarch64")]
pub fn data_sync_barrier() {
    unsafe {
        core::arch::asm!(
            "dsb sy",
            options(nostack)
        );
    }
}

#[cfg(target_arch = "aarch64")]
pub fn data_mem_barrier() {
    unsafe {
        core::arch::asm!(
            "dmb sy",
            options(nostack)
        );
    }
}

//
// Compiler barrier
//
pub fn compiler_barrier() {
    compiler_fence(Ordering::SeqCst);
}

#[cfg(target_arch = "arm")]
pub fn enable_interrupts() {
    core::arch::asm!(
        "cpsie i",
        options(nostack)
    )
}

#[cfg(target_arch = "arm")]
pub fn disable_interrupts() {
    core::arch::asm!(
        "cpsid i",
        options(nostack)
    )
}

#[cfg(target_arch = "aarch64")]
pub fn enable_interrupts() {
    core::arch::asm!(
        "msr DAIFClr, #2",
        options(nostack),
    )
}

#[cfg(target_arch = "aarch64")]
pub fn disable_interrupts() {
    core::arch::asm!(
        "msr DAIFSet, #2",
        options(nostack),
    )
}

pub static s_nCriticalLevel: AtomicU32 = AtomicU32::new(0); 
pub static s_bWereEnabled: AtomicBool = AtomicBool::new(false);

pub fn uspi_enter_critical() {
    let n_flags: usize = {
        #[cfg(target_arch = "arm")]
        {
            let mut v: u32;
            unsafe {
                core::arch::asm!(
                    "mrs {}, cpsr",
                    out(reg) v,
                    options(nostack, nomem)
                );
            }
            v as usize
        }

        #[cfg(target_arch = "aarch64")]
        {
            let mut v: u64;
            unsafe {
                core::arch::asm!(
                    "mrs {}, daif",
                    out(reg) v,
                    options(nostack, nomem)
                );
            }
            v as usize
        }
    };

    disable_interrupts();

    if s_nCriticalLevel.fetch_add(1, Relaxed) == 0 {
        s_bWereEnabled.store((n_flags & 0x80) == 0, SeqCst);
    }

    data_mem_barrier();
}

pub fn uspi_leave_critical() {
    data_mem_barrier();

    assert!(s_nCriticalLevel.load(Relaxed) > 0);

    if s_nCriticalLevel.fetch_sub(1, Relaxed) == 1 {
        if s_bWereEnabled.load(Relaxed) {
            enable_interrupts();
        }
    }
}

#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub const DATA_CACHE_LINE_LENGTH: u32 = 32;
#[cfg(all(target_arch = "arm", feature = "rpi1"))]
pub fn uspi_clean_and_invalidate_data_cache_range(mut nAddress: u32, mut nLength: u32) {
    nLength += DATA_CACHE_LINE_LENGTH;
    loop {
        unsafe {
            core::arch::asm!(
                "mcr p15, 0, {0}, c7, c14",
                in(reg) nAddress,
                options(nostack)
            )
        }

        if (nLength < DATA_CACHE_LINE_LENGTH) {
            break;
        }

        nAddress += DATA_CACHE_LINE_LENGTH;
        nLength  -= DATA_CACHE_LINE_LENGTH;
    }
}

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const L1_DATA_CACHE_LINE_LENGTH: u32 =  64;
#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const L2_CACHE_LINE_LENGTH: u32 =       64;
#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const DATA_CACHE_LINE_LENGTH_MIN: u32 = 64; // min(L1_DATA_CACHE_LINE_LENGTH, L2_CACHE_LINE_LENGTH)

#[cfg(all(
    target_arch = "arm",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn uspi_clean_and_invalidate_data_cache_range(mut nAddress: u32, mut nLength: u32) {
    nLength += DATA_CACHE_LINE_LENGTH_MIN;

    loop {
        unsafe {
            core::arch::asm!(
                "mcr p15, 0, {0}, c7, c14,  1",
                in(reg) nAddress,
                options(nostack),
            )
        }

        if nLength < DATA_CACHE_LINE_LENGTH_MIN {
            break;
        }

        nAddress += DATA_CACHE_LINE_LENGTH_MIN;
        nLength  -= DATA_CACHE_LINE_LENGTH_MIN;
    }
}

#[cfg(all(
    target_arch = "aarch64",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const L1_DATA_CACHE_LINE_LENGTH: u64 =  64;
#[cfg(all(
    target_arch = "aarch64",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const L2_CACHE_LINE_LENGTH: u64 =       64;
#[cfg(all(
    target_arch = "aarch64",
    any(feature = "rpi2", feature = "rpi3")
))]
pub const DATA_CACHE_LINE_LENGTH_MIN: u64 = 64;

#[cfg(all(
    target_arch = "aarch64",
    any(feature = "rpi2", feature = "rpi3")
))]
pub fn uspi_clean_and_invalidate_data_cache_range(mut nAddress: u64, mut nLength: u64) {
    nLength += DATA_CACHE_LINE_LENGTH_MIN;

    loop {
        unsafe {
            core::arch::asm!(
                "dc civac, {0}",
                in(reg) nAddress,
                options(nostack),
            )
        }

        if nLength < DATA_CACHE_LINE_LENGTH_MIN {
            break;
        }

        nAddress += DATA_CACHE_LINE_LENGTH_MIN;
        nLength  -= DATA_CACHE_LINE_LENGTH_MIN;
    }
}