#!/bin/bash

set -eu

echo "Installing cross compiler for Rust"

CROSS_VERSION="v0.2.5"

kernel_name=$(uname -s)
case "$kernel_name" in
Darwin*)
    cargo install cross --git https://github.com/cross-rs/cross
    ;;
Linux*)
    url="https://github.com/cross-rs/cross/releases/download/${CROSS_VERSION}/cross-x86_64-unknown-linux-gnu.tar.gz"
    curl -SsL "$download_url" |
        tar -xvz -C ~/.cargo/bin
    ;;
*)
    echo "Unsupported kernel: $kernel_name"
    exit 1
    ;;
esac
