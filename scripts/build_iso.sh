#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "You must run this script from the root of the repository"

# Check that limine is installed and build it if necessary
if [ ! -e bin/src/limine/limine-uefi-cd.bin ] || 
   [ ! -e bin/src/limine/limine-bios-cd.bin ] ||
   [ ! -e bin/src/limine/limine-bios.sys ]; then
    echo "Limine is not installed. Downloading and building it..."
    ./scripts/build_limine.sh
fi

# Copy the limine bootloader inside the ISO directory
cp -v                                   \
    bin/src/limine/limine-uefi-cd.bin   \
    bin/src/limine/limine-bios-cd.bin   \
    bin/src/limine/limine-bios.sys      \
    iso/boot/

# Install the kernel
# Verify if debug and release builds coexist. If so, copy the most recent one: it is probably
# the one that the user wants to run.
if [ -e kernel/target/x86_64/debug/kernel ] && [ -e kernel/target/x86_64/release/kernel ]; then
    if [ kernel/target/x86_64/debug/kernel -nt kernel/target/x86_64/release/kernel ]; then
        cp -v kernel/target/x86_64/debug/kernel iso/boot/helium.elf
    else
        cp -v kernel/target/x86_64/release/kernel iso/boot/helium.elf
    fi
elif [ -e kernel/target/x86_64/debug/kernel ]; then
    cp -v kernel/target/x86_64/debug/kernel iso/boot/helium.elf
elif [ -e kernel/target/x86_64/release/kernel ]; then
    cp -v kernel/target/x86_64/release/kernel iso/boot/helium.elf
else
    die "No kernel executable found"
fi

# Create the ISO
xorriso -as mkisofs -b boot/limine-bios-cd.bin			  \
		-no-emul-boot -boot-load-size 4 -boot-info-table 	\
		--efi-boot boot/limine-uefi-cd.bin 					      \
		-efi-boot-part --efi-boot-image  					        \
		--protective-msdos-label iso -o bin/helium.iso

# Deploy Limine to the ISO
./bin/src/limine/limine bios-install bin/helium.iso
