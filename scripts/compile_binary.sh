#!/bin/bash

set -euo pipefail

CROSS_VERSION="v0.2.5"

if [ $# != 1 ]; then
    >&2 echo "Error: $0 <TARGET>"
    exit 1
fi
TARGET="$1"

kernel_name=$(uname -s)
case "$kernel_name" in
Darwin*)
    # on mac (for apple-darwin targets), rely on host compiler's targets
    rustup target add "$TARGET"
    compiler_command="cargo"
    ;;
Linux*)
    # on linux, rely on cross compiler
    download_url="https://github.com/cross-rs/cross/releases/download/${CROSS_VERSION}/cross-x86_64-unknown-linux-gnu.tar.gz"
    curl -SsL "$download_url" |
        tar -xvz -C /tmp
    compiler_command="/tmp/cross"
    ;;
*)
    >&2 echo "Unsupported kernel: $kernel_name"
    exit 1
    ;;
esac

"$compiler_command" build --release --target="$TARGET"
