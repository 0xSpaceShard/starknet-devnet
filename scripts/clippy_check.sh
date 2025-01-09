#!/bin/bash

set -eu

cargo clippy --workspace --all-targets -- -D warnings
