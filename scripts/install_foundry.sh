#!/bin/bash

# If foundryup becomes buggy, consider switching to directly downloading binary from GitHub tag:
# git_asset="https://github.com/foundry-rs/foundry/releases/download/$foundry_version/foundry_nightly_linux_amd64.tar.gz"
# foundry_bin="$HOME/.foundry/bin"
# mkdir -p "$foundry_bin"
# curl -L "$git_asset" | tar -xvz -C "$foundry_bin"
# export PATH="$PATH:$HOME/$foundry_bin"

set -eu

foundry_version="nightly-5b7e4cb3c882b28f3c32ba580de27ce7381f415a"

echo "Installing foundryup $foundry_version"

curl -L https://foundry.paradigm.xyz | bash || echo "As expected, received a non-zero exit code"

# make command available in PATH
export PATH="$PATH:$HOME/.foundry/bin"
if [ -n "$CIRCLE_BRANCH" ]; then
    # needed by further testing steps on CircleCI
    echo 'export PATH="$PATH:$HOME/.foundry/bin"' >>"$BASH_ENV"
fi

echo "Installing foundry"
foundryup --install "$foundry_version"

# assert it works
anvil --version
