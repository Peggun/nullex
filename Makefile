# ---------------------------
# Kernel Makefile
# ---------------------------

TARGET := x86_64-unknown-none
CC := x86_64-linux-gnu-gcc
AR := ar
LD := ld
QEMU_SYSTEM := qemu-system-x86_64

CFLAGS := -m64 -march=x86-64 -O2 -pipe -ffreestanding -fno-builtin \
          -fno-stack-protector -fno-common -fno-pie -nostdlib -nostartfiles \
          -static -e _start -Wl,--entry=_start

USR_CFLAGS := -m64 -march=x86-64 -O2 -pipe -fno-stack-protector \
              -fno-common -fno-pie -ffreestanding -fno-builtin \
              -Iprograms/include

QEMU_RUN_ARGS = \
    -cdrom $(iso) \
    -serial stdio \
    -monitor vc \
    -machine q35 \
    -netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
	-device virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56,vectors=3,csum=off,guest_csum=off,guest_tso4=off,guest_tso6=off,guest_ecn=off,guest_ufo=off \
    -rtc base=localtime \
    -cpu max,+rdrand,smep=on,smap=on \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \

QEMU_DEBUG_ARGS = \
    -S -s \
    -cdrom $(iso) \
    -serial stdio \
    -monitor vc \
    -machine q35 \
    -netdev tap,id=net0,ifname=tap0,script=no,downscript=no \
	-device virtio-net-pci,netdev=net0,mac=52:54:00:12:34:56,vectors=3,csum=off,guest_csum=off,guest_tso4=off,guest_tso6=off,guest_ecn=off,guest_ufo=off \
    -rtc base=localtime \
    -cpu max,+rdrand,smep=on,smap=on \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04

ASM_EXT := asm
ASM_BUILD_CMD = nasm -felf64

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

CARGO_FEATURES ?=

# ---------------------------
# Paths and outputs
# ---------------------------

kernel := build/kernel-x86_64.bin
iso := build/os-x86_64.iso
rust_os := target/$(TARGET_STEM)/debug/libnullex.a

ARCH_DIR := src/arch/x86_64
linker_script := $(ARCH_DIR)/linker.ld
grub_cfg := $(ARCH_DIR)/grub.cfg

