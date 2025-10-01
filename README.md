# Nullex Kernel
Small, modular hobby kernel written in Rust — designed to be easy to extend and learn from.

## Getting Started

### Prerequisites

You can install all prerequisites through the setup-dev scripts for both Windows, and Linux / Unix.
Just run the script and it will install everything for you. 
If something doesnt work. Please setup a GitHub Issue so that I can fix it as fast as possible.

#### Building
You can build the project through running
```sh
make build
```
Which will build everything and output a .bin file within:
target/x86_64-unknown-none/debug/bootimage-nullex.bin

#### Testing
To run the test suite (which isnt really used currently) you can run:
```bash
cargo test
```

#### Running
Run the QEMU Emulator:
```bash
cargo run -- -drive format=raw,file=ext2test.img,index=1,media=disk,if=ide -serial mon:stdio
```
or (recommended)
```bash
make run
```

### Contributing
Contributions are welcome! Please check out our [CONTRIBUTING.md](https://github.com/Peggun/nullex/blob/master/CONTRIBUTING.md) for details on our code of conduct, and the process for submitting pull requests.

### License
This project is licensed under the MIT License – see the [LICENSE](https://github.com/Peggun/nullex/blob/master/LICENSE) file for details.

### Contact
For questions, suggestions, or contributions, please open an issue in this repository.
