#!/bin/bash

set -euo pipefail

cargo install cargo-get --version 1.1.0 --locked

for workspace_member in $(cargo get --delimiter " " workspace.members); do
    package_name=$(cargo get --entry "$workspace_member" package.name)
    if [ $package_name = "integration" ]; then
        continue
    fi

    package_version=$(cargo get --entry "$workspace_member" package.version)

    # if local version not present on crates.io, publish it
    crates_io_url="https://crates.io/api/v1/crates/$package_name"
    if ! curl -sSLf "$crates_io_url" | jq -r '.versions[].num' | grep -q "^$package_version$"; then
        echo "The local version of $package_name is $package_version, which is not present on crates.io"

        cargo login "$CRATES_IO_API_KEY"
        cargo publish -p "$package_name"
    else
        echo "$package_name v$package_version already published"
    fi
done
