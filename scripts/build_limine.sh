#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "you must run this script from the root of the repository"

mkdir -p bin
mkdir -p bin/src
cd bin/src
echo "Downloading limine..."
git clone https://github.com/limine-bootloader/limine.git \
    --branch=v7.0.5-binary \
    --depth=1
echo "Building limine..."
make -C limine
