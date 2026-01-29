# Nullex Kernel
Small, modular hobby kernel written in Rust — designed to be easy to extend and learn from.

This project uses code segments from third party crates. All information can be found [here](https://github.com/Peggun/nullex/blob/master/THIRD_PARTY_LICENSES.md). Please let me know if there is an issue with this, so I can update accordingly.

## Getting Started

### Prerequisites

You can install all prerequisites through the setup-dev scripts for Linux / Unix. Windows is not supported fully currently unfortuantely.
Just run the script and it will install everything for you. 
If something doesnt work. Please setup a GitHub Issue so that I can fix it as fast as possible.

#### Building
You can build the project through running:
```sh
make build
```
Which will build everything and output a `.bin` and `.iso` file within:
`build/os-x86_64.iso`
and
`build/kernel-x86_64.bin`

#### Testing
To run the test suite (which isnt really used currently) you can run:
```bash
cargo test
```

#### Running
Currently the nullex kernel does support networking, however very small ping and resolving. 
To be able to set this up, we need to make our HOST operating system (Linux) to allow for this
TAP network from QEMU to be setup to allow Virtio-Net to access the World Wide Web.
You can easily do this by running the `setup-vn.sh` file inside of the scripts directory.
```bash
sudo ./scripts/setup-vn.sh 
```

Run the QEMU Emulator:
```bash
make run
```

### Contributing
Contributions are welcome! Please check out the [CONTRIBUTING.md](https://github.com/Peggun/nullex/blob/master/CONTRIBUTING.md) for details on the code of conduct, and the process for submitting pull requests.

### License
This project is licensed under the MIT License – see the [LICENSE](https://github.com/Peggun/nullex/blob/master/LICENSE) file for details.

### Contact
For questions, suggestions, or contributions, please open an issue in this repository.