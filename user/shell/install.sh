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
if [ -e target/x86_64-unknown-helium/debug/shell ] && [ -e target/x86_64-unknown-helium/release/shell ]; then
    if [ target/x86_64-unknown-helium/debug/shell -nt target/x86_64-unknown-helium/release/shell ]; then
        cp -v target/x86_64-unknown-helium/debug/shell ../../iso/boot/shell.elf
    else
        cp -v target/x86_64-unknown-helium/release/shell ../../iso/boot/shell.elf
    fi
elif [ -e target/x86_64-unknown-helium/debug/shell ]; then
    cp -v target/x86_64-unknown-helium/debug/shell ../../iso/boot/shell.elf
elif [ -e target/x86_64-unknown-helium/release/shell ]; then
    cp -v target/x86_64-unknown-helium/release/shell ../../iso/boot/shell.elf
else
    die "No shell executable found"
fi
