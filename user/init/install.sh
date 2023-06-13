#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./Cargo.toml ]   \
    || die "You must run this script from the root of the repository"

# Verify if debug and release builds coexist. If so, copy the most recent one: it is probably
# the one that the user wants to run.
if [ -e target/x86_64/debug/init ] && [ -e target/x86_64/release/init ]; then
    if [ target/x86_64/debug/init -nt target/x86_64/release/init ]; then
        cp -v target/x86_64/debug/init ../../iso/boot/init.elf
    else
        cp -v target/x86_64/release/init ../../iso/boot/init.elf
    fi
elif [ -e target/x86_64/debug/init ]; then
    cp -v target/x86_64/debug/init ../../iso/boot/init.elf
elif [ -e target/x86_64/release/init ]; then
    cp -v target/x86_64/release/init ../../iso/boot/init.elf
else
    die "No init executable found"
fi
