#!/bin/bash

set -euo pipefail

cd website && npm ci

if [ "$CIRCLECI" = "true" ]; then
    git config --global user.email "0xSpaceShard@users.noreply.github.com"
    git config --global user.name "0xSpaceShard"
fi

USE_SSH="true" npm run deploy
