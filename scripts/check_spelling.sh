#!/bin/bash

set -eu

# should skip if already installed
cargo +nightly-2024-07-08 install typos-cli

typos
