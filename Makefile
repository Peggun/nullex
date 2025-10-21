VERSION := 1.0

# --- OS-specific configuration ---
ifeq ($(OS), Windows_NT)
	RM := rmdir /s /q
	CP := copy
	UEFI := False
else
	UNAME_S := $(shell uname -s)
	ifeq ($(UNAME_S), Linux)
		RM := rm -rf
		CP := cp -a
		UEFI := True
	else
		RM := rm -rf
		CP := cp -a
		UEFI := False
	endif
endif

# --- Build variables ---
arch ?= x86_64
kernel := build/kernel-$(arch).bin
KERNEL_BASENAME := $(notdir $(kernel))
iso := build/nullex-$(arch).iso
kernel_iso := build/nullex-kernel-$(arch).iso
uefi_iso := build/nullex-uefi-$(arch).iso

# directories for temporary ISO assembly
ISO_TMP := build/isofiles
KERNEL_ISO_TMP := build/kernel_isofiles
UEFI_ISO_TMP := build/uefi_isofiles

target ?= $(arch)-unknown-none
rust_os := target/$(target)/debug/libnullex.a

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

# UEFI-specific variables
WORKSPACE = $(HOME)/edk2
PACKAGES_PATH = $(HOME)/nullex:$(WORKSPACE)/edk2:$(WORKSPACE)/edk2-platforms
EDK_TOOLS_PATH = $(WORKSPACE)/edk2/BaseTools

# output directory for the UEFI FAT image (no trailing slash)
UEFI_OUTPUT_DIR := build/uefi

# Find built artifacts produced by edk2 (first match)
UEFI_EFI_FILE := $(WORKSPACE)/Build/OvmfX64/RELEASE_GCC5/X64/NullexUefi.efi
UEFI_DLL_FILE := $(shell find $(WORKSPACE)/Build -type f -iname 'NullexUefi.dll' -print -quit)

# your OVMF files (absolute paths)
OVMF_CODE := $(HOME)/nullex/uefi/OVMF/OVMF_CODE.fd
OVMF_VARS := $(HOME)/nullex/uefi/OVMF/OVMF_VARS.fd

# --- Safety checks at parse time ---
ifeq ($(UEFI_EFI_FILE),)
UEFI_EFI_FILE_NOT_FOUND := 1
endif

# --- Phony targets ---
.PHONY: all clean clean-cargo clean-kernel run run-kernel run-uefi run-uefi-kernel \
	iso kernel uefi uefi-kernel build-uefi run-uefi-debug \
	make-kernel make-uefi make-kernel-uefi kernel-iso uefi-iso build-bin verify-tools

# --- Convenience aliases (explicit names requested) ---
make-kernel: kernel
	@echo "alias: make-kernel -> kernel"

make-uefi: uefi
	@echo "alias: make-uefi -> uefi"

# alias combining kernel + uefi build
make-kernel-uefi: uefi-kernel
	@echo "alias: make-kernel-uefi -> uefi-kernel"

# --- Helper target: ensure required host tools exist ---
verify-tools:
	@command -v qemu-system-x86_64 >/dev/null 2>&1 || (echo "qemu-system-x86_64 not found in PATH" && exit 1)
	@command -v grub-mkrescue >/dev/null 2>&1 || (echo "grub-mkrescue not found in PATH; install grub2-common or grub2-tools" && exit 1)

# --- Target rules ---
# default: build kernel binary + a kernel-only ISO
all: kernel iso

clean:
	@$(RM) build

clean-cargo:
	@cargo clean
	@$(RM) uefi/build

clean-kernel:
	@cargo clean

# Run the kernel with BIOS/GRUB boot (uses kernel-only ISO)
run: run-kernel

run-kernel: $(kernel_iso)
	@qemu-system-x86_64 -cdrom $(kernel_iso) -serial mon:stdio

# Build only the kernel (cargo + link)
# kernel now only builds the .bin. Use 'make kernel-iso' to produce a kernel-only ISO.
kernel: build-bin

# a lightweight target to only produce the .bin without ISO
build-bin: $(kernel)

$(kernel): $(rust_os) $(assembly_object_files) $(linker_script)
	@mkdir -p $(dir $(kernel))
	@echo "Linking kernel -> $(kernel)"
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) \
	    $(assembly_object_files) $(rust_os)

# Create a kernel-only ISO (grub-based) that boots from CD-ROM using grub-mkrescue
kernel-iso: verify-tools $(kernel) $(grub_cfg)
	@echo "Creating kernel-only ISO: $(kernel_iso)"
	@mkdir -p $(KERNEL_ISO_TMP)/boot/grub
	@$(CP) $(kernel) $(KERNEL_ISO_TMP)/boot/$(KERNEL_BASENAME)
	@$(CP) $(grub_cfg) $(KERNEL_ISO_TMP)/boot/grub/grub.cfg
	@grub-mkrescue -o $(kernel_iso) $(KERNEL_ISO_TMP) 2> /dev/null || (echo "grub-mkrescue failed" && exit 1)
	@$(RM) $(KERNEL_ISO_TMP)

# Build only UEFI (EDK2). This target will also create a UEFI ISO (uefi_iso)
uefi:
ifeq ($(UEFI), True)
	@$(MAKE) build-uefi
	@$(MAKE) uefi-iso
