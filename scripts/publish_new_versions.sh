#!/bin/bash

set -eu

cargo install cargo-get
workspace_members=$(cargo get --delimiter ";" workspace.members)
current_dir="$(pwd)"

IFS=';' read -a array <<< "$workspace_members"

for workspace_member in "${array[@]}" 
do
    crate_dir="$current_dir/$workspace_member";

    package_name=$(cargo get --entry $crate_dir package.name)
    package_version=$(cargo get --entry $crate_dir package.version)
    cratesio_version=$(cargo search $package_name --limit 1 | head -n 1 | sed -n 's/'$package_name' = "\([^"]*\)".*/\1/p');

    # if local version is different from crates.io version, then publish
    if [ $cratesio_version != $package_version ]; then
        echo "Local version of $package_name is $package_version, while the one on crates.io is $cratesio_version";
        # Publish to crates.io
        cargo publish -p $package_name
    fi

done

# for workspace_member in $workspace_members
# do
#     cargo get --entry "$current_dir/$workspace_member" package.name
#     crate_dir="${current_dir}/${workspace_member}";
#     if [ -d "$crate_dir" ]; then
#         echo "Directory $crate_dir exists"
#     elif [ -f "$crate_dir" ]; then
#         echo "File $crate_dir exists"
#     else
#         echo "File or directory $crate_dir does not exist"
#     fi
#     #cargo get --entry="${crate_dir}" package.name
#     # if [ -d "$crate_dir" ]; then
#     #     package_name=$(cargo get --entry "$crate_dir" package.name)
#     #     echo "Package name: $package_name"
#     # else
#     #     echo "Directory $crate_dir does not exist"
#     # fi

#     #echo "Checking if ${crate_name} version ${crate_version} is newer than the one on crates.io"
# done
# # cargo search starknet-devnet --limit 1 | head -n 1 | sed -n 's/starknet-devnet = "\([^"]*\)".*/\1/p'