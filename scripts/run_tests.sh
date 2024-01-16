#!/bin/sh
die() {
    echo "error: $@" >&2
    exit 1
}

[ -e ./README.md ]   \
    || die "you must run this script from the root of the repository"

qemu-system-x86_64 -m 128                                   \
    -drive format=raw,media=cdrom,file=bin/helium.iso       \
    -device isa-debug-exit                                  \
    -rtc base=localtime                                     \
    -no-reboot                                              \
    -nographic                                              \
    -cpu max                                                \
    -smp 2                                                  \

code=$?
if [ $code -eq 3 ]; then
    exit 0
else
    exit $code
fi
