#!/bin/bash

set -eu

cargo clippy --all --tests -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
