build:
	cd init && cargo build
	cd shell && cargo build

build-release:
	cd init && cargo build --release
	cd shell && cargo build --release
	
install:
	cd init && ./install.sh
	cd shell && ./install.sh

fmt:
	cd iron && cargo fmt
	cd init && cargo fmt
	cd shell && cargo fmt

clean:
	cd iron && cargo clean
	cd init && cargo clean
	cd shell && cargo clean
