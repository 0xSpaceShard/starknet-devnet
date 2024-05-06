#!/bin/bash

set -eu

# should skip if already installed
cargo +nightly install typos-cli

typos
