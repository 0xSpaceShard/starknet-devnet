#!/bin/bash

set -eu

cargo +nightly-2024-07-08 fmt --all

# Format documentation
npm --prefix website run format
