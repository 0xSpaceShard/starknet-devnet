#!/bin/bash

set -euo pipefail

cargo install cargo-get --version 1.1.0 --locked

for workspace_member in $(cargo get --delimiter " " workspace.members); do
    package_name=$(cargo get --entry "$workspace_member" package.name)
    package_version=$(cargo get --entry "$workspace_member" package.version)
    cratesio_version=$(cargo search "$package_name" | sed -n 's/'$package_name' = "\([^"]*\)".*/\1/p')

    # if local version is different from crates.io version, then publish to crates.io
    if [ "$package_version" != "$cratesio_version" ]; then
        echo "Local version of $package_name is $package_version, while the one on crates.io is $cratesio_version"

        cargo login "$CRATES_IO_API_KEY"
        cargo publish -p "$package_name"
    else
        echo "$package_name v$package_version already published"
    fi
done
