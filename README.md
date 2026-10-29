 # Nullex OS
An experimental x86_64 operating system with a Rust kernel and C userspac

![GitHub branch check runs](https://img.shields.io/github/check-runs/Peggun/nullex/master)
![GitHub Repo stars](https://img.shields.io/github/stars/Peggun/nullex)
![GitHub Release](https://img.shields.io/github/v/release/Peggun/nullex?include-prereleases)


> [!WARNING]
> **Nullex is still experimental and under active development.**
> It is intended for learning and experimentation only, not production use. Kernel interfaces, system calls, and other features may change without notice as development progresses.
> 
> Some subsystems are incomplete or have significant limitations. See the [Experimental Features](https://github.com/Peggun/nullex#experimental-features) and [Unsupported Features](https://github.com/Peggun/nullex#unsupported-features-and-platforms) section for more information.

The documentation for the Nullex OS is found [here](https://peggun.github.io/nullex).

This project uses code segments from third party crates. All information can be found [here](https://github.com/Peggun/nullex/blob/master/THIRD_PARTY_LICENSES.md). Please let me know if there is an issue with this, so I can update accordingly.

If you would like to see the packages available for nullex, they are located [here](https://peggun.github.io/nullex-packages).

## Getting Started

### Development Setup

Nullex currently uses a Linux-based development environment.
The easiest way to install the required development tools is
through the provided setup script:

```bash
./scripts/setup-dev.sh
``` 

#### Testing
To run the test suite you can run:
```bash
make test
```

#### Running
The default `make run` target currently connects
Nullex to a VirtIO network device using a TAP
interface named `tap0`.

> [!NOTE]
> This script modifies the host system's network configuration to allow the TAP interface to be used by QEMU.

On Linux, configure the virtual network before
running the emulator. The setup script configures
the TAP interface, DHCP, IP forwarding and NAT.
```bash
sudo ./scripts/setup-vn.sh 
```

Run the QEMU Emulator:
```bash
make run
```

### Features and Roadmap
For the features and roadmap, please refer to the [Notion](https://app.notion.com/p/a7f08e4b78f9468aa1e42b5c8a92b99b?v=19b7e3120851423e9b35bd9e6a0497eb&source=copy_link) table.

### Experimental Features
Currently, the following features are experimental and may be further secured, and stabilized later on.
- **The overall kernel** - it's in beta so yeah not production ready
- **x86_64 implementation** - low-level memory, interrupts, paging, GDT, APIC, and user-mode code are still evolving.
- **User-space process execution** - ELF loading, address spaces, context switching, and syscall transitions are implemented, but not yet fully complete.
- **System-call ABI** - the syscall interface is still incomplete, and unsafe at the user/kernel boundary.
- **RAM filesystem** - currently the only filesystem, but data is memory-backed and not persistent yet.
- **Networking stack** - VirtIO networking, Ethernet, ARP, IPv4, TCP, UDP, DNS, HTTP, HTTPS are limited implementations
- **HTTPS** - experimental because it uses `UnsecureProvider`; TLS peer/certificate verification are not yet implemented.
- **HTTP client/downloads** - supports only `GET` with limited functionality.
- **DNS resolution** - fixed resolver configuration, IPv4 only lookups and no DNSSEC or encryption.
- **Process scheduling** - basic process queue and wake-up logic but not yet a mature multitasking implementation.
- **User programs** - currently small statically linked ELF binaries designed specifically for this kernel.
- **Synchronization primitives** - basic synchronization primitives such as mutexes and condition variables are from external crates
- **Custom memory allocators** - all memory allocations are from scrach, with allocator selection being unfinished.
- **Testing infrastructure** - once again, the testing infrastructure is not yet complete and mature, and chances are it will change.
- **QEMU networking environment** - networking depends on the current TAP, VirtIO, DHCP, DNS and firewall setup in `setup-vn.sh`

### Unsupported Platforms and architectures
These are not currently supported by the kernel:
- **Architectures other than x86_64**
  - No ARM or AArch64 target
  - No RISC-V target
  - No 32-bit x86 target
  - No PowerPC, MIPS, or other architecture target
  - The build defaults to `x86_64-unknown-none`.
- **Non-x86_64 boot paths**
  - Boot assembly, linker scripts, Multiboot handling, GDT setup, APIC setup, paging assumptions, and user-mode transitions are all x86_64-specific.
- **Windows development hosts**
  - The README explicitly says Windows is not fully supported.
- **Unsupported Linux distributions/package managers**
  - `scripts/setup-dev.sh` only handles apt, pacman, dnf, and Homebrew.
- **Bare-metal hardware outside the tested environment**
  - The kernel is primarily configured around QEMU, VirtIO networking, Multiboot/GRUB, and x86_64 PC hardware.
- **General-purpose physical networking hardware**
  - The active network driver targets VirtIO; other NICs are not generally supported.
- **Persistent storage as a normal filesystem**
  - ATA support is marked as unused, and the active filesystem is RAM-backed.
- **MMIO PCI devices**
  - PCI I/O BAR handling exists, but MMIO support is incomplete.
- **General PCI driver support**
  - PCI devices can be discovered, but only drivers registered with usable probe callbacks can be bound; broad hardware support is not present.
- **IPv6 networking**
  - The networking code is configured around IPv4.
- **General-purpose application compatibility**
  - Linux, POSIX, Unix, or standard C application compatibility is not supported. Programs must use the nlibc libary for support.
- **Directory enumeration**
  - Userspace has no directory-listing syscall yet.
- **Socket syscalls**
  - Socket syscall identifiers exist, but the dispatcher does not currently implement the socket operations.
- **Sleep syscall**
  - The `nap` syscall is currently a placeholder and does not sleep.
- **Process waiting**
  - The wait syscall is currently incomplete and does not implement normal process waiting.
- **Advanced keyboard/scancode sets**
  - The implementation is focused on the current PS/2/scancode path and has an unreachable fallback that calls `unimplemented!()`.

### Windows Support
Native Windows is not currently supported.
However, WSL2 with a supported Linux distribution
can be used for development.

### Known Limitations
As I've stated many times, Nullex is under active development and should be considered experimental. The following limitations are known and show the current state of the project:
- **Supported architecture:** Nullex currently targets **x86_64 only**. ARM, AArch64, RISC-V, x86 32-bit, and other architectures are unsupported.
- **Supported environment:** The kernel is primarily tested with **QEMU, GRUB/Multiboot, x86_64 hardware, and VirtIO devices**. Other bootloaders, hypervisors, and hardware may not work.
- **Development host:** Linux is the primary supported development environment. Windows is not fully supported, and the setup script only supports selected package managers.
- **Rust toolchain:** Building requires the **nightly Rust toolchain** and `rust-src`.
- **Networking:** Networking is limited to the current VirtIO, Ethernet, ARP, IPv4, TCP, UDP, DNS, HTTP, and QEMU TAP setup. It requires privileged host configuration and is not a general-purpose networking implementation.
- **IPv6:** IPv6 is not currently supported.
- **DNS:** DNS resolution uses a fixed IPv4 resolver and does not currently provide DNSSEC, encrypted DNS, configurable resolver selection, or comprehensive DNS record support.
- **HTTPS:** HTTPS is experimental. TLS currently uses `UnsecureProvider`, so server certificate validation and trusted-root verification are not implemented. HTTPS must not be considered suitable for sensitive data.
- **HTTP client:** HTTP/HTTPS downloads currently support a limited request flow, primarily `GET` requests. Protocol coverage and response handling are incomplete.
- **HTTPS downgrade:** Redirect handling may follow redirects from HTTPS back to HTTP. Confidentiality is therefore not guaranteed unless the requested URL and all redirects remain HTTPS.
- **Filesystem:** The active filesystem is an in-memory RAM filesystem. Data is not persistent across reboots.
- **Storage hardware:** ATA support is present only as preliminary code and is not currently used as the main filesystem backend.
- **PCI devices:** PCI discovery exists, but broad hardware driver support is not implemented. MMIO PCI support is incomplete.
- **Userspace programs:** Userspace applications must be statically compiled ELF programs targeting Nullex’s custom userspace interface. Linux, POSIX, and general Unix application compatibility is not provided.
- **System calls:** The syscall ABI is experimental. Some operations are incomplete, including process waiting, sleeping, and socket-related syscalls.
- **Directory operations:** Userspace does not currently have a directory-enumeration syscall.
- **Process execution:** ELF loading, address-space management, scheduling, and user/kernel transitions are still under active development.
- **Memory safety boundaries:** User-provided syscall pointers are not yet protected by a complete copy-in/copy-out and validation layer. Malformed userspace input may cause kernel faults.
- **Keyboard support:** Keyboard handling is currently focused on the implemented PS/2/scancode path. Additional keyboard devices and scancode sets may be unsupported.
- **Testing:** Automated testing is limited. The test suite is not comprehensive, and CI currently focuses primarily on Clippy analysis.
- **Stability:** APIs, syscall numbers, filesystem behavior, networking behavior, and userspace interfaces may change without backward compatibility guarantees.

### Contributing
Contributions are welcome! Please check out the [CONTRIBUTING.md](https://github.com/Peggun/nullex/blob/master/CONTRIBUTING.md) for details on the code of conduct, and the process for submitting pull requests.

### License
This project is licensed under the MIT License – see the [LICENSE](https://github.com/Peggun/nullex/blob/master/LICENSE) file for details.

### Contact
For questions, suggestions, or contributions, please open an issue in this repository.