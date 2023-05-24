#!/bin/sh
set -e

# Used exclusively by cargo, not intended to be run manually
# This script allow us to run our kernel with cargo run as if it was a normal binary
./scripts/build_iso.sh

# Check the return code of the previous command. If it's 0, then the ISO was
# successfully built and we can run it. Otherwise, we exit with the same return
# code as the previous command.
if [ $? -ne 0 ]; then
    exit $?
fi

./scripts/run.sh
