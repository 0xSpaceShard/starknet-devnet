#!/bin/bash

set -eu

cargo clippy --all --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
