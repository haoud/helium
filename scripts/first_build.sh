#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "you must run this script from the root of the repository"

./scripts/download_rust.sh
./scripts/patch_rust.sh
./scripts/build_rust.sh
./scripts/add_user_toolchain.sh
./scripts/build_limine.sh
