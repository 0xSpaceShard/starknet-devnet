#!/bin/bash

# Cache is available only on our custom runner
if [ "$CIRCLECI_RUNNER_CLASS" != "spaceshard/ax41" ]; then
    echo "Custom cache available only on self-hosted spaceshard/ax41 runner"
    exit 0
fi

if [ $# -eq 0 ]; then
    echo "Please provide a cache action argument. (save, load or cleanup)"
    exit 1
fi

action="$1"

# VARIABLES
cache_key_files=("Cargo.lock" "rust-toolchain.toml")
cached_dirs_array=("target/release/.fingerprint target/release/build target/release/deps")


cache_base_dir="/cache"
cache_key=$(for file in "${cache_key_files[@]}"; do shasum "$file" | cut -c 1-10; done | paste  -sd "-" -)
cached_dirs=$(for dir in "${cached_dirs_array[@]}"; do echo $dir ; done | paste  -sd " " -)

cache_file="$cache_base_dir/$cache_key.tar.gz"
cache_cleanup_interval=7


case "$action" in
    "load")
        echo "Loading cache..."
        if [ ! -f "$cache_file" ]; then
            echo "Cache does not exist."
            exit 0
        fi
        tar xvf "$cache_file"
        ;;
    "save")
        echo "Creating cache..."
        if [ -f "$cache_file" ]; then
            echo "Cache already exists."
            exit 0
        fi
        tar czvf "$cache_file".tmp $cached_dirs # Create a temporary cache file for atomicity
        mv czvf "$cache_file".tmp "$cache_file"
        ;;
    "cleanup")
        echo "Cleaning up cache..."
        rm -f "$cache_base_dir"/*.tmp # Remove temporary cache files if they are leftover
        find "$cache_base_dir" -atime "+$cache_cleanup_interval" -name '*.gz' -exec rm {} \;
        exit 0
        ;;
    *)
        echo "Invalid action argument."
        exit 1
        ;;
esac