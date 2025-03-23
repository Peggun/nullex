ifeq ($(OS),Windows_NT) 
RM = del /q
DISK_IMAGE_CMD = powershell -Command "$$bytes = [System.IO.File]::ReadAllBytes('src/userspace/userprog.bin'); [System.IO.File]::WriteAllBytes('ext2test.img', $$bytes)"
ifdef ComSpec
SHELL := $(ComSpec)
endif
ifdef COMSPEC
SHELL := $(COMSPEC)
endif
else
RM = rm -rf
DISK_IMAGE_CMD = cp src/userspace/userprog.bin ext2test.img
endif

userspace:
	cargo build --manifest-path src/userspace/Cargo.toml --release --target x86_64-unknown-none.json --bin userprog
	llvm-objcopy -O binary target\x86_64-unknown-none\release\userprog src\userspace\userprog.bin

disk_image: userspace
	$(DISK_IMAGE_CMD)

run: disk_image
	cargo run -p kernel -- -drive format=raw,file=ext2test.img,index=1,media=disk,if=ide -serial mon:stdio

debug: disk_image
	cargo run -p kernel -- -drive format=raw,file=ext2test.img,index=1,media=disk,if=ide -serial mon:stdio -s -S

full-debug: disk_image
	make clean
	cargo build
	$(RM) output.txt
	objdump -D target\x86_64-unknown-none\debug\kernel >> output.txt
	cargo run -p kernel -- -drive format=raw,file=ext2test.img,index=1,media=disk,if=ide -serial mon:stdio -s -S -d int,cpu_reset,guest_errors -D debug.log

clean:
	cargo clean
	cargo clean --manifest-path src/kernel/Cargo.toml
	cargo clean --manifest-path src/userspace/Cargo.toml
	cargo clean --manifest-path src/orchestrator/Cargo.toml
	$(RM) src\userspace\userprog.bin output.txt debug.log