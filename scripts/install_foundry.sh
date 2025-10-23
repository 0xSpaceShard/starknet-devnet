#!/bin/bash

# If foundryup becomes buggy, consider switching to directly downloading binary from GitHub tag:
# git_asset="https://github.com/foundry-rs/foundry/releases/download/$foundry_version/foundry_nightly_linux_amd64.tar.gz"
# foundry_bin="$HOME/.foundry/bin"
# mkdir -p "$foundry_bin"
# curl -L "$git_asset" | tar -xvz -C "$foundry_bin"
# export PATH="$PATH:$HOME/$foundry_bin"

set -eu

foundry_version="nightly-5b7e4cb3c882b28f3c32ba580de27ce7381f415a"

echo "Installing foundry $foundry_version"

echo "Installing foundryup"
curl -L https://foundry.paradigm.xyz | bash || echo "As expected, received a non-zero exit code"

# Source the profile to make foundryup available in current shell
if [ -f "$HOME/.bashrc" ]; then
    source "$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then
    source "$HOME/.bash_profile"
elif [ -f "$HOME/.zshrc" ]; then
    source "$HOME/.zshrc"
fi

# make command available in PATH
export PATH="$PATH:$HOME/.foundry/bin"
if [ -n "${CIRCLE_BRANCH:-}" ]; then
    # needed by further testing steps on CircleCI
    echo 'export PATH="$PATH:$HOME/.foundry/bin"' >>"$BASH_ENV"
elif [ -n "${GITHUB_ACTIONS:-}" ]; then
    # needed by further testing steps on GitHub Actions
    echo 'PATH="$PATH:$HOME/.foundry/bin"' >> $GITHUB_ENV
fi

echo "Installing foundry"
# Try to use foundryup, but fall back to direct installation if not available
if command -v foundryup &> /dev/null; then
    foundryup --install "$foundry_version"
else
    echo "foundryup not found in PATH, attempting direct installation..."
    # Direct installation method
    FOUNDRY_DIR="$HOME/.foundry"
    BIN_DIR="$FOUNDRY_DIR/bin"
    mkdir -p "$BIN_DIR"
    
    # Download and extract binaries directly from GitHub
    PLATFORM="linux_amd64"
    if [[ "$(uname)" == "Darwin" ]]; then
        PLATFORM="macos_amd64"
    fi
    
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"
    
    # Use the foundry version from above
    URL="https://github.com/foundry-rs/foundry/releases/download/$foundry_version/foundry_${foundry_version#nightly-}_$PLATFORM.tar.gz"
    echo "Downloading from $URL"
    curl -L "$URL" | tar -xz
    
    # Move binaries to foundry bin dir
    mv anvil cast chisel forge "$BIN_DIR/"
    chmod +x "$BIN_DIR"/*
    
    # Clean up
    cd -
    rm -rf "$TEMP_DIR"
    
    # Update PATH to include foundry binaries
    export PATH="$PATH:$BIN_DIR"
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        echo "PATH=$PATH:$BIN_DIR" >> $GITHUB_ENV
    fi
fi

# assert it works
echo "Verifying installation:"
if command -v anvil &> /dev/null; then
    anvil --version
else
    echo "ERROR: anvil not found in PATH after installation"
    echo "PATH=$PATH"
    exit 1
fi
