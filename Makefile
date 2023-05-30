# Build the kernel
build:
	cd kernel && cargo build

# Build the kernel in release mode
build-release:
	cd kernel && cargo build --release

# Build and run the last built kernel in Qemu
run: build
	./scripts/runner.sh

# Build and run the last built kernel in Qemu in release mode
run-release: build-release
	./scripts/runner.sh

# Run the last built kernel in Qemu and run the tests
run-test: 
	cd kernel && cargo build --features test
	./scripts/build_iso.sh
	./scripts/run_tests.sh

# Format the code using rustfmt
fmt:
	cd kernel/crates/helium-macros && cargo fmt
	cd kernel/crates/helium-x86_64 && cargo fmt
	cd kernel/crates/helium-utils && cargo fmt
	cd kernel/crates/helium-addr && cargo fmt
	cd kernel/crates/helium-sync && cargo fmt
	cd kernel/crates/helium-user && cargo fmt
	cd kernel/crates/helium-mm && cargo fmt
	cd kernel && cargo fmt

# Clean the build artifacts
clean:
	cd kernel && cargo clean
