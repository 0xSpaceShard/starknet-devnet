#!/bin/bash
set -eu

IMAGE=shardlabs/starknet-devnet-rs

cargo install cargo-get --version 1.1.0 --locked

echo "Logging in to docker hub"
docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

# Get the version of the binary crate
bin_crate_version=$(cargo get --entry crates/starknet-devnet package.version)

function image_exists() {
    docker manifest inspect "$1" >/dev/null 2>&1
}

function create_and_push_manifest() {
    local manifest_prefix="$1"
    local seed_suffix="$2"

    local joint_manifest="$IMAGE:${manifest_prefix}${seed_suffix}"

    docker manifest create $joint_manifest \
        "$IMAGE:${CIRCLE_SHA1}-aarch64${seed_suffix}" \
        "$IMAGE:${CIRCLE_SHA1}-x86_64${seed_suffix}"

    docker manifest push "$joint_manifest"
}

echo "Creating a joint docker manifest for each pair of -aarch64 and -x86_64 images."

# construct the image tag from the version
# check if the image tag exists in docker registry
# If it does, do not publish the version
for seed_suffix in "" "-seed0"; do
    # Pull the pair
    # and tag the image with the crate version if not done yet
    for image_suffix in "-aarch64" "-x86_64"; do
        image_tag_with_commit_hash="$IMAGE:${CIRCLE_SHA1}${image_suffix}${seed_suffix}"
        docker pull "$image_tag_with_commit_hash"

        image_tag_with_version="$IMAGE:${bin_crate_version}${image_suffix}${seed_suffix}"

        if image_exists $image_tag_with_version; then
            echo "image: ($image_tag_with_version) already exists"
        else
            echo "image: ($image_tag_with_version) does not exist"
            docker tag "$image_tag_with_commit_hash" "$image_tag_with_version"
            docker push "$image_tag_with_version"
        fi
    done

    # Create and push the joint manifest
    create_and_push_manifest "$CIRCLE_SHA1" "$seed_suffix"

    image_manifest_with_version="$IMAGE:${bin_crate_version}${seed_suffix}"

    if image_exists $image_manifest_with_version; then
        echo "manifest: ($image_manifest_with_version) already exists"
    else
        echo "manifest: ($image_manifest_with_version) does not exist"
        create_and_push_manifest "$bin_crate_version" "$seed_suffix"

        echo "Creating and pushing the joint manifest with the latest tag"
        create_and_push_manifest "latest" "$seed_suffix"
    fi
done
