#!/bin/bash

set -eu

cargo +nightly-2024-07-08 fmt --all

# Format documentation code
cd website
npm run format
cd ..
