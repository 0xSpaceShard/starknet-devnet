#!/bin/bash

set -eu

if [ $# -ne 1 ]; then
    >&2 echo "$0: <VERSION>"
    exit 1
fi

version="$1"

echo "The crates that need a semver increment since git revision '$version' are:"

git diff "$version" --name-status | grep -o -E 'crates/[^/]*' | uniq

echo "Note that this does not reflect dependency changes in Cargo.toml or changes that one Devnet crate may have had on another!"
