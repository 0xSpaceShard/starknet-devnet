#!/bin/bash

set -eu

echo "Installing foundryup"
curl -L https://foundry.paradigm.xyz | bash || echo "As expected, received a non-zero exit code"

# make command available in PATH
export PATH="$PATH:$HOME/.foundry/bin"
if [ -n "$CIRCLE_BRANCH" ]; then
    # needed by further testing steps on CircleCI
    echo 'export PATH="$PATH:$HOME/.foundry/bin"' >>"$BASH_ENV"
fi

echo "Installing foundry"
foundryup --version  nightly-f460583ba4ea5fc7d3917eb071c544c1fbd3eadd

# assert it works
anvil --version
