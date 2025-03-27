#!/bin/bash

set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo >&2 "Error: $0 <TARGET>"
    exit 1
fi
TARGET="$1"

CARGO_CONFIG=~/.cargo/config.toml

case "$TARGET" in
x86_64-unknown-linux-gnu | x86_64-apple-darwin | aarch64-apple-darwin)
    echo "Target requires no extra actions: $TARGET"
    ;;

x86_64-unknown-linux-musl)
    sudo apt-get update
    sudo apt-get install musl-tools
    musl-gcc --version && echo "Musl successfully installed"
    ;;

aarch64-unknown-linux-gnu)
    sudo apt-get update
    sudo apt-get install gcc-aarch64-linux-gnu

    aarch64-linux-gnu-gcc --version
    echo "Cross compiler successfully installed"

    echo '[target.aarch64-unknown-linux-gnu]' >>"$CARGO_CONFIG"
    echo 'linker = "aarch64-linux-gnu-gcc"' >>"$CARGO_CONFIG"
    ;;

*)
    echo >&2 "Error: Unsupported compilation target: $TARGET"
    exit 2
    ;;
esac

rustup target add "$TARGET"
cargo build --release --target="$TARGET" --bin starknet-devnet
