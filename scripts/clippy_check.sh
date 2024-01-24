#!/bin/bash

set -eu

cargo clippy --all -- -D warnings
cargo clippy --tests -- -D warnings
