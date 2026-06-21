# ---------------------------
# Arch-specific configuration
# ---------------------------

ARCH ?= x86_64

ifeq ($(ARCH),x86_64)
    TARGET := x86_64-unknown-none
    CC := x86_64-linux-gnu-gcc
    AR := ar
    LD := ld
    QEMU_SYSTEM := qemu-system-x86_64

    CFLAGS := -m64 -march=x86-64 -O2 -pipe -ffreestanding -fno-builtin \
              -fno-stack-protector -fno-common -fno-pie -nostdlib -nostartfiles \
              -static -e _start -Wl,--entry=_start

    USR_CFLAGS := -m64 -march=x86-64 -O2 -pipe -fno-stack-protector \
                  -fno-common -fPIE -ffreestanding -fno-builtin \
                  -Iprograms/include

    QEMU_RUN_ARGS = \
        -cdrom $(iso) \
        -serial stdio \
        -monitor vc \
        -machine q35 \
        -netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
        -device virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56,vectors=3,csum=off,guest_csum=off,guest_tso4=off,guest_tso6=off,guest_ecn=off,guest_ufo=off \
        -rtc base=localtime \
        -cpu qemu64,+rdrand \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04

    QEMU_DEBUG_ARGS = \
        -S -s \
        -cdrom $(iso) \
        -serial stdio \
        -monitor vc \
        -machine q35 \
        -netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
        -device virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56,vectors=3 \
        -rtc base=localtime \
        -cpu qemu64,+rdrand \
        -device isa-debug-exit,iobase=0xf4,iosize=0x04

    ASM_EXT := asm
    ASM_BUILD_CMD = nasm -felf64

else ifeq ($(ARCH),aarch64)
    TARGET := aarch64-unknown-none-softfloat
    CC := aarch64-linux-gnu-gcc
    AR := aarch64-linux-gnu-ar
    LD := aarch64-linux-gnu-ld
    QEMU_SYSTEM := qemu-system-aarch64

    CFLAGS := -O2 -pipe -ffreestanding -fno-builtin -fno-stack-protector \
              -fno-common -fno-pie -nostdlib -nostartfiles -static \
              -march=armv8-a

    USR_CFLAGS := -O2 -pipe -fno-stack-protector -fno-common -fPIE \
                  -ffreestanding -fno-builtin -march=armv8-a \
                  -Iprograms/include

    QEMU_RUN_ARGS = \
        -M raspi3b \
        -cpu cortex-a53 \
        -kernel $(kernel) \
        -serial stdio \
        -display none \
        -no-reboot

    QEMU_DEBUG_ARGS = \
        -M raspi3b \
        -cpu cortex-a53 \
        -S -s \
        -kernel $(kernel) \
        -serial stdio \
        -display none \
        -no-reboot

    ASM_EXT := S
    ASM_BUILD_CMD = $(CC) -c -m64 -masm=intel

else
    $(error Unsupported ARCH '$(ARCH)')
endif

# ---------------------------
# Cargo target selection
# ---------------------------

TARGET_JSON := targets/$(TARGET).json
TARGET_IS_JSON := $(wildcard $(TARGET_JSON))
TARGET_STEM := $(basename $(notdir $(if $(TARGET_IS_JSON),$(TARGET_JSON),$(TARGET))))

ifeq ($(TARGET_IS_JSON),)
  CARGO_CMD := cargo
  CARGO_TARGET_FLAGS := --target $(TARGET) $(CARGO_FLAGS)
else
  CARGO_CMD := cargo +nightly
  CARGO_TARGET_FLAGS := --target $(abspath $(TARGET_JSON))
endif

# ---------------------------
# Paths and outputs
# ---------------------------

kernel := build/kernel-$(ARCH).bin
iso := build/os-$(ARCH).iso
rust_os := target/$(TARGET_STEM)/debug/libnullex.a

ARCH_DIR := src/arch/$(ARCH)
linker_script := $(ARCH_DIR)/linker.ld
grub_cfg := $(ARCH_DIR)/grub.cfg

