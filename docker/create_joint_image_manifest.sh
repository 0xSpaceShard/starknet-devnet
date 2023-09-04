#!/bin/bash
set -eu

echo "Creating a joint docker manifest for each pair of -arm and -amd images."

IMAGE=shardlabs/starknet-devnet-rs

docker login --username "$DOCKER_USER" --password "$DOCKER_PASS"

# TODO
echo "Temporarily pushing tag latest. Once semver is established for this project, this should be done conditionally (if version incremented), as with devnet-py"

for seed_suffix in "" "-seed0"; do
    # Pull the pair
    for image_suffix in "-arm" "-amd"; do
        docker pull "$IMAGE:${CIRCLE_SHA1}${image_suffix}${seed_suffix}"
    done

    # Create and push the joint manifest
    for manifest_prefix in "$CIRCLE_SHA1" "latest"; do
        joint_manifest="$IMAGE:${manifest_prefix}${seed_suffix}"

        docker manifest create $joint_manifest \
            "$IMAGE:${CIRCLE_SHA1}-arm${seed_suffix}" \
            "$IMAGE:${CIRCLE_SHA1}-amd${seed_suffix}"

        docker manifest push $joint_manifest
    done
done
