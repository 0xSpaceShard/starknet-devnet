#!/bin/bash

set -eu

cargo +nightly fmt --all

# Format documentation code
cd website
npm run format
cd ..