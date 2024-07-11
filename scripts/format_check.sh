#!/bin/bash

set -euo pipefail

cargo +nightly-2024-07-08 fmt --all --check
cd website
npm run format-check
cd ..
