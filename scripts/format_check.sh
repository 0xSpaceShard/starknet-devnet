#!/bin/bash

set -euo pipefail

cargo +nightly-2024-07-08 fmt --all --check

# Format documentation
npm --prefix website run format-check
