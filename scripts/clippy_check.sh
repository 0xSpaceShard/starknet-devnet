#!/bin/bash

set -eu

cargo clippy --all -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
cargo clippy --tests -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
