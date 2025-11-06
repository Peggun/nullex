use crate::{println, serial_println};

pub fn backtrace_from_rbp(mut rbp: usize) {
    serial_println!("=== backtrace_from_rbp start ===");
    println!("=== backtrace start ===");

    const MAX_FRAMES: usize = 64;
    for i in 0..MAX_FRAMES {
        if rbp == 0 {
            serial_println!("frame {}: rbp == 0, stopping", i);
            break;
        }

        // Validate canonicalness of the rbp to avoid wild derefs.
        // VirtAddr::is_canonical is not available here; validate manually from the raw address.
        let addr = rbp as u64;
        let high = addr >> 48;
        if !(high == 0 || high == 0xffff) {
            serial_println!("frame {}: non-canonical rbp {:#x}, stopping", i, rbp);
            println!("frame {}: non-canonical rbp {:#x}, stopping", i, rbp);
            break;
        }

        // Safe-ish reads: still unsafe, but we validated canonical address first.
        unsafe {
            // previous rbp is stored at [rbp]
            let prev_rbp_ptr = rbp as *const usize;
            // return address is at [rbp + usize size]
            let ret_addr_ptr = (rbp + core::mem::size_of::<usize>()) as *const usize;

            // Use core::ptr::read to avoid potential alignment optimizations.
            let prev_rbp = core::ptr::read(prev_rbp_ptr);
            let ret_addr = core::ptr::read(ret_addr_ptr);

            serial_println!("#{:02} ret {:#018x}  rbp {:#018x}", i, ret_addr, rbp);
            println!("#{:<2} {:#018x} rbp={:#018x}", i, ret_addr, rbp);

            // Stop if prev_rbp is null or does not move up the stack (prevents loops)
            if prev_rbp == 0 {
                serial_println!("frame {}: previous rbp == 0, stopping", i);
                break;
            }
            if prev_rbp <= rbp {
                serial_println!(
                    "frame {}: previous rbp {:#x} <= current rbp {:#x}, stopping",
                    i,
                    prev_rbp,
                    rbp
                );
                break;
            }

            rbp = prev_rbp;
        }
    }

    serial_println!("=== backtrace end ===");
    println!("=== backtrace end ===");
}

#[inline(always)]
pub fn backtrace_current() {
    let rbp: usize;
    unsafe {
        core::arch::asm!("mov {}, rbp", out(reg) rbp, options(nomem, nostack, preserves_flags));
    }
    backtrace_from_rbp(rbp);
}