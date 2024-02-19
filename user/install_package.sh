#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

# Verify that at least one argument was provided
[ $# -ge 1 ] || die "Usage: $0 <package name>"

# Verify if debug and release builds coexist. If so, copy the most recent one
if [ -e $1/target/x86_64/debug/$1 ] && [ -e $1/target/x86_64/release/$1 ]; then
    if [ $1/target/x86_64/debug/$1 -nt $1/target/x86_64/release/$1 ]; then
        cp -v $1/target/x86_64/debug/$1 ../iso/boot/$1.elf
    else
        cp -v $1/target/x86_64/release/$1 ../iso/boot/$1.elf
    fi
elif [ -e $1/target/x86_64/debug/$1 ]; then
    cp -v $1/target/x86_64/debug/$1 ../iso/boot/$1.elf
elif [ -e $1/target/x86_64/release/$1 ]; then
    cp -v $1/target/x86_64/release/$1 ../iso/boot/$1.elf
else
    die "$1 package executable not found"
fi

