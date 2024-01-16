#!/bin/sh
set -e
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "you must run this script from the root of the repository"

qemu-system-x86_64 -daemonize -s -S -m 128                  \
    -drive format=raw,media=cdrom,file=bin/helium.iso       \
    -rtc base=localtime                                     \
    -no-reboot                                              \
    -no-shutdown                                            \
    -serial file:serial.log                                 \
    -cpu max                                                \
    -smp 2                                                  
