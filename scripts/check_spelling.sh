#!/bin/bash

set -euo pipefail

# should skip if already installed
cargo +nightly-2025-02-20 install typos-cli --version 1.36.2 --locked

typos && echo "No spelling errors!"
