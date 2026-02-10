#!/bin/bash

set -eu

cargo +nightly-2025-10-25 fmt --all

# Format documentation
npm --prefix website run format
