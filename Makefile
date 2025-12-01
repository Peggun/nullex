arch ?= x86_64
kernel := build/kernel-$(arch).bin
iso := build/os-$(arch).iso
target ?= $(arch)-unknown-none
rust_os := target/$(target)/debug/libnullex.a

linker_script := src/arch/$(arch)/linker.ld
grub_cfg := src/arch/$(arch)/grub.cfg
assembly_source_files := $(wildcard src/arch/$(arch)/*.asm)
assembly_object_files := $(patsubst src/arch/$(arch)/%.asm, \
	build/arch/$(arch)/%.o, $(assembly_source_files))

CARGO_FLAGS ?=

.PHONY: all clean run iso kernel build test test-ci

all: $(kernel)

clean:
	@rm -rf build
	@cargo clean

run: $(iso)
	@echo "Running QEMU (CI=$(CI))"; \
	if [ -n "$(CI)" ]; then \
	  mkdir -p build; \
	  qemu-system-x86_64 -cdrom $(iso) -serial file:build/serial.log -net nic -rtc base=localtime -device isa-debug-exit,iobase=0xf4,iosize=0x04 -nographic; \
	else \
	  qemu-system-x86_64 -cdrom $(iso) -serial mon:stdio -net nic -rtc base=localtime -device isa-debug-exit,iobase=0xf4,iosize=0x04; \
	fi; \
	EXIT=$$?; \
	echo "qemu host exit code: $$EXIT"; \
	if [ -z "$$EXIT" ]; then \
	  echo "Warning: QEMU exit code empty; using 1"; EXIT=1; \
	fi; \
	if [ "$$EXIT" -eq 0 ]; then \
	  echo "QEMU exited normally (host=0)."; \
	  if [ -n "$(CI)" ]; then echo "--- serial log (build/serial.log) ---"; cat build/serial.log || true; echo "-----------------------------------"; fi; \
	  exit 0; \
	fi; \
	if [ "$$EXIT" -eq 1 ]; then \
	  echo "QEMU host=1 (maps to guest=0 on many QEMU builds)"; \
	  if [ -n "$(CI)" ]; then echo "--- serial log (build/serial.log) ---"; cat build/serial.log || true; echo "-----------------------------------"; fi; \
	  exit 0; \
	fi; \
	if [ `expr $$EXIT % 2` -eq 1 ]; then \
	  GUEST_EXIT=`expr \( $$EXIT - 1 \) / 2`; \
	  echo "QEMU debug-exit: mapped host $$EXIT -> guest $$GUEST_EXIT"; \
	  if [ -n "$(CI)" ]; then echo "--- serial log (build/serial.log) ---"; cat build/serial.log || true; echo "-----------------------------------"; fi; \
	  exit $$GUEST_EXIT; \
	else \
	  echo "QEMU host exit $$EXIT is even; passing it through as guest code"; \
	  if [ -n "$(CI)" ]; then echo "--- serial log (build/serial.log) ---"; cat build/serial.log || true; echo "-----------------------------------"; fi; \
	  exit $$EXIT; \
	fi

debug: $(iso)
	qemu-system-x86_64 -cdrom $(iso) -serial mon:stdio -net nic -device isa-debug-exit,iobase=0xf4,iosize=0x04 -D ./qemu.log -d int

build: $(iso)

test:
	@$(MAKE) run CARGO_FLAGS="--features test"

test-ci:
	@$(MAKE) run CARGO_FLAGS="--features test" CI=1

iso: $(iso)

$(iso): $(kernel) $(grub_cfg)
	@mkdir -p build/isofiles/boot/grub
	@cp $(kernel) build/isofiles/boot/kernel.bin
	@cp $(grub_cfg) build/isofiles/boot/grub
	@grub-mkrescue -o $(iso) build/isofiles 2> /dev/null
	@rm -r build/isofiles

$(kernel): kernel $(rust_os) $(assembly_object_files) $(linker_script)
	@ld -n --gc-sections -T $(linker_script) -o $(kernel) \
		$(assembly_object_files) $(rust_os)

kernel:
	@cargo build --target $(target) $(CARGO_FLAGS)

# compile assembly files
build/arch/$(arch)/%.o: src/arch/$(arch)/%.asm
	@mkdir -p $(shell dirname $@)
	@nasm -felf64 $< -o $@
