#!/bin/bash

set -euo pipefail

cargo +nightly-2025-10-25 fmt --all --check

# Format documentation
npm --prefix website run format-check
