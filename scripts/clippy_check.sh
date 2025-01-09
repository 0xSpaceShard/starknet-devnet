#!/bin/bash

set -eu

# checks for errors in production code
cargo clippy --workspace -- -Dwarnings
# checks for errors in production code with enabled all features
cargo clippy --workspace --all-features -- -Dwarnings
# checks for errors in testing code
cargo clippy --workspace --tests -- -Dwarnings
