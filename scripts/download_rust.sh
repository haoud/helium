#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "you must run this script from the root of the repository"

cd bin/src
git clone --depth 1 --branch 1.73.0 https://github.com/rust-lang/rust.git

cd rust
git submodule update --depth 1 --init src/llvm-project
git submodule update --depth 1 --init src/tools/cargo
git submodule update --depth 1 --init library/backtrace
git submodule update --depth 1 --init library/stdarch

