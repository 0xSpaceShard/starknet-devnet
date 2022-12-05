#!/bin/bash

set -eu
set -o pipefail

echo "Checking if starknet contracts are compiled using the latest compiler..."

# Use contract.cairo to test
COMPILED=$(jq -r ".program.compiler_version" test/artifacts/contracts/cairo/contract.cairo/contract.json)
echo "Compiled with: $COMPILED"
INSTALLED=$(poetry run starknet-compile --version | sed -rn "s/^starknet-compile (.*)$/\1/p")
echo "Installed: $INSTALLED"

if [ "$COMPILED" != "$INSTALLED" ]; then
    echo "Error: Compiled with version: $COMPILED, installed version: $INSTALLED"
    exit 1
fi
