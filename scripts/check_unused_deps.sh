#!/bin/bash

set -eu

# should skip if already installed
cargo install cargo-machete

cargo machete
