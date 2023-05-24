# Helium

Helium is a hobby microkernel x86_64 kernel written from scratch in Rust which aims to be
compatible with Linux at binary level (running unmodified Linux binaries).
Helium aims to be simple, readable and well-documented, but relatively fast and efficient.

## Building and running the kernel

The building process can only be done on Linux machines. If you are on Windows, you can use WSL to
achieve the same result.
For running the kernel, simply type `make run` in the root directory of the project. This will build the kernel and run it in QEMU.

## Building dependencies
A non exhaustive list of dependencies needed to build the kernel (excluding the Rust toolchain) is given below:
- `qemu-system-x86_64
- `xorriso`
- `git`
- `make`

## License

Helium is dual-licensed under the Apache License, Version 2.0 and the MIT license.
See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
