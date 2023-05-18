#!/bin/bash

set -e

echo "npm: $(npm --version)"
echo "npm: $(node --version)"
echo "pip: $(pip --version)"
echo "pip3: $(pip3 --version)"
echo "python: $(python --version)"
echo "python3: $(python3 --version)"

./scripts/install_poetry.sh
echo "poetry: $(poetry --version)"

# https://www.rust-lang.org/tools/install
# need rust to install cairo-rs-py
if rustc --version; then
    echo "rustc installed"
else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

# setup cairo1 compiler
if [ -z "$CAIRO_1_COMPILER_MANIFEST" ]; then
    mkdir cairo-compiler
    git clone git@github.com:starkware-libs/cairo.git cairo-compiler \
        --branch v1.0.0-rc0 \
        --single-branch
    CAIRO_1_COMPILER_MANIFEST="cairo-compiler/Cargo.toml"

    if [ -n "$CIRCLE_BRANCH" ]; then
        # needed by further testing steps
        echo "export CAIRO_1_COMPILER_MANIFEST=$CAIRO_1_COMPILER_MANIFEST" >>"$BASH_ENV"
        echo "source ~/.cargo/env" >>"$BASH_ENV"
    fi
fi

echo "Using Cairo compiler at $CAIRO_1_COMPILER_MANIFEST"
cargo build \
    --bin starknet-compile \
    --bin starknet-sierra-compile \
    --manifest-path "$CAIRO_1_COMPILER_MANIFEST"

# install dependencies
poetry install --no-ansi
poetry lock --check
npm ci