assembly_source_files := $(wildcard $(ARCH_DIR)/*.$(ASM_EXT))
assembly_object_files := $(patsubst $(ARCH_DIR)/%.$(ASM_EXT),build/arch/$(ARCH)/%.o,$(assembly_source_files))

# ---------------------------
# Userspace
# ---------------------------

LIBNULLEX_SRCS := $(wildcard programs/lib/*.c)
LIBNULLEX_OBJS := $(patsubst programs/lib/%.c,build/userspace/lib/%.o,$(LIBNULLEX_SRCS))
LIBNULLEX_OUT  := build/userspace/libnullex.a

USR_LDFLAGS ?= -nostdlib -lgcc -static -L$(abspath build/userspace) -lnullex
USR_LINKER_SCRIPT ?=
USR_CRT0 := programs/_start.c

PROG_SRCS := $(shell find programs -type f -name '*.c' ! -name '_start.c' 2>/dev/null)
PROGRAM_MAKEFILES := $(shell find programs -mindepth 2 -maxdepth 2 -type f -name Makefile 2>/dev/null)
PROGRAM_DIRS := $(sort $(patsubst %/Makefile,%,$(PROGRAM_MAKEFILES)))
PROGS := $(patsubst programs/%,build/userspace/%.elf,$(PROGRAM_DIRS))

# ---------------------------
# Global
# ---------------------------

CI ?= false

.PHONY: all clean clean-all clean-progs run debug iso kernel build test test-ci miri userspace libnullex

all: $(kernel)

build: $(if $(filter x86_64,$(ARCH)),$(iso),$(kernel))

# ---------------------------
# Cargo kernel build
# ---------------------------

kernel: userspace
	@echo "Building kernel with Cargo..."
	@$(CARGO_CMD) build $(CARGO_TARGET_FLAGS)

# ---------------------------
# Link final kernel
# ---------------------------

$(kernel): userspace kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@echo "Linking kernel..."
	@mkdir -p $(@D)
	@$(LD) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) \
		--whole-archive $(rust_os) --no-whole-archive

# ---------------------------
# ISO image (x86_64 only)
# ---------------------------

ifeq ($(ARCH),x86_64)
iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@echo "Creating ISO image..."
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles
else
iso:
	@echo "ISO images are only supported for ARCH=x86_64"
	@false
endif

# ---------------------------
# QEMU run/debug
# ---------------------------

ifeq ($(ARCH),x86_64)
run: $(iso)
	@echo "Starting QEMU..."
	@sudo $(QEMU_SYSTEM) $(QEMU_RUN_ARGS)

debug: $(iso)
	@echo "Starting QEMU in debug mode..."
	@sudo $(QEMU_SYSTEM) $(QEMU_DEBUG_ARGS)
else ifeq ($(ARCH),aarch64)
run: $(kernel)
	@echo "Starting QEMU for AArch64..."
	@sudo $(QEMU_SYSTEM) $(QEMU_RUN_ARGS)

debug: $(kernel)
	@echo "Starting QEMU in debug mode for AArch64..."
	@sudo $(QEMU_SYSTEM) $(QEMU_DEBUG_ARGS)
endif

# ---------------------------
# Assembly
# ---------------------------

ifeq ($(ARCH),x86_64)
build/arch/$(ARCH)/%.o: $(ARCH_DIR)/%.$(ASM_EXT)
	@echo "Compiling assembly file $<..."
	@mkdir -p $(@D)
	@nasm -felf64 $< -o $@
else ifeq ($(ARCH),aarch64)
build/arch/$(ARCH)/%.o: $(ARCH_DIR)/%.$(ASM_EXT)
	@echo "Compiling assembly file $<..."
	@mkdir -p $(@D)
	@$(CC) -c $< -o $@
endif

# ---------------------------
# libnullex userspace library
# ---------------------------

libnullex: $(LIBNULLEX_OUT)

build/userspace/lib/%.o: programs/lib/%.c $(wildcard programs/include/*.h)
	@echo "Compiling libnullex: $<"
	@mkdir -p $(@D)
	@$(CC) $(USR_CFLAGS) -c $< -o $@

$(LIBNULLEX_OUT): $(LIBNULLEX_OBJS)
	@echo "Archiving libnullex -> $@"
	@mkdir -p $(@D)
	@$(AR) rcs $@ $^

# ---------------------------
# Userspace build
# ---------------------------

define program_sources
$(shell find $(1) -type f \( -name '*.c' -o -name '*.h' \) 2>/dev/null) programs/_start.c $(wildcard programs/include/*.h) $(wildcard programs/lib/*.c) $(wildcard programs/lib/*.h)
endef

userspace: $(LIBNULLEX_OUT) $(PROGS)
	@echo "Userspace programs built: $(words $(PROGS))"

define build_userspace_rule
build/userspace/$(notdir $(1)).elf: $(1)/Makefile $(LIBNULLEX_OUT) $$(call program_sources,$(1))
	@echo "Building userspace program: $(notdir $(1))"
	@mkdir -p $$(@D)
	@$(MAKE) -C $(1) OUT="$$(abspath $$@)" CC="$(CC)" AR="$(AR)" CFLAGS="$(USR_CFLAGS)" LDFLAGS="$(USR_LDFLAGS)" ARCH="$(ARCH)"
endef

$(foreach d,$(PROGRAM_DIRS),$(eval $(call build_userspace_rule,$(d))))

clean:
	@echo "Cleaning build directory..."
	@rm -rf build
	@cargo clean

clean-progs:
	@echo "Cleaning userspace programs..."
	@for dir in $(PROGRAM_DIRS); do \
		$(MAKE) -C $$dir clean; \
	done

clean-all: clean clean-progs
	@echo "Finished cleaning all build artifacts."
