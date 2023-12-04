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
foundryup
anvil --version
