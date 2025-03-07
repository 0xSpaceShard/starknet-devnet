#!/bin/bash

set -euo pipefail

CROSS_VERSION="v0.2.5"

if [ $# != 1 ]; then
    echo >&2 "Error: $0 <TARGET>"
    exit 1
fi
TARGET="$1"

[[ "$TARGET" == *unknown-linux-musl ]] && apt-get install musl

arch_name=$(uname -m)
case "$arch_name" in
"x86_64")
    rustup target add "$TARGET"
    compiler_command="cargo"
    ;;
"aarch64")
    kernel_name=$(uname -s)
    case "$kernel_name" in
    Linux*)
        download_url="https://github.com/cross-rs/cross/releases/download/${CROSS_VERSION}/cross-x86_64-unknown-linux-gnu.tar.gz"
        curl -SsL "$download_url" | tar -xvz -C /tmp
        compiler_command="/tmp/cross"
        ;;
    Darwin*)
        rustup target add "$TARGET"
        compiler_command="cargo"
        ;;
    *)
        echo >&2 "Unsupported kernel: $kernel_name"
        exit 1
        ;;
    esac
    ;;
*)
    echo >&2 "Unsupported arch: $arch_name"
    exit 1
    ;;
esac

"$compiler_command" build --release --target="$TARGET" --bin starknet-devnet
