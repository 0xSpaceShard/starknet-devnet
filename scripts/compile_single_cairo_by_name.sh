#!/bin/bash

set -eu

CONTRACT=$1

mkdir -p "test/artifacts/contracts/cairo/$CONTRACT.cairo"

poetry run starknet-compile-deprecated \
    --disable_hint_validation test/contracts/cairo/$CONTRACT.cairo \
    --abi test/artifacts/contracts/cairo/$CONTRACT.cairo/${CONTRACT}_abi.json \
    --output test/artifacts/contracts/cairo/$CONTRACT.cairo/${CONTRACT}.json