else
	@echo "UEFI build / run targets are only supported on Linux in this Makefile." && exit 1
endif

# Build both UEFI and kernel and produce a single UEFI ISO that contains EFI and boot/kernel.bin
uefi-kernel:
ifeq ($(UEFI), True)
	@$(MAKE) build-uefi
	@$(MAKE) build-bin
	@$(MAKE) uefi-iso
else
	@echo "UEFI build / run targets are only supported on Linux in this Makefile." && exit 1
endif

# Create an ISO that contains the UEFI application and the kernel under /boot/ so it can be
# booted as a UEFI CD-ROM image (still requires OVMF to provide firmware when running qemu).
# This single ISO is what 'make uefi-kernel' and 'make uefi' will produce and what
# 'run-uefi-kernel' will boot from.
uefi-iso: verify-tools $(UEFI_EFI_FILE) $(kernel)
	@echo "Creating UEFI ISO: $(uefi_iso)"
	@mkdir -p $(UEFI_ISO_TMP)/EFI/BOOT
	@mkdir -p $(UEFI_ISO_TMP)/boot/grub
	@$(CP) "$(UEFI_EFI_FILE)" $(UEFI_ISO_TMP)/EFI/BOOT/BOOTX64.EFI
	@$(CP) $(kernel) $(UEFI_ISO_TMP)/boot/$(KERNEL_BASENAME)
	# Optionally include a grub cfg for BIOS fallback; keep minimal
	@if [ -f $(grub_cfg) ]; then \
		$(CP) $(grub_cfg) $(UEFI_ISO_TMP)/boot/grub/grub.cfg; \
	fi
	@grub-mkrescue -o $(uefi_iso) $(UEFI_ISO_TMP) 2> /dev/null || (echo "grub-mkrescue failed" && exit 1)
	@$(RM) $(UEFI_ISO_TMP)

# Run UEFI only (uses the generated uefi_iso, boots OVMF and mounts the ISO as CD-ROM)
run-uefi: uefi-iso
	@mkdir -p $(UEFI_OUTPUT_DIR)
	@if [ ! -f $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd ]; then \
	    $(CP) $(OVMF_VARS) $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd || true; \
	fi
	@qemu-system-x86_64 \
	    -m 1024 \
	    -drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
	    -drive if=pflash,format=raw,file=$(UEFI_OUTPUT_DIR)/OVMF_VARS.fd \
	    -cdrom $(uefi_iso) \
	    -serial mon:stdio \
	    -no-reboot

# Build UEFI and kernel, then run the kernel from the single UEFI ISO
run-uefi-kernel: uefi-kernel
	@mkdir -p $(UEFI_OUTPUT_DIR)
	@if [ ! -f $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd ]; then \
	    $(CP) $(OVMF_VARS) $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd || true; \
	fi
	@qemu-system-x86_64 \
	    -m 1024 \
	    -drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
	    -drive if=pflash,format=raw,file=$(UEFI_OUTPUT_DIR)/OVMF_VARS.fd \
	    -cdrom $(uefi_iso) \
	    -serial mon:stdio \
	    -no-reboot

# ISO target for full BIOS/GRUB boot (keeps previous behaviour)
iso: $(iso)

$(iso): kernel $(grub_cfg)
	@echo "Creating full ISO: $(iso)"
	@mkdir -p $(ISO_TMP)/boot/grub
	# Place kernel into /boot/ on the ISO with its full basename (e.g. kernel-x86_64.bin)
	@$(CP) $(kernel) $(ISO_TMP)/boot/$(KERNEL_BASENAME)
	# copy grub config (ensure filename)
	@$(CP) $(grub_cfg) $(ISO_TMP)/boot/grub/grub.cfg
	@grub-mkrescue -o $(iso) $(ISO_TMP) 2> /dev/null || (echo "grub-mkrescue failed" && exit 1)
	@$(RM) $(ISO_TMP)

# Compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

# Rust kernel build
$(rust_os):
	@cargo build

# UEFI build using edk2
build-uefi:
ifeq ($(UEFI), True)
	@echo "Building UEFI with EDK2..."
	@mkdir -p $(WORKSPACE)/Conf
	@export WORKSPACE=$(WORKSPACE) && \
	export PACKAGES_PATH=$(PACKAGES_PATH) && \
	export EDK_TOOLS_PATH=$(EDK_TOOLS_PATH) && \
	export GCC5_FLAGS="-O0 -g -fno-omit-frame-pointer -fno-inline -mno-red-zone" && \
	. $(WORKSPACE)/edk2/edksetup.sh && \
	build -p OvmfPkg/OvmfPkgX64.dsc -a X64 -t GCC5 --buildtarget=RELEASE
else
	@echo "UEFI development is only supported on Linux."
endif

# Debug run: start QEMU paused with gdb server and save qemu log
run-uefi-debug: build-uefi
	@mkdir -p $(UEFI_OUTPUT_DIR)
	@qemu-system-x86_64 \
	    -S -s \
	    -m 1024 \
	    -drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
	    -drive if=pflash,format=raw,file=$(abspath $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd) \
	    -drive file=fat:rw:$(abspath $(UEFI_OUTPUT_DIR)),format=raw \
	    -d int -D $(abspath build/qemu.log) \
	    -serial mon:stdio
