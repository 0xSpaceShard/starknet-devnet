#!/bin/bash

set -eu

cargo +nightly-2025-02-20 fmt --all

# Format documentation
npm --prefix website run format
