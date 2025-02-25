#!/bin/bash

set -eu

# should skip if already installed
cargo install cargo-machete --version 0.7.0

cargo machete
