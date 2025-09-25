#!/bin/bash

set -euo pipefail

# should skip if already installed
cargo +nightly-2025-02-20 install typos-cli --version 1.31.0 --locked

typos && echo "No spelling errors!"
