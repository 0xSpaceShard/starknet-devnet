#!/bin/bash

set -eu
set -o pipefail

echo "Checking if starknet contracts are compiled using the latest compiler..."

COMPILED=$(jq -r ".program.compiler_version" test/artifacts/contracts/cairo/contract.cairo/contract.json)
INSTALLED=$(poetry run starknet-compile --version | sed -rn "s/^starknet-compile (.*)$/\1/p")

if [ "$COMPILED" != "$INSTALLED" ]; then
    echo "Error: Compiled with version: $COMPILED, installed version: $INSTALLED"
    exit 1
fi
