VERSION=1.0

run:
	cargo run -- -drive format=raw,file=ext2test.img,index=1,media=disk,if=ide -serial mon:stdio

clean:
	cargo clean