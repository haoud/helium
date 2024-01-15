TOOLCHAIN = dev-x86_64-unknown-helium
TARGET = x86_64-unknown-helium

build:
	cd init && cargo build
	cd shell && cargo +$(TOOLCHAIN) build --target $(TARGET)

build-release:
	cd init && cargo build --release
	cd shell && cargo +$(TOOLCHAIN) build --release --target $(TARGET)
	
install:
	cd init && ./install.sh
	cd shell && ./install.sh

fmt:
	cd iron && cargo fmt
	cd init && cargo fmt
	cd shell && cargo fmt
	cd syscall && cargo fmt

clean:
	cd iron && cargo clean
	cd init && cargo clean
	cd shell && cargo clean
	cd syscall && cargo clean
