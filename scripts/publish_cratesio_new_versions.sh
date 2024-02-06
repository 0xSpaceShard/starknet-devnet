#!/bin/bash

set -eu

cargo install cargo-get
workspace_members=$(cargo get --delimiter ";" workspace.members)
current_dir="$(pwd)"

cargo login $CRATES_IO_API_KEY

IFS=';' read -a array <<< "$workspace_members"

for workspace_member in "${array[@]}" 
do
    crate_dir="$current_dir/$workspace_member";

    package_name=$(cargo get --entry $crate_dir package.name)
    package_version=$(cargo get --entry $crate_dir package.version)
    cratesio_version=$(cargo search $package_name --limit 1 | head -n 1 | sed -n 's/'$package_name' = "\([^"]*\)".*/\1/p');

    # if local version is different from crates.io version, then publish
    if [ $package_version != $cratesio_version ]; then
        echo "Local version of $package_name is $package_version, while the one on crates.io is $cratesio_version";
        # Publish to crates.io
        cargo publish -p $package_name
    fi

done