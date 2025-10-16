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
iso := build/nullex-$(arch).iso
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
.PHONY: all clean clean-cargo clean-kernel run run-iso run-uefi iso kernel build-uefi run-uefi-debug

# --- Target rules ---

all: $(kernel)

clean:
	@$(RM) build

clean-cargo:
	@cargo clean
	@$(RM) uefi/build

clean-kernel:
	@cargo clean

run: $(iso)
	@qemu-system-x86_64 -cdrom $(iso) -serial mon:stdio

run-all: $(iso)

# Run UEFI: create a writable vars copy in build/uefi and explicitly set format=raw for the fat drive
run-uefi:
	@mkdir -p $(UEFI_OUTPUT_DIR)
	@if [ ! -f $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd ]; then \
		cp -a $(OVMF_VARS) $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd || true; \
	fi
	@qemu-system-x86_64 \
		-m 1024 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(OVMF_VARS) \
		-drive format=raw,file=fat:rw:build/uefi
		-serial mon:stdio \
		-no-reboot

# ISO target
iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@$(CP) $(kernel) build/isofiles/boot/kernel.bin
	@$(CP) $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@$(RM) build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

kernel:
	@cargo build

# Compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@

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

	@mkdir $(UEFI_OUTPUT_DIR)/EFI/BOOT/
	@$(CP) "$(UEFI_EFI_FILE)" "$(UEFI_OUTPUT_DIR)/EFI/BOOT/BOOTX64.EFI"
else
	@echo "UEFI development is only supported on Linux."
endif

# Debug run: start QEMU paused with gdb server and save qemu log
run-uefi-debug:
	@mkdir -p $(UEFI_OUTPUT_DIR)
	@qemu-system-x86_64 \
		-S -s \
		-m 1024 \
		-drive if=pflash,format=raw,readonly=on,file=$(OVMF_CODE) \
		-drive if=pflash,format=raw,file=$(abspath $(UEFI_OUTPUT_DIR)/OVMF_VARS.fd) \
		-drive file=fat:rw:$(abspath $(UEFI_OUTPUT_DIR)),format=raw \
		-d int -D $(abspath build/qemu.log) \
		-serial mon:stdio