assembly_source_files := $(wildcard $(ARCH_DIR)/*.$(ASM_EXT))
assembly_object_files := $(patsubst $(ARCH_DIR)/%.$(ASM_EXT),build/arch/x86_64/%.o,$(assembly_source_files))

NDM_GENERATOR := tools/generate_ndm.py
NDM := build/nullex.ndm
STAGE1_KERNEL := build/kernel-x86_64.stage1.bin

# ---------------------------
# Userspace
# ---------------------------

LIBNULLEX_SRCS := $(wildcard programs/lib/*.c)
LIBNULLEX_OBJS := $(patsubst programs/lib/%.c,build/userspace/lib/%.o,$(LIBNULLEX_SRCS))
LIBNULLEX_OUT  := build/userspace/libnullex.a

USR_LDFLAGS ?= -nostdlib -static -no-pie -Wl,-T,$(abspath $(USR_LINKER_SCRIPT))
USR_LINKER_SCRIPT ?= programs/link.ld
USR_LDLIBS ?= -lgcc
USR_CRT0 := programs/_start.c

PROG_SRCS := $(shell find programs -type f -name '*.c' ! -name '_start.c' 2>/dev/null)
PROGRAM_MAKEFILES := $(shell find programs -mindepth 2 -maxdepth 2 -type f -name Makefile 2>/dev/null)
PROGRAM_DIRS := $(sort $(patsubst %/Makefile,%,$(PROGRAM_MAKEFILES)))
PROGS := $(patsubst programs/%,build/userspace/%.elf,$(PROGRAM_DIRS))

# ---------------------------
# Global
# ---------------------------

CI ?= false

.PHONY: all clean clean-all clean-progs run debug iso kernel build test test-ci miri userspace libnullex compdb

all: $(kernel)

fmt:
	@cargo fmt --all
	@find . -name "*.c" -o -name "*.h" | xargs clang-format -i --style=Google

build: $(iso)

kernel: userspace
	@echo "Building kernel with Cargo..."
	@mkdir -p build
	@touch $(NDM)
	@$(CARGO_CMD) build $(CARGO_TARGET_FLAGS)

$(kernel): userspace $(assembly_object_files) $(linker_script) $(NDM_GENERATOR)
	@echo "Building stage 1 kernel..."
	@mkdir -p $(@D)

	@touch $(NDM)

	@$(CARGO_CMD) build $(CARGO_TARGET_FLAGS) $(CARGO_FEATURES)

	@$(LD) -n --gc-sections -T $(linker_script) -o $(STAGE1_KERNEL) \
		$(assembly_object_files) \
		--whole-archive $(rust_os) --no-whole-archive

	@echo "Generating Nullex debug map..."
	@python3 $(NDM_GENERATOR) $(STAGE1_KERNEL) $(NDM)

	@echo "Rebuilding kernel with NDM..."
	@$(CARGO_CMD) build $(CARGO_TARGET_FLAGS) $(CARGO_FEATURES)

	@echo "Linking final kernel..."
	@$(LD) -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) \
		--whole-archive $(rust_os) --no-whole-archive

	@rm -f $(STAGE1_KERNEL)

# ---------------------------
# ISO image
# ---------------------------

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@echo "Creating ISO image..."
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

# ---------------------------
# QEMU run/debug
# ---------------------------

run: $(iso)
	@echo "Starting QEMU..."
	@sudo $(QEMU_SYSTEM) $(QEMU_RUN_ARGS)

debug: $(iso)
	@echo "Starting QEMU in debug mode..."
	@sudo $(QEMU_SYSTEM) $(QEMU_DEBUG_ARGS)

# ---------------------------
# Assembly
# ---------------------------

build/arch/x86_64/%.o: $(ARCH_DIR)/%.$(ASM_EXT)
	@echo "Compiling assembly file $<..."
	@mkdir -p $(@D)
	@nasm -felf64 $< -o $@

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
	@$(MAKE) -C $(1) OUT="$$(abspath $$@)" CC="$(CC)" AR="$(AR)" CFLAGS="$(USR_CFLAGS)" LDFLAGS="$(USR_LDFLAGS)" LDLIBS="$(USR_LDLIBS)" ARCH="x86_64"
endef

$(foreach d,$(PROGRAM_DIRS),$(eval $(call build_userspace_rule,$(d))))

# ---------------------------
# Compilation database
# ---------------------------

compdb:
	@echo "Generating compile_commands.json..."
	@rm -f compile_commands.json
	@bear -- make -B userspace
	@echo "compile_commands.json generated."

# ---------------------------
# Cleanup
# ---------------------------

clean:
	@echo "Cleaning kernel build directory..."
	@rm -rf build
	@echo "Cleaning Rust/Cargo artifacts..."
	@cargo clean
	@echo "Cleaning all userspace program build directories..."
	@find programs -type d -name "build" -exec rm -rf {} + 2>/dev/null || true
	@find programs -type f \( -name "*.o" -o -name "*.d" -o -name "*.elf" \) -delete 2>/dev/null || true
	@echo "Finished cleaning all build artifacts."

clean-progs:
	@echo "Cleaning userspace programs via sub-makefiles..."
	@for dir in $(PROGRAM_DIRS); do \
		$(MAKE) -C $$dir clean; \
	done

clean-all: clean
	@echo "Finished comprehensive clean."

test:
	@echo "Building test kernel..."
	@$(MAKE) -B build CARGO_FEATURES="--features test"
	@echo "Running tests..."
	@sudo $(QEMU_SYSTEM) $(QEMU_RUN_ARGS)
	@echo "Tests completed."