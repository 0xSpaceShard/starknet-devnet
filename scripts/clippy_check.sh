#!/bin/bash

set -eu

# `cargo clippy --workspace --all-targets` dont catch clippy errors for unknown to me reason.
# The workaround is to check for production code, production code with all conditional compilation flags enabled and finally for testing code.

# checks for errors in production code
cargo clippy --workspace -- -Dwarnings
# checks for errors in production code with enabled all features
cargo clippy --workspace --all-features -- -Dwarnings
# checks for errors in testing code
cargo clippy --workspace --tests -- -Dwarnings
