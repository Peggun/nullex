# Nullex Kernel

Nullex is a kernel written fully in Rust. It is easily extendable and modular, making it easy to implement and make changes to. 
This kernel currently only runs in QEMU, because I have not tested creating a ISO file.

My goal is to create a fully fledged kernel in Rust, similar to Linux and RedosOS, aiming to support multiple different architectures, and other things
and to also get the community involved in having fun in coding something very tedious, like a Kernel / Operating System.

Thanks so much to [Philipp Oppermann's Blog OS Tutorial](https://os.phil-opp.com/) I highly recommend it if you want to get started into OS and Kernel Programming.
This project was started because of him so thanks so much.

Just a quick note: When using `cargo test` no tests are found. If you want to test something, please use `cargo test --test test_name`. I will need to fix this in the future.

## Features

- **Rust-powered:** Leverages Rust’s safety guarantees.
- **Modular design:** Easily extendable and maintainable.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (install the appropriate version; nightly is required)
- [Cargo](https://doc.rust-lang.org/cargo/)
- [QEMU](https://www.qemu.org/download/)
- [CMake](https://cmake.org/download)

### Installation

Clone the repository:

```bash
git clone https://github.com/Peggun/nullex.git
cd nullex
```

#### Install Cargo tools
Install bootimage:
```bash
cargo install bootimage
```

#### Building
Build the project in Release mode
```bash
cargo build
```

#### Testing
Run the test suite:
```bash
cargo test
```

#### Running
Run the QEMU Emulator:
```bash
cargo run
```
or
```bash
make run
```

### Contributing
Contributions are welcome! Please check out our [CONTRIBUTING.md](https://github.com/Peggun/nullex/blob/master/CONTRIBUTING.md) for details on our code of conduct, and the process for submitting pull requests.

### License
This project is licensed under the MIT License – see the [LICENSE](https://github.com/Peggun/nullex/blob/master/LICENSE) file for details.

### Contact
For questions, suggestions, or contributions, please open an issue in this repository.