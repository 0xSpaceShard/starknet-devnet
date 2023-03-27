#!/bin/bash
set -eu

[ -f .env ] && source .env

PYPI_VERSIONS=$(curl -Ls https://pypi.org/pypi/starknet-devnet/json | jq '.releases|keys')
echo "PyPI versions: $PYPI_VERSIONS"

LOCAL_VERSION=$(./scripts/get_version.sh version)
echo "Local version: $LOCAL_VERSION"

# locate the local version inside the keys array; "null" if not present
LOCAL_VERSION_INDEX=$(echo "$PYPI_VERSIONS" | jq "index( \"$LOCAL_VERSION\" )")
echo "Index of local version in PyPI versions: $LOCAL_VERSION_INDEX"

# Building is executed regardles of versions
poetry build

if [ "$LOCAL_VERSION_INDEX" != "null" ]; then
    echo "Latest PyPI version is already equal to the local version."
    echo "Publishing skipped"
else
    # publish Python package
    poetry publish --username "$PYPI_USER" --password "$PYPI_PASS"

    # publish docs
    cd page && npm ci

    git config --global user.email "$GIT_USER@users.noreply.github.com"
    git config --global user.name "$GIT_USER"
    echo "machine github.com login $GIT_USER password $GITHUB_TOKEN" >~/.netrc

    # [skip ci] to avoid gh-pages erroring on circleci
    LATEST_COMMIT_HASH=$(git rev-parse HEAD)
    CUSTOM_COMMIT_MESSAGE="Deploy website - based on $LATEST_COMMIT_HASH [skip ci]" \
        npm run deploy
fi
