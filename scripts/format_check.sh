#!/bin/bash

set -euo pipefail

cargo +nightly-2025-02-20 fmt --all --check

# Format documentation
npm --prefix website run format-check
