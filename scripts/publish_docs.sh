#!/bin/bash

set -euo pipefail

cd website && npm ci

user="0xSpaceShard"
git config --global user.email "$user@users.noreply.github.com"
git config --global user.name "$user"

# [skip ci] to avoid deployment branch erroring on CircleCI
LATEST_COMMIT_HASH=$(git rev-parse HEAD)
export CUSTOM_COMMIT_MESSAGE="Deploy website - based on $LATEST_COMMIT_HASH [skip ci]"
export USE_SSH="true"
npm run deploy
